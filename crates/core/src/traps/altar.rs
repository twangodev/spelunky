//! Kali offerings and the traps attached to lifting an idol.
//!
//! Reference: oSacAltarLeft, oEnemy/oDamsel sacrifice steps, scrGetFavorMsg,
//! and the Gold Idol pickup branch of oPlayer1 Step (Classic 1.1).
#[cfg(test)]
use crate::model::Rect;
use crate::model::{Area, CollisionPrecision, Game, GameEvent, Vec2};

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
    let view = game.simulation_view_rect();
    for (id, pos) in &surfaces {
        let visible = pos.x > view.x - 20.0
            && pos.x < view.x + (view.w + 4.0)
            && pos.y > view.y - 20.0
            && pos.y < view.y + (view.h + 4.0);
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
    let view = game.simulation_view_rect();
    if pos.x > view.x - 20.
        && pos.x < view.x + (view.w + 4.)
        && pos.y > view.y - 20.
        && pos.y < view.y + (view.h + 4.)
        && !game.world.solid_point_active(
            Vec2::new(pos.x, pos.y + 16.0),
            CollisionPrecision::BoundingBox,
            None,
        )
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
    // Both source offering callers create Flame, play the sound and create
    // capped Blood before scrGetFavorMsg's equipment roll.
    game.spawn("oFlame", pos);
    game.sound("sndSmallExplode", pos);
    crate::enemies::create_blood(game, entity, pos, 3);
    let prior = game.data.get("favor").copied().unwrap_or(0.);
    let devoured = prior <= -8.;
    if !devoured {
        *game.data.entry("favor".into()).or_default() += amount;
    }
    if prior < 0. && !devoured && favor > 0. {
        entity.data.insert("favor".into(), 0.);
    }
    let first = match (damsel, devoured) {
        (true, true) => "KALI DEVOURS YOUR SACRIFICE!",
        (true, false) => "KALI ACCEPTS YOUR SACRIFICE!",
        (false, true) => "KALI DEVOURS THE SACRIFICE!",
        (false, false) => "KALI ACCEPTS THE SACRIFICE!",
    };
    let outcome = reward(game, altar, entity.pos.y);
    if outcome.failed {
        // The failed expression aborts the enclosing source code block before
        // messageTimer, shake and instance_destroy. Prior message writes stay.
        game.events.push(GameEvent::Dialogue(crate::SourceMessage {
            first: first.into(),
            second: outcome.second.into(),
            ticks: game
                .data
                .get("message_timer")
                .copied()
                .unwrap_or(0.)
                .max(0.) as u32,
        }));
        *game.data.entry("source_runtime_errors".into()).or_default() += 1.;
        game.events.push(GameEvent::Message(
            "SOURCE RUNTIME ERROR: scrGetFavorMsg: oSacAltarRight.x has no active instance; sacrifice code block aborted."
                .into(),
        ));
        return;
    }
    crate::enemies::shop_dialogue::say(game, first, outcome.second, 200);
    game.data.insert("shake".into(), 10.);
    entity.alive = false;
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

/// Direct object-qualified reads select the first current active member,
/// irrespective of whether it is represented as a tile or dynamic instance.
fn right_altar_position(game: &mut Game) -> Option<Vec2> {
    crate::dispatcher::register_instances(game);
    crate::dispatcher::descendant_instances(game, "oSacAltarRight")
        .into_iter()
        .filter(|instance| crate::dispatcher::is_active(game, *instance))
        .find_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => game.world.entity(id).map(|altar| altar.pos),
            crate::dispatcher::Instance::Terrain(index) => game
                .world
                .tiles
                .get(index)
                .filter(|tile| !tile.object.is_empty())
                .map(|_| instance.position(game)),
            crate::dispatcher::Instance::Player => None,
        })
}

struct RewardOutcome {
    second: &'static str,
    failed: bool,
}

fn reward(game: &mut Game, _altar: Vec2, offering_y: f64) -> RewardOutcome {
    let favor = game.data.get("favor").copied().unwrap_or(0.0);
    let gift = game.data.get("kali_gift").copied().unwrap_or(0.0);
    let message = if favor <= -8.0 {
        "SHE SEEMS VERY ANGRY WITH YOU!"
    } else if favor < 0.0 {
        "SHE SEEMS ANGRY WITH YOU."
    } else if favor == 0.0 {
        "SHE HAS FORGIVEN YOU!"
    } else if favor >= 32.0 {
        if gift >= 3.0 && favor < 32.0 + (gift - 2.0) * 16.0 {
            "SHE SEEMS ECSTATIC WITH YOU!"
        } else if gift < 3.0 && game.player.bombs < 80.0 {
            game.player.bombs = 99.0;
            game.data.insert("kali_gift".into(), 3.0);
            "YOUR SATCHEL FEELS VERY FULL NOW!"
        } else {
            game.player.health += 4.0 + f64::from(game.rng.range(5));
            game.data.insert("kali_gift".into(), gift + 1.0);
            "YOU FEEL INVIGORATED!"
        }
    } else if favor >= 16.0 && gift < 2.0 {
        game.data.insert("kali_gift".into(), 2.0);
        if let Some(right) = right_altar_position(game) {
            spawn_gift(game, "oKapala", Vec2::new(right.x, offering_y - 8.0));
        } else {
            // The unguarded getter errors before instance_create invokes its
            // builtin; propagate through scrGetFavorMsg to its caller block.
            // The original modal Ignore/Abort choice remains unmodeled.
            return RewardOutcome {
                second: "SHE BESTOWS A GIFT UPON YOU!",
                failed: true,
            };
        }
        "SHE BESTOWS A GIFT UPON YOU!"
    } else if favor >= 16.0 {
        "SHE SEEMS VERY HAPPY WITH YOU!"
    } else if favor >= 8.0 && gift < 1.0 {
        game.data.insert("kali_gift".into(), 1.0);
        let Some(altar) = right_altar_position(game) else {
            return RewardOutcome {
                second: "SHE BESTOWS A GIFT UPON YOU!",
                failed: false,
            };
        };
        let gift_pos = Vec2::new(altar.x, offering_y - 8.0);
        for x_velocity in [-1.0, 1.0] {
            let id = game.spawn("oPoof", gift_pos);
            game.world.entity_mut(id).unwrap().vel = Vec2::new(x_velocity, 0.0);
        }
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
        "SHE BESTOWS A GIFT UPON YOU!"
    } else if favor >= 8.0 {
        "SHE SEEMS HAPPY WITH YOU."
    } else {
        "SHE SEEMS PLEASED WITH YOU."
    };
    RewardOutcome {
        second: message,
        failed: false,
    }
}

fn spawn_gift(game: &mut Game, object: &str, pos: Vec2) {
    let id = game.spawn(object, pos);
    if let Some(gift) = game.world.entity_mut(id) {
        gift.data.insert("cost".into(), 0.0);
        gift.data.insert("forSale".into(), 0.0);
    }
}

fn punish(game: &mut Game) {
    let count = game.data.get("kali_punish").copied().unwrap_or(0.0) as u32;
    crate::enemies::shop_dialogue::say(
        game,
        "YOU DARE DEFILE MY ALTAR?",
        "I WILL PUNISH YOU!",
        200,
    );
    crate::camera_shake::request(game, 10.0);
    *game.data.entry("favor".into()).or_default() -= 16.0;
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
    // Source increments AFTER punishment births, BEFORE its with-family.
    game.data.insert("kali_punish".into(), (count + 1) as f64);
    crate::dispatcher::register_instances(game);
    let targets = crate::dispatcher::descendant_instances(game, "oSacAltarLeft");
    game.flags.insert("altar_defile_cascade".into());
    for target in targets {
        if !crate::dispatcher::is_active(game, target) {
            continue;
        }
        match target {
            crate::dispatcher::Instance::Entity(id) => {
                let Some(entity) = game.world.entity_mut(id) else {
                    continue;
                };
                entity.data.insert("defile".into(), 0.0);
                crate::items::explosives::destroy_solid_self(game, id);
            }
            crate::dispatcher::Instance::Terrain(index) => {
                if game.world.tiles[index].object.is_empty() {
                    continue;
                }
                // The terrain callback adapter publishes the same defile=false
                // assignment to its metadata before any inner child Creates.
                crate::items::explosives::destroy_terrain_self(
                    game,
                    index % game.world.width,
                    index / game.world.width,
                );
            }
            crate::dispatcher::Instance::Player => {}
        }
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
    let view = game.simulation_view_rect();
    if !ghost_exists(game) {
        let x = if game.player.pos.x > game.world.width as f64 * 8.0 {
            view.x + (view.w + 8.0)
        } else {
            view.x - 32.0
        };
        game.spawn("oGhost", Vec2::new(x, view.y + (view.h / 2.0).floor()));
        game.flags.insert("ghost_exists".into());
    }
}

// Stock B producers store TrapBlock as live entities. Raw terrain TrapBlock
// needs its own state/Step adapter and remains outside this callback adapter.
fn idol_trap_receivers(game: &mut Game) -> Vec<u64> {
    crate::dispatcher::register_instances(game);
    crate::dispatcher::descendant_instances(game, "oTrapBlock")
        .into_iter()
        .filter_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => game
                .world
                .entity(id)
                .filter(|entity| entity.engine_active)
                .map(|entity| entity.id),
            _ => None,
        })
        .collect()
}

/// Classic `with`, `instance_exists`, and `instance_nearest` queries only see
/// active instances. Keeping dispatcher order also preserves tie behavior for
/// nearest-instance queries.
fn active_trap_entities(game: &Game, object: &str) -> Vec<u64> {
    crate::dispatcher::descendant_instances(game, object)
        .into_iter()
        .filter(|instance| crate::dispatcher::is_active(game, *instance))
        .filter_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => game
                .world
                .entity(id)
                .filter(|entity| entity.alive)
                .map(|entity| entity.id),
            _ => None,
        })
        .collect()
}

fn nearest_active_trap_entity(game: &Game, object: &str, point: Vec2) -> Option<u64> {
    let mut nearest = None;
    let mut nearest_distance = f64::INFINITY;
    for id in active_trap_entities(game, object) {
        let Some(entity) = game.world.entity(id) else {
            continue;
        };
        let distance = entity.pos.distance(point);
        if nearest.is_none() || distance < nearest_distance {
            nearest = Some(id);
            nearest_distance = distance;
        }
    }
    nearest
}

fn idol_trap_distance_to_character(game: &mut Game, id: u64) -> f64 {
    use crate::dispatcher::Instance;
    crate::dispatcher::register_instances(game);
    let Some(receiver) = game.world.entity(id) else {
        return 1_000_000.0;
    };
    let receiver = receiver.collision_mask().point_bounds();
    crate::dispatcher::descendant_instances(game, "oCharacter")
        .into_iter()
        .filter(|instance| crate::dispatcher::is_active(game, *instance))
        .filter_map(|instance| match instance {
            Instance::Player => Some(game.player.collision_mask().point_bounds()),
            Instance::Entity(target) if target != id => game
                .world
                .entity(target)
                .map(|entity| entity.collision_mask().point_bounds()),
            _ => None,
        })
        .fold(1_000_000.0_f64, |distance, mask| {
            distance.min(crate::world_events::mask_distance(receiver, mask))
        })
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
    crate::dispatcher::register_instances(game);
    *game.data.entry("idols_grabbed".into()).or_default() += 1.;
    game.lifetime_stats
        .increment(crate::statistics::LifetimeCounter::IdolsGrabbed);
    match game.world.area {
        Area::Mines => {
            if let Some(head) = nearest_active_trap_entity(
                game,
                "oGiantTikiHead",
                Vec2::new(game.player.pos.x, game.player.pos.y - 64.0),
            ) {
                game.world.entity_mut(head).unwrap().alarms[0] = 100;
            }
            crate::camera_shake::request(game, 100.0);
        }
        Area::Jungle => {
            if game.flags.contains("cemetery") || game.flags.contains("cemetary") {
                spawn_ghost(game);
            }
            for id in idol_trap_receivers(game) {
                let dist = idol_trap_distance_to_character(game, id);
                if let Some(entity) = game.world.entity_mut(id) {
                    entity.data.insert("dist".into(), dist);
                    if dist < 90.0 {
                        entity.data.insert("dying".into(), 1.0);
                    }
                }
            }
        }
        Area::Temple | Area::Boss => {
            let ceilings = active_trap_entities(game, "oCeilingTrap");
            if !ceilings.is_empty() {
                for id in ceilings {
                    if let Some(entity) = game.world.entity_mut(id) {
                        entity.state = 1;
                        entity.vel.y = 0.5;
                    }
                }
                for target in [
                    Vec2::new(game.player.pos.x - 64.0, game.player.pos.y - 64.0),
                    Vec2::new(game.player.pos.x + 64.0, game.player.pos.y - 64.0),
                ] {
                    if let Some(door) = nearest_active_trap_entity(game, "oDoor", target) {
                        if let Some(entity) = game.world.entity_mut(door) {
                            entity.state = 1;
                            entity.vel.y = 1.0;
                        }
                    }
                }
                crate::camera_shake::request(game, 20.0);
            } else {
                for id in idol_trap_receivers(game) {
                    let dist = idol_trap_distance_to_character(game, id);
                    let Some(entity) = game.world.entity_mut(id) else {
                        continue;
                    };
                    let origin = entity.pos;
                    entity.data.insert("dist".into(), dist);
                    if dist < 90.0 {
                        crate::items::explosives::destroy_solid_self(game, id);
                    }
                    game.sound("sndThump", origin);
                    crate::camera_shake::request(game, 10.0);
                }
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
    let view = game.simulation_view_rect();
    // Own Code follows action_inherited, including a parent Lava Destroy.
    // Read only the entered receiver; do not restore it to live membership.
    let Some(entity) = game.world.entities.iter().find(|entity| entity.id == id) else {
        return;
    };
    let pos = entity.pos;
    if pos.x <= view.x - 8.
        || pos.x >= view.x + (view.w + 8.)
        || pos.y <= view.y - 8.
        || pos.y >= view.y + (view.h + 8.)
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
        game.world
            .entities
            .iter_mut()
            .find(|entity| entity.id == id)
            .unwrap()
            .alive = false;
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

    fn source_favor8_unowned_gift(draw: u32) -> &'static str {
        match draw {
            0 => "oCapePickup",
            1 => "oGloves",
            2 => "oSpectacles",
            3 => "oMitt",
            4 => "oSpringShoes",
            5 => "oSpikeShoes",
            6 => "oPaste",
            7 => "oCompass",
            _ => unreachable!(),
        }
    }

    fn source_detritus_draws(rng: &mut crate::Rng) {
        rng.unit();
        rng.unit();
        rng.unit();
        rng.range(6);
    }

    #[test]
    fn natural_live_damsel_carry_throw_sacrifice_creates_ordered_reward_poofs() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = Game::shell(17, preset);
            for x in 0..game.world.width {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.world.set_tile(11, 10, "oSacAltarLeft");
            game.world.set_tile(12, 10, "oSacAltarRight");
            game.world.set_tile(11, 11, "oBlock");
            game.world.set_tile(12, 11, "oBlock");
            game.player.pos = Vec2::new(96., 152.);
            game.player.previous_pos = game.player.pos;
            let donor = game.spawn("oDamsel", game.player.pos);
            let action = crate::InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            };
            game.step(&action);
            assert_eq!(game.player.held, Some(donor));
            assert_eq!(game.world.entity(donor).unwrap().health, 4);
            for _ in 0..60 {
                if game.player.pos.x >= 160. {
                    break;
                }
                game.step(&crate::InputFrame {
                    right: true,
                    ..Default::default()
                });
                assert_eq!(game.data.get("favor").copied().unwrap_or(0.), 0.);
            }
            for _ in 0..16 {
                game.step(&crate::InputFrame::default());
            }
            assert_eq!(game.player.held, Some(donor));
            assert!(game.player.pos.x >= 160. && game.player.pos.x < 176.);
            game.step(&action);
            assert_eq!(game.player.held, None);
            assert_eq!(game.world.entity(donor).unwrap().state, 2);
            assert!(!game.world.entity(donor).unwrap().held);
            let mut witnessed = false;
            for _ in 0..180 {
                let offering = game.world.entity(donor).unwrap();
                let at_callback = offering.value("sacrifice_count") == 0.
                    && offering.vel == Vec2::default()
                    && game.world.tiles_point(
                        Vec2::new(offering.pos.x, offering.pos.y + 8.),
                        "oSacAltarLeft",
                    );
                let offering_pos = offering.pos;
                let mut expected_rng = game.rng.clone();
                if at_callback {
                    assert_eq!(offering.health, 4, "live sacrifice from actual carry/throw");
                    source_detritus_draws(&mut expected_rng); // unconditional Flame
                    for _ in 0..3 {
                        source_detritus_draws(&mut expected_rng); // source Blood cap admits all
                    }
                }
                let expected_gift = if at_callback {
                    Some(source_favor8_unowned_gift(expected_rng.range(8)))
                } else {
                    None
                };
                game.step(&crate::InputFrame::default());
                if game.world.entity(donor).is_some() {
                    continue;
                }
                assert!(
                    at_callback,
                    "offering must disappear through its real countdown"
                );
                assert_eq!(game.data["favor"], 8.);
                assert_eq!(game.data["kali_gift"], 1.);
                assert_eq!(game.data["shake"], 10.);
                // Sacrifice writes200 in enemy NormalStep, before bright Draw.
                assert_eq!(game.presentation().data["source_message_draw_timer"], 200.);
                assert_eq!(game.data["message_timer"], 199.);
                assert!(game.events.iter().any(|event| matches!(event,
                    GameEvent::Dialogue(message) if message.first == "KALI ACCEPTS YOUR SACRIFICE!"
                        && message.second == "SHE BESTOWS A GIFT UPON YOU!" && message.ticks == 200
                )));
                let flame = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.object == "oFlame")
                    .unwrap();
                let blood: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBlood")
                    .collect();
                assert_eq!(blood.len(), 3);
                let poofs: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oPoof")
                    .collect();
                assert_eq!(poofs.len(), 2);
                let gift = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.object == expected_gift.unwrap())
                    .unwrap();
                assert!(
                    blood
                        .iter()
                        .all(|e| e.creation_serial > flame.creation_serial)
                );
                assert!(
                    blood
                        .iter()
                        .all(|e| e.creation_serial < poofs[0].creation_serial)
                );
                assert!(poofs[0].creation_serial < poofs[1].creation_serial);
                assert!(poofs[1].creation_serial < gift.creation_serial);
                assert_eq!(gift.value("cost"), 0.);
                assert_eq!(gift.value("forSale"), 0.);
                for (poof, direction) in poofs.iter().zip([-1., 1.]) {
                    assert_eq!(poof.vel, Vec2::new(direction, 0.));
                    assert_eq!(poof.pos, Vec2::new(192. + direction, offering_pos.y - 8.));
                    assert_eq!(poof.image_speed(), 0.4);
                    let drawn = game.presentation().world.entity(poof.id).unwrap();
                    assert_eq!(drawn.animation, 0.);
                }
                assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
                let ids = [poofs[0].id, poofs[1].id];
                for _ in 0..13 {
                    game.step(&crate::InputFrame::default());
                }
                assert!(ids.iter().all(|id| game.world.entity(*id).is_some()));
                game.step(&crate::InputFrame::default());
                assert!(ids.iter().all(|id| game.world.entity(*id).is_none()));
                witnessed = true;
                break;
            }
            assert!(
                witnessed,
                "input-thrown live Damsel must naturally settle and be sacrificed"
            );
        }
    }

    #[test]
    fn natural_live_damsel_missing_right_favor16_aborts_sacrifice_after_prior_effects() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = Game::shell(17, preset);
            for x in 0..game.world.width {
                game.world.set_tile(x, 10, "oBlock");
            }
            // Authored partial altar: both supported surfaces are left halves;
            // no right receiver exists for the unguarded favor16 x expression.
            for x in [11, 12] {
                game.world.set_tile(x, 10, "oSacAltarLeft");
                game.world.set_tile(x, 11, "oBlock");
            }
            game.data.insert("favor".into(), 8.);
            game.player.pos = Vec2::new(96., 152.);
            game.player.previous_pos = game.player.pos;
            let donor = game.spawn("oDamsel", game.player.pos);
            let action = crate::InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            };
            game.step(&action);
            assert_eq!(game.player.held, Some(donor));
            for _ in 0..60 {
                if game.player.pos.x >= 160. {
                    break;
                }
                game.step(&crate::InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            for _ in 0..16 {
                game.step(&crate::InputFrame::default());
            }
            assert_eq!(game.player.held, Some(donor));
            assert!(game.player.pos.x >= 160. && game.player.pos.x < 176.);
            game.step(&action);
            assert_eq!(game.player.held, None);
            assert_eq!(game.world.entity(donor).unwrap().state, 2);
            let mut witnessed = false;
            for _ in 0..180 {
                let offering = game.world.entity(donor).unwrap();
                let at_callback = offering.value("sacrifice_count") == 0.
                    && offering.vel == Vec2::default()
                    && game.world.tiles_point(
                        Vec2::new(offering.pos.x, offering.pos.y + 8.),
                        "oSacAltarLeft",
                    );
                let mut expected_rng = game.rng.clone();
                if at_callback {
                    assert_eq!(offering.health, 4);
                    for _ in 0..4 {
                        source_detritus_draws(&mut expected_rng);
                    }
                }
                game.step(&crate::InputFrame::default());
                if game
                    .data
                    .get("source_runtime_errors")
                    .copied()
                    .unwrap_or(0.)
                    == 0.
                {
                    assert!(
                        game.world.entity(donor).is_some(),
                        "source error precedes Destroy"
                    );
                    continue;
                }
                assert!(
                    at_callback,
                    "error must arise from the real sacrifice countdown"
                );
                let retained = game.world.entity(donor).unwrap();
                assert_eq!(retained.health, 4);
                assert_eq!(game.data["favor"], 16.);
                assert_eq!(game.data["kali_gift"], 2.);
                assert_eq!(game.data.get("message_timer").copied().unwrap_or(0.), 0.);
                assert_eq!(game.data.get("shake").copied().unwrap_or(0.), 0.);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oBlood")
                        .count(),
                    3
                );
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oFlame")
                        .count(),
                    1
                );
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|e| e.object == "oKapala" || e.object == "oPoof")
                );
                assert!(game.events.iter().any(|event| matches!(event,
                    GameEvent::Dialogue(message) if message.first == "KALI ACCEPTS YOUR SACRIFICE!"
                        && message.second == "SHE BESTOWS A GIFT UPON YOU!" && message.ticks == 0
                )));
                assert!(game.events.iter().any(|event| matches!(event,
                    GameEvent::Message(message) if message.contains("SOURCE RUNTIME ERROR")
                )));
                assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
                witnessed = true;
                break;
            }
            assert!(
                witnessed,
                "actual carry/throw must reach the missing-right error"
            );
        }
    }

    #[test]
    fn favor8_missing_or_inactive_right_altar_latches_without_effects_or_rng() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for inactive in [false, true] {
                let mut game = Game::shell(3, preset);
                if inactive {
                    let id = game.spawn("oSacAltarRight", Vec2::new(80., 64.));
                    game.world.entity_mut(id).unwrap().engine_active = false;
                }
                game.data.insert("favor".into(), 8.);
                let mut expected_rng = game.rng.clone();
                reward(&mut game, Vec2::new(64., 64.), 56.);
                assert_eq!(game.data["kali_gift"], 1.);
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|e| e.object == "oPoof" || crate::items::is_item(&e.object))
                );
                assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
            }
        }
    }

    #[test]
    fn favor16_missing_inactive_or_destroyed_right_reports_source_error_without_fabricated_gift() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for target in ["absent", "inactive", "destroyed"] {
                let mut game = Game::shell(17, preset);
                if target != "absent" {
                    let right = game.spawn("oSacAltarRight", Vec2::new(224., 64.));
                    let entity = game.world.entity_mut(right).unwrap();
                    if target == "inactive" {
                        entity.engine_active = false;
                    } else {
                        entity.alive = false;
                    }
                }
                game.spawn("oSacAltarLeft", Vec2::new(64., 64.));
                let donor = game.spawn("oDamsel", Vec2::new(72., 56.));
                let index = game
                    .world
                    .entities
                    .iter()
                    .position(|e| e.id == donor)
                    .unwrap();
                let mut donor = game.world.entities.remove(index);
                donor.state = 2;
                donor.data.insert("sacrifice_count".into(), 0.);
                game.data.insert("favor".into(), 8.);
                let mut expected_rng = game.rng.clone();
                for _ in 0..4 {
                    source_detritus_draws(&mut expected_rng);
                }
                offering_entity(&mut game, &mut donor);
                assert!(donor.alive, "failed reward must not reach donor Destroy");
                assert_eq!(game.data["favor"], 16.);
                assert_eq!(game.data.get("message_timer").copied().unwrap_or(0.), 0.);
                assert_eq!(game.data.get("shake").copied().unwrap_or(0.), 0.);
                assert_eq!(game.data["kali_gift"], 2.);
                assert_eq!(game.data["source_runtime_errors"], 1.);
                assert!(game.events.iter().any(|event| matches!(event,
                    GameEvent::Message(message) if message.contains("SOURCE RUNTIME ERROR")
                        && message.contains("scrGetFavorMsg")
                        && message.contains("oSacAltarRight.x")
                )));
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|entity| { entity.object == "oKapala" || entity.object == "oPoof" })
                );
                assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
            }
        }
    }

    #[test]
    fn favor8_right_altar_uses_active_source_order_across_tiles_entities_and_visibility() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for tile_active in [false, true] {
                let mut game = Game::shell(11, preset);
                game.world.set_tile(5, 4, "oSacAltarRight");
                game.world.tiles[4 * game.world.width + 5].engine_active = tile_active;
                let later = game.spawn("oSacAltarRight", Vec2::new(224., 64.));
                game.world
                    .entity_mut(later)
                    .unwrap()
                    .data
                    .insert("visible".into(), 0.);
                game.data.insert("favor".into(), 8.);
                let mut expected_rng = game.rng.clone();
                let object = source_favor8_unowned_gift(expected_rng.range(8));
                reward(&mut game, Vec2::new(64., 64.), 56.);
                let x = if tile_active { 80. } else { 224. };
                let poofs: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oPoof")
                    .collect();
                assert_eq!(poofs.len(), 2);
                assert_eq!(poofs[0].pos, Vec2::new(x, 48.));
                assert_eq!(poofs[1].pos, Vec2::new(x, 48.));
                assert_eq!(poofs[0].vel.x, -1.);
                assert_eq!(poofs[1].vel.x, 1.);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .find(|e| e.object == object)
                        .unwrap()
                        .pos,
                    Vec2::new(x, 48.)
                );
                assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
            }
        }
    }

    #[test]
    fn sacrifice_blood_cap_precedes_uncapped_poofs_and_equipment_roll() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for initial_detritus in [0, 14, 15, 16, 31, 32] {
                let mut game = altar_room();
                game.rules = preset;
                for _ in 0..initial_detritus {
                    game.spawn("oBone", Vec2::new(200., 100.));
                }
                let id = game.spawn("oDamsel", Vec2::new(72., 56.));
                let index = game.world.entities.iter().position(|e| e.id == id).unwrap();
                let mut donor = game.world.entities.remove(index);
                donor.state = 2;
                donor.data.insert("sacrifice_count".into(), 0.);
                let admitted_blood = (15_i32 - initial_detritus).clamp(0, 3) as usize;
                let mut expected_rng = game.rng.clone();
                source_detritus_draws(&mut expected_rng);
                for _ in 0..admitted_blood {
                    source_detritus_draws(&mut expected_rng);
                }
                let object = source_favor8_unowned_gift(expected_rng.range(8));
                offering_entity(&mut game, &mut donor);
                assert!(!donor.alive);
                assert_eq!(game.data["favor"], 8.);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oBlood")
                        .count(),
                    admitted_blood
                );
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oPoof")
                        .count(),
                    2
                );
                assert!(
                    game.world
                        .entities
                        .iter()
                        .any(|e| e.alive && e.object == object)
                );
                assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
            }
        }
    }

    #[test]
    fn favor8_owned_equipment_cycle_and_full_inventory_fallback_use_one_roll() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for seed in 0..8 {
                for jetpack in [false, true] {
                    let mut game = altar_room();
                    game.rules = preset;
                    game.rng = crate::Rng::new(seed);
                    for name in [
                        "Cape",
                        "Gloves",
                        "Spectacles",
                        "Mitt",
                        "SpringShoes",
                        "SpikeShoes",
                        "Paste",
                        "Compass",
                    ] {
                        game.player.equipment.insert(name.into());
                    }
                    if jetpack {
                        game.player.equipment.insert("Jetpack".into());
                    }
                    game.data.insert("favor".into(), 8.);
                    let mut expected_rng = game.rng.clone();
                    expected_rng.range(8);
                    reward(&mut game, Vec2::new(64., 64.), 56.);
                    let expected = if jetpack { "oBombBox" } else { "oJetpack" };
                    assert!(
                        game.world
                            .entities
                            .iter()
                            .any(|e| e.object == expected && e.value("cost") == 0.)
                    );
                    assert_eq!(
                        game.world
                            .entities
                            .iter()
                            .filter(|e| e.object == "oPoof")
                            .count(),
                        2
                    );
                    assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
                }
            }
        }
    }

    #[test]
    fn later_kali_reward_thresholds_do_not_create_favor8_poofs_or_extra_rolls() {
        for (favor, gift, bombs, expected_gift, heal) in [
            (4., 0., 4.0, 0., false),
            (8., 1., 4.0, 1., false),
            (16., 0., 4.0, 2., false),
            (16., 2., 4.0, 2., false),
            (32., 0., 79.0, 3., false),
            (32., 0., 80.0, 1., true),
            (47., 3., 4.0, 3., false),
            (48., 3., 4.0, 4., true),
        ] {
            let mut game = altar_room();
            game.data.insert("favor".into(), favor);
            game.data.insert("kali_gift".into(), gift);
            game.player.bombs = bombs;
            let health = game.player.health;
            let mut expected_rng = game.rng.clone();
            let expected_health = health
                + if heal {
                    4.0 + f64::from(expected_rng.range(5))
                } else {
                    0.0
                };
            reward(&mut game, Vec2::new(64., 64.), 56.);
            assert_eq!(game.data["kali_gift"], expected_gift);
            assert_eq!(game.player.health, expected_health);
            assert_eq!(
                game.player.bombs,
                if favor == 32. && bombs == 79.0 {
                    99.0
                } else {
                    bombs
                }
            );
            assert!(!game.world.entities.iter().any(|e| e.object == "oPoof"));
            assert_eq!(
                game.world.entities.iter().any(|e| e.object == "oKapala"),
                favor == 16. && gift == 0.
            );
            assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
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
                12,
                "origin emits outer and finite defile=false inner bodies; two siblings emit one each"
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
                let player_mask = game.player.collision_mask().point_bounds();
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

#[cfg(test)]
#[path = "idol_trap_character_query_tests.rs"]
mod idol_trap_character_query_tests;
#[cfg(test)]
#[path = "idol_trap_instance_query_tests.rs"]
mod idol_trap_instance_query_tests;
