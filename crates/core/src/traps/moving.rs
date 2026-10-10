//! Moving trap rules transcribed from Classic 1.1's object events.
use crate::dispatcher::Instance;
#[cfg(test)]
use crate::model::PlayerState;
use crate::model::{CollisionPrecision, Entity, Game, GameEvent, Rect, Vec2};

/// The global oGame platform pass precedes character input and object AI.
pub(super) fn pre_tick(game: &mut Game) {
    // Source with(oMovingSolid), then with(oMoveableSolid), traverses the
    // engine instance arrays. Storage order may differ after reactivation.
    crate::dispatcher::register_instances(game);
    let mut carried_horizontally = false;
    for parent in ["oMovingSolid", "oMoveableSolid"] {
        // Neither platform-pass body creates, destroys, or activates instances;
        // this fixed-count snapshot therefore preserves its live source order.
        let instances = crate::dispatcher::descendant_instances(game, parent);
        for instance in instances {
            let Instance::Entity(id) = instance else {
                continue;
            };
            let Some(mut entity) = game.world.entity(id).cloned() else {
                continue;
            };
            entity.solid = true;
            if parent == "oMovingSolid" {
                entity.vel.x += entity.value("xAcc");
                entity.vel.y += entity.value("yAcc");
                if entity.vel.x.abs() < 0.1 {
                    entity.vel.x = 0.0;
                }
                if entity.vel.y.abs() < 0.1 {
                    entity.vel.y = 0.0;
                }
                for key in ["xAcc", "yAcc"] {
                    if entity.value(key).abs() < 0.1 {
                        entity.data.insert(key.into(), 0.0);
                    }
                }
                let velocity = entity.vel;
                move_solid(game, &mut entity, velocity, &mut carried_horizontally);
            } else {
                // Source oMoveableSolid is a separate global gravity pass.
                let active = entity.pos.x > game.camera.x - 16.0
                    && entity.pos.x < game.camera.x + 320.0
                    && entity.pos.y > game.camera.y - 16.0
                    && entity.pos.y < game.camera.y + 240.0;
                if active {
                    let gravity = entity.data.get("push_gravity").copied().unwrap_or(0.6);
                    entity.vel.y = (entity.vel.y + gravity).min(8.0);
                    let speed = entity.vel.y;
                    if move_body(
                        game,
                        &mut entity,
                        Vec2::new(0.0, speed.ceil()),
                        CollisionPrecision::BoundingBox,
                    ) {
                        if speed > 0.6 {
                            game.sound("sndThud", entity.pos);
                        }
                        entity.vel.y = 0.0;
                    }
                }
            }
            // Find by identity: engine order deliberately does not index storage.
            if let Some(current) = game.world.entity_mut(id) {
                *current = entity;
            }
        }
    }
}

pub(super) fn handles(object: &str) -> bool {
    matches!(
        object,
        "oSmashTrap"
            | "oSmashTrapLit"
            | "oBoulder"
            | "oPushBlock"
            | "oIceBlock"
            | "oSpringTrap"
            | "oThinIce"
            | "oDarkFall"
            | "oThwompTrap"
            | "oCeilingTrap"
            | "oDoor"
    )
}

pub(super) fn step_instance(game: &mut Game, instance: Instance) {
    let Instance::Entity(id) = instance else {
        return;
    };
    let Some(mut entity) = game.world.entity(id).cloned() else {
        return;
    };
    match entity.object.as_str() {
        "oSmashTrap" | "oSmashTrapLit" => smash(game, &mut entity),
        "oBoulder" => boulder(game, &mut entity),
        "oPushBlock" => push_block(game, &mut entity),
        "oIceBlock" => ice_block(game, &mut entity),
        "oSpringTrap" => spring(game, &mut entity),
        "oThinIce" => thin_ice(game, &mut entity),
        "oDarkFall" => dark_fall(game, &mut entity),
        "oThwompTrap" => thwomp(game, &mut entity),
        "oCeilingTrap" => ceiling(game, &mut entity),
        "oDoor" => door(game, &mut entity),
        _ => return,
    }
    store(game, entity);
}

fn store(game: &mut Game, entity: Entity) {
    let id = entity.id;
    let destroyed = !entity.alive;
    if let Some(index) = game.world.entities.iter().position(|e| e.id == id) {
        game.world.entities[index] = entity;
    }
    if destroyed {
        crate::items::explosives::destroy_solid_self(game, id);
    }
}

/// Compatibility driver for focused module tests; Game.step uses phase hooks.
#[cfg(test)]
pub(super) fn tick(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && handles(&e.object))
        .map(|e| e.id)
        .collect();
    for id in ids {
        step_instance(game, Instance::Entity(id));
    }
}

/// These objects use script xVel/yVel, never GameMaker's hspeed/vspeed.
pub(super) fn builtin_motion(_game: &mut Game, _instance: Instance) {}

pub(super) fn outside_room(game: &mut Game, instance: Instance) {
    let Instance::Entity(id) = instance else {
        return;
    };
    let Some(entity) = game.world.entity(id) else {
        return;
    };
    if entity.object == "oPushBlock" {
        crate::items::explosives::destroy_solid_self(game, id);
    }
}

/// Fixed source Create properties, established before caller overrides.
pub(super) fn init_entity(entity: &mut Entity) {
    if !handles(&entity.object) || entity.data.contains_key("moving_fixed_initialized") {
        return;
    }
    entity.data.insert("moving_fixed_initialized".into(), 1.0);
    match entity.object.as_str() {
        "oCeilingTrap" => {
            entity.set_image_speed(0.4);
            entity.timer = 3;
            entity.data.insert("ceiling_initialized".into(), 1.0);
            entity.data.insert("viscidTop".into(), 1.0);
        }
        "oSmashTrap" | "oSmashTrapLit" | "oThwompTrap" | "oDoor" => {
            entity.data.insert("viscidTop".into(), 1.0);
        }
        "oBoulder" => {
            entity.data.insert("invincible".into(), 1.0);
            entity.data.insert("viscidTop".into(), 1.0);
        }
        "oDarkFall" => {
            entity.data.insert("timeFall".into(), 20.0);
            entity.data.insert("viscidTop".into(), 1.0);
        }
        "oThinIce" => {
            entity.data.insert("thickness".into(), 60.0);
        }
        _ => {}
    }
}

pub(super) fn create(game: &mut Game, instance: Instance) {
    let Instance::Entity(id) = instance else {
        return;
    };
    let Some(entity) = game.world.entity(id) else {
        return;
    };
    if !handles(&entity.object) || entity.data.contains_key("moving_context_initialized") {
        return;
    }
    let direction = if matches!(entity.object.as_str(), "oSmashTrap" | "oSmashTrapLit") {
        Some(f64::from(game.rng.range(4)))
    } else {
        None
    };
    let Some(entity) = game.world.entity_mut(id) else {
        return;
    };
    entity.data.insert("moving_context_initialized".into(), 1.0);
    if let Some(direction) = direction {
        entity.data.entry("direction".into()).or_insert(direction);
    }
}

pub(super) fn animation_end(game: &mut Game, instance: Instance) {
    let Instance::Entity(id) = instance else {
        return;
    };
    let Some(entity) = game.world.entity_mut(id) else {
        return;
    };
    if entity.object == "oCeilingTrap" && entity.sprite == "sCeilingTrapS" {
        entity.set_sprite("sCeilingTrap");
    } else if entity.object == "oSpringTrap" && entity.state == 1 {
        entity.state = 0;
        entity.set_sprite("sSpringTrap");
    }
}

pub(super) fn collision(game: &mut Game, this: Instance, other: Instance) {
    let Instance::Entity(id) = this else { return };
    if this == other {
        return;
    }
    let Some(mut entity) = game.world.entity(id).cloned() else {
        return;
    };
    let other_object = other.object(game).to_owned();
    match entity.object.as_str() {
        "oBoulder" if crate::catalog::is_a(&other_object, "oSolid") => {
            let other_pos = other.position(game);
            let invincible = match other {
                Instance::Entity(other_id) => game
                    .world
                    .entity(other_id)
                    .is_some_and(|e| e.value("invincible") != 0.0 || e.object == "oHardBlock"),
                Instance::Terrain(index) => {
                    !game.world.tiles[index].is_destructible()
                        || game.world.entities.iter().any(|e| {
                            e.alive
                                && e.object == "oTerrainMetadata"
                                && e.pos == Vec2::new(other_pos.x + 8.0, other_pos.y + 8.0)
                                && e.value("invincible") != 0.0
                        })
                }
                Instance::Player => false,
            };
            if invincible || entity.vel.x.abs() < 1.0 {
                entity.vel.x *= -0.5;
            } else {
                if other_pos.y > entity.pos.y + 13.0 {
                    entity.pos.y -= 1.0;
                } else {
                    // This cleanup belongs to Boulder Collision, even when
                    // the target overrides oSolid's inherited Destroy event.
                    let decoration_point = Vec2::new(other_pos.x, other_pos.y - 16.0);
                    if let Some(decoration) = game.world.entities.iter_mut().find(|e| {
                        e.alive
                            && e.object == "oGenerationDecoration"
                            && e.value("depth") == 3.0
                            && e.sprite_bounds().contains(decoration_point)
                    }) {
                        decoration.alive = false;
                    }
                    let spike_point = Vec2::new(other_pos.x + 8.0, other_pos.y - 1.0);
                    if let Some(spikes) = game.world.entities.iter_mut().find(|e| {
                        e.alive && e.object == "oSpikes" && e.sprite_bounds().contains(spike_point)
                    }) {
                        spikes.alive = false;
                    }
                    match other {
                        Instance::Entity(other_id) => {
                            crate::items::explosives::destroy_solid_instance(game, other_id);
                        }
                        Instance::Terrain(index) => crate::items::explosives::destroy_terrain(
                            game,
                            index % game.world.width,
                            index / game.world.width,
                        ),
                        Instance::Player => {}
                    }
                    game.sound("sndCrunch", entity.pos);
                }
                entity.vel.x -= entity.vel.x.signum() * 0.1;
                if entity.vel.x.abs() < 1.0 {
                    entity.vel.x = 0.0;
                }
            }
        }
        "oDarkFall" if crate::catalog::is_a(&other_object, "oSolid") => {
            game.sound("sndBreak", entity.pos);
            game.world.spawn(
                "oSmokePuff",
                Vec2::new(entity.pos.x + 8.0, entity.pos.y + 8.0),
            );
            for _ in 0..3 {
                let pos = Vec2::new(
                    entity.pos.x + 2.0 + f64::from(game.rng.range(13)),
                    entity.pos.y + 2.0 + f64::from(game.rng.range(13)),
                );
                let rubble = game.world.spawn("oRubbleDark", pos);
                let velocity = Vec2::new(
                    f64::from(game.rng.range(3)) - f64::from(game.rng.range(3)),
                    -f64::from(game.rng.range(4)),
                );
                game.world.entity_mut(rubble).unwrap().vel = velocity;
            }
            entity.alive = false;
        }
        "oSpringTrap" if entity.state == 0 && entity.timer == 0 => {
            let sprung = match other {
                Instance::Player => {
                    if (game.player.pos.x - entity.pos.x - 8.0).abs() < 6.0
                        && matches!(
                            game.player.state,
                            crate::PlayerState::Standing
                                | crate::PlayerState::Running
                                | crate::PlayerState::Ducking
                                | crate::PlayerState::LookingUp
                                | crate::PlayerState::Stunned
                        )
                        && !matches!(game.player.sprite.as_str(), "sPExit" | "sDamselExit")
                    {
                        game.player.pos.y -= 16.0;
                        game.player.vel.y = -16.0;
                        true
                    } else {
                        false
                    }
                }
                Instance::Entity(other_id) => {
                    let Some(target) = game.world.entity_mut(other_id) else {
                        return;
                    };
                    let enemy = is_enemy(&target.object);
                    let item = is_item(&target.object);
                    let aligned =
                        (target.pos.x - entity.pos.x - if enemy { 0.0 } else { 8.0 }).abs() < 6.0;
                    if aligned
                        && ((enemy && target.value("flying") == 0.0)
                            || (item && !target.held && target.data.get("active") != Some(&0.0)))
                    {
                        target.pos.y -= if enemy { 16.0 } else { 24.0 };
                        target.vel.y = -8.0;
                        if enemy || target.object == "oDamsel" {
                            target.vel.x += if target.facing < 0 { -1.0 } else { 1.0 };
                        }
                        true
                    } else {
                        false
                    }
                }
                Instance::Terrain(_) => false,
            };
            if sprung {
                entity.state = 1;
                entity.timer = 10;
                entity.set_sprite("sSpringTrapSprung");
                game.sound("sndBoing", entity.pos);
            }
        }
        _ => return,
    }
    store(game, entity);
}

fn blood(game: &mut Game, count: usize) {
    if game.player.data.get("bloodless") == Some(&1.0) {
        return;
    }
    for _ in 0..count {
        if game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && crate::catalog::is_a(&e.object, "oDetritus"))
            .count()
            < 16
        {
            game.world.spawn("oBlood", game.player.pos);
        }
    }
}

/// Source oPlayer1 Step1563, before projectile and trap hurt.
pub(super) fn player_crush(game: &mut Game) {
    if !super::player_hurt_enabled(game)
        || !game.world.solid_at(game.player.pos.x, game.player.pos.y)
    {
        return;
    }
    if game.player.health > 0 && crate::statistics::is_real_level(game) {
        if game.world.area == crate::Area::Boss {
            game.lifetime_stats
                .enemy_death(crate::statistics::EnemyDeath::Olmec);
        } else {
            game.lifetime_stats
                .misc_death(crate::statistics::MiscDeath::Crushed);
        }
    }
    game.player.health -= 99;
    game.events.push(GameEvent::Damage { amount: 99 });
    game.player.data.insert("active".into(), 0.0);
    game.player.vel.y = -3.0;
    game.sound("sndDie", game.player.pos);
    blood(game, 3);
    game.player.data.insert("hidden".into(), 1.0);
}

/// Source oPlayer1 Step1719/1740, once per family, independent of invincibility.
pub(super) fn player_hurt(game: &mut Game) {
    // The enclosing source hurt guard was evaluated before crush/projectiles;
    // visibility or life changes inside that block do not leave it early.
    let pos = game.player.pos;
    let smash_probe = Rect::new(pos.x - 6.0, pos.y - 6.0, 13.0, 14.0);
    if game
        .world
        .entities
        .iter()
        .any(|e| e.alive && e.object == "oSmashTrap" && e.sprite_bounds().intersects(smash_probe))
    {
        // instance_nearest is independent of which instance intersects the query.
        let nearest = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oSmashTrap")
            .min_by(|a, b| a.pos.distance(pos).total_cmp(&b.pos.distance(pos)))
            .cloned()
            .unwrap();
        if game.player.health > 0 && crate::statistics::is_real_level(game) {
            game.lifetime_stats
                .misc_death(crate::statistics::MiscDeath::SmashTrap);
        }
        game.player.health -= 10;
        game.events.push(GameEvent::Damage { amount: 10 });
        let speed = 4.0 + f64::from(game.rng.range(3));
        game.player.vel.x = if nearest.pos.x + 8.0 < pos.x {
            -speed
        } else {
            speed
        };
        game.player.vel.y = if nearest.value("direction") == 1.0 {
            4.0
        } else {
            -6.0
        };
        blood(game, 1);
        if let Some(id) = game.player.held.take() {
            if let Some(held) = game.world.entity_mut(id) {
                held.held = false;
            }
        }
    }
    let ceiling_probe = Rect::new(pos.x - 2.0, pos.y - 9.0, 5.0, 3.0);
    let ceiling = game.world.entities.iter().find(|e| {
        e.alive && e.object == "oCeilingTrap" && e.sprite_bounds().intersects(ceiling_probe)
    });
    if ceiling.is_some_and(|e| e.state > 0) {
        if game.player.health > 0 && crate::statistics::is_real_level(game) {
            game.lifetime_stats
                .misc_death(crate::statistics::MiscDeath::CeilingTrap);
        }
        game.player.health -= 10;
        game.events.push(GameEvent::Damage { amount: 10 });
        blood(game, 1);
    }
}

fn bounds(entity: &Entity) -> Rect {
    match entity.object.as_str() {
        "oBoulder" => Rect::new(entity.pos.x - 14.0, entity.pos.y - 16.0, 28.0, 32.0),
        "oSmashTrap" | "oSmashTrapLit" => {
            Rect::new(entity.pos.x + 1.0, entity.pos.y + 1.0, 14.0, 14.0)
        }
        "oDoor" => Rect::new(entity.pos.x + 1.0, entity.pos.y, 14.0, 32.0),
        "oDarkFall" => Rect::new(entity.pos.x, entity.pos.y, 16.0, 8.0),
        "oPushBlock" | "oIceBlock" | "oMovingSolid" | "oPlatform" | "oThwompTrap"
        | "oCeilingTrap" => Rect::new(entity.pos.x, entity.pos.y, 16.0, 16.0),
        _ => entity.bounds(),
    }
}

fn shifted(mut rect: Rect, delta: Vec2) -> Rect {
    rect.x += delta.x;
    rect.y += delta.y;
    rect
}

fn blocked(game: &Game, entity: &Entity, delta: Vec2) -> bool {
    let rect = shifted(bounds(entity), delta);
    collides_except(game, rect, entity.id)
}

/// The platform scripts query a single edge line, rather than translating the
/// entire body. Existing overlap on another edge must not block this query.
fn edge_blocked(game: &Game, entity: &Entity, direction: u8, distance: f64) -> bool {
    let b = bounds(entity);
    let left = b.x.round_ties_even();
    let right = (b.x + b.w - 1.0).round_ties_even();
    let top = b.y.round_ties_even();
    let bottom = (b.y + b.h - 1.0).round_ties_even();
    let line = match direction {
        0 => Rect::new(
            (b.x + b.w + distance - 1.0).round_ties_even(),
            top,
            1.0,
            bottom - top + 1.0,
        ),
        1 => Rect::new(
            left,
            (b.y + b.h + distance - 1.0).round_ties_even(),
            right - left + 1.0,
            1.0,
        ),
        2 => Rect::new(
            (b.x - distance).round_ties_even(),
            top,
            1.0,
            bottom - top + 1.0,
        ),
        _ => Rect::new(
            left,
            (b.y - distance).round_ties_even(),
            right - left + 1.0,
            1.0,
        ),
    };
    precise_collides_except(game, line, entity.id)
}

/// Explicit `prec=1` source edge probes. Keep implicit event/placement and
/// explicit `prec=0` callers on their separate bounding-box path below.
fn precise_collides_except(game: &Game, rect: Rect, excluded: u64) -> bool {
    game.world
        .tiles_collide_precision(rect, "oSolid", CollisionPrecision::Precise)
        || game.world.entities.iter().any(|entity| {
            entity.id != excluded
                && entity.alive
                && entity.solid
                && !entity.held
                && entity.mask_intersects(rect, CollisionPrecision::Precise)
        })
}

fn collides_except(game: &Game, rect: Rect, excluded: u64) -> bool {
    game.world.collides_except(rect, excluded)
}

fn touches_lava(game: &Game, rect: Rect) -> bool {
    let x0 = (rect.x / 16.0).floor() as i32;
    let x1 = ((rect.x + rect.w - 0.001) / 16.0).floor() as i32;
    let y0 = (rect.y / 16.0).floor() as i32;
    let y1 = ((rect.y + rect.h - 0.001) / 16.0).floor() as i32;
    (y0..=y1).any(|y| (x0..=x1).any(|x| game.world.tile(x, y).is_some_and(|t| t.object == "oLava")))
        || game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oLava" && e.sprite_bounds().intersects(rect))
}

fn smash(game: &mut Game, entity: &mut Entity) {
    let active = entity.pos.x > game.camera.x - 16.0
        && entity.pos.x < game.camera.x + 336.0
        && entity.pos.y > game.camera.y - 16.0
        && entity.pos.y < game.camera.y + 256.0;
    if entity.state == 99 && active {
        entity.data.insert("xv".into(), 0.0);
        entity.data.insert("yv".into(), 0.0);
        entity.pos.y += 0.05;
        if touches_lava(game, Rect::new(entity.pos.x, entity.pos.y - 1.0, 1.0, 1.0)) {
            entity.alive = false;
        }
    } else if entity.state == 0 && active {
        entity.timer = entity.timer.saturating_sub(1);
        if entity.timer == 0 && entity.pos.distance(game.player.pos) < 90.0 {
            let dx = game.player.pos.x - entity.pos.x - 8.0;
            let dy = game.player.pos.y - entity.pos.y - 8.0;
            let candidates = [
                (0, Vec2::new(0.5, 0.0), dy.abs() < 8.0 && dx > 0.0),
                (1, Vec2::new(0.0, 0.5), dx.abs() < 8.0 && dy > 0.0),
                (2, Vec2::new(-0.5, 0.0), dy.abs() < 8.0 && dx < 0.0),
                (3, Vec2::new(0.0, -0.5), dx.abs() < 8.0 && dy < 0.0),
            ];
            for (direction, acceleration, aligned) in candidates {
                if aligned && !edge_blocked(game, entity, direction, 2.0) {
                    entity.state = 1;
                    entity.data.insert("direction".into(), direction as f64);
                    entity.data.insert("xa".into(), acceleration.x);
                    entity.data.insert("ya".into(), acceleration.y);
                    break;
                }
            }
        }
    } else if entity.state == 1 && active {
        let contacts = [0, 1, 2, 3].map(|direction| edge_blocked(game, entity, direction, 1.0));
        let mut xv = entity.value("xv");
        let mut yv = entity.value("yv");
        if xv.abs() < 4.0 {
            xv += entity.value("xa");
        }
        if yv.abs() < 4.0 {
            yv += entity.value("ya");
        }
        entity.data.insert("xv".into(), xv);
        entity.data.insert("yv".into(), yv);
        // Smash uses raw xv/yv, distinct from moveTo's quantized xVel/yVel.
        entity.pos.x += xv;
        entity.pos.y += yv;
        let direction = entity.value("direction") as u8;
        let mut hit = entity.value("hit") != 0.0;
        if contacts[usize::from(direction.min(3))] {
            let correction = if edge_blocked(game, entity, direction, 2.0) {
                3.0
            } else {
                1.0
            };
            match direction {
                0 => entity.pos.x -= correction,
                1 => entity.pos.y -= correction,
                2 => entity.pos.x += correction,
                _ => entity.pos.y += correction,
            }
            hit = true;
        }
        let lord_probe = Rect::new(entity.pos.x - 1.0, entity.pos.y - 1.0, 19.0, 19.0);
        hit |= game.world.entities.iter().any(|other| {
            other.alive
                && other.object == "oTombLord"
                && other.sprite_bounds().intersects(lord_probe)
        });
        if hit {
            entity.data.insert("xv".into(), 0.0);
            entity.data.insert("yv".into(), 0.0);
            entity.data.insert("xa".into(), 0.0);
            entity.data.insert("ya".into(), 0.0);
            entity.data.insert("hit".into(), 1.0);
        }
        if hit && !contacts.into_iter().any(|contact| contact) {
            entity.state = 0;
            entity.timer = 50;
            entity.data.insert("hit".into(), 0.0);
        }
    }
    if active
        && touches_lava(
            game,
            Rect::new(entity.pos.x + 1.0, entity.pos.y + 1.0, 15.0, 15.0),
        )
    {
        entity.state = 99;
    }
}

/// Source global platform movement goes X then Y. It does not collide the
/// moving solid with terrain; its own object Step handles those contacts.
/// Only a side-pushed character blocked by another solid stops this pass.
fn move_solid(game: &mut Game, entity: &mut Entity, delta: Vec2, carried: &mut bool) -> bool {
    let can_carry = !*carried;
    let mut obstructed = false;
    let viscid =
        entity
            .data
            .get("viscidTop")
            .copied()
            .unwrap_or(if entity.object == "oMovingSolid" {
                0.0
            } else {
                1.0
            })
            != 0.0;
    for (horizontal, speed) in [(true, delta.x), (false, delta.y)] {
        let distance = scheduled_delta(speed, game.engine_time());
        let step = if horizontal {
            Vec2::new(distance.signum(), 0.0)
        } else {
            Vec2::new(0.0, distance.signum())
        };
        for _ in 0..distance.abs() as u32 {
            let player = game.player.bounds();
            let lb = player.x.round_ties_even();
            let rb = (player.x + player.w - 1.0).round_ties_even();
            let tb = player.y.round_ties_even();
            let bb = (player.y + player.h).round_ties_even();
            let top_line = Rect::new(lb, bb, rb - lb + 1.0, 1.0);
            let bottom_line = Rect::new(lb, tb - 1.0, rb - lb + 1.0, 1.0);
            let rider = if horizontal {
                viscid && can_carry && entity.mask_intersects(top_line, CollisionPrecision::Precise)
            } else if step.y > 0.0 {
                viscid
                    && entity.mask_intersects(
                        shifted(top_line, Vec2::new(0.0, 1.0)),
                        CollisionPrecision::Precise,
                    )
            } else {
                entity.mask_intersects(top_line, CollisionPrecision::Precise)
            };
            let side = if step.x > 0.0 {
                Rect::new(lb - 1.0, tb, 1.0, (bb - 1.0).round_ties_even() - tb + 1.0)
            } else {
                Rect::new(rb + 1.0, tb, 1.0, (bb - 1.0).round_ties_even() - tb + 1.0)
            };
            let touching = if horizontal {
                entity.mask_intersects(side, CollisionPrecision::Precise)
            } else {
                entity.mask_intersects(bottom_line, CollisionPrecision::Precise)
            };
            // oGame cancels variable jumping under a rising platform without
            // attaching the character to its underside.
            if !horizontal && step.y < 0.0 && touching && game.player.jump_time < 10 {
                game.player.vel.y = -2.0;
                game.player.jump_time = 10;
            }
            let obstruction = if horizontal {
                Rect::new(
                    if step.x > 0.0 { rb + 1.0 } else { lb - 1.0 },
                    tb,
                    1.0,
                    bb - tb,
                )
            } else {
                Rect::new(
                    lb,
                    if step.y > 0.0 { bb } else { tb - 1.0 },
                    rb - lb + 1.0,
                    1.0,
                )
            };
            if rider {
                if !precise_collides_except(game, obstruction, entity.id) {
                    game.player.pos.x += step.x;
                    game.player.pos.y += step.y;
                    game.player.grounded = true;
                    if horizontal {
                        *carried = true;
                    }
                } else if !horizontal && step.y < 0.0 {
                    obstructed = true;
                    break;
                }
            } else if touching && (horizontal || step.y > 0.0) {
                if precise_collides_except(game, obstruction, entity.id) {
                    obstructed = true;
                    break;
                }
                game.player.pos.x += step.x;
                game.player.pos.y += step.y;
            }
            entity.pos.x += step.x;
            entity.pos.y += step.y;
        }
    }
    obstructed
}

fn scheduled_delta(speed: f64, tick: u64) -> f64 {
    let magnitude = speed.abs();
    let fraction = magnitude.fract();
    let period = if fraction > 0.0 {
        (1.0 / fraction).round_ties_even() as u64
    } else {
        0
    };
    let extra = if period > 0 && tick % period == 0 {
        1.0
    } else {
        0.0
    };
    (magnitude.floor() + extra) * speed.signum()
}

/// Smash traps use their own xv/yv, rather than the platform engine's xVel/yVel.
/// Consequently the collision event must see their overlap with a character.
fn move_body(game: &Game, entity: &mut Entity, delta: Vec2, precision: CollisionPrecision) -> bool {
    let steps = delta.x.abs().max(delta.y.abs()).ceil() as u32;
    if steps == 0 {
        return false;
    }
    let step = Vec2::new(delta.x / f64::from(steps), delta.y / f64::from(steps));
    for _ in 0..steps {
        let blocked = if precision == CollisionPrecision::Precise {
            // moveTo's getIdCollisionLeft/Right starts five pixels below
            // the custom top; vertical isCollisionTop/Bottom uses full width.
            let body = bounds(entity);
            let probe = if step.x != 0.0 {
                let top = (body.y + 5.0).round_ties_even();
                let bottom = (body.y + body.h - 1.0).round_ties_even();
                Rect::new(
                    (if step.x > 0.0 {
                        body.x + body.w
                    } else {
                        body.x - 1.0
                    })
                    .round_ties_even(),
                    top,
                    1.0,
                    bottom - top + 1.0,
                )
            } else {
                let left = body.x.round_ties_even();
                let right = (body.x + body.w - 1.0).round_ties_even();
                Rect::new(
                    left,
                    (if step.y > 0.0 {
                        body.y + body.h
                    } else {
                        body.y - 1.0
                    })
                    .round_ties_even(),
                    right - left + 1.0,
                    1.0,
                )
            };
            precise_collides_except(game, probe, entity.id)
        } else {
            blocked(game, entity, step)
        };
        if blocked {
            return true;
        }
        entity.pos.x += step.x;
        entity.pos.y += step.y;
    }
    false
}

fn thwomp(game: &mut Game, entity: &mut Entity) {
    let source = entity.sprite_bounds();
    let target = game.player.sprite_bounds();
    let dx = (source.x - (target.x + target.w - 1.0))
        .max(target.x - (source.x + source.w - 1.0))
        .max(0.0);
    let dy = (source.y - (target.y + target.h - 1.0))
        .max(target.y - (source.y + source.h - 1.0))
        .max(0.0);
    match entity.state {
        0 if game.player.pos.y > entity.pos.y
            && dx.hypot(dy) < 96.0
            && (game.player.pos.x - entity.pos.x).abs() < 8.0 =>
        {
            entity.state = 1
        }
        1 => {
            entity.vel.y = (entity.vel.y + 1.0).min(6.0);
            if edge_blocked(game, entity, 1, 1.0) {
                entity.state = 2;
                entity.vel.y = 0.0;
                entity.timer = 100;
            }
        }
        2 => {
            if edge_blocked(game, entity, 1, 1.0) {
                entity.pos.y -= 1.0;
            }
            if entity.timer > 0 {
                entity.timer -= 1;
            } else {
                entity.state = 3;
                entity.vel.y = -1.0;
            }
        }
        3 if edge_blocked(game, entity, 3, 1.0) => {
            entity.vel.y = 0.0;
            entity.state = 0;
        }
        _ => {}
    }
}

fn door(game: &mut Game, entity: &mut Entity) {
    match entity.state {
        1 => {
            entity.vel.y = (entity.vel.y + 1.0).min(6.0);
            if edge_blocked(game, entity, 1, 1.0) {
                entity.state = 2;
                entity.vel.y = 0.0;
                entity.timer = 100;
                entity.data.insert("depth".into(), 100.0);
            }
        }
        2 => {
            if edge_blocked(game, entity, 1, 1.0) {
                entity.pos.y -= 1.0;
            }
        }
        _ => {}
    }
}

fn ceiling(game: &mut Game, entity: &mut Entity) {
    if !entity.data.contains_key("ceiling_initialized") {
        entity.data.insert("ceiling_initialized".into(), 1.0);
        entity.set_image_speed(0.4);
        if entity.timer == 0 {
            entity.timer = 3;
        }
    }
    if entity.state == 1 {
        if entity.timer > 0 {
            entity.timer -= 1;
        } else {
            entity.timer = 3;
            // This trap deliberately descends into the player instead of carrying.
            entity.pos.y += 1.0;
        }
        entity.vel.y = 0.0;
        if game.world.solid_point_except(
            Vec2::new(entity.pos.x + 8.0, entity.pos.y + 17.0),
            entity.id,
        ) {
            entity.state = 2;
        }
        if entity.sprite == "sBlock" {
            entity.set_sprite("sCeilingTrapS");
        }
    } else if entity.state == 2 {
        entity.vel.y = 0.0;
        if edge_blocked(game, entity, 1, 1.0) {
            entity.pos.y -= 1.0;
        }
    }
}

fn boulder(game: &mut Game, entity: &mut Entity) {
    let dx = scheduled_delta(entity.vel.x, game.engine_time());
    let dy = scheduled_delta(entity.vel.y, game.engine_time());
    move_body(
        game,
        entity,
        Vec2::new(dx, 0.0),
        CollisionPrecision::Precise,
    );
    move_body(
        game,
        entity,
        Vec2::new(0.0, dy),
        CollisionPrecision::Precise,
    );
    if entity.vel.y < 8.0 {
        entity.vel.y += 0.6;
    }
    if entity.pos.x - 17.0 <= 16.0 && entity.vel.x < 0.0 {
        entity.pos.x += 1.0;
        entity.vel.x = -entity.vel.x;
    }
    if entity.pos.x + 17.0 >= 656.0 && entity.vel.x > 0.0 {
        entity.pos.x -= 1.0;
        entity.vel.x = -entity.vel.x;
    }
    if edge_blocked(game, entity, 3, 1.0) && entity.vel.y < 0.0 {
        entity.vel.y = -entity.vel.y * 0.8;
    }
    if edge_blocked(game, entity, 1, 1.0) {
        entity.vel.y = if entity.vel.y > 3.0 {
            -entity.vel.y * 0.3
        } else {
            0.0
        };
        entity.vel.x *= 0.99;
        if entity.value("bounced") == 0.0 && entity.vel.x == 0.0 {
            entity.vel.x = if game.player.pos.x < entity.pos.x {
                -4.5
            } else {
                4.5
            };
            entity.data.insert("bounced".into(), 1.0);
        }
        if entity.vel.x.abs() < 0.5 {
            entity.vel.x = 0.0;
        }
    }
    if !game
        .world
        .solid_point_except(Vec2::new(entity.pos.x, entity.pos.y + 16.0), entity.id)
    {
        let left = collides_except(
            game,
            Rect::new(entity.pos.x - 16.0, entity.pos.y - 16.0, 9.0, 33.0),
            entity.id,
        );
        let right = collides_except(
            game,
            Rect::new(entity.pos.x + 8.0, entity.pos.y - 16.0, 9.0, 33.0),
            entity.id,
        );
        if left && !right {
            entity.pos.x += 1.0;
        } else if right && !left {
            entity.pos.x -= 1.0;
        }
    }
    entity.set_image_speed(entity.vel.x.abs() / 5.0);
    entity.set_sprite(if entity.vel.x < 0.0 {
        "sBoulderRotateL"
    } else if entity.vel.x > 0.0 {
        "sBoulderRotateR"
    } else {
        "sBoulder"
    });
}

fn push_block(game: &mut Game, entity: &mut Entity) {
    if touches_lava(
        game,
        Rect::new(entity.pos.x + 8.0, entity.pos.y + 14.0, 1.0, 1.0),
    ) && !game.world.solid_point_except(
        Vec2::new(entity.pos.x + 8.0, entity.pos.y + 17.0),
        entity.id,
    ) {
        entity.vel.y = 0.0;
        entity.data.insert("push_gravity".into(), 0.0);
        // scrMoveableSolidRecurseDrop moves the entire stack by the same
        // sinking amount before the lowest block descends.
        let mut current = bounds(entity);
        let mut excluded = vec![entity.id];
        while let Some(above) = game
            .world
            .entities
            .iter()
            .find(|e| {
                e.alive
                    && !excluded.contains(&e.id)
                    && crate::catalog::is_a(&e.object, "oMoveableSolid")
                    && e.pos.y < current.y
                    && shifted(current, Vec2::new(0.0, -1.0)).intersects(e.bounds())
            })
            .map(|e| (e.id, bounds(e)))
        {
            excluded.push(above.0);
            current = above.1;
        }
        for id in excluded.into_iter().skip(1) {
            if let Some(above) = game.world.entity_mut(id) {
                above.pos.y += 0.05;
            }
        }
        entity.pos.y += 0.05;
        return;
    }
}

fn ice_block(game: &mut Game, entity: &mut Entity) {
    if touches_lava(
        game,
        Rect::new(entity.pos.x + 8.0, entity.pos.y + 16.0, 1.0, 1.0),
    ) && !game.world.solid_point_except(
        Vec2::new(entity.pos.x + 8.0, entity.pos.y + 17.0),
        entity.id,
    ) {
        entity.vel.y = 0.0;
        entity.data.insert("push_gravity".into(), 0.0);
        entity.pos.y += 0.05;
    }
    if entity.pos.y > 576.0 {
        entity.alive = false;
    }
}

fn thin_ice(game: &mut Game, entity: &mut Entity) {
    let mut thickness = entity.data.get("thickness").copied().unwrap_or(60.0);
    if Rect::new(entity.pos.x, entity.pos.y - 1.0, 17.0, 3.0)
        .intersects(game.player.sprite_bounds())
    {
        thickness -= 2.0;
        if game.rng.range(100) == 0 {
            let x = entity.pos.x + f64::from(game.rng.range(17));
            game.world.spawn("oDrip", Vec2::new(x, entity.pos.y + 9.0));
        }
    }
    entity.data.insert("thickness".into(), thickness);
    if thickness <= 0.0 {
        entity.alive = false;
    } else {
        entity.sprite = format!("sThinIce{}", ((60.0 - thickness) / 10.0).floor() as u32 + 1);
    }
}
fn dark_fall(game: &mut Game, entity: &mut Entity) {
    let player = game.player.bounds();
    let left = player.x.round_ties_even();
    let right = (player.x + player.w - 1.0).round_ties_even();
    let touching = entity.mask_intersects(
        Rect::new(
            left,
            (player.y + player.h).round_ties_even(),
            right - left + 1.0,
            1.0,
        ),
        CollisionPrecision::Precise,
    );
    let mut time = entity.data.get("timeFall").copied().unwrap_or(20.0);
    if touching {
        time -= 1.0;
        if time <= 0.0 {
            entity.data.insert("yAcc".into(), 1.0);
        }
    } else if time < 20.0 {
        time += 1.0;
    }
    entity.data.insert("timeFall".into(), time);
    entity.vel.y = entity.vel.y.min(10.0);
}

fn spring(game: &mut Game, entity: &mut Entity) {
    let active = entity.pos.x > game.camera.x - 16.0
        && entity.pos.x < game.camera.x + 320.0
        && entity.pos.y > game.camera.y - 16.0
        && entity.pos.y < game.camera.y + 240.0;
    if active {
        entity.timer = entity.timer.saturating_sub(1);
    }
    let support = Rect::new(entity.pos.x, entity.pos.y + 16.0, 1.0, 1.0);
    if active
        && !game.world.collides(support)
        && !game
            .world
            .entities
            .iter()
            .any(|e| e.id != entity.id && e.alive && e.solid && bounds(e).intersects(support))
    {
        entity.alive = false;
        return;
    }
}

fn is_enemy(object: &str) -> bool {
    crate::catalog::is_a(object, "oEnemy")
}

fn is_item(object: &str) -> bool {
    crate::catalog::is_a(object, "oItem")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RulesPreset;

    fn authored_standing_game() -> Game {
        let mut level = crate::level::Level::default();
        // Authored life is a source-supported level setting; the 10-point
        // trap controls stay alive and prove counters are pre-damage checks.
        level.starting_life = 20;
        level.set_tile(6, 7, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 8, '1').unwrap();
        }
        let mut game = Game::shell(13, RulesPreset::Classic);
        game.load_custom(&level, "moving-phase", "");
        for _ in 0..8 {
            game.step(&crate::InputFrame::default());
        }
        assert!(game.player.grounded);
        game
    }

    #[test]
    fn game_step_thin_ice_self_destroy_runs_callbacks_despite_attack_immunity() {
        let mut game = authored_standing_game();
        let pos = Vec2::new(game.player.pos.x - 8.0, game.player.pos.y + 8.0);
        let ice = game.world.spawn("oThinIce", pos);
        let spikes = game.world.spawn("oSpikes", Vec2::new(pos.x, pos.y - 16.0));
        // Controlled final-thickness and attack-immunity flags isolate the
        // self-event contract. Player interaction uses its settled position.
        let entity = game.world.entity_mut(ice).unwrap();
        entity.data.insert("thickness".into(), 2.0);
        entity.data.insert("invincible".into(), 1.0);
        assert!(!crate::items::explosives::destroy_solid_instance(
            &mut game, ice
        ));
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(ice).is_none());
        assert!(game.world.entity(spikes).is_none());
    }

    #[test]
    fn push_block_outside_room_self_event_bypasses_attack_immunity_once() {
        let mut game = authored_standing_game();
        let id = game.world.spawn("oPushBlock", Vec2::new(-32.0, 80.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("invincible".into(), 1.0);
        assert!(!crate::items::explosives::destroy_solid_instance(
            &mut game, id
        ));
        // Invoke the source Outside Room callback with an explicitly outside
        // controlled fixture; engine spatial dispatch is tested separately.
        outside_room(&mut game, Instance::Entity(id));
        let entity = game.world.entities.iter().find(|e| e.id == id).unwrap();
        assert!(!entity.alive);
        assert_eq!(entity.value("destroy_done"), 1.0);
        let count = game.world.entities.len();
        outside_room(&mut game, Instance::Entity(id));
        assert_eq!(game.world.entities.len(), count);
    }

    #[test]
    fn game_step_smash_hurt_is_once_per_source_family_and_excludes_lit_sibling() {
        for (object, damage) in [("oSmashTrap", 10), ("oSmashTrapLit", 0)] {
            for real_level in [false, true] {
                let mut game = authored_standing_game();
                // Controlled eligibility fixture, not a claimed campaign route.
                if real_level {
                    game.flags.remove("custom");
                    game.flags.remove("custom_level");
                }
                let health = game.player.health;
                for _ in 0..2 {
                    let id = game.world.spawn(
                        object,
                        Vec2::new(game.player.pos.x + 6.0, game.player.pos.y - 5.0),
                    );
                    game.world.entity_mut(id).unwrap().timer = 100;
                }
                game.step(&crate::InputFrame::default());
                assert_eq!(
                    game.player.health,
                    (health - damage).max(0),
                    "{object}/{real_level}"
                );
                assert_eq!(
                    game.events
                        .iter()
                        .filter(|event| matches!(event, GameEvent::Damage { amount: 10 }))
                        .count(),
                    usize::from(damage != 0)
                );
                assert_eq!(
                    game.lifetime_stats.misc_deaths[8],
                    u64::from(real_level && damage != 0)
                );
                assert_eq!(game.lifetime_stats.misc_deaths[2], 0);
            }
        }
    }

    #[test]
    fn game_step_ceiling_head_contact_counts_only_real_level_positive_life() {
        for real_level in [false, true] {
            let mut game = authored_standing_game();
            if real_level {
                game.flags.remove("custom");
                game.flags.remove("custom_level");
            }
            let health = game.player.health;
            let id = game.world.spawn(
                "oCeilingTrap",
                Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 24.0),
            );
            let ceiling = game.world.entity_mut(id).unwrap();
            ceiling.state = 1;
            ceiling.timer = 3;
            game.step(&crate::InputFrame::default());
            assert_eq!(game.player.health, (health - 10).max(0));
            assert_eq!(game.lifetime_stats.misc_deaths[9], u64::from(real_level));
            assert_eq!(game.lifetime_stats.misc_deaths[2], 0);
        }
    }

    #[test]
    fn game_step_boulder_overlap_has_generic_crush_cause_never_boulder_counter() {
        for (boss, real_level) in [(false, false), (false, true), (true, true)] {
            let mut game = authored_standing_game();
            if real_level {
                game.flags.remove("custom");
                game.flags.remove("custom_level");
            }
            if boss {
                game.world.area = crate::Area::Boss;
                game.world.level = 16;
            }
            let health = game.player.health;
            game.world.spawn("oBoulder", game.player.pos);
            game.step(&crate::InputFrame::default());
            assert_eq!(game.player.health, (health - 99).max(0));
            assert_eq!(game.player.data.get("hidden"), Some(&1.0));
            assert_eq!(
                game.lifetime_stats.misc_deaths[2],
                u64::from(real_level && !boss)
            );
            assert_eq!(
                game.lifetime_stats.enemy_deaths[22],
                u64::from(real_level && boss)
            );
            assert_eq!(game.lifetime_stats.misc_deaths[5], 0);
        }
    }

    #[test]
    fn game_step_smash_arriving_after_player_hurt_waits_until_next_step() {
        let mut game = authored_standing_game();
        let health = game.player.health;
        let id = game.world.spawn(
            "oSmashTrap",
            Vec2::new(game.player.pos.x + 10.0, game.player.pos.y - 8.0),
        );
        let trap = game.world.entity_mut(id).unwrap();
        trap.state = 1;
        trap.data.insert("direction".into(), 2.0);
        trap.data.insert("xv".into(), -4.0);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.health, health);
        assert_eq!(
            game.world.entity(id).unwrap().pos.x,
            game.player.pos.x + 6.0
        );
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.health, (health - 10).max(0));
    }

    #[test]
    fn game_step_boulder_solid_collision_destroys_support_and_explicit_spikes() {
        for (object, destroyed) in [("oBlock", true), ("oHardBlock", false)] {
            let mut game = authored_standing_game();
            game.world.set_tile(12, 5, object);
            let spikes = game.world.spawn("oSpikes", Vec2::new(192.0, 64.0));
            let id = game.world.spawn("oBoulder", Vec2::new(184.0, 80.0));
            game.world.entity_mut(id).unwrap().vel.x = 2.0;
            game.step(&crate::InputFrame::default());
            assert_eq!(
                game.world.tiles[5 * game.world.width + 12]
                    .object
                    .is_empty(),
                destroyed
            );
            assert_eq!(game.world.entity(spikes).is_none(), destroyed);
            assert_eq!(
                game.world.entity(id).unwrap().vel.x,
                if destroyed { 1.9 } else { -1.0 }
            );
        }
    }

    #[test]
    fn game_step_boulder_gravity_can_cross_eight_before_next_source_step() {
        let mut game = authored_standing_game();
        let id = game.world.spawn("oBoulder", Vec2::new(260.0, 64.0));
        game.world.entity_mut(id).unwrap().vel.y = 7.8;
        game.step(&crate::InputFrame::default());
        assert!((game.world.entity(id).unwrap().vel.y - 8.4).abs() < 1e-9);
    }

    #[test]
    fn game_step_ceiling_animation_end_preserves_wrapped_fraction() {
        let mut game = authored_standing_game();
        let id = game.world.spawn("oCeilingTrap", Vec2::new(200.0, 64.0));
        let ceiling = game.world.entity_mut(id).unwrap();
        ceiling.state = 1;
        ceiling.timer = 3;
        ceiling.set_sprite("sCeilingTrapS");
        ceiling.animation = 5.8;
        ceiling.set_image_speed(0.4);
        game.step(&crate::InputFrame::default());
        let ceiling = game.world.entity(id).unwrap();
        assert_eq!(ceiling.sprite, "sCeilingTrap");
        assert!((ceiling.animation - 0.2).abs() < 1e-9);
    }

    #[test]
    fn game_step_platform_carry_follows_reactivated_source_array_in_either_storage_order() {
        for reverse_storage in [false, true] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            game.world = crate::World::new(80, 40, crate::Area::Mines, 1);
            game.world.name = "platform-order-fixture".into();
            game.player.pos = Vec2::new(96.0, 72.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Standing;
            game.player.grounded = true;
            game.player.health = 99;
            let thwomp = game.spawn("oThwompTrap", Vec2::new(80.0, 79.0));
            let boulder = game.spawn("oBoulder", Vec2::new(112.0, 96.0));
            game.spawn("oPoof", Vec2::new(600.0, 64.0));
            game.step(&crate::InputFrame::default());
            // Re-enter oPoof's actual source Step after oLevel deactivation.
            crate::dispatcher::activate_object(&mut game, "oPoof");
            // Controlled pose/velocity reset isolates shared platform ownership
            // after reactivation; this is not a claimed natural campaign route.
            game.player.pos = Vec2::new(96.0, 72.0);
            game.player.previous_pos = game.player.pos;
            game.player.vel = Vec2::default();
            game.player.health = 99;
            game.player.stun = 0;
            game.player.grounded = true;
            game.player.state = PlayerState::Standing;
            let trap = game.world.entity_mut(thwomp).unwrap();
            trap.pos = Vec2::new(80.0, 79.0);
            trap.vel = Vec2::new(0.0, -1.0);
            trap.state = 3;
            let rock = game.world.entity_mut(boulder).unwrap();
            rock.pos = Vec2::new(112.0, 96.0);
            rock.vel = Vec2::new(1.0, 0.0);
            if reverse_storage {
                game.world.entities.reverse();
            }
            assert_eq!(
                crate::dispatcher::descendant_instances(&game, "oMovingSolid"),
                vec![Instance::Entity(boulder), Instance::Entity(thwomp)]
            );
            game.step(&crate::InputFrame::default());
            assert_eq!(
                game.player.pos,
                Vec2::new(97.0, 71.0),
                "reverse_storage={reverse_storage}"
            );
            assert_eq!(
                game.world.entity(thwomp).unwrap().pos,
                Vec2::new(80.0, 78.0)
            );
            assert_eq!(
                game.world.entity(boulder).unwrap().pos,
                Vec2::new(114.0, 96.0)
            );
        }
    }

    #[test]
    fn global_platform_carry_rejects_transparent_corner_and_accepts_opaque_neighbor() {
        // sDarkFall row zero is .##############.; the source's precise
        // character-bottom line therefore differs from its bounding box.
        for (player_x, expected_x, expected_y, expected_countdown) in
            [(76.0, 76.0, 73.0, 20.0), (77.0, 78.0, 72.0, 19.0)]
        {
            let mut game = Game::shell(1, RulesPreset::Classic);
            let id = game.world.spawn("oDarkFall", Vec2::new(80.0, 80.0));
            game.world.entity_mut(id).unwrap().vel.x = 1.0;
            game.player.pos = Vec2::new(player_x, 72.0);
            game.player.state = PlayerState::Standing;
            game.player.grounded = true;
            game.step(&crate::InputFrame::default());
            assert_eq!(game.world.entity(id).unwrap().pos.x, 81.0);
            assert_eq!(game.player.pos, Vec2::new(expected_x, expected_y));
            assert_eq!(
                game.world.entity(id).unwrap().value("timeFall"),
                expected_countdown
            );
        }
    }

    #[test]
    fn thwomp_precise_floor_line_distinguishes_transparent_and_opaque_platform_pixels() {
        for (trap_x, expected_state, expected_velocity) in [(65.0, 1, 1.0), (66.0, 2, 0.0)] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            let id = game.world.spawn("oThwompTrap", Vec2::new(trap_x, 64.0));
            game.world.entity_mut(id).unwrap().state = 1;
            game.world.spawn("oDarkFall", Vec2::new(80.0, 80.0));
            game.step(&crate::InputFrame::default());
            let trap = game.world.entity(id).unwrap();
            assert_eq!(trap.state, expected_state);
            assert_eq!(trap.vel.y, expected_velocity);
        }
    }

    #[test]
    fn rising_platform_ends_variable_jump_under_its_underside() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oThwompTrap", Vec2::new(80.0, 64.0));
        let trap = game.world.entity_mut(id).unwrap();
        trap.state = 3;
        trap.vel.y = -2.0;
        game.player.pos = Vec2::new(88.0, 88.0);
        game.player.state = PlayerState::Jumping;
        game.player.vel.y = -4.0;
        game.player.jump_time = 2;
        game.step(&crate::InputFrame {
            jump: true,
            ..Default::default()
        });
        assert_eq!(game.world.entity(id).unwrap().pos.y, 62.0);
        assert_eq!(game.player.jump_time, 10);
        assert!(game.player.vel.y > -3.0);
    }

    #[test]
    fn blocked_side_push_still_moves_platform_vertically() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(7, 5, "oHardBlock");
        let id = game.world.spawn("oMovingSolid", Vec2::new(86.0, 80.0));
        let platform = game.world.entity_mut(id).unwrap();
        platform.sprite = "sBlock".into();
        platform.vel = Vec2::new(1.0, 1.0);
        game.player.pos = Vec2::new(107.0, 88.0);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos, Vec2::new(86.0, 81.0));
    }

    #[test]
    fn platform_carry_can_escape_an_existing_side_overlap() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(4, 4, "oHardBlock");
        let id = game.world.spawn("oMovingSolid", Vec2::new(80.0, 80.0));
        let platform = game.world.entity_mut(id).unwrap();
        platform.sprite = "sBlock".into();
        platform.vel.x = 1.0;
        platform.data.insert("viscidTop".into(), 1.0);
        game.player.pos = Vec2::new(83.0, 72.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.pos.x, 84.0);
    }

    #[test]
    fn frozen_ice_block_inherits_gravity_and_lava_sinking() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oIceBlock", Vec2::new(80.0, 64.0));
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.y, 65.0);
        game.world.set_tile(5, 5, "oLava");
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().value("push_gravity"), 0.0);
        assert_eq!(game.world.entity(id).unwrap().vel.y, 0.0);
        assert!((game.world.entity(id).unwrap().pos.y - 67.05).abs() < 0.001);
    }

    #[test]
    fn falling_platform_floor_event_uses_nine_pixel_sprite_mask() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(5, 6, "oHardBlock");
        let id = game.world.spawn("oDarkFall", Vec2::new(80.0, 88.0));
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert!(game.world.entities.iter().any(|e| e.object == "oSmokePuff"));
    }

    #[test]
    fn thin_ice_self_destruction_runs_inherited_supported_fixture_cleanup() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let ice = game.world.spawn("oThinIce", Vec2::new(80.0, 80.0));
        game.world
            .entity_mut(ice)
            .unwrap()
            .data
            .insert("thickness".into(), 2.0);
        let spikes = game.world.spawn("oSpikes", Vec2::new(80.0, 64.0));
        game.player.pos = Vec2::new(88.0, 72.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(ice).is_none());
        assert!(game.world.entity(spikes).is_none());
    }

    #[test]
    fn offscreen_spring_preserves_cooldown_and_support_until_visible() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oSpringTrap", Vec2::new(400.0, 80.0));
        game.world.entity_mut(id).unwrap().timer = 4;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().timer, 4);
        game.camera.x = 200.0;
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).is_none());
    }

    #[test]
    fn thwomp_activation_uses_sprite_distance_instead_of_origin_distance() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oThwompTrap", Vec2::new(80.0, 80.0));
        game.player.pos = Vec2::new(80.0, 184.0);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().state, 1);
    }

    #[test]
    fn offscreen_smash_trap_waits_for_source_viewport_activation() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oSmashTrap", Vec2::new(400.0, 80.0));
        let trap = game.world.entity_mut(id).unwrap();
        trap.state = 1;
        trap.data.insert("xv".into(), 2.0);
        trap.data.insert("xa".into(), 0.5);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.x, 400.0);
        assert_eq!(game.world.entity(id).unwrap().value("xv"), 2.0);
    }

    #[test]
    fn smash_uses_raw_velocity_then_source_contact_correction() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(6, 5, "oHardBlock");
        let id = game.world.spawn("oSmashTrap", Vec2::new(81.0, 80.0));
        let trap = game.world.entity_mut(id).unwrap();
        trap.state = 1;
        trap.data.insert("xv".into(), 4.0);
        trap.data.insert("direction".into(), 0.0);
        game.step(&crate::InputFrame::default());
        let trap = game.world.entity(id).unwrap();
        assert_eq!(trap.pos.x, 82.0); // 81 + xv4 - source correction3
        assert_eq!(trap.vel.x, 0.0);
        assert_eq!(trap.value("hit"), 1.0);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.x, 79.0);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().state, 0);
        assert_eq!(game.world.entity(id).unwrap().timer, 50);
    }

    #[test]
    fn ceiling_drop_checks_source_center_point_seventeen_pixels_below_origin() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oCeilingTrap", Vec2::new(80.0, 64.0));
        game.world.entity_mut(id).unwrap().state = 1;
        game.world.spawn("oDoor", Vec2::new(80.0, 81.0));
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.y, 64.0);
        assert_eq!(game.world.entity(id).unwrap().state, 2);
    }

    #[test]
    fn boulder_uses_explicit_classic_side_limits_in_an_open_world() {
        for (start, velocity, expected, expected_velocity) in
            [(34.0, -1.0, 33.0, 1.0), (638.0, 1.0, 639.0, -1.0)]
        {
            let mut game = Game::shell(1, RulesPreset::Classic);
            let id = game.world.spawn("oBoulder", Vec2::new(start, 80.0));
            game.world.entity_mut(id).unwrap().vel.x = velocity;
            game.step(&crate::InputFrame::default());
            assert_eq!(game.world.entity(id).unwrap().pos.x, expected);
            assert_eq!(game.world.entity(id).unwrap().vel.x, expected_velocity);
        }
    }

    #[test]
    fn olmec_receives_global_and_own_movement_and_both_source_carries() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oOlmec", Vec2::new(80.0, 80.0));
        let boss = game.world.entity_mut(id).unwrap();
        crate::enemies::init_entity(boss);
        boss.sprite = "sOlmec".into();
        boss.state = 0;
        boss.vel.x = 1.0;
        boss.data.insert("carryPlayer".into(), 1.0);
        game.player.pos = Vec2::new(112.0, 72.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.x, 82.0);
        assert_eq!(game.player.pos.x, 114.0);
        assert_eq!(game.world.entity(id).unwrap().value("carryPlayer"), 1.0);
    }

    #[test]
    fn olmec_own_carry_uses_raw_fractional_velocity_before_quantized_move() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oOlmec", Vec2::new(80.0, 80.0));
        let boss = game.world.entity_mut(id).unwrap();
        crate::enemies::init_entity(boss);
        boss.sprite = "sOlmec".into();
        boss.state = 0;
        boss.vel.x = 0.25;
        boss.data.insert("carryPlayer".into(), 1.0);
        game.player.pos = Vec2::new(112.0, 72.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.engine_time(), 2);
        assert_eq!(game.world.entity(id).unwrap().pos.x, 80.0);
        assert_eq!(game.player.pos.x, 112.25);
    }
    #[test]
    fn spring_launch_matches_reference_and_cannot_retrigger_during_cooldown() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(5, 6, "oBlock");
        let id = game.world.spawn("oSpringTrap", Vec2::new(80.0, 80.0));
        game.player.pos = Vec2::new(88.0, 88.0);
        game.player.state = PlayerState::Standing;
        tick(&mut game);
        collision(&mut game, Instance::Entity(id), Instance::Player);
        assert_eq!(game.player.vel.y, -16.0);
        assert_eq!(game.player.pos.y, 72.0);
        assert_eq!(game.world.entity(id).unwrap().timer, 10);
        game.player.pos = Vec2::new(88.0, 88.0);
        tick(&mut game);
        collision(&mut game, Instance::Entity(id), Instance::Player);
        assert_eq!(game.player.pos.y, 88.0);
    }
    #[test]
    fn smash_accelerates_only_after_alignment_trigger() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oSmashTrap", Vec2::new(80.0, 80.0));
        game.player.pos = Vec2::new(140.0, 88.0);
        tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().state, 1);
        assert_eq!(game.world.entity(id).unwrap().pos.x, 80.0);
        tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().value("xv"), 0.5);
        assert_eq!(game.world.entity(id).unwrap().pos.x, 80.5);
    }
    #[test]
    fn boulder_first_landing_rolls_toward_player() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        for x in 0..game.world.width {
            game.world.set_tile(x, 8, "oHardBlock");
        }
        let id = game.world.spawn("oBoulder", Vec2::new(160.0, 112.0));
        game.player.pos = Vec2::new(40.0, 112.0);
        tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().vel.x, -4.5);
        assert_eq!(game.world.entity(id).unwrap().vel.y, 0.0);
    }
    #[test]
    fn ceiling_drop_advances_one_pixel_every_four_ticks() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oCeilingTrap", Vec2::new(160.0, 64.0));
        game.world.entity_mut(id).unwrap().state = 1;
        for _ in 0..3 {
            tick(&mut game);
        }
        assert_eq!(game.world.entity(id).unwrap().pos.y, 64.0);
        tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().pos.y, 65.0);
        for _ in 0..4 {
            tick(&mut game);
        }
        assert_eq!(game.world.entity(id).unwrap().pos.y, 66.0);
    }
    #[test]
    fn unsupported_spring_is_removed() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oSpringTrap", Vec2::new(80.0, 80.0));
        tick(&mut game);
        assert!(game.world.entity(id).is_none());
    }
    #[test]
    fn crusher_damage_bypasses_normal_enemy_invulnerability() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.spawn("oSmashTrap", Vec2::new(80.0, 80.0));
        game.player.pos = Vec2::new(88.0, 88.0);
        game.player.invincible = 60;
        player_hurt(&mut game);
        assert_eq!(game.player.health, -6);
        assert_eq!(game.player.state, PlayerState::Falling);
    }

    #[test]
    fn actual_door_exit_excludes_crusher_hurt_without_freezing_traps() {
        use crate::characters::{self, Character};
        use crate::{InputFrame, RunState};
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for object in ["oSmashTrap", "oCeilingTrap", "oBoulder", "oPushBlock"] {
                let mut level = crate::level::Level::default();
                level.set_tile(6, 7, '@').unwrap();
                level.set_tile(7, 7, 'X').unwrap();
                for x in 0..40 {
                    level.set_tile(x, 8, '1').unwrap();
                }
                let mut game = Game::shell(11, RulesPreset::Classic);
                game.load_custom(&level, "crusher-exit", "");
                characters::apply(&mut game, character);
                for _ in 0..8 {
                    game.step(&InputFrame::default());
                }
                let mut ordinary = game.clone();
                for _ in 0..30 {
                    game.step(&InputFrame {
                        right: true,
                        up: true,
                        ..Default::default()
                    });
                    if game.player.is_exiting() {
                        break;
                    }
                }
                assert_eq!(game.state, RunState::Ending);
                assert!(game.player.is_exiting());
                for exiting in [true, false] {
                    let target = if exiting { &mut game } else { &mut ordinary };
                    let health = target.player.health;
                    let velocity = target.player.vel;
                    // An arriving, already active hazard is a controlled world
                    // fixture. Exit onset itself uses the actual door/input
                    // path, with no player state, health or velocity injection.
                    let id = target.world.spawn(object, target.player.pos);
                    let contact = Vec2::new(
                        target.player.pos.x,
                        target.player.pos.y - if object == "oCeilingTrap" { 8. } else { 0. },
                    );
                    let hazard = target.world.entity_mut(id).unwrap();
                    let mask = hazard.sprite_bounds();
                    hazard.pos.x += contact.x - (mask.x + mask.w / 2.);
                    hazard.pos.y += contact.y - (mask.y + mask.h / 2.);
                    if object == "oCeilingTrap" {
                        hazard.state = 1;
                    }
                    let age = hazard.age;
                    target.step(&InputFrame::default());
                    assert!(target.world.entity(id).unwrap().age > age);
                    if exiting {
                        assert_eq!(target.player.health, health, "{character:?}/{object}");
                        assert_eq!(target.player.vel, velocity, "{character:?}/{object}");
                        assert!(target.player.is_exiting(), "{character:?}/{object}");
                        assert_eq!(target.state, RunState::Ending);
                    } else {
                        assert!(target.player.health < health, "{character:?}/{object}");
                        assert_eq!(target.state, RunState::Dead);
                    }
                }
            }
        }
    }
    #[test]
    fn moving_solid_fractional_speed_uses_global_tick_schedule() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oMovingSolid", Vec2::new(80.0, 80.0));
        game.world.entity_mut(id).unwrap().vel.x = 0.25;
        for t in 1..4 {
            game.tick = t;
            game.level_tick = t - 1;
            pre_tick(&mut game);
            tick(&mut game);
            assert_eq!(game.world.entity(id).unwrap().pos.x, 80.0);
        }
        game.tick = 4;
        game.level_tick = 3;
        pre_tick(&mut game);
        tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().pos.x, 81.0);
    }
    #[test]
    fn moving_solid_carries_player_at_original_top_left_bounds() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oMovingSolid", Vec2::new(80.0, 80.0));
        game.world.entity_mut(id).unwrap().sprite = "sBlock".into();
        game.world.entity_mut(id).unwrap().vel.x = 1.0;
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("viscidTop".into(), 1.0);
        game.player.pos = Vec2::new(88.0, 72.0);
        game.player.vel.y = 0.0;
        pre_tick(&mut game);
        tick(&mut game);
        assert_eq!(game.player.pos, Vec2::new(89.0, 72.0));
        assert!(game.player.grounded);
    }
    #[test]
    fn authored_spring_launches_only_after_landing_and_preserves_source_state() {
        let mut level = crate::level::Level::default();
        level.set_tile(4, 4, '_').unwrap();
        level.set_tile(4, 5, '1').unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.load_custom(&level, "spring", "NONE");
        game.player.pos = Vec2::new(88.0, 88.0);
        game.player.state = PlayerState::Falling;
        game.player.vel.y = 1.0;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.vel.y, -16.0);
        assert_eq!(game.player.pos.y, 72.0);
        assert_eq!(game.player.state, PlayerState::Standing);
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.object == "oSpringTrap" && e.timer == 10)
        );
    }
    #[test]
    fn airborne_spring_overlap_does_not_launch_before_grounded_state() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(5, 6, "oBlock");
        game.world.spawn("oSpringTrap", Vec2::new(80.0, 80.0));
        game.player.pos = Vec2::new(88.0, 88.0);
        game.player.state = PlayerState::Falling;
        game.player.vel.y = 2.0;
        tick(&mut game);
        assert_eq!(game.player.vel.y, 2.0);
    }
    #[test]
    fn authored_falling_platform_waits_twenty_standing_ticks_then_falls_and_breaks() {
        let mut level = crate::level::Level::default();
        level.set_tile(4, 4, 'd').unwrap();
        level.set_tile(4, 8, '1').unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.load_custom(&level, "falling_platform", "NONE");
        game.player.pos = Vec2::new(88.0, 72.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        let id = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oDarkFall")
            .unwrap()
            .id;
        for _ in 0..19 {
            game.step(&crate::InputFrame::default());
        }
        assert_eq!(game.world.entity(id).unwrap().pos.y, 80.0);
        assert_eq!(game.world.entity(id).unwrap().value("yAcc"), 0.0);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().value("yAcc"), 1.0);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.y, 81.0);
        assert_eq!(game.player.pos.y, 73.0);
        for _ in 0..20 {
            game.step(&crate::InputFrame::default());
            if game.world.entity(id).is_none() {
                break;
            }
        }
        assert!(game.world.entity(id).is_none());
        assert!(game.world.entities.iter().any(|e| e.object == "oSmokePuff"));
    }
    #[test]
    fn rising_source_thwomp_carries_before_jump_input() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oThwompTrap", Vec2::new(80.0, 80.0));
        let e = game.world.entity_mut(id).unwrap();
        e.state = 3;
        e.vel.y = -1.0;
        game.player.pos = Vec2::new(88.0, 73.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.step(&crate::InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.world.entity(id).unwrap().pos.y, 79.0);
        assert!(game.player.pos.y <= 68.0);
        assert!(game.player.vel.y < 0.0);
    }
    #[test]
    fn authored_lava_sinks_entire_push_block_stack() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let lava = game.world.spawn("oLava", Vec2::new(80.0, 80.0));
        let lower = game.world.spawn("oPushBlock", Vec2::new(80.0, 72.0));
        let upper = game.world.spawn("oPushBlock", Vec2::new(80.0, 56.0));
        game.player.pos = Vec2::new(40.0, 40.0);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(lava).is_some());
        assert!((game.world.entity(lower).unwrap().pos.y - 73.05).abs() < 0.001);
        assert!((game.world.entity(upper).unwrap().pos.y - 57.05).abs() < 0.001);
    }
    #[test]
    fn descending_source_door_stops_when_player_is_trapped_against_floor() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(5, 6, "oBlock");
        let id = game.world.spawn("oDoor", Vec2::new(80.0, 48.0));
        let door = game.world.entity_mut(id).unwrap();
        door.state = 1;
        door.vel.y = 1.0;
        game.player.pos = Vec2::new(88.0, 88.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.y, 48.0);
        assert_eq!(game.player.pos.y, 88.0);
        assert_eq!(game.player.health, 4);
    }
    #[test]
    fn source_boulder_gets_global_and_own_movement_with_room_local_fraction_phase() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.tick = 100;
        let id = game.world.spawn("oBoulder", Vec2::new(120.0, 80.0));
        game.world.entity_mut(id).unwrap().vel.x = 0.5;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.engine_time(), 2);
        assert_eq!(game.world.entity(id).unwrap().pos.x, 122.0);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.engine_time(), 3);
        assert_eq!(game.world.entity(id).unwrap().pos.x, 122.0);
    }
    #[test]
    fn rising_thwomp_stops_before_trapping_rider_against_ceiling() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(5, 3, "oBlock");
        let id = game.world.spawn("oThwompTrap", Vec2::new(80.0, 80.0));
        let trap = game.world.entity_mut(id).unwrap();
        trap.state = 3;
        trap.vel.y = -2.0;
        game.player.pos = Vec2::new(88.0, 73.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.y, 79.0);
        assert_eq!(game.player.pos.y, 72.0);
        assert_eq!(game.player.health, 4);
    }
    #[test]
    fn blocked_horizontal_rider_stays_while_boulder_moves_beneath_them() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(9, 3, "oBlock");
        let id = game.world.spawn("oBoulder", Vec2::new(120.0, 80.0));
        game.world.entity_mut(id).unwrap().vel.x = 4.5;
        game.player.pos = Vec2::new(139.0, 56.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.pos.x, 139.0);
        assert_eq!(game.world.entity(id).unwrap().pos.x, 130.0);
        assert_eq!(game.player.health, 4);
    }
    #[test]
    fn thin_ice_loses_sixty_thickness_over_thirty_standing_game_ticks() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oThinIce", Vec2::new(80.0, 80.0));
        game.world.entity_mut(id).unwrap().sprite = "sThinIce1".into();
        game.player.pos = Vec2::new(88.0, 72.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        for _ in 0..5 {
            game.step(&crate::InputFrame::default());
        }
        assert_eq!(game.world.entity(id).unwrap().value("thickness"), 50.0);
        assert_eq!(game.world.entity(id).unwrap().sprite, "sThinIce2");
        for _ in 0..25 {
            game.step(&crate::InputFrame::default());
        }
        assert!(game.world.entity(id).is_none());
        game.step(&crate::InputFrame::default());
        assert!(game.player.pos.y > 72.0);
    }
}
