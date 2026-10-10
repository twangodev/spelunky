//! Treasure and pickup effects translated from Classic 1.1's collectible Create
//! events, oPlayer1 Step, and scrStealItem. Ordinary item motion belongs to items.
use crate::model::{Area, Entity, Game, GameEvent, InputFrame, Rect, Vec2};

pub(super) fn treasure_value(object: &str) -> Option<u32> {
    Some(match object {
        "oGoldChunk" => 100,
        "oGoldNugget" | "oGoldBar" => 500,
        "oGoldBars" => 1000,
        "oEmerald" => 200,
        "oSapphire" | "oRuby" => 400,
        "oEmeraldBig" => 800,
        "oSapphireBig" => 1200,
        "oRubyBig" => 1600,
        "oDiamond" => 5000,
        _ => return None,
    })
}

fn equipment(object: &str) -> Option<&'static str> {
    Some(match object {
        "oParaPickup" => "Parachute",
        "oCapePickup" => "Cape",
        "oJetpack" => "Jetpack",
        "oSpringShoes" => "SpringShoes",
        "oSpikeShoes" => "SpikeShoes",
        "oGloves" => "Gloves",
        "oMitt" => "Mitt",
        "oCompass" => "Compass",
        "oSpectacles" => "Spectacles",
        "oPaste" => "Paste",
        "oUdjatEye" => "UdjatEye",
        "oAnkh" => "Ankh",
        "oCrown" => "Crown",
        "oKapala" => "Kapala",
        "oJordans" => "Jordans",
        _ => return None,
    })
}

pub fn is_collectible(object: &str) -> bool {
    treasure_value(object).is_some()
        || equipment(object).is_some()
        || matches!(
            object,
            "oBombBag"
                | "oBombBox"
                | "oRopePile"
                | "oCrate"
                | "oChest"
                | "oLockedChest"
                | "oKey"
                | "oFlareCrate"
        )
}

/// Initialize source collision bounds and prices without overwriting editor or
/// shop overrides. A positive cost alone is not a sale marker in this port.
pub fn init_entity(entity: &mut Entity) {
    if !is_collectible(&entity.object) {
        return;
    }
    if entity.value("collectibles_initialized") > 0. {
        return;
    }
    entity.data.insert("collectibles_initialized".into(), 1.);
    if entity.object == "oFlareCrate" {
        entity.data.entry("flare_spark_timer".into()).or_insert(1.);
    }
    let (left, top, right, bottom) = match entity.object.as_str() {
        "oGoldChunk" | "oEmerald" | "oSapphire" | "oRuby" => (-2., -2., 2., 2.),
        "oGoldBars" => (-7., -8., 7., 8.),
        "oGoldNugget" | "oGoldBar" | "oEmeraldBig" | "oSapphireBig" | "oRubyBig" | "oDiamond"
        | "oKey" => (-4., -4., 4., 4.),
        "oCrate" | "oChest" | "oFlareCrate" => (-6., 0., 6., 8.),
        "oLockedChest" | "oBombBox" => (-6., -2., 6., 8.),
        "oBombBag" | "oPaste" => (-6., -2., 6., 6.),
        "oRopePile" => (-6., -5., 6., 5.),
        "oJetpack" => (-5., -5., 5., 8.),
        "oAnkh" => (-4., -6., 4., 8.),
        "oGloves" | "oMitt" | "oCrown" | "oKapala" => (-6., -6., 6., 8.),
        _ => (-6., -6., 6., 6.),
    };
    for (key, value) in [
        ("bboxLeft", left),
        ("bboxTop", top),
        ("bboxRight", right),
        ("bboxBottom", bottom),
    ] {
        entity.data.entry(key.into()).or_insert(value);
    }
    if let Some(value) = treasure_value(&entity.object) {
        // oTreasure Create marks its independent movement engine ACTIVE.
        entity.state = 1;
        entity
            .data
            .entry("value".into())
            .or_insert(f64::from(value));
        // oTreasure inherits oDrawnSprite, not oItem; it cannot be carried.
        entity.data.insert("canPickUp".into(), 0.);
    }
    let cost = match entity.object.as_str() {
        "oBombBag" | "oRopePile" => 2500.,
        "oBombBox" => 10000.,
        "oParaPickup" => 2000.,
        "oCompass" | "oPaste" => 3000.,
        "oCapePickup" => 12000.,
        "oJetpack" => 20000.,
        "oAnkh" => 50000.,
        "oGloves" | "oSpectacles" => 8000.,
        "oMitt" | "oSpikeShoes" => 4000.,
        "oSpringShoes" => 5000.,
        "oJordans" => 50000.,
        "oCrown" | "oKapala" => 999999.,
        _ => 0.,
    };
    entity.data.entry("cost".into()).or_insert(cost);
    if matches!(
        entity.object.as_str(),
        "oCrate" | "oChest" | "oFlareCrate" | "oLockedChest" | "oBombBox" | "oJetpack"
    ) {
        entity.data.entry("heavy".into()).or_insert(1.);
    }
}

fn bounds(entity: &Entity) -> Rect {
    // GM collision_point/rectangle and collision events use sprite masks;
    // setCollisionBounds supplies the separate movement-engine rectangle.
    entity.sprite_bounds()
}

fn origin_is_solid(game: &Game, entity: &Entity) -> bool {
    game.world
        .solid_at_except(entity.pos.x, entity.pos.y, entity.id)
}

fn spawn(game: &mut Game, object: &str, pos: Vec2, vel: Vec2) -> u64 {
    let id = game.world.spawn(object, pos);
    let entity = game.world.entity_mut(id).expect("new entity");
    init_entity(entity);
    entity.vel = vel;
    entity.data.insert("cost".into(), 0.);
    entity.data.insert("forSale".into(), 0.);
    id
}

/// Apply a consumed pickup after free pickup, purchase, or theft. Shop policy is
/// the caller's responsibility; this is the common equivalent of scrStealItem.
pub fn grant(game: &mut Game, id: u64) -> bool {
    let Some(entity) = game.world.entity(id) else {
        return false;
    };
    let object = entity.object.clone();
    let pos = entity.pos;
    let message = match object.as_str() {
        "oBombBag" => {
            game.player.bombs += 3;
            "YOU GOT 3 MORE BOMBS!"
        }
        "oBombBox" => {
            game.player.bombs += 12;
            "YOU GOT 12 MORE BOMBS!"
        }
        "oRopePile" => {
            game.player.ropes += 3;
            "YOU GOT 3 MORE ROPES!"
        }
        _ => {
            let Some(key) = equipment(&object) else {
                return false;
            };
            game.player.equipment.insert(key.into());
            match key {
                "Cape" if game.player.equipment.remove("Jetpack") => {
                    spawn(game, "oJetpack", game.player.pos, Vec2::new(0., -1.));
                }
                "Jetpack" if game.player.equipment.remove("Cape") => {
                    spawn(game, "oCapePickup", game.player.pos, Vec2::new(0., -1.));
                    for e in &mut game.world.entities {
                        if e.object == "oCape" {
                            e.alive = false;
                        }
                    }
                }
                _ => {}
            }
            match key {
                "Parachute" => "YOU GOT A PARACHUTE!",
                "Cape" => "YOU GOT A CAPE!",
                "Jetpack" => "YOU GOT A JETPACK!",
                "SpringShoes" => "YOU GOT SPRING SHOES!",
                "SpikeShoes" => "YOU GOT SPIKE SHOES!",
                "Gloves" => "YOU GOT CLIMBING GLOVES!",
                "Mitt" => "YOU GOT A PITCHER'S MITT!",
                "Compass" => "YOU GOT A COMPASS!",
                "Spectacles" => "YOU GOT SPECTACLES!",
                "Paste" => "YOU GOT STICKY BOMBS!",
                "UdjatEye" => "YOU GOT THE UDJAT EYE!",
                "Ankh" => "YOU GOT THE ANKH!",
                "Crown" => "YOU GOT THE HEDJET!",
                "Kapala" => "YOU GOT THE KAPALA!",
                "Jordans" => "YOU GOT JORDANS!",
                _ => unreachable!(),
            }
        }
    };
    if let Some(entity) = game.world.entity_mut(id) {
        entity.alive = false;
        entity.held = false;
    }
    if game.player.held == Some(id) {
        game.player.held = None;
    }
    game.sound("sndPickup", pos);
    game.events.push(GameEvent::Collected { object, value: 0 });
    game.events.push(GameEvent::Message(message.into()));
    true
}

/// Advance treasure collection and held-key unlocking. Does not move items.
pub fn tick(game: &mut Game) {
    for entity in &mut game.world.entities {
        init_entity(entity);
    }
    let counter = game.data.get("collectCounter").copied().unwrap_or(0.);
    let pending = game.data.get("collect").copied().unwrap_or(0.);
    if counter > 0. {
        game.data.insert("collectCounter".into(), counter - 1.);
    } else if pending > 0. {
        let amount = pending.min(100.) as u32;
        game.player.money = game.player.money.saturating_add(amount);
        game.data
            .insert("collect".into(), pending - f64::from(amount));
    }

    // Only large gems are transformed into diamonds by the ghost.
    let ghosts: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oGhost")
        .map(bounds)
        .collect();
    let transformed: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            e.alive
                && matches!(
                    e.object.as_str(),
                    "oEmeraldBig" | "oSapphireBig" | "oRubyBig"
                )
                && ghosts.iter().any(|b| bounds(e).intersects(*b))
        })
        .map(|e| (e.id, e.pos))
        .collect();
    for (id, pos) in transformed {
        game.world.entity_mut(id).unwrap().alive = false;
        spawn(game, "oDiamond", pos, Vec2::default());
    }

    if super::player_dead(game) || game.player.is_stunned() {
        return;
    }
    let contact = Rect::new(game.player.pos.x - 8., game.player.pos.y - 8., 17., 17.);
    // The original collects at most one nearest treasure each simulation tick.
    let treasure = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && treasure_value(&e.object).is_some() && contact.intersects(bounds(e)))
        .min_by(|a, b| {
            a.pos
                .distance(game.player.pos)
                .total_cmp(&b.pos.distance(game.player.pos))
        })
        .map(|e| e.id);
    if let Some(id) = treasure {
        let e = game.world.entity(id).unwrap();
        let delayed = !e.object.starts_with("oGold");
        let collect_delay = e.value("collect_delay").max(0.) as u32;
        let can_collect = e.age >= collect_delay
            && e.data
                .get("canCollect")
                .map_or(!delayed || e.age >= 20, |v| {
                    *v > 0. || (collect_delay > 0 && e.age >= collect_delay)
                });
        if can_collect {
            let base = e.value("value") as u32;
            let area = match game.world.area {
                Area::Mines => 0,
                Area::Jungle => 1,
                Area::Ice => 2,
                Area::Temple | Area::Boss => 3,
            };
            let value = base.saturating_add(base.div_ceil(4).saturating_mul(area));
            let object = e.object.clone();
            let pos = e.pos;
            game.world.entity_mut(id).unwrap().alive = false;
            *game.data.entry("collect".into()).or_default() += f64::from(value);
            let counter = game.data.entry("collectCounter".into()).or_default();
            *counter = (*counter + 20.).min(100.);
            *game.data.entry(format!("collected:{object}")).or_default() += 1.;
            game.sound(
                if object.starts_with("oGold") {
                    "sndCoin"
                } else {
                    "sndGem"
                },
                pos,
            );
            game.events.push(GameEvent::Collected { object, value });
        }
    }
    for object in ["oBombBag", "oBombBox", "oRopePile"] {
        let id = game
            .world
            .entities
            .iter()
            .find(|e| {
                e.alive
                    && !e.held
                    && e.object == object
                    && e.value("forSale") <= 0.
                    && contact.intersects(bounds(e))
                    && !origin_is_solid(game, e)
            })
            .map(|e| e.id);
        if let Some(id) = id {
            grant(game, id);
        }
    }
    if let Some(key_id) = game.player.held {
        if let Some(key) = game.world.entity(key_id).filter(|e| e.object == "oKey") {
            let key_bounds = bounds(key);
            let chest = game
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oLockedChest" && bounds(e).intersects(key_bounds))
                .map(|e| (e.id, e.pos));
            if let Some((id, pos)) = chest {
                game.world.entity_mut(key_id).unwrap().alive = false;
                game.world.entity_mut(id).unwrap().alive = false;
                game.player.held = None;
                let vx = game.rng.range(4) as f64 - game.rng.range(4) as f64;
                spawn(game, "oUdjatEye", pos, Vec2::new(vx, -2.));
                game.sound("sndChestOpen", pos);
            }
        }
    }
}

/// Returns true when the attack edge was consumed by a container or equipment.
pub fn interact(game: &mut Game, input: &InputFrame) -> bool {
    if !input.attack_pressed || super::player_dead(game) || game.player.is_stunned() {
        return false;
    }
    for entity in &mut game.world.entities {
        init_entity(entity);
    }
    if input.up {
        let container = game
            .world
            .entities
            .iter()
            .find(|e| {
                e.alive
                    && e.state == 0
                    && matches!(e.object.as_str(), "oChest" | "oCrate" | "oFlareCrate")
                    && bounds(e).contains(game.player.pos)
            })
            .map(|e| (e.id, e.object.clone(), e.pos));
        if let Some((id, object, pos)) = container {
            if object == "oChest" {
                open_chest(game, id, pos);
            } else {
                game.world.entity_mut(id).unwrap().alive = false;
                if game.player.held == Some(id) {
                    game.player.held = None;
                }
                if object == "oFlareCrate" {
                    for _ in 0..3 {
                        let vx = game.rng.range(4) as f64 - game.rng.range(4) as f64;
                        let vy = -f64::from(game.rng.range(3) + 1);
                        spawn(game, "oFlare", pos, Vec2::new(vx, vy));
                    }
                } else {
                    let loot = crate_loot(game);
                    spawn(game, loot, pos, Vec2::default());
                    *game.data.entry("totalCrates".into()).or_default() += 1.;
                }
                game.sound("sndPickup", pos);
            }
            return true;
        }
    }
    if input.down && game.player.held.is_none() {
        let reach = Rect::new(game.player.pos.x - 8., game.player.pos.y, 17., 9.);
        // Respect the nearest item so an equipment pickup cannot reach through
        // another ordinary item selected by the normal holding system.
        let pickup = game
            .world
            .entities
            .iter()
            .filter(|e| {
                e.alive
                    && !e.held
                    && super::is_item(&e.object)
                    && treasure_value(&e.object).is_none()
                    && reach.intersects(bounds(e))
            })
            .min_by(|a, b| {
                a.pos
                    .distance(game.player.pos)
                    .total_cmp(&b.pos.distance(game.player.pos))
            })
            .filter(|e| {
                equipment(&e.object).is_some()
                    && e.value("forSale") <= 0.
                    && !origin_is_solid(game, e)
            })
            .map(|e| e.id);
        if let Some(id) = pickup {
            return grant(game, id);
        }
    }
    false
}

fn crate_loot(game: &mut Game) -> &'static str {
    if game.flags.contains("tutorial") {
        return "oBombBag";
    }
    for (odds, item) in [
        (500, "oJetpack"),
        (200, "oCapePickup"),
        (100, "oShotgun"),
        (100, "oMattock"),
        (100, "oTeleporter"),
        (90, "oGloves"),
        (90, "oSpectacles"),
        (80, "oWebCannon"),
        (80, "oPistol"),
        (80, "oMitt"),
        (60, "oPaste"),
        (60, "oSpringShoes"),
        (60, "oSpikeShoes"),
        (60, "oMachete"),
        (40, "oBombBox"),
        (40, "oBow"),
        (20, "oCompass"),
        (10, "oParaPickup"),
        (2, "oRopePile"),
    ] {
        if game.rng.range(odds) == 0 {
            return item;
        }
    }
    "oBombBag"
}

fn open_chest(game: &mut Game, id: u64, pos: Vec2) {
    let chest = game.world.entity_mut(id).unwrap();
    chest.sprite = "sChestOpen".into();
    chest.state = 1;
    *game.data.entry("totalChests".into()).or_default() += 1.;
    if game.rng.range(12) == 0 && game.world.level > 0 {
        let vx = game.rng.range(4) as f64 - game.rng.range(4) as f64;
        let bomb = spawn(game, "oBomb", pos, Vec2::new(vx, -2.));
        let e = game.world.entity_mut(bomb).unwrap();
        super::explosives::arm_bomb(e);
        e.timer = 40;
        game.sound("sndTrap", pos);
    } else {
        game.sound("sndChestOpen", pos);
        let count = game.rng.range(2) + 3;
        for _ in 0..count {
            let object = ["oEmerald", "oSapphire", "oRuby"][game.rng.range(3) as usize];
            let vx = game.rng.range(4) as f64 - game.rng.range(4) as f64;
            spawn(game, object, pos, Vec2::new(vx, -2.));
        }
        if game.rng.range(4) == 0 {
            let object = ["oEmeraldBig", "oSapphireBig", "oRubyBig"][game.rng.range(3) as usize];
            let vx = game.rng.range(4) as f64 - game.rng.range(4) as f64;
            spawn(game, object, pos, Vec2::new(vx, -2.));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RulesPreset;
    fn game() -> Game {
        Game::shell(7, RulesPreset::Classic)
    }

    #[test]
    fn airborne_stun_cannot_collect_or_open_after_its_timer_expires() {
        let mut g = game();
        g.player.state = crate::PlayerState::Stunned;
        g.player.stun = 0;
        let pos = g.player.pos;
        let gold = g.world.spawn("oGoldChunk", pos);
        let chest = g.world.spawn("oChest", pos);
        g.step(&InputFrame {
            up: true,
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert!(g.player.is_stunned());
        assert!(g.world.entity(gold).is_some());
        assert_eq!(g.world.entity(chest).unwrap().state, 0);
        assert_eq!(g.data.get("collect").copied().unwrap_or(0.0), 0.0);
    }

    #[test]
    fn original_treasure_values_delay_area_bonus_and_money_countup() {
        assert_eq!(treasure_value("oRuby"), Some(400));
        let mut g = game();
        g.world.area = Area::Jungle;
        let pos = g.player.pos;
        let id = g.world.spawn("oEmeraldBig", pos);
        tick(&mut g);
        assert!(g.world.entity(id).is_some());
        g.world.entity_mut(id).unwrap().age = 20;
        tick(&mut g);
        assert!(g.world.entity(id).is_none());
        assert_eq!(g.player.money, 0);
        assert_eq!(g.data["collect"], 1000.);
        for _ in 0..20 {
            tick(&mut g);
        }
        assert_eq!(g.player.money, 0);
        tick(&mut g);
        assert_eq!(g.player.money, 100);
        for _ in 0..9 {
            tick(&mut g);
        }
        assert_eq!(g.player.money, 1000);
        assert_eq!(g.data["collect"], 0.);
    }

    #[test]
    fn shop_stock_and_embedded_supplies_are_not_automatically_collected() {
        let mut g = game();
        let pos = g.player.pos;
        let stock = g.world.spawn("oBombBag", pos);
        g.world
            .entity_mut(stock)
            .unwrap()
            .data
            .insert("forSale".into(), 1.);
        tick(&mut g);
        assert_eq!(g.player.bombs, 4);
        assert!(g.world.entity(stock).is_some());
        g.world
            .entity_mut(stock)
            .unwrap()
            .data
            .insert("forSale".into(), 0.);
        g.world.set_tile(2, 2, "oBlock");
        tick(&mut g);
        assert_eq!(g.player.bombs, 4);
        g.world.set_tile(2, 2, "");
        tick(&mut g);
        assert_eq!(g.player.bombs, 7);
        assert!(g.world.entity(stock).is_none());
    }

    #[test]
    fn equipment_requires_pickup_and_replaces_back_slot() {
        let mut g = game();
        let pos = g.player.pos;
        g.player.equipment.insert("Jetpack".into());
        let cape = g.world.spawn("oCapePickup", pos);
        tick(&mut g);
        assert!(g.world.entity(cape).is_some());
        assert!(interact(
            &mut g,
            &InputFrame {
                down: true,
                attack_pressed: true,
                ..Default::default()
            }
        ));
        assert!(g.player.has("Cape"));
        assert!(!g.player.has("Jetpack"));
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oJetpack")
        );
    }

    #[test]
    fn equipment_pickup_does_not_reach_through_nearer_item() {
        let mut g = game();
        let pos = g.player.pos;
        let rock = g.world.spawn("oRock", Vec2::new(pos.x, pos.y + 1.));
        g.world.spawn("oCapePickup", Vec2::new(pos.x, pos.y + 5.));
        assert!(!interact(
            &mut g,
            &InputFrame {
                down: true,
                attack_pressed: true,
                ..Default::default()
            }
        ));
        assert!(!g.player.has("Cape"));
        assert!(g.world.entity(rock).is_some());
    }

    #[test]
    fn held_key_opens_locked_chest_once() {
        let mut g = game();
        let pos = g.player.pos;
        let key = g.world.spawn("oKey", pos);
        let chest = g.world.spawn("oLockedChest", pos);
        tick(&mut g);
        assert!(g.world.entity(chest).is_some());
        g.player.held = Some(key);
        g.world.entity_mut(key).unwrap().held = true;
        tick(&mut g);
        tick(&mut g);
        assert!(g.world.entity(key).is_none());
        assert!(g.world.entity(chest).is_none());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oUdjatEye")
                .count(),
            1
        );
    }

    #[test]
    fn container_requires_up_attack_and_cannot_open_twice() {
        let mut g = game();
        let pos = g.player.pos;
        let chest = g.world.spawn("oChest", pos);
        assert!(!interact(
            &mut g,
            &InputFrame {
                attack_pressed: true,
                ..Default::default()
            }
        ));
        let input = InputFrame {
            up: true,
            attack_pressed: true,
            ..Default::default()
        };
        assert!(interact(&mut g, &input));
        assert_eq!(g.world.entity(chest).unwrap().state, 1);
        assert!(!interact(&mut g, &input));
        assert_eq!(g.data["totalChests"], 1.);
    }

    #[test]
    fn trapped_chest_bomb_uses_integrated_forty_tick_fuse() {
        let seed = (0..1000)
            .find(|seed| crate::model::Rng::new(*seed).range(12) == 0)
            .unwrap();
        let mut g = Game::shell(seed, RulesPreset::Classic);
        let pos = g.player.pos;
        g.world.spawn("oChest", pos);
        assert!(interact(
            &mut g,
            &InputFrame {
                up: true,
                attack_pressed: true,
                ..Default::default()
            }
        ));
        let bomb = g
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oBomb")
            .unwrap();
        let bomb_id = bomb.id;
        assert_eq!(bomb.value("armed"), 1.);
        assert_eq!(bomb.timer, 40);
        assert_eq!(bomb.sprite, "sBombArmed");
        for _ in 0..39 {
            super::super::tick(&mut g, &InputFrame::default());
        }
        assert!(g.world.entity(bomb_id).is_some());
        assert!(
            !g.events
                .iter()
                .any(|e| matches!(e, GameEvent::Explosion { .. }))
        );
        super::super::tick(&mut g, &InputFrame::default());
        assert!(g.world.entity(bomb_id).is_none());
        assert_eq!(
            g.events
                .iter()
                .filter(|e| matches!(e, GameEvent::Explosion { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn ghost_only_upgrades_large_gems() {
        let mut g = game();
        let ghost = g.world.spawn("oGhost", Vec2::new(100., 100.));
        // The source ghost's origin is outside its [4, 0, 12, 16] mask;
        // equal instance origins merely touch a large gem's right edge.
        let ghost_bounds = g.world.entity(ghost).unwrap().bounds();
        let pos = Vec2::new(
            ghost_bounds.x + ghost_bounds.w / 2.,
            ghost_bounds.y + ghost_bounds.h / 2.,
        );
        let large = g.world.spawn("oRubyBig", pos);
        let small = g.world.spawn("oRuby", pos);
        let outside = g.world.spawn("oEmeraldBig", Vec2::new(pos.x + 40., pos.y));
        tick(&mut g);
        assert!(g.world.entity(large).is_none());
        assert!(g.world.entity(small).is_some());
        assert!(g.world.entity(outside).is_some());
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oDiamond")
        );
    }

    fn floor_game() -> Game {
        let mut g = game();
        for x in 0..8 {
            g.world.set_tile(x, 3, "oBlock");
        }
        g.player.grounded = true;
        g.player.state = crate::model::PlayerState::Standing;
        g
    }

    #[test]
    fn game_step_does_not_collect_supplies_embedded_in_dynamic_solid() {
        let mut g = floor_game();
        let block = g.world.spawn("oPushBlock", Vec2::new(48., 32.));
        let bag = g.world.spawn("oBombBag", Vec2::new(54., 40.));
        g.step(&InputFrame::default());
        assert_eq!(g.player.bombs, 4);
        assert!(g.world.entity(bag).is_some());
        g.world.entity_mut(block).unwrap().alive = false;
        g.step(&InputFrame::default());
        assert_eq!(g.player.bombs, 7);
        assert!(g.world.entity(bag).is_none());
    }

    #[test]
    fn game_step_uses_crate_sprite_mask_for_up_attack_point() {
        let mut g = floor_game();
        g.flags.insert("tutorial".into());
        // sCrate extends two pixels above its instance origin, unlike the
        // movement rectangle's zero top offset.
        let crate_id = g.world.spawn("oCrate", Vec2::new(40., 42.));
        g.step(&InputFrame {
            up: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(g.world.entity(crate_id).is_none());
        assert_eq!(g.data["totalCrates"], 1.);
    }

    #[test]
    fn game_step_held_key_uses_source_mask_overlap() {
        let mut g = floor_game();
        let key = g.world.spawn("oKey", g.player.pos);
        g.player.held = Some(key);
        g.world.entity_mut(key).unwrap().held = true;
        // Key is attached at x44: its sprite right edge overlaps this chest,
        // while the narrower movement rectangles remain separated.
        let chest = g.world.spawn("oLockedChest", Vec2::new(55., 40.));
        g.step(&InputFrame::default());
        assert!(g.world.entity(key).is_none());
        assert!(g.world.entity(chest).is_none());
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oUdjatEye")
        );
    }

    #[test]
    fn game_step_monkey_gold_waits_for_twenty_tick_collection_alarm() {
        let mut g = floor_game();
        let nugget = g.world.spawn("oGoldNugget", g.player.pos);
        let e = g.world.entity_mut(nugget).unwrap();
        e.data.insert("collect_delay".into(), 20.);
        e.data.insert("canCollect".into(), 0.);
        for _ in 0..19 {
            g.step(&InputFrame::default());
        }
        assert!(g.world.entity(nugget).is_some());
        assert_eq!(g.player.money, 0);
        g.step(&InputFrame::default());
        assert!(g.world.entity(nugget).is_none());
        assert_eq!(g.data["collect"], 500.);
    }
}
