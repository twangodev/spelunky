//! Kali offerings and the traps attached to lifting an idol.
//!
//! Reference: oSacAltarLeft, oEnemy/oDamsel sacrifice steps, scrGetFavorMsg,
//! and the Gold Idol pickup branch of oPlayer1 Step (Classic 1.1).
use crate::model::{Area, Game, GameEvent, Rect, Vec2};

pub(super) fn tick(game: &mut Game) {
    altars(game);
    idols(game);
}

fn altars(game: &mut Game) {
    let mut defiled = false;
    let mut surfaces = Vec::new();
    let level = game.world.level as f64;
    // Tile destructors do not produce object events. Retain each altar's cell
    // until its destruction has been observed once, scoped to this level.
    for (index, tile) in game.world.tiles.iter().enumerate() {
        if matches!(tile.object.as_str(), "oSacAltarLeft" | "oSacAltarRight") {
            game.data.insert(format!("altar_tile:{index}"), level);
            surfaces.push((
                None,
                Vec2::new(
                    (index % game.world.width) as f64 * 16.0,
                    (index / game.world.width) as f64 * 16.0,
                ),
            ));
        }
    }
    let remembered: Vec<_> = game
        .data
        .iter()
        .filter_map(|(key, value)| {
            if *value != level {
                return None;
            }
            let index = key.strip_prefix("altar_tile:")?.parse::<usize>().ok()?;
            Some((key.clone(), index))
        })
        .collect();
    for (key, index) in remembered {
        if !game
            .world
            .tiles
            .get(index)
            .is_some_and(|tile| matches!(tile.object.as_str(), "oSacAltarLeft" | "oSacAltarRight"))
        {
            game.data.remove(&key);
            defiled = true;
        }
    }
    for entity in &mut game.world.entities {
        if !matches!(entity.object.as_str(), "oSacAltarLeft" | "oSacAltarRight") {
            continue;
        }
        // Destroyed altar objects remain available until the tick's cleanup.
        // Record the reaction so two broken halves never count as two offenses.
        if !entity.alive {
            if entity.value("altar_defiled") == 0.0 {
                entity.data.insert("altar_defiled".into(), 1.0);
                defiled = true;
            }
        } else {
            surfaces.push((Some(entity.id), entity.pos));
        }
    }
    // Like the source's viewport guard, missing support destroys visible altars.
    for (id, pos) in &surfaces {
        let visible = pos.x > game.camera.x - 20.0
            && pos.x < game.camera.x + 324.0
            && pos.y > game.camera.y - 20.0
            && pos.y < game.camera.y + 244.0;
        if visible && !game.world.solid_at(pos.x + 8.0, pos.y + 16.0) {
            if let Some(id) = id {
                if let Some(entity) = game.world.entity_mut(*id) {
                    entity.alive = false;
                    entity.data.insert("altar_defiled".into(), 1.0);
                }
            } else {
                let x = (pos.x / 16.0) as usize;
                let y = (pos.y / 16.0) as usize;
                game.world.set_tile(x, y, "");
                game.data
                    .remove(&format!("altar_tile:{}", y * game.world.width + x));
            }
            defiled = true;
        }
    }
    if defiled {
        punish(game);
        return;
    }

    let mut offerings = Vec::new();
    for entity in &mut game.world.entities {
        if !entity.alive {
            continue;
        }
        let Some(base_favor) = sacrifice_value(&entity.object) else {
            continue;
        };
        let status = entity
            .data
            .get("status")
            .copied()
            .unwrap_or(entity.state as f64);
        let damsel = entity.object == "oDamsel";
        let dead = entity.health <= 0 || status == 99.0;
        let stunned = status == 98.0 || entity.value("stun") > 0.0;
        if !(dead || stunned || (damsel && status == 2.0)) {
            continue;
        }
        let feet = if damsel {
            Vec2::new(entity.pos.x, entity.pos.y + 8.0)
        } else {
            Vec2::new(entity.pos.x + 8.0, entity.pos.y + 16.0)
        };
        let altar = surfaces
            .iter()
            .find(|(_, pos)| Rect::new(pos.x, pos.y - 1.0, 16.0, 4.0).contains(feet));
        if entity.held || entity.vel.x.abs() > 0.01 || entity.vel.y.abs() > 0.01 {
            entity.data.insert("sacrifice_count".into(), 20.0);
            continue;
        }
        if let Some((_, pos)) = altar {
            let count = entity.data.get("sacrifice_count").copied().unwrap_or(20.0);
            if count > 0.0 {
                entity.data.insert("sacrifice_count".into(), count - 1.0);
            } else {
                entity.alive = false;
                let favor = entity.data.get("favor").copied().unwrap_or(base_favor);
                let amount = if damsel {
                    if dead { favor } else { favor * 1.5 }
                } else if dead {
                    favor / 2.0
                } else {
                    favor
                };
                offerings.push((entity.pos, *pos, amount));
            }
        }
    }
    for (pos, altar, amount) in offerings {
        let favor = game.data.entry("favor".into()).or_default();
        let devoured = *favor <= -8.0;
        if !devoured {
            *favor += amount;
        }
        game.world.spawn("oFlame", pos);
        game.sound("sndSmallExplode", pos);
        game.data.insert("shake".into(), 10.0);
        game.events.push(GameEvent::Message(if devoured {
            "KALI DEVOURS THE SACRIFICE!".into()
        } else {
            "KALI ACCEPTS THE SACRIFICE!".into()
        }));
        reward(game, altar);
    }
}

fn sacrifice_value(object: &str) -> Option<f64> {
    Some(match object {
        "oDamsel" => 8.0,
        "oShopkeeper" => 12.0,
        "oHawkman" => 6.0,
        "oYeti" => 4.0,
        "oCaveman" | "oManTrap" => 2.0,
        "oSnake" | "oSpider" | "oGiantSpider" | "oBat" | "oSkeleton" | "oFrog" | "oFireFrog"
        | "oVampire" | "oJiangShi" | "oAlien" | "oUfo" | "oScorpion" | "oMummy" => 1.0,
        _ => return None,
    })
}

fn reward(game: &mut Game, altar: Vec2) {
    let favor = game.data.get("favor").copied().unwrap_or(0.0);
    let gift = game.data.get("kali_gift").copied().unwrap_or(0.0);
    let gift_pos = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oSacAltarRight")
        .map(|e| Vec2::new(e.pos.x, altar.y - 8.0))
        .or_else(|| {
            game.world
                .tiles
                .iter()
                .position(|tile| tile.object == "oSacAltarRight")
                .map(|index| Vec2::new((index % game.world.width) as f64 * 16.0, altar.y - 8.0))
        })
        .unwrap_or(Vec2::new(altar.x + 16.0, altar.y - 8.0));
    let message = if favor <= -8.0 {
        "SHE SEEMS VERY ANGRY WITH YOU!"
    } else if favor < 0.0 {
        "SHE SEEMS ANGRY WITH YOU."
    } else if favor == 0.0 {
        "SHE HAS FORGIVEN YOU!"
    } else if favor >= 32.0 {
        if gift >= 3.0 && favor < 32.0 + (gift - 2.0) * 16.0 {
            "SHE SEEMS ECSTATIC WITH YOU!"
        } else if gift < 3.0 && game.player.bombs < 80 {
            game.player.bombs = 99;
            game.data.insert("kali_gift".into(), 3.0);
            "YOUR SATCHEL FEELS VERY FULL NOW!"
        } else {
            game.player.health += 4 + game.rng.range(5) as i32;
            game.data.insert("kali_gift".into(), gift + 1.0);
            "YOU FEEL INVIGORATED!"
        }
    } else if favor >= 16.0 && gift < 2.0 {
        spawn_gift(game, "oKapala", gift_pos);
        game.data.insert("kali_gift".into(), 2.0);
        "SHE BESTOWS A GIFT UPON YOU!"
    } else if favor >= 16.0 {
        "SHE SEEMS VERY HAPPY WITH YOU!"
    } else if favor >= 8.0 && gift < 1.0 {
        let objects = [
            "oCapePickup",
            "oGloves",
            "oSpectacles",
            "oMitt",
            "oSpringShoes",
            "oSpikeShoes",
            "oPaste",
            "oCompass",
        ];
        let equipment = [
            "Cape",
            "Gloves",
            "Spectacles",
            "Mitt",
            "SpringShoes",
            "SpikeShoes",
            "Paste",
            "Compass",
        ];
        let start = game.rng.range(8) as usize;
        let object = (0..8)
            .map(|n| (start + n) % 8)
            .find(|&n| !game.player.has(equipment[n]) && !(n == 0 && game.player.has("Jetpack")))
            .map(|n| objects[n])
            .unwrap_or(if game.player.has("Jetpack") {
                "oBombBox"
            } else {
                "oJetpack"
            });
        spawn_gift(game, object, gift_pos);
        game.data.insert("kali_gift".into(), 1.0);
        "SHE BESTOWS A GIFT UPON YOU!"
    } else if favor >= 8.0 {
        "SHE SEEMS HAPPY WITH YOU."
    } else {
        "SHE SEEMS PLEASED WITH YOU."
    };
    game.events.push(GameEvent::Message(message.into()));
}

fn spawn_gift(game: &mut Game, object: &str, pos: Vec2) {
    let id = game.world.spawn(object, pos);
    if let Some(gift) = game.world.entity_mut(id) {
        gift.data.insert("cost".into(), 0.0);
        gift.data.insert("forSale".into(), 0.0);
    }
}

fn punish(game: &mut Game) {
    *game.data.entry("favor".into()).or_default() -= 16.0;
    let count = game.data.get("kali_punish").copied().unwrap_or(0.0) as u32;
    game.data.insert("kali_punish".into(), (count + 1) as f64);
    game.data.insert("shake".into(), 10.0);
    game.events.push(GameEvent::Message(
        "YOU DARE DEFILE MY ALTAR? I WILL PUNISH YOU!".into(),
    ));
    for entity in &mut game.world.entities {
        if matches!(entity.object.as_str(), "oSacAltarLeft" | "oSacAltarRight") {
            entity.alive = false;
            entity.data.insert("altar_defiled".into(), 1.0);
        }
    }
    for tile in &mut game.world.tiles {
        if matches!(tile.object.as_str(), "oSacAltarLeft" | "oSacAltarRight") {
            tile.object.clear();
        }
    }
    game.data.retain(|key, _| !key.starts_with("altar_tile:"));
    if count == 1 {
        game.world.spawn("oBall", game.player.pos);
        for link in 1..=4 {
            let id = game.world.spawn("oChain", game.player.pos);
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("link_val".into(), link as f64);
        }
    } else if count >= 2 && !(game.world.dark && ghost_exists(game)) {
        game.world.dark = true;
        spawn_ghost(game);
    } else {
        let origins: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oKaliHead")
            .map(|e| e.pos)
            .collect();
        for pos in origins {
            for _ in 0..6 {
                let id = game.world.spawn("oSpider", pos);
                let velocity = Vec2::new(
                    game.rng.range(4) as f64 - game.rng.range(4) as f64,
                    -(1.0 + game.rng.range(3) as f64),
                );
                game.world.entity_mut(id).unwrap().vel = velocity;
            }
            game.sound("sndThump", pos);
        }
    }
}

fn ghost_exists(game: &Game) -> bool {
    game.world
        .entities
        .iter()
        .any(|e| e.alive && e.object == "oGhost")
}

fn spawn_ghost(game: &mut Game) {
    if !ghost_exists(game) {
        let x = if game.player.pos.x > game.world.width as f64 * 8.0 {
            game.camera.x + 328.0
        } else {
            game.camera.x - 32.0
        };
        game.world
            .spawn("oGhost", Vec2::new(x, game.camera.y + 120.0));
        game.flags.insert("ghost_exists".into());
    }
}

fn idols(game: &mut Game) {
    let mut triggers = Vec::new();
    let mut unsupported = Vec::new();
    for entity in &game.world.entities {
        if !entity.alive || !matches!(entity.object.as_str(), "oGoldIdol" | "oCrystalSkull") {
            continue;
        }
        let trigger = entity.data.get("trigger").copied().unwrap_or(1.0) != 0.0;
        if entity.value("idol_trap_triggered") != 0.0 || !trigger {
            continue;
        }
        // Source oPlayer1 pickup runs the trap; oGoldIdol's own Step only
        // consumes the trigger/stat if its supporting floor disappeared.
        if entity.held || game.player.held == Some(entity.id) {
            triggers.push((entity.id, entity.pos, entity.object == "oCrystalSkull"));
        } else if !game.world.solid_at(entity.pos.x, entity.pos.y + 5.0) {
            unsupported.push(entity.id);
        }
    }
    for id in unsupported {
        if let Some(entity) = game.world.entity_mut(id) {
            entity.data.insert("trigger".into(), 0.0);
            entity.data.insert("idol_trap_triggered".into(), 1.0);
        }
        *game.data.entry("idols_grabbed".into()).or_default() += 1.0;
    }
    for (id, pos, skull) in triggers {
        if let Some(entity) = game.world.entity_mut(id) {
            entity.data.insert("trigger".into(), 0.0);
            entity.data.insert("idol_trap_triggered".into(), 1.0);
        }
        *game.data.entry("idols_grabbed".into()).or_default() += 1.0;
        match game.world.area {
            Area::Mines => {
                if let Some(entity) = game
                    .world
                    .entities
                    .iter_mut()
                    .filter(|e| e.alive && e.object == "oGiantTikiHead")
                    .min_by(|a, b| a.pos.distance(pos).total_cmp(&b.pos.distance(pos)))
                {
                    entity.data.insert("idol_alarm".into(), 100.0);
                }
                game.data.insert("shake".into(), 100.0);
            }
            Area::Jungle => {
                if skull || game.flags.contains("cemetery") || game.flags.contains("cemetary") {
                    spawn_ghost(game);
                }
                for entity in &mut game.world.entities {
                    if entity.alive
                        && entity.object == "oTrapBlock"
                        && entity.pos.distance(game.player.pos) < 90.0
                    {
                        entity.data.insert("dying".into(), 1.0);
                    }
                }
            }
            Area::Temple | Area::Boss => {
                let ceiling = game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oCeilingTrap");
                if ceiling {
                    for entity in &mut game.world.entities {
                        if entity.alive && entity.object == "oCeilingTrap" {
                            entity.state = 1;
                            entity.vel.y = 0.5;
                        }
                    }
                    for target in [
                        Vec2::new(pos.x - 64.0, pos.y - 64.0),
                        Vec2::new(pos.x + 64.0, pos.y - 64.0),
                    ] {
                        if let Some(door) = game
                            .world
                            .entities
                            .iter_mut()
                            .filter(|e| e.alive && e.object == "oDoor")
                            .min_by(|a, b| {
                                a.pos.distance(target).total_cmp(&b.pos.distance(target))
                            })
                        {
                            door.state = 1;
                            door.vel.y = 1.0;
                        }
                    }
                    game.data.insert("shake".into(), 20.0);
                } else {
                    for entity in &mut game.world.entities {
                        if entity.alive
                            && entity.object == "oTrapBlock"
                            && entity.pos.distance(game.player.pos) < 90.0
                        {
                            entity.alive = false;
                        }
                    }
                    game.sound("sndThump", pos);
                    game.data.insert("shake".into(), 10.0);
                }
            }
            Area::Ice => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RulesPreset;

    fn altar_room() -> Game {
        let mut game = Game::shell(17, RulesPreset::Classic);
        game.world.set_tile(4, 5, "oBlock");
        game.world.set_tile(5, 5, "oBlock");
        game.world.spawn("oSacAltarLeft", Vec2::new(64.0, 64.0));
        game.world.spawn("oSacAltarRight", Vec2::new(80.0, 64.0));
        game
    }

    #[test]
    fn live_damsel_offering_grants_twelve_favor_after_twenty_ticks() {
        let mut game = altar_room();
        let id = game.world.spawn("oDamsel", Vec2::new(72.0, 56.0));
        game.world.entity_mut(id).unwrap().state = 98;
        for _ in 0..20 {
            tick(&mut game);
        }
        assert!(game.world.entity(id).is_some());
        tick(&mut game);
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.data["favor"], 12.0);
        assert_eq!(game.data["kali_gift"], 1.0);
        let count = game.world.entities.len();
        tick(&mut game);
        assert_eq!(game.data["favor"], 12.0);
        assert_eq!(game.world.entities.len(), count);
    }

    #[test]
    fn defiling_both_halves_counts_once() {
        let mut game = altar_room();
        tick(&mut game);
        game.world.set_tile(4, 5, "");
        game.world.set_tile(5, 5, "");
        tick(&mut game);
        assert_eq!(game.data["favor"], -16.0);
        assert_eq!(game.data["kali_punish"], 1.0);
        tick(&mut game);
        assert_eq!(game.data["favor"], -16.0);
    }

    #[test]
    fn held_idol_triggers_once_without_moving_item() {
        let mut game = Game::shell(2, RulesPreset::Classic);
        let head = game.world.spawn("oGiantTikiHead", Vec2::new(100.0, 50.0));
        let id = game.world.spawn("oGoldIdol", Vec2::new(100.0, 100.0));
        game.world.entity_mut(id).unwrap().held = true;
        tick(&mut game);
        tick(&mut game);
        assert_eq!(game.world.entity(head).unwrap().value("idol_alarm"), 100.0);
        assert_eq!(game.data["idols_grabbed"], 1.0);
        assert_eq!(game.world.entity(id).unwrap().pos, Vec2::new(100.0, 100.0));
    }

    #[test]
    fn unsupported_idol_consumes_trigger_without_starting_trap() {
        let mut game = Game::shell(2, RulesPreset::Classic);
        let head = game.world.spawn("oGiantTikiHead", Vec2::new(100.0, 50.0));
        let idol = game.world.spawn("oGoldIdol", Vec2::new(100.0, 100.0));
        tick(&mut game);
        assert_eq!(game.data["idols_grabbed"], 1.0);
        assert_eq!(game.world.entity(head).unwrap().value("idol_alarm"), 0.0);
        assert_eq!(game.world.entity(idol).unwrap().value("trigger"), 0.0);
        game.world.entity_mut(idol).unwrap().held = true;
        tick(&mut game);
        assert_eq!(game.data["idols_grabbed"], 1.0);
        assert_eq!(game.world.entity(head).unwrap().value("idol_alarm"), 0.0);
    }

    #[test]
    fn negative_eight_favor_devours_without_credit() {
        let mut game = altar_room();
        game.data.insert("favor".into(), -8.0);
        let id = game.world.spawn("oDamsel", Vec2::new(72.0, 56.0));
        let victim = game.world.entity_mut(id).unwrap();
        victim.state = 98;
        victim.data.insert("sacrifice_count".into(), 0.0);
        tick(&mut game);
        assert_eq!(game.data["favor"], -8.0);
        assert!(game.world.entity(id).is_none());
    }

    #[test]
    fn moving_stunned_enemy_resets_offering_count_without_favor() {
        let mut game = altar_room();
        let id = game.world.spawn("oCaveman", Vec2::new(64.0, 48.0));
        let victim = game.world.entity_mut(id).unwrap();
        victim.state = 98;
        victim.vel = Vec2::new(2.5, 0.0);
        victim.data.insert("sacrifice_count".into(), 0.0);
        tick(&mut game);
        assert!(game.world.entity(id).is_some());
        assert_eq!(
            game.world.entity(id).unwrap().value("sacrifice_count"),
            20.0
        );
        assert_eq!(game.data.get("favor").copied().unwrap_or(0.0), 0.0);
    }

    #[test]
    fn tile_altar_accepts_offerings_and_observes_destruction() {
        let mut game = Game::shell(17, RulesPreset::Classic);
        game.world.set_tile(4, 4, "oSacAltarLeft");
        game.world.set_tile(5, 4, "oSacAltarRight");
        game.world.set_tile(4, 5, "oBlock");
        game.world.set_tile(5, 5, "oBlock");
        let id = game.world.spawn("oCaveman", Vec2::new(64.0, 48.0));
        let victim = game.world.entity_mut(id).unwrap();
        victim.state = 98;
        victim.vel = Vec2::default(); // The source requires a stationary offering.
        victim.data.insert("sacrifice_count".into(), 0.0);
        tick(&mut game);
        assert_eq!(game.data["favor"], 2.0);
        game.world.set_tile(4, 4, "");
        tick(&mut game);
        assert_eq!(game.data["favor"], -14.0);
        assert!(game.world.tile(5, 4).unwrap().object.is_empty());
        tick(&mut game);
        assert_eq!(game.data["favor"], -14.0);
    }
}
