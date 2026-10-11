//! Kali offerings and the traps attached to lifting an idol.
//!
//! Reference: oSacAltarLeft, oEnemy/oDamsel sacrifice steps, scrGetFavorMsg,
//! and the Gold Idol pickup branch of oPlayer1 Step (Classic 1.1).
#[cfg(test)]
use crate::model::Rect;
use crate::model::{Area, Game, GameEvent, Vec2};

#[cfg(test)]
pub(super) fn tick(game: &mut Game) {
    altars(game);
    idols(game);
}

#[cfg(test)]
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
        if visible && !game.world.solid_at(pos.x, pos.y + 16.0) {
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
        let dead = status == 99.0;
        if if damsel {
            status != 2.0 && !dead
        } else {
            status < 98.0
        } {
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
                    // Classic 1.1 admits only THROWN (2) and DEAD (99)
                    // damsels; its status-98 multiplier is unreachable.
                    favor
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
        game.spawn("oFlame", pos);
        game.sound("sndSmallExplode", pos);
        game.data.insert("shake".into(), 10.0);
        game.events.push(GameEvent::Message(if devoured {
            "KALI DEVOURS THE SACRIFICE!".into()
        } else {
            "KALI ACCEPTS THE SACRIFICE!".into()
        }));
        reward(game, altar, pos.y);
    }
}

/// The altar itself only checks support. Sacrifice countdowns belong to each
/// donor's inherited enemy/Damsel Step, never a global altar scan.
pub(super) fn step_instance(game: &mut Game, instance: crate::dispatcher::Instance) {
    let pos = instance.position(game);
    if pos.x > game.camera.x - 20.
        && pos.x < game.camera.x + 324.
        && pos.y > game.camera.y - 20.
        && pos.y < game.camera.y + 244.
        && !game.world.solid_at(pos.x, pos.y + 16.)
    {
        match instance {
            crate::dispatcher::Instance::Entity(id) => {
                crate::items::explosives::destroy_solid_self(game, id);
            }
            crate::dispatcher::Instance::Terrain(index) => {
                crate::items::explosives::destroy_terrain_self(
                    game,
                    index % game.world.width,
                    index / game.world.width,
                );
            }
            _ => {}
        }
    }
}

/// Called synchronously by the actual altar Destroy body, including tiles.
pub(crate) fn destroyed(game: &mut Game, source: &crate::Entity) {
    if !game.flags.contains("altar_defile_cascade")
        && source.data.get("defile").copied().unwrap_or(1.) != 0.
        && source.value("altar_defiled") == 0.
    {
        punish(game);
    }
}

/// Source oEnemy Step 145 and oDamsel Step337 own this countdown.
pub(crate) fn offering_entity(game: &mut Game, entity: &mut crate::Entity) {
    if !entity.alive {
        return;
    }
    let Some(base_favor) = sacrifice_value(&entity.object) else {
        return;
    };
    let damsel = entity.object == "oDamsel";
    let status = entity
        .data
        .get("status")
        .copied()
        .unwrap_or(entity.state as f64);
    if if damsel {
        status != 2. && status != 99.
    } else {
        status < 98.
    } {
        return;
    }
    if entity.held || entity.vel.x != 0. || entity.vel.y != 0. {
        entity.data.insert("sacrifice_count".into(), 20.);
        return;
    }
    let feet = if damsel {
        Vec2::new(entity.pos.x, entity.pos.y + 8.)
    } else {
        Vec2::new(entity.pos.x + 8., entity.pos.y + 16.)
    };
    let altar = game
        .world
        .entities
        .iter()
        .find(|e| {
            e.alive
                && crate::catalog::is_a(&e.object, "oSacAltarLeft")
                && e.sprite_bounds().contains(feet)
        })
        .map(|e| e.pos)
        .or_else(|| {
            let x = (feet.x / 16.).floor() as i32;
            let y = (feet.y / 16.).floor() as i32;
            game.world
                .tile(x, y)
                .filter(|t| crate::catalog::is_a(&t.object, "oSacAltarLeft"))
                .map(|_| Vec2::new(f64::from(x) * 16., f64::from(y) * 16.))
        });
    let Some(altar) = altar else {
        return;
    };
    let count = entity.data.get("sacrifice_count").copied().unwrap_or(20.);
    if count > 0. {
        entity.data.insert("sacrifice_count".into(), count - 1.);
        return;
    }
    let favor = entity.data.get("favor").copied().unwrap_or(base_favor);
    let amount = if damsel || status == 98. {
        favor
    } else {
        favor / 2.
    };
    let pos = if damsel {
        entity.pos
    } else {
        Vec2::new(entity.pos.x + 8., entity.pos.y + 8.)
    };
    let prior = game.data.get("favor").copied().unwrap_or(0.);
    let devoured = prior <= -8.;
    if !devoured {
        *game.data.entry("favor".into()).or_default() += amount;
    }
    if prior < 0. && !devoured && favor > 0. {
        entity.data.insert("favor".into(), 0.);
    }
    entity.alive = false;
    game.spawn("oFlame", pos);
    game.sound("sndSmallExplode", pos);
    game.data.insert("shake".into(), 10.);
    game.events.push(GameEvent::Message(
        if devoured {
            "KALI DEVOURS THE SACRIFICE!"
        } else {
            "KALI ACCEPTS THE SACRIFICE!"
        }
        .into(),
    ));
    reward(game, altar, entity.pos.y);
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

fn reward(game: &mut Game, altar: Vec2, offering_y: f64) {
    let favor = game.data.get("favor").copied().unwrap_or(0.0);
    let gift = game.data.get("kali_gift").copied().unwrap_or(0.0);
    let gift_pos = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oSacAltarRight")
        .map(|e| Vec2::new(e.pos.x, offering_y - 8.0))
        .or_else(|| {
            game.world
                .tiles
                .iter()
                .position(|tile| tile.object == "oSacAltarRight")
                .map(|index| Vec2::new((index % game.world.width) as f64 * 16.0, offering_y - 8.0))
        })
        .unwrap_or(Vec2::new(altar.x + 16.0, offering_y - 8.0));
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
    let id = game.spawn(object, pos);
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
    if count == 1 {
        game.spawn("oBall", game.player.pos);
        for link in 1..=4 {
            let id = game.spawn("oChain", game.player.pos);
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("linkVal".into(), link as f64);
        }
    } else if count >= 2 && !(game.world.dark && ghost_exists(game)) {
        game.world.dark = true;
        spawn_ghost(game);
    } else {
        for entity in &mut game.world.entities {
            if entity.alive && entity.object == "oKaliHead" {
                entity.alarms[0] = 1;
            }
        }
        for tile in &mut game.world.tiles {
            if tile.object == "oKaliHead" {
                tile.alarms[0] = 1;
            }
        }
    }
    // Source with(oSacAltarLeft) includes its Right child. Each remaining
    // altar runs its normal Destroy body/rubble with defile disabled.
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && crate::catalog::is_a(&e.object, "oSacAltarLeft"))
        .map(|e| e.id)
        .collect();
    let tiles: Vec<_> = game
        .world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, t)| crate::catalog::is_a(&t.object, "oSacAltarLeft"))
        .map(|(index, _)| index)
        .collect();
    game.flags.insert("altar_defile_cascade".into());
    for id in ids {
        if let Some(entity) = game.world.entity_mut(id) {
            entity.data.insert("defile".into(), 0.);
        }
        crate::items::explosives::destroy_solid_self(game, id);
    }
    for index in tiles {
        crate::items::explosives::destroy_terrain_self(
            game,
            index % game.world.width,
            index / game.world.width,
        );
    }
    game.flags.remove("altar_defile_cascade");
    game.data.retain(|key, _| !key.starts_with("altar_tile:"));
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
        game.spawn("oGhost", Vec2::new(x, game.camera.y + 120.0));
        game.flags.insert("ghost_exists".into());
    }
}

/// oPlayer1 Step 1264-1323: invoked once by a successful pickup.
/// Jungle and Ice deliberately retain their trigger, permitting another pickup.
pub(crate) fn idol_pickup(game: &mut Game, id: u64) {
    let Some(entity) = game.world.entity(id) else {
        return;
    };
    if !crate::catalog::is_a(&entity.object, "oGoldIdol")
        || entity.data.get("trigger").copied().unwrap_or(1.) == 0.
        || game.is_load_level()
    {
        return;
    }
    let pos = entity.pos;
    *game.data.entry("idols_grabbed".into()).or_default() += 1.;
    game.lifetime_stats
        .increment(crate::statistics::LifetimeCounter::IdolsGrabbed);
    let character_masks: Vec<_> = std::iter::once(game.player.sprite_bounds())
        .chain(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && crate::catalog::is_a(&e.object, "oCharacter"))
                .map(crate::Entity::sprite_bounds),
        )
        .collect();
    let near_character = |entity: &crate::Entity| {
        character_masks
            .iter()
            .any(|mask| crate::world_events::mask_distance(entity.sprite_bounds(), *mask) < 90.)
    };
    match game.world.area {
        Area::Mines => {
            if let Some(entity) = game
                .world
                .entities
                .iter_mut()
                .filter(|e| e.alive && e.object == "oGiantTikiHead")
                .min_by(|a, b| {
                    a.pos
                        .distance(Vec2::new(game.player.pos.x, game.player.pos.y - 64.))
                        .total_cmp(
                            &b.pos
                                .distance(Vec2::new(game.player.pos.x, game.player.pos.y - 64.)),
                        )
                })
            {
                entity.alarms[0] = 100;
            }
            game.data.insert("shake".into(), 100.0);
        }
        Area::Jungle => {
            if game.flags.contains("cemetery") || game.flags.contains("cemetary") {
                spawn_ghost(game);
            }
            for entity in &mut game.world.entities {
                if entity.alive && entity.object == "oTrapBlock" && near_character(entity) {
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
                    Vec2::new(game.player.pos.x - 64.0, game.player.pos.y - 64.0),
                    Vec2::new(game.player.pos.x + 64.0, game.player.pos.y - 64.0),
                ] {
                    if let Some(door) = game
                        .world
                        .entities
                        .iter_mut()
                        .filter(|e| e.alive && e.object == "oDoor")
                        .min_by(|a, b| a.pos.distance(target).total_cmp(&b.pos.distance(target)))
                    {
                        door.state = 1;
                        door.vel.y = 1.0;
                    }
                }
                game.data.insert("shake".into(), 20.0);
            } else {
                let ids: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|entity| {
                        entity.alive && entity.object == "oTrapBlock" && near_character(entity)
                    })
                    .map(|entity| entity.id)
                    .collect();
                for id in ids {
                    crate::items::explosives::destroy_solid_self(game, id);
                }
                game.sound("sndThump", pos);
                game.data.insert("shake".into(), 10.0);
            }
        }
        Area::Ice => {}
    }
    if matches!(game.world.area, Area::Mines | Area::Temple | Area::Boss) {
        if let Some(entity) = game.world.entity_mut(id) {
            entity.data.insert("trigger".into(), 0.);
        }
    }
}

/// GoldIdol Step follows the inherited item body. Shop redemption and the
/// unsupported trigger share the narrower viewport, without an isRealLevel gate.
pub(crate) fn idol_step(game: &mut Game, id: u64) {
    let Some(entity) = game.world.entity(id) else {
        return;
    };
    let pos = entity.pos;
    if pos.x <= game.camera.x - 8.
        || pos.x >= game.camera.x + 328.
        || pos.y <= game.camera.y - 8.
        || pos.y >= game.camera.y + 248.
    {
        return;
    }
    let unsupported =
        entity.value("colBot") == 0. && entity.data.get("trigger").copied().unwrap_or(1.) != 0.;
    let feet = Vec2::new(pos.x, pos.y + 4.);
    let smooth_brick = game.world.tiles_point(feet, "oBrickSmooth")
        || game.world.entities.iter().any(|brick| {
            brick.alive
                && crate::catalog::is_a(&brick.object, "oBrickSmooth")
                && brick.mask_contains(feet, crate::model::CollisionPrecision::BoundingBox)
        });
    // The source reads oShopkeeper.status/x directly, rather than selecting a
    // nearby eligible keeper. Use the current active family array's first one.
    let keeper = crate::dispatcher::descendant_instances(game, "oShopkeeper")
        .into_iter()
        .find_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => {
                game.world.entity(id).filter(|keeper| keeper.alive)
            }
            _ => None,
        });
    if game.is_level()
        && !entity.held
        && smooth_brick
        && game.data.get("thiefLevel").copied().unwrap_or(0.) == 0.
        && !game.flags.contains("murderer")
        && keeper.is_some_and(|keeper| keeper.state == 0 && (pos.x - keeper.pos.x).abs() < 80.)
    {
        let multiplier = match game.world.area {
            Area::Mines => 1.,
            Area::Jungle => 2.,
            Area::Ice => 3.,
            Area::Temple => 4.,
            Area::Boss => 5.,
        };
        let amount = entity.value("value") * multiplier;
        *game.data.entry("collect".into()).or_default() += amount;
        let counter = game.data.entry("collectCounter".into()).or_default();
        *counter = (*counter + 20.).min(100.);
        *game.data.entry("idols".into()).or_default() += 1.;
        game.sound("sndCoin", pos);
        game.spawn("oBigCollect", Vec2::new(pos.x, pos.y - 8.));
        crate::enemies::shop_dialogue::say(game, "PLEASURE DOING BUSINESS!", "", 100);
        game.world.entity_mut(id).unwrap().alive = false;
    }
    // instance_destroy does not stop the remainder of the current source event.
    if !unsupported {
        return;
    }
    game.world
        .entities
        .iter_mut()
        .find(|entity| entity.id == id)
        .unwrap()
        .data
        .insert("trigger".into(), 0.);
    *game.data.entry("idols_grabbed".into()).or_default() += 1.;
    game.lifetime_stats
        .increment(crate::statistics::LifetimeCounter::IdolsGrabbed);
}

#[cfg(test)]
fn idols(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && crate::catalog::is_a(&e.object, "oGoldIdol"))
        .map(|e| (e.id, e.held || game.player.held == Some(e.id)))
        .collect();
    for (id, held) in ids {
        if held {
            idol_pickup(game, id);
        } else {
            let pos = game.world.entity(id).unwrap().pos;
            let col_bot = game.world.solid_at(pos.x, pos.y + 5.);
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("colBot".into(), f64::from(col_bot));
            idol_step(game, id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RulesPreset;

    fn idol_shop(
        preset: RulesPreset,
        object: &str,
        smooth: bool,
        keeper_x: f64,
    ) -> (Game, u64, u64) {
        let mut game = Game::shell(31, preset);
        for x in 0..game.world.width {
            game.world.set_tile(
                x,
                10,
                if smooth && (6..=10).contains(&x) {
                    "oBrickSmooth"
                } else {
                    "oBlock"
                },
            );
        }
        game.player.pos = Vec2::new(208., 152.);
        game.player.previous_pos = game.player.pos;
        game.player.grounded = true;
        let keeper = game.spawn("oShopkeeper", Vec2::new(keeper_x, 144.));
        game.world
            .entity_mut(keeper)
            .unwrap()
            .text
            .insert("style".into(), "Bomb".into());
        let idol = game.spawn(object, Vec2::new(208., 156.));
        (game, keeper, idol)
    }

    fn carry_to_shop(game: &mut Game, idol: u64) {
        game.step(&crate::InputFrame::default());
        game.step(&crate::InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(idol));
        for _ in 0..60 {
            if game.player.pos.x <= 144. {
                break;
            }
            game.step(&crate::InputFrame {
                left: true,
                ..Default::default()
            });
        }
        assert!((128. ..=144.).contains(&game.player.pos.x));
        for _ in 0..16 {
            game.step(&crate::InputFrame::default());
        }
        assert_eq!(game.player.held, Some(idol));
        assert_eq!(game.data.get("idols").copied().unwrap_or(0.), 0.);
    }

    fn drop_idol(game: &mut Game, idol: u64) -> Vec<crate::SourceMessage> {
        game.step(&crate::InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, None);
        let mut messages = Vec::new();
        for _ in 0..60 {
            messages.extend(game.events.iter().filter_map(|event| match event {
                GameEvent::Dialogue(message) => Some(message.clone()),
                _ => None,
            }));
            if game.world.entity(idol).is_none() {
                break;
            }
            game.step(&crate::InputFrame::default());
        }
        messages
    }

    #[test]
    fn natural_idol_carry_drop_shop_redeems_both_descendants_in_both_presets() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (object, value) in [("oGoldIdol", 5000.), ("oCrystalSkull", 15000.)] {
                let (mut game, _, idol) = idol_shop(preset, object, true, 96.);
                carry_to_shop(&mut game, idol);
                let messages = drop_idol(&mut game, idol);
                assert!(game.world.entity(idol).is_none(), "{preset:?} {object}");
                assert_eq!(
                    f64::from(game.player.money) + game.data.get("collect").copied().unwrap_or(0.),
                    value
                );
                assert_eq!(game.data["idols"], 1.);
                assert_eq!(game.data.get("skulls").copied().unwrap_or(0.), 0.);
                assert_eq!(game.lifetime_stats.idols_converted, 0);
                assert_eq!(game.data.get("idolsConverted").copied().unwrap_or(0.), 0.);
                assert!(
                    game.world
                        .entities
                        .iter()
                        .any(|entity| entity.object == "oBigCollect")
                );
                assert_eq!(
                    messages
                        .iter()
                        .filter(|message| message.first == "PLEASURE DOING BUSINESS!"
                            && message.second.is_empty()
                            && message.ticks == 100)
                        .count(),
                    1
                );
                for _ in 0..30 {
                    game.step(&crate::InputFrame::default());
                }
                assert_eq!(game.data["idols"], 1.);
            }
        }
    }

    #[test]
    fn natural_held_idol_and_drop_without_brick_or_horizontal_keeper_range_do_not_redeem() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (smooth, keeper_x) in [(true, 96.), (false, 96.), (true, 16.)] {
                let (mut game, _, idol) = idol_shop(preset, "oGoldIdol", smooth, keeper_x);
                carry_to_shop(&mut game, idol);
                // Holding on the shop's smooth floor never deposits an idol.
                assert!(game.world.entity(idol).unwrap().held);
                if !smooth || keeper_x == 16. {
                    let messages = drop_idol(&mut game, idol);
                    assert!(game.world.entity(idol).is_some());
                    assert_eq!(game.data.get("collect").copied().unwrap_or(0.), 0.);
                    assert_eq!(game.data.get("idols").copied().unwrap_or(0.), 0.);
                    assert!(
                        !messages
                            .iter()
                            .any(|message| message.first == "PLEASURE DOING BUSINESS!")
                    );
                }
            }
        }
    }

    #[test]
    fn natural_following_keeper_does_not_redeem_a_dropped_idol() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, keeper, idol) = idol_shop(preset, "oGoldIdol", true, 96.);
            // The upward throw travels left. Author smooth floor across its
            // entire return route so missing support cannot explain rejection.
            for x in 0..=10 {
                game.world.set_tile(x, 10, "oBrickSmooth");
            }
            for x in [136., 140.] {
                let stock = game.spawn("oBombBag", Vec2::new(x, 156.));
                let stock = game.world.entity_mut(stock).unwrap();
                stock.data.insert("cost".into(), 2500.);
                stock.data.insert("forSale".into(), 1.);
            }
            carry_to_shop(&mut game, idol);
            game.step(&crate::InputFrame {
                up: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.held, None);
            game.step(&crate::InputFrame::default());
            game.step(&crate::InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            let held = game
                .player
                .held
                .expect("natural stock pickup while idol is airborne");
            assert_eq!(game.world.entity(held).unwrap().object, "oBombBag");
            let mut followed = false;
            let mut supported_in_range_while_following = false;
            for _ in 0..35 {
                game.step(&crate::InputFrame::default());
                let keeper_entity = game.world.entity(keeper).unwrap();
                let idol_entity = game
                    .world
                    .entity(idol)
                    .expect("FOLLOW must keep the idol alive");
                followed |= keeper_entity.state == 5;
                let pos = idol_entity.pos;
                let eligible = !idol_entity.held
                    && idol_entity.value("colBot") != 0.
                    && game
                        .world
                        .tiles_point(Vec2::new(pos.x, pos.y + 4.), "oBrickSmooth")
                    && pos.x > game.camera.x - 8.
                    && pos.x < game.camera.x + 328.
                    && pos.y > game.camera.y - 8.
                    && pos.y < game.camera.y + 248.
                    && (pos.x - keeper_entity.pos.x).abs() < 80.;
                if eligible && keeper_entity.state == 5 {
                    assert_eq!(
                        crate::dispatcher::descendant_instances(&game, "oShopkeeper").first(),
                        Some(&crate::dispatcher::Instance::Entity(keeper))
                    );
                    assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.), 0.);
                    assert!(!game.flags.contains("murderer"));
                    supported_in_range_while_following = true;
                }
            }
            assert!(followed, "keeper enters FOLLOW from held priced stock");
            assert!(
                supported_in_range_while_following,
                "dropped idol must reach on-camera smooth support within80px of the selected FOLLOW keeper"
            );
            assert!(game.world.entity(idol).is_some());
            assert_eq!(game.data.get("idols").copied().unwrap_or(0.), 0.);
            assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.), 0.);
            assert!(!game.flags.contains("murderer"));
        }
    }

    #[test]
    fn idol_shop_uses_first_current_keeper_instead_of_nearest_eligible_keeper() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, first, idol) = idol_shop(preset, "oGoldIdol", true, 320.);
            let nearby = game.spawn("oShopkeeper", Vec2::new(96., 144.));
            game.world
                .entity_mut(nearby)
                .unwrap()
                .text
                .insert("style".into(), "Bomb".into());
            carry_to_shop(&mut game, idol);
            let current = crate::dispatcher::descendant_instances(&game, "oShopkeeper");
            assert_eq!(
                current.first(),
                Some(&crate::dispatcher::Instance::Entity(first))
            );
            assert_eq!(game.world.entity(nearby).unwrap().state, 0);
            drop_idol(&mut game, idol);
            assert!(game.world.entity(idol).is_some());
            assert_eq!(game.data.get("idols").copied().unwrap_or(0.), 0.);
        }
    }

    #[test]
    fn source_shop_idol_values_counter_cap_and_effect_create_do_not_draw_rng() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (area, multiplier) in [
                (Area::Mines, 1.),
                (Area::Jungle, 2.),
                (Area::Ice, 3.),
                (Area::Temple, 4.),
                (Area::Boss, 5.),
            ] {
                for (object, value) in [("oGoldIdol", 5000.), ("oCrystalSkull", 15000.)] {
                    let (mut game, _, idol) = idol_shop(preset, object, true, 96.);
                    // Authored arriving objects/counter; all behavior runs in
                    // one ordinary Step, with no intermediate state writes.
                    game.world.area = area;
                    game.world.entity_mut(idol).unwrap().pos = Vec2::new(140., 156.);
                    game.data.insert("collectCounter".into(), 95.);
                    let mut expected_rng = game.rng.clone();
                    game.step(&crate::InputFrame::default());
                    assert!(game.world.entity(idol).is_none());
                    assert_eq!(game.data["collect"], value * multiplier);
                    assert_eq!(game.data["collectCounter"], 100.);
                    assert_eq!(game.data["idols"], 1.);
                    assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
                    let effect = game
                        .world
                        .entities
                        .iter()
                        .find(|entity| entity.object == "oBigCollect")
                        .unwrap();
                    assert_eq!(effect.pos.x, 140.);
                    assert_eq!(effect.alarms[0], 30);
                }
            }
        }
    }

    #[test]
    fn source_embedded_idol_redemption_uses_persisted_bottom_probe() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oGoldIdol", "oCrystalSkull"] {
                for (y, grabbed) in [(156., 0.), (160., 0.)] {
                    let (mut game, _, idol) = idol_shop(preset, object, true, 96.);
                    // Compose with the canonical inherited Item probe fix:
                    // both arrivals refresh colBot before Idol's own Step, so
                    // neither owes an unsupported-trigger counter increment.
                    game.world.entity_mut(idol).unwrap().pos = Vec2::new(140., y);
                    assert_eq!(game.world.entity(idol).unwrap().value("colBot"), 0.);
                    assert_eq!(game.world.entity(idol).unwrap().value("trigger"), 1.);
                    game.step(&crate::InputFrame::default());
                    assert!(game.world.entity(idol).is_none());
                    assert_eq!(game.data["idols"], 1.);
                    assert_eq!(
                        game.data.get("idols_grabbed").copied().unwrap_or(0.),
                        grabbed
                    );
                    assert_eq!(game.lifetime_stats.idols_grabbed, grabbed as u64);
                }
            }
        }
    }

    #[test]
    fn natural_idol_exit_deposit_still_uses_existing_conversion_statistics() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (object, counter, value) in [
                ("oGoldIdol", "idols", 5000.),
                ("oCrystalSkull", "skulls", 15000.),
            ] {
                let (mut game, _, idol) = idol_shop(preset, object, false, 16.);
                game.spawn("oExit", Vec2::new(128., 144.));
                // The route uses pickup and walking, never fabricated held state.
                game.step(&crate::InputFrame::default());
                game.step(&crate::InputFrame {
                    down: true,
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                });
                assert_eq!(game.player.held, Some(idol));
                for _ in 0..60 {
                    if game.world.entity(idol).is_none() {
                        break;
                    }
                    game.step(&crate::InputFrame {
                        left: true,
                        ..Default::default()
                    });
                }
                assert!(game.world.entity(idol).is_none());
                assert_eq!(game.data[counter], 1.);
                assert_eq!(game.lifetime_stats.idols_converted, 1);
                assert_eq!(
                    f64::from(game.player.money) + game.data.get("collect").copied().unwrap_or(0.),
                    value
                );
            }
        }
    }

    fn altar_room() -> Game {
        let mut game = Game::shell(17, RulesPreset::Classic);
        game.world.set_tile(4, 5, "oBlock");
        game.world.set_tile(5, 5, "oBlock");
        game.world.spawn("oSacAltarLeft", Vec2::new(64.0, 64.0));
        game.world.spawn("oSacAltarRight", Vec2::new(80.0, 64.0));
        game
    }

    #[test]
    fn thrown_damsel_offering_grants_source_eight_favor_after_twenty_ticks() {
        let mut game = altar_room();
        let id = game.world.spawn("oDamsel", Vec2::new(72.0, 56.0));
        game.world.entity_mut(id).unwrap().state = 2;
        for _ in 0..20 {
            tick(&mut game);
        }
        assert!(game.world.entity(id).is_some());
        tick(&mut game);
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.data["favor"], 8.0);
        assert_eq!(game.data["kali_gift"], 1.0);
        let count = game.world.entities.len();
        tick(&mut game);
        assert_eq!(game.data["favor"], 8.0);
        assert_eq!(game.world.entities.len(), count);
    }

    #[test]
    fn game_step_sacrifices_thrown_damsel_and_spawns_gift_at_offering_height() {
        let mut game = altar_room();
        let id = game.world.spawn("oDamsel", Vec2::new(72.0, 56.0));
        let damsel = game.world.entity_mut(id).unwrap();
        damsel.state = 2;
        damsel.data.insert("sacrifice_count".into(), 0.0);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.data["favor"], 8.0);
        let gift = game
            .world
            .entities
            .iter()
            .find(|e| e.data.get("forSale") == Some(&0.0) && crate::items::is_item(&e.object))
            .unwrap();
        assert_eq!(gift.pos, Vec2::new(80.0, 48.0));
    }

    #[test]
    fn game_step_enemy_gift_uses_enemy_origin_and_damsel_status_98_is_ineligible() {
        let mut game = altar_room();
        game.data.insert("favor".into(), 6.0);
        let id = game.world.spawn("oCaveman", Vec2::new(64.0, 48.0));
        let caveman = game.world.entity_mut(id).unwrap();
        crate::enemies::init_entity(caveman);
        caveman.state = 98;
        caveman.vel = Vec2::default();
        caveman.timer = 200;
        caveman.data.insert("sacrifice_count".into(), 0.0);
        game.step(&crate::InputFrame::default());
        assert!(
            game.world.entity(id).is_none(),
            "{:?}",
            game.world.entity(id)
        );
        assert_eq!(game.data["favor"], 8.0);
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| { e.data.get("forSale") == Some(&0.0) && e.pos == Vec2::new(80.0, 40.0) })
        );

        let mut game = altar_room();
        let id = game.world.spawn("oDamsel", Vec2::new(72.0, 56.0));
        let damsel = game.world.entity_mut(id).unwrap();
        damsel.state = 98;
        damsel.data.insert("sacrifice_count".into(), 0.0);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).is_some());
        assert_eq!(game.data.get("favor"), None);
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
    fn actual_unsupported_altar_runs_every_destroy_body_and_defers_kali_alarm() {
        for supported in [false, true] {
            let mut level = crate::level::Level::default();
            level.set_tile(6, 7, '@').unwrap();
            for x in 0..40 {
                level.set_tile(x, 8, '1').unwrap();
            }
            let mut game = Game::shell(17, RulesPreset::Classic);
            game.load_custom(&level, "altar-destroy", "");
            // Controlled arriving source objects; the player remains at its
            // authored entrance and every event runs through Game.step.
            let left = game.spawn("oSacAltarLeft", Vec2::new(64., 64.));
            let right = game.spawn("oSacAltarRight", Vec2::new(80., 64.));
            game.world.set_tile(6, 4, "oSacAltarLeft");
            let head = game.spawn("oKaliHead", Vec2::new(96., 32.));
            if supported {
                for x in 4..=6 {
                    game.world.set_tile(x, 5, "oBlock");
                }
            }
            game.step(&crate::InputFrame::default());
            if supported {
                assert!(game.world.entity(left).is_some());
                assert!(game.world.entity(right).is_some());
                assert_eq!(game.data.get("favor"), None);
                assert_eq!(game.world.entity(head).unwrap().alarms[0], -1);
                continue;
            }
            assert!(game.world.entity(left).is_none());
            assert!(game.world.entity(right).is_none());
            assert!(game.world.tile(6, 4).unwrap().object.is_empty());
            assert_eq!(game.data["favor"], -16.);
            assert_eq!(game.data["kali_punish"], 1.);
            assert_eq!(game.world.entity(head).unwrap().alarms[0], 1);
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| matches!(e.sprite.as_str(), "sRubbleTan" | "sRubbleTanSmall"))
                    .count(),
                9,
                "all three altar Destroy bodies emit their three pieces"
            );
            assert!(!game.world.entities.iter().any(|e| e.object == "oSpider"));
            game.step(&crate::InputFrame::default());
            assert_eq!(game.world.entity(head).unwrap().alarms[0], 0);
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oSpider")
                    .count(),
                6
            );
            assert_eq!(game.data["favor"], -16.);
            assert_eq!(game.data["kali_punish"], 1.);
        }
    }

    #[test]
    fn idol_pickup_events_keep_jungle_and_ice_triggers_but_clear_mines_and_temple() {
        // Controlled source event entries verify the literal branch contract;
        // this does not claim a naturally reachable repeated-pickup route.
        for area in [
            Area::Mines,
            Area::Jungle,
            Area::Ice,
            Area::Temple,
            Area::Boss,
        ] {
            let mut game = Game::shell(17, RulesPreset::Classic);
            game.world.area = area;
            let id = game.spawn("oGoldIdol", game.player.pos);
            idol_pickup(&mut game, id);
            let retained = matches!(area, Area::Jungle | Area::Ice);
            assert_eq!(
                game.world.entity(id).unwrap().value("trigger") != 0.,
                retained,
                "{area:?}"
            );
            idol_pickup(&mut game, id);
            assert_eq!(
                game.lifetime_stats.idols_grabbed,
                if retained { 2 } else { 1 },
                "{area:?}"
            );
        }
    }

    #[test]
    fn actual_supported_idol_pickup_counts_once_and_custom_room_excludes_pickup_counter() {
        for custom in [false, true] {
            let mut game = if custom {
                let mut level = crate::level::Level::default();
                level.set_tile(6, 7, '@').unwrap();
                for x in 0..40 {
                    level.set_tile(x, 8, '1').unwrap();
                }
                let mut game = Game::shell(17, RulesPreset::Classic);
                game.load_custom(&level, "idol-pickup", "");
                game
            } else {
                Game::new(42, RulesPreset::Classic)
            };
            for _ in 0..8 {
                game.step(&crate::InputFrame::default());
            }
            let p = game.player.pos;
            let id = game.spawn("oGoldIdol", Vec2::new(p.x, p.y + 4.));
            game.step(&crate::InputFrame::default());
            assert_eq!(game.world.entity(id).unwrap().value("colBot"), 1.);
            game.step(&crate::InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.held, Some(id));
            assert_eq!(game.lifetime_stats.idols_grabbed, u64::from(!custom));
            assert_eq!(
                game.world.entity(id).unwrap().value("trigger") != 0.,
                custom
            );
        }
    }

    #[test]
    fn actual_idol_pickup_runs_temple_destroy_and_jungle_source_delay() {
        for area in [Area::Jungle, Area::Temple] {
            for pick_up in [false, true] {
                let mut level = crate::level::Level::default();
                level.set_tile(6, 7, '@').unwrap();
                for x in 0..40 {
                    level.set_tile(x, 8, '1').unwrap();
                }
                let mut game = Game::shell(42, RulesPreset::Classic);
                game.load_custom(&level, "idol-collapse", "");
                for _ in 0..8 {
                    game.step(&crate::InputFrame::default());
                }
                // Controlled source levelType and arriving items isolate the
                // pickup branches; no claim of a natural campaign room route.
                game.world.area = area;
                game.flags.remove("custom");
                game.flags.remove("custom_level");
                let p = game.player.pos;
                let idol = game.spawn("oGoldIdol", Vec2::new(p.x, p.y + 4.));
                game.step(&crate::InputFrame::default());
                assert_eq!(game.world.entity(idol).unwrap().value("colBot"), 1.);
                game.step(&crate::InputFrame {
                    down: true,
                    ..Default::default()
                });
                let player_mask = game.player.sprite_bounds();
                let right_edge = player_mask.x + player_mask.w - 1.;
                let block = game.spawn("oTrapBlock", Vec2::new(p.x + 32., p.y - 8.));
                game.world
                    .entity_mut(block)
                    .unwrap()
                    .data
                    .insert("deathTimer".into(), 2.);
                let near_edge = game.spawn("oTrapBlock", Vec2::new(right_edge + 89., p.y - 8.));
                // Keep the radius control away from fresh near-block rubble.
                let far_edge = game.spawn("oTrapBlock", Vec2::new(player_mask.x - 105., p.y - 8.));
                assert_eq!(
                    crate::world_events::mask_distance(
                        game.world.entity(far_edge).unwrap().sprite_bounds(),
                        player_mask
                    ),
                    90.
                );
                let input = if pick_up {
                    crate::InputFrame {
                        down: true,
                        attack: true,
                        attack_pressed: true,
                        ..Default::default()
                    }
                } else {
                    crate::InputFrame::default()
                };
                game.step(&input);
                if pick_up {
                    assert_eq!(game.player.held, Some(idol));
                    if area == Area::Jungle {
                        assert!(game.world.entity(near_edge).is_none());
                        assert_eq!(
                            game.world
                                .entities
                                .iter()
                                .filter(|e| matches!(
                                    e.sprite.as_str(),
                                    "sRubbleTan" | "sRubbleTanSmall"
                                ))
                                .count(),
                            3,
                            "zero-delay edge block emits its source rubble immediately"
                        );
                        let entity = game
                            .world
                            .entity(block)
                            .expect("source delay keeps first tick alive");
                        assert_eq!(entity.value("dying"), 1.);
                        assert_eq!(entity.value("deathTimer"), 1.);
                        game.step(&crate::InputFrame::default());
                        assert_eq!(game.world.entity(block).unwrap().value("deathTimer"), 0.);
                        game.step(&crate::InputFrame::default());
                        assert!(game.events.iter().any(
                            |e| matches!(e, GameEvent::Sound { name, .. } if name == "sndThump")
                        ));
                    }
                    assert!(game.world.entity(block).is_none());
                    assert!(game.world.entity(near_edge).is_none());
                    assert_eq!(game.world.entity(far_edge).unwrap().value("dying"), 0.);
                    assert_eq!(
                        game.world
                            .entities
                            .iter()
                            .filter(|e| matches!(
                                e.sprite.as_str(),
                                "sRubbleTan" | "sRubbleTanSmall"
                            ))
                            .count(),
                        6,
                        "{area:?}"
                    );
                } else {
                    for _ in 0..3 {
                        game.step(&crate::InputFrame::default());
                    }
                    let entity = game.world.entity(block).unwrap();
                    assert_eq!(entity.value("dying"), 0.);
                    assert_eq!(entity.value("deathTimer"), 2.);
                }
            }
        }
    }

    #[test]
    fn held_idol_triggers_once_without_moving_item() {
        let mut game = Game::shell(2, RulesPreset::Classic);
        let head = game.world.spawn("oGiantTikiHead", Vec2::new(100.0, 50.0));
        let id = game.world.spawn("oGoldIdol", Vec2::new(100.0, 100.0));
        game.world.entity_mut(id).unwrap().held = true;
        tick(&mut game);
        tick(&mut game);
        assert_eq!(game.world.entity(head).unwrap().alarms[0], 100);
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
        assert_eq!(game.world.entity(head).unwrap().alarms[0], -1);
        assert_eq!(game.world.entity(idol).unwrap().value("trigger"), 0.0);
        game.world.entity_mut(idol).unwrap().held = true;
        tick(&mut game);
        assert_eq!(game.data["idols_grabbed"], 1.0);
        assert_eq!(game.world.entity(head).unwrap().alarms[0], -1);
    }

    #[test]
    fn negative_eight_favor_devours_without_credit() {
        let mut game = altar_room();
        game.data.insert("favor".into(), -8.0);
        let id = game.world.spawn("oDamsel", Vec2::new(72.0, 56.0));
        let victim = game.world.entity_mut(id).unwrap();
        victim.state = 2;
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
