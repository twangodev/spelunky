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

fn treasure_category(object: &str) -> &'static str {
    match object {
        "oGoldChunk" => "gold",
        "oGoldNugget" => "nuggets",
        "oGoldBar" => "goldbar",
        "oGoldBars" => "goldbars",
        "oEmerald" => "emeralds",
        "oEmeraldBig" => "bigemeralds",
        "oSapphire" => "sapphires",
        "oSapphireBig" => "bigsapphires",
        "oRuby" => "rubies",
        "oRubyBig" => "bigrubies",
        "oDiamond" => "diamonds",
        _ => unreachable!("validated treasure family"),
    }
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
        entity.alarms[0] = 1;
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
        let gold = entity.object.starts_with("oGold");
        entity
            .data
            .entry("canCollect".into())
            .or_insert(if gold { 1. } else { 0. });
        if !gold {
            entity.alarms[0] = 20;
        }
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
    let id = game.spawn(object, pos);
    let entity = game.world.entity_mut(id).expect("new entity");
    init_entity(entity);
    entity.vel = vel;
    entity.data.insert("cost".into(), 0.);
    entity.data.insert("forSale".into(), 0.);
    id
}

/// oGold/oGoldBig are static DrawnSprite blocks, distinct from oTreasure.
/// Their own Destroy creates loot synchronously before the caller continues.
pub(crate) fn destroy_gold(game: &mut Game, id: u64) -> bool {
    let Some(source) = game.world.entities.iter_mut().find(|source| {
        source.id == id
            && source.alive
            && source.value("gold_destroyed") == 0.0
            && matches!(source.object.as_str(), "oGold" | "oGoldBig")
    }) else {
        return false;
    };
    let pos = source.pos;
    let big = source.object == "oGoldBig";
    source.alive = false;
    source.data.insert("gold_destroyed".into(), 1.0);
    if big {
        spawn_gold_fragment(game, "oGoldNugget", pos);
        let mut index = 0;
        // Literal source for-condition rerolls rand(2,3), including its final
        // failed comparison. Caching a chunk count changes RNG and outcomes.
        while index < 2 + game.rng.range(2) {
            spawn_gold_fragment(game, "oGoldChunk", pos);
            index += 1;
        }
    } else {
        for _ in 0..3 {
            spawn_gold_fragment(game, "oGoldChunk", pos);
        }
    }
    true
}

fn spawn_gold_fragment(game: &mut Game, object: &str, origin: Vec2) {
    let pos = Vec2::new(
        origin.x + f64::from(game.rng.range(3)) - f64::from(game.rng.range(3)),
        origin.y + f64::from(game.rng.range(3)) - f64::from(game.rng.range(3)),
    );
    // Treasure/Create is fixed and uncapped. Complete it before the caller's
    // two horizontal and one vertical velocity draws, preserving source order.
    let id = game.spawn(object, pos);
    let velocity = Vec2::new(
        f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
        3.0 + f64::from(game.rng.range(4)),
    );
    game.world.entity_mut(id).unwrap().vel = velocity;
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
    let second = match object.as_str() {
        "oUdjatEye" => "YOU FEEL AWAKENED.",
        "oAnkh" => "YOU FEEL PROTECTED.",
        "oCrown" => "IT GLOWS A BRILLIANT WHITE.",
        "oKapala" => "IT THIRSTS FOR BLOOD...",
        "oParaPickup" => "IT WILL DEPLOY AUTOMATICALLY.",
        "oSpectacles" => "YOUR EYESIGHT SEEMS IMPROVED...",
        "oSpringShoes" => "YOU FEEL BOUNCY.",
        "oJordans" => "YOU FEEL LIGHT ON YOUR FEET.",
        _ => "",
    };
    game.sound("sndPickup", pos);
    game.events.push(GameEvent::Collected { object, value: 0 });
    crate::enemies::shop_dialogue::say(game, message, second, 120);
    true
}

/// Source oPlayer1 collection/countup block. Automatic collisions run later.
pub fn player_step(game: &mut Game) {
    for entity in &mut game.world.entities {
        init_entity(entity);
    }
    let counter = game.data.get("collectCounter").copied().unwrap_or(0.);
    let pending = game.data.get("collect").copied().unwrap_or(0.);
    if counter > 0. {
        game.data.insert("collectCounter".into(), counter - 1.);
    } else if pending > 0. {
        let amount = pending.min(100.) as u32;
        let previous_money = game.player.money;
        game.player.money = game.player.money.saturating_add(amount);
        // oPlayer1 snapshots money immediately before this countup and adds
        // its resulting delta to the current level's earnings (global.xmoney).
        *game.data.entry("xmoney".into()).or_default() +=
            f64::from(game.player.money - previous_money);
        game.data
            .insert("collect".into(), pending - f64::from(amount));
    }

    if super::player_dead(game) || game.player.is_stunned() {
        return;
    }
    let contact = Rect::new(game.player.pos.x - 8., game.player.pos.y - 8., 17., 17.);
    // Source first tests contact with any treasure, then independently selects
    // instance_nearest across the whole treasure family (not only the hits).
    crate::dispatcher::register_instances(game);
    let treasures: Vec<_> = crate::dispatcher::descendant_instances(game, "oTreasure")
        .into_iter()
        .filter_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => game.world.entity(id),
            _ => None,
        })
        .filter(|entity| entity.engine_active)
        .collect();
    let touching_treasure = treasures
        .iter()
        .any(|entity| contact.intersects(bounds(entity)));
    let treasure = touching_treasure
        .then(|| {
            treasures
                .into_iter()
                .fold(None, |nearest: Option<&Entity>, entity| {
                    if nearest.is_none_or(|current| {
                        entity.pos.distance(game.player.pos) < current.pos.distance(game.player.pos)
                    }) {
                        Some(entity)
                    } else {
                        nearest
                    }
                })
                .map(|entity| entity.id)
        })
        .flatten();
    if let Some(id) = treasure {
        let e = game.world.entity(id).unwrap();
        let can_collect = e.value("canCollect") > 0.;
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
            *game
                .data
                .entry(treasure_category(&object).into())
                .or_default() += 1.;
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
            .find(|e| e.alive && e.object == object && contact.intersects(bounds(e)))
            .filter(|e| !e.held && e.value("forSale") <= 0. && !origin_is_solid(game, e))
            .map(|e| e.id);
        if let Some(id) = id {
            grant(game, id);
        }
    }
}

/// Legacy callers retain Player Step behavior, with no automatic event scan.
pub fn tick(game: &mut Game) {
    player_step(game);
}

pub(crate) fn handles(object: &str) -> bool {
    is_collectible(object)
}

pub(crate) fn step_instance(game: &mut Game, id: u64) {
    // oJetpack inherits Item Step, then checks the existing Player's visibility.
    if !game.player.engine_active || game.player.data.get("visible").copied().unwrap_or(1.0) != 0.0
    {
        return;
    }
    if let Some(entity) = game.world.entity_mut(id) {
        if entity.object == "oJetpack" {
            entity.alive = false;
            // Inherited oItem Destroy clears only holdItem when held.
            if entity.held {
                game.player.held = None;
            }
        }
    }
}
pub(crate) fn builtin_motion(_game: &mut Game, _id: u64) {}
pub(crate) fn animation_end(_game: &mut Game, _id: u64) {}

pub(crate) fn alarm(game: &mut Game, id: u64, alarm: u8) {
    if alarm != 0 {
        return;
    }
    if let Some(entity) = game.world.entity_mut(id) {
        if treasure_value(&entity.object).is_some()
            && entity.object != "oGoldChunk"
            && entity.object != "oGoldBar"
            && entity.object != "oGoldBars"
        {
            entity.data.insert("canCollect".into(), 1.);
        }
    }
}

pub(crate) fn collision(game: &mut Game, id: u64, other: crate::dispatcher::Instance) {
    let crate::dispatcher::Instance::Entity(other_id) = other else {
        return;
    };
    let Some(entity) = game.world.entity(id) else {
        return;
    };
    let object = entity.object.clone();
    let pos = entity.pos;
    let sprite = entity.sprite.clone();
    let Some(other) = game.world.entity(other_id) else {
        return;
    };
    if matches!(object.as_str(), "oEmeraldBig" | "oSapphireBig" | "oRubyBig")
        && other.object == "oGhost"
    {
        spawn(game, "oDiamond", pos, Vec2::default());
        game.world.entity_mut(id).unwrap().alive = false;
    } else if object == "oLockedChest"
        && sprite == "sLockedChest"
        && other.object == "oKey"
        && other.held
    {
        let key = game.world.entity_mut(other_id).unwrap();
        key.held = false;
        key.alive = false;
        game.player.held = None;
        game.player.pickup_item_type.clear();
        let chest = game.world.entity_mut(id).unwrap();
        chest.set_sprite("sLockedChestOpen");
        game.sound("sndChestOpen", pos);
        let vx = game.rng.range(4) as f64 - game.rng.range(4) as f64;
        spawn(game, "oUdjatEye", pos, Vec2::new(vx, -2.));
        spawn(game, "oPoof", pos, Vec2::new(-0.4, 0.));
        spawn(game, "oPoof", pos, Vec2::new(0.4, 0.));
        game.world.entity_mut(id).unwrap().alive = false;
    }
}

// Player Step gates each family with collision_point, then independently
// selects instance_place using the player's full mask and current family order.
// Birth registration belongs to the caller's synchronous Create/dispatcher.
fn container_query(game: &Game, family: &str) -> (bool, Option<u64>) {
    let targets: Vec<_> = crate::dispatcher::descendant_instances(game, family)
        .into_iter()
        .filter_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => game.world.entity(id),
            _ => None,
        })
        .filter(|entity| entity.engine_active)
        .collect();
    let point_hit = targets.iter().any(|entity| {
        entity.mask_contains(game.player.pos, crate::CollisionPrecision::BoundingBox)
    });
    if !point_hit {
        return (false, None);
    }
    let player_mask = game.player.collision_mask();
    let receiver = targets
        .into_iter()
        .find(|entity| player_mask.intersects_mask(&entity.collision_mask()))
        .map(|entity| entity.id);
    (true, receiver)
}

/// Returns true when the attack edge was consumed by a container.
pub fn interact(game: &mut Game, input: &InputFrame) -> bool {
    if !input.attack_pressed || super::player_dead(game) || game.player.is_stunned() {
        return false;
    }
    for entity in &mut game.world.entities {
        init_entity(entity);
    }
    if input.up {
        // Player Step checks Chest before Crate, and counts the point-hit even
        // when the chest is already open. An open chest leaves Attack pressed
        // for the subsequent source branches.
        let (point_hit, chest) = container_query(game, "oChest");
        if point_hit {
            if crate::statistics::is_real_level(game) {
                game.lifetime_stats
                    .increment(crate::statistics::LifetimeCounter::TotalChests);
                *game.data.entry("totalChests".into()).or_default() += 1.;
            }
            if let Some((id, pos)) = chest
                .and_then(|id| game.world.entity(id))
                .filter(|entity| entity.sprite == "sChest")
                .map(|entity| (entity.id, entity.pos))
            {
                open_chest(game, id, pos);
                return true;
            }
        }
        let container = ["oCrate", "oFlareCrate"]
            .into_iter()
            .find_map(|family| container_query(game, family).1)
            .and_then(|id| game.world.entity(id))
            .map(|e| (e.id, e.object.clone(), e.pos));
        if let Some((id, object, pos)) = container {
            if object == "oCrate" && crate::statistics::is_real_level(game) {
                game.lifetime_stats
                    .increment(crate::statistics::LifetimeCounter::TotalCrates);
                *game.data.entry("totalCrates".into()).or_default() += 1.;
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
            }
            game.sound("sndPickup", pos);
            if game.player.held == Some(id) {
                game.player.held = None;
                game.player.pickup_item_type.clear();
            }
            // oPlayer1 Step686/708 creates its Poof before retiring the crate.
            game.spawn("oPoof", pos);
            game.world.entity_mut(id).unwrap().alive = false;
            return true;
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
    chest.set_sprite("sChestOpen");
    chest.state = 1;
    if game.rng.range(12) == 0 && game.world.level > 0 {
        let vx = game.rng.range(4) as f64 - game.rng.range(4) as f64;
        let bomb = spawn(game, "oBomb", pos, Vec2::new(vx, -2.));
        let e = game.world.entity_mut(bomb).unwrap();
        // Chest traps bypass ordinary arming: no Alarm0 delay or slow flash.
        super::explosives::init_entity(e);
        e.data.insert("armed".into(), 1.);
        e.set_sprite("sBombArmed");
        e.set_image_speed(1.);
        e.alarms[1] = 40;
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
    fn natural_treasure_pickups_record_source_categories_and_lifetime_totals_once() {
        let families = [
            ("oGoldChunk", "gold"),
            ("oGoldNugget", "nuggets"),
            ("oGoldBar", "goldbar"),
            ("oGoldBars", "goldbars"),
            ("oEmerald", "emeralds"),
            ("oEmeraldBig", "bigemeralds"),
            ("oSapphire", "sapphires"),
            ("oSapphireBig", "bigsapphires"),
            ("oRuby", "rubies"),
            ("oRubyBig", "bigrubies"),
            ("oDiamond", "diamonds"),
        ];
        for (object, category) in families {
            let mut g = game();
            for x in 0..g.world.width {
                g.world.set_tile(x, 3, "oBlock");
            }
            for (_, key) in families {
                g.data.insert(key.into(), 4.);
            }
            let lifetime_key = format!("collected:{object}");
            g.data.insert(lifetime_key.clone(), 9.);
            let id = g.spawn(object, g.player.pos);
            for _ in 0..20 {
                g.step(&InputFrame::default());
            }
            assert!(g.world.entity(id).is_none(), "{object}");
            for (_, key) in families {
                assert_eq!(
                    g.data[key],
                    if key == category { 5. } else { 4. },
                    "{object}: {key}"
                );
            }
            assert_eq!(g.data[&lifetime_key], 10., "{object}");
            g.step(&InputFrame::default());
            assert_eq!(g.data[category], 5., "{object} counted twice");
            assert_eq!(g.data[&lifetime_key], 10., "{object} counted twice");
        }
    }

    #[test]
    fn treasure_countup_records_actual_level_earnings_after_the_delay() {
        let mut g = game();
        g.world.area = Area::Jungle;
        g.world.level = 5;
        g.player.money = 1000;
        g.data.insert("xmoney".into(), 25.);
        for x in 0..g.world.width {
            g.world.set_tile(x, 3, "oBlock");
        }
        let gold = g.spawn("oGoldChunk", g.player.pos);
        g.step(&InputFrame::default());
        assert!(g.world.entity(gold).is_none());
        assert_eq!(g.data["collect"], 125.);
        assert_eq!(g.player.money, 1000);
        assert_eq!(g.data["xmoney"], 25.);
        for _ in 0..20 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.player.money, 1000);
        assert_eq!(g.data["xmoney"], 25.);
        g.step(&InputFrame::default());
        assert_eq!(g.player.money, 1100);
        assert_eq!(g.data["xmoney"], 125.);
        assert_eq!(g.data["collect"], 25.);
        g.step(&InputFrame::default());
        assert_eq!(g.player.money, 1125);
        assert_eq!(g.data["xmoney"], 150.);
        assert_eq!(g.data["collect"], 0.);
        g.step(&InputFrame::default());
        assert_eq!(g.data["xmoney"], 150.);
    }

    #[test]
    fn airborne_stun_cannot_collect_or_open_after_its_timer_expires() {
        let mut g = game();
        g.player.state = crate::PlayerState::Stunned;
        g.player.stun = 0;
        let pos = g.player.pos;
        let gold = g.spawn("oGoldChunk", pos);
        let chest = g.spawn("oChest", pos);
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
        let id = g.spawn("oEmeraldBig", pos);
        tick(&mut g);
        assert!(g.world.entity(id).is_some());
        alarm(&mut g, id, 0);
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
        let stock = g.spawn("oBombBag", pos);
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
        // The shell starts falling. Author support so Down reaches the
        // source pickup branch rather than the preceding airborne attack.
        for x in 0..g.world.width {
            g.world.set_tile(x, 3, "oHardBlock");
        }
        let pos = g.player.pos;
        g.player.equipment.insert("Jetpack".into());
        let cape = g.spawn("oCapePickup", pos);
        tick(&mut g);
        assert!(g.world.entity(cape).is_some());
        g.world
            .entity_mut(cape)
            .unwrap()
            .data
            .insert("cost".into(), 0.0);
        g.step(&InputFrame {
            down: true,
            ..Default::default()
        });
        assert_eq!(g.player.state, crate::PlayerState::Ducking);
        assert_eq!(g.player.attack_timer, 0);
        g.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
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
        // The shell starts falling. Author support so Down reaches the
        // source pickup branch rather than the preceding airborne attack.
        for x in 0..g.world.width {
            g.world.set_tile(x, 3, "oHardBlock");
        }
        let pos = g.player.pos;
        let rock = g.spawn("oRock", Vec2::new(pos.x, pos.y + 1.));
        g.spawn("oCapePickup", Vec2::new(pos.x, pos.y + 5.));
        g.step(&InputFrame {
            down: true,
            ..Default::default()
        });
        assert_eq!(g.player.state, crate::PlayerState::Ducking);
        assert_eq!(g.player.attack_timer, 0);
        g.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, Some(rock));
        assert!(!g.player.has("Cape"));
        assert!(g.world.entity(rock).is_some());
    }

    #[test]
    fn held_key_opens_locked_chest_once() {
        let mut g = game();
        let pos = g.player.pos;
        let key = g.spawn("oKey", pos);
        let chest = g.spawn("oLockedChest", pos);
        tick(&mut g);
        assert!(g.world.entity(chest).is_some());
        g.player.held = Some(key);
        g.world.entity_mut(key).unwrap().held = true;
        collision(&mut g, chest, crate::dispatcher::Instance::Entity(key));
        collision(&mut g, chest, crate::dispatcher::Instance::Entity(key));
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
        let chest = g.spawn("oChest", pos);
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
        assert_eq!(g.data["totalChests"], 2.);
        assert_eq!(g.lifetime_stats.total_chests, 2);
    }

    #[test]
    fn trapped_chest_bomb_uses_integrated_forty_tick_fuse() {
        let seed = (0..1000)
            .find(|seed| crate::model::Rng::new(*seed).range(12) == 0)
            .unwrap();
        let mut g = Game::shell(seed, RulesPreset::Classic);
        // Keep the player and bomb near the same camera region throughout the
        // fuse. In an empty shell the falling player can deactivate the bomb,
        // which correctly pauses its alarms rather than counting forty ticks.
        for x in 0..g.world.width {
            g.world.set_tile(x, 10, "oHardBlock");
        }
        g.player.pos = Vec2::new(160.0, 152.0);
        let pos = g.player.pos;
        g.spawn("oChest", pos);
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
        assert_eq!(bomb.alarms[0], -1);
        assert_eq!(bomb.alarms[1], 40);
        assert_eq!(bomb.image_speed(), 1.);
        assert_eq!(bomb.sprite, "sBombArmed");
        for elapsed in 1..=39 {
            assert!(g.world.entity(bomb_id).unwrap().engine_active);
            g.step(&InputFrame::default());
            let bomb = g.world.entity(bomb_id).unwrap();
            assert!(bomb.engine_active, "fuse tick {elapsed}");
            assert_eq!(bomb.alarms[1], 40 - elapsed, "fuse tick {elapsed}");
        }
        assert!(g.world.entity(bomb_id).is_some());
        assert!(
            !g.events
                .iter()
                .any(|e| matches!(e, GameEvent::Explosion { .. }))
        );
        g.step(&InputFrame::default());
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
        let ghost = g.spawn("oGhost", Vec2::new(100., 100.));
        // The source ghost's origin is outside its [4, 0, 12, 16] mask;
        // equal instance origins merely touch a large gem's right edge.
        let ghost_bounds = g.world.entity(ghost).unwrap().bounds();
        let pos = Vec2::new(
            ghost_bounds.x + ghost_bounds.w / 2.,
            ghost_bounds.y + ghost_bounds.h / 2.,
        );
        let large = g.spawn("oRubyBig", pos);
        let small = g.spawn("oRuby", pos);
        let outside = g.spawn("oEmeraldBig", Vec2::new(pos.x + 40., pos.y));
        collision(&mut g, large, crate::dispatcher::Instance::Entity(ghost));
        collision(&mut g, small, crate::dispatcher::Instance::Entity(ghost));
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

    fn treasure_order(game: &Game) -> Vec<u64> {
        crate::dispatcher::descendant_instances(game, "oTreasure")
            .into_iter()
            .filter_map(|instance| match instance {
                crate::dispatcher::Instance::Entity(id) if game.world.entity(id).is_some() => {
                    Some(id)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn game_step_treasure_ties_follow_natural_camera_reactivation_order() {
        for start_x in [493.0, 499.0, 500.0] {
            let mut g = Game::shell(42, crate::RulesPreset::Classic);
            for x in 0..g.world.width {
                g.world.set_tile(x, 20, "oBlock");
                if !(4..=5).contains(&x) {
                    g.world.set_tile(x, 10, "oBlock");
                }
            }
            g.player.pos = Vec2::new(start_x, 152.0);
            g.player.previous_pos = g.player.pos;
            g.player.grounded = true;
            let emerald = g.spawn("oEmerald", Vec2::new(72.0, 312.0));
            let ruby = g.spawn("oRuby", Vec2::new(88.0, 312.0));
            assert_eq!(treasure_order(&g), vec![emerald, ruby]);
            let mut saw_inactive = false;
            let mut collected = false;
            for _ in 0..220 {
                let order = treasure_order(&g);
                let emerald_pos = g.world.entity(emerald).unwrap().pos;
                let ruby_pos = g.world.entity(ruby).unwrap().pos;
                let storage: Vec<_> = g
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oEmerald" || e.object == "oRuby")
                    .map(|e| e.id)
                    .collect();
                saw_inactive |= !g.world.entity(emerald).unwrap().engine_active
                    && !g.world.entity(ruby).unwrap().engine_active;
                let before = g.player.pos;
                g.step(&InputFrame {
                    left: before.x > 80.0,
                    right: before.x < 80.0,
                    ..Default::default()
                });
                if g.world.entity(emerald).is_none() || g.world.entity(ruby).is_none() {
                    assert!(saw_inactive, "start={start_x}");
                    assert_eq!(order, vec![ruby, emerald], "start={start_x}");
                    assert_eq!(storage, vec![emerald, ruby]);
                    let emerald_distance = emerald_pos.distance(g.player.pos);
                    let ruby_distance = ruby_pos.distance(g.player.pos);
                    if start_x == 500.0 {
                        assert!(ruby_distance < emerald_distance, "unique nearest control");
                    } else {
                        assert_eq!(emerald_distance, 10.0);
                        assert_eq!(ruby_distance, 10.0);
                    }
                    assert!(g.world.entity(emerald).is_some());
                    assert!(g.world.entity(ruby).is_none());
                    assert!(g.events.iter().any(|event|matches!(event,GameEvent::Collected{object,value:400} if object=="oRuby")));
                    collected = true;
                    break;
                }
            }
            assert!(collected, "start={start_x}");
        }
    }

    #[test]
    fn game_step_treasure_ties_ignore_reversed_storage_without_reactivation() {
        for reverse_storage in [false, true] {
            let mut g = Game::shell(42, crate::RulesPreset::Classic);
            for x in 0..g.world.width {
                g.world.set_tile(x, 20, "oBlock");
            }
            g.player.pos = Vec2::new(80.0, 312.0);
            g.player.previous_pos = g.player.pos;
            g.player.grounded = true;
            let emerald = g.spawn("oEmerald", Vec2::new(72.0, 312.0));
            let ruby = g.spawn("oRuby", Vec2::new(88.0, 312.0));
            if reverse_storage {
                g.world.entities.reverse();
            }
            let mut collected = false;
            for _ in 0..25 {
                assert_eq!(treasure_order(&g), vec![emerald, ruby]);
                g.step(&InputFrame::default());
                if g.world.entity(emerald).is_none() || g.world.entity(ruby).is_none() {
                    assert!(g.world.entity(emerald).is_none());
                    assert!(g.world.entity(ruby).is_some());
                    assert!(g.events.iter().any(|event|matches!(event,GameEvent::Collected{object,value:200} if object=="oEmerald")));
                    collected = true;
                    break;
                }
            }
            assert!(collected, "reverse storage={reverse_storage}");
        }
    }

    #[test]
    fn game_step_does_not_collect_supplies_embedded_in_dynamic_solid() {
        let mut g = floor_game();
        let block = g.spawn("oPushBlock", Vec2::new(48., 32.));
        let bag = g.spawn("oBombBag", Vec2::new(54., 40.));
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
        let crate_id = g.spawn("oCrate", Vec2::new(40., 42.));
        g.step(&InputFrame {
            up: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(g.world.entity(crate_id).is_none());
        assert_eq!(g.data.get("totalCrates").copied().unwrap_or(0.), 0.);
        assert_eq!(g.lifetime_stats.total_crates, 0);
    }

    #[test]
    fn game_step_held_key_uses_source_mask_overlap() {
        let mut g = floor_game();
        let key = g.spawn("oKey", g.player.pos);
        g.player.held = Some(key);
        g.world.entity_mut(key).unwrap().held = true;
        // Key is attached at x44: its sprite right edge overlaps this chest,
        // while the narrower movement rectangles remain separated.
        let chest = g.spawn("oLockedChest", Vec2::new(55., 40.));
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
        let nugget = g.spawn("oGoldNugget", g.player.pos);
        let e = g.world.entity_mut(nugget).unwrap();
        init_entity(e);
        e.alarms[0] = 20;
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
    #[test]
    fn real_level_container_stats_count_open_chest_retry_and_preserve_lifetime() {
        let input = InputFrame {
            up: true,
            attack_pressed: true,
            ..InputFrame::default()
        };
        for context in [
            "rLevel",
            "rLevel2",
            "rLevel3",
            "rOlmec",
            "rLoadLevel",
            "rTutorial",
            "rMainMenu",
        ] {
            let mut g = floor_game();
            g.world.name = context.into();
            g.lifetime_stats.total_chests = 7;
            let chest = g.spawn("oChest", g.player.pos);
            g.step(&input);
            assert_eq!(
                g.world.entity(chest).unwrap().sprite,
                "sChestOpen",
                "{context}"
            );
            let produced = g
                .world
                .entities
                .iter()
                .filter(|e| treasure_value(&e.object).is_some() || e.object == "oBomb")
                .count();
            g.step(&input);
            assert_eq!(
                g.world
                    .entities
                    .iter()
                    .filter(|e| treasure_value(&e.object).is_some() || e.object == "oBomb")
                    .count(),
                produced,
                "{context}: retry spawned loot"
            );
            let expected = if matches!(context, "rLevel" | "rLevel2" | "rLevel3" | "rOlmec") {
                9
            } else {
                7
            };
            assert_eq!(g.lifetime_stats.total_chests, expected, "{context}");
        }
    }

    #[test]
    fn crate_stats_exclude_flare_crates_and_non_gameplay_contexts() {
        for (object, flag, expected) in [
            ("oCrate", "", 1),
            ("oFlareCrate", "", 0),
            ("oCrate", "tutorial", 0),
            ("oCrate", "custom_level", 0),
            ("oCrate", "hub", 0),
        ] {
            let mut g = floor_game();
            if !flag.is_empty() {
                g.flags.insert(flag.into());
            }
            let id = g.spawn(object, g.player.pos);
            g.step(&InputFrame {
                up: true,
                attack_pressed: true,
                ..InputFrame::default()
            });
            assert!(g.world.entity(id).is_none(), "{object} {flag}");
            assert_eq!(g.lifetime_stats.total_crates, expected, "{object} {flag}");
        }
    }

    #[test]
    fn locked_chest_collision_runs_while_player_is_stunned_and_spawns_source_poofs() {
        let mut g = floor_game();
        let chest = g.spawn("oLockedChest", g.player.pos);
        let key = g.spawn("oKey", g.player.pos);
        g.world.entity_mut(key).unwrap().held = true;
        g.player.held = Some(key);
        g.player.state = crate::PlayerState::Stunned;
        collision(&mut g, chest, crate::dispatcher::Instance::Entity(key));
        assert!(g.world.entity(chest).is_none());
        assert!(g.world.entity(key).is_none());
        assert_eq!(g.player.held, None);
        let velocities: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oPoof")
            .map(|e| e.vel.x)
            .collect();
        assert_eq!(velocities, vec![-0.4, 0.4]);
    }
    #[test]
    fn game_step_treasure_contact_selects_nearest_even_outside_contact_mask() {
        let mut g = floor_game();
        let far = g.spawn("oGoldBars", Vec2::new(55., 40.));
        let near = g.spawn("oGoldChunk", Vec2::new(51., 40.));
        let contact = Rect::new(32., 32., 17., 17.);
        assert!(contact.intersects(g.world.entity(far).unwrap().sprite_bounds()));
        assert!(!contact.intersects(g.world.entity(near).unwrap().sprite_bounds()));
        g.step(&InputFrame::default());
        assert!(g.world.entity(near).is_none());
        assert!(g.world.entity(far).is_some());
        assert_eq!(g.data["collect"], 100.);
    }

    #[test]
    fn game_step_supply_first_collision_blocks_later_eligible_supply() {
        let mut g = floor_game();
        let first = g.spawn("oBombBag", g.player.pos);
        g.world
            .entity_mut(first)
            .unwrap()
            .data
            .insert("forSale".into(), 1.);
        let second = g.spawn("oBombBag", g.player.pos);
        g.step(&InputFrame::default());
        assert_eq!(g.player.bombs, 4);
        assert!(g.world.entity(first).is_some());
        assert!(g.world.entity(second).is_some());
        g.world
            .entity_mut(first)
            .unwrap()
            .data
            .insert("forSale".into(), 0.);
        g.step(&InputFrame::default());
        assert_eq!(g.player.bombs, 7);
        assert!(g.world.entity(first).is_none());
        assert!(g.world.entity(second).is_some());
    }
    #[test]
    fn free_supply_and_equipment_grants_emit_source_two_line_dialogue() {
        for (object, first, second) in [
            ("oBombBag", "YOU GOT 3 MORE BOMBS!", ""),
            ("oUdjatEye", "YOU GOT THE UDJAT EYE!", "YOU FEEL AWAKENED."),
            ("oAnkh", "YOU GOT THE ANKH!", "YOU FEEL PROTECTED."),
            (
                "oCrown",
                "YOU GOT THE HEDJET!",
                "IT GLOWS A BRILLIANT WHITE.",
            ),
            ("oKapala", "YOU GOT THE KAPALA!", "IT THIRSTS FOR BLOOD..."),
            (
                "oParaPickup",
                "YOU GOT A PARACHUTE!",
                "IT WILL DEPLOY AUTOMATICALLY.",
            ),
            (
                "oSpectacles",
                "YOU GOT SPECTACLES!",
                "YOUR EYESIGHT SEEMS IMPROVED...",
            ),
            ("oSpringShoes", "YOU GOT SPRING SHOES!", "YOU FEEL BOUNCY."),
            (
                "oJordans",
                "YOU GOT JORDANS!",
                "YOU FEEL LIGHT ON YOUR FEET.",
            ),
        ] {
            let mut g = game();
            let id = g.spawn(object, g.player.pos);
            assert!(grant(&mut g, id));
            assert!(
                g.events
                    .iter()
                    .any(|event| matches!(event, GameEvent::Dialogue(message)
                if message.first == first && message.second == second && message.ticks == 120)),
                "{object}"
            );
            assert_eq!(g.data["message_timer"], 120., "{object}");
        }
    }
}
