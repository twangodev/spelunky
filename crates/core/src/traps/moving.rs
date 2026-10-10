//! Moving trap rules transcribed from Classic 1.1's object events.
use crate::model::{Entity, Game, GameEvent, PlayerState, Rect, Vec2};

/// The global oGame platform pass precedes character input and object AI.
pub(super) fn pre_tick(game: &mut Game) {
    let mut carried_horizontally = false;
    let count = game.world.entities.len();
    for phase in 0..2 {
        for index in 0..count {
            let mut entity = game.world.entities[index].clone();
            if !entity.alive {
                continue;
            }
            match entity.object.as_str() {
                "oBoulder" | "oThwompTrap" | "oCeilingTrap" | "oDoor" | "oMovingSolid"
                | "oDarkFall" | "oOlmec"
                    if phase == 0 =>
                {
                    entity.solid = true;
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
                }
                "oPushBlock" | "oIceBlock" if phase == 1 => {
                    entity.solid = true;
                    // Source oMoveableSolid is a separate global gravity pass.
                    let active = entity.pos.x > game.camera.x - 16.0
                        && entity.pos.x < game.camera.x + 320.0
                        && entity.pos.y > game.camera.y - 16.0
                        && entity.pos.y < game.camera.y + 240.0;
                    if active {
                        let gravity = entity.data.get("push_gravity").copied().unwrap_or(0.6);
                        entity.vel.y = (entity.vel.y + gravity).min(8.0);
                        let speed = entity.vel.y;
                        if move_body(game, &mut entity, Vec2::new(0.0, speed.ceil())) {
                            if speed > 0.6 {
                                game.sound("sndThud", entity.pos);
                            }
                            entity.vel.y = 0.0;
                        }
                    }
                }
                _ => continue,
            }
            game.world.entities[index] = entity;
        }
    }
}

pub(super) fn tick(game: &mut Game) {
    let count = game.world.entities.len();
    for index in 0..count {
        let mut entity = game.world.entities[index].clone();
        if !entity.alive {
            continue;
        }
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
            _ => continue,
        }
        let destroyed = (!entity.alive).then_some(entity.id);
        game.world.entities[index] = entity;
        if let Some(id) = destroyed {
            crate::items::explosives::destroy_solid_instance(game, id);
        }
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
    collides_except(game, line, entity.id)
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
        entity.vel = Vec2::default();
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
        if entity.vel.x.abs() < 4.0 {
            entity.vel.x += entity.value("xa");
        }
        if entity.vel.y.abs() < 4.0 {
            entity.vel.y += entity.value("ya");
        }
        // Smash uses raw xv/yv, distinct from moveTo's quantized xVel/yVel.
        entity.pos.x += entity.vel.x;
        entity.pos.y += entity.vel.y;
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
            entity.vel = Vec2::default();
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
    if game.player.health > 0
        && entity.sprite_bounds().intersects(Rect::new(
            game.player.pos.x - 6.0,
            game.player.pos.y - 6.0,
            13.0,
            14.0,
        ))
    {
        trap_damage(game, 10);
        let speed = 4.0 + f64::from(game.rng.range(3));
        game.player.vel.x = if entity.pos.x + 8.0 < game.player.pos.x {
            -speed
        } else {
            speed
        };
        game.player.vel.y = if entity.value("direction") == 1.0 {
            4.0
        } else {
            -6.0
        };
        if let Some(id) = game.player.held.take() {
            if let Some(held) = game.world.entity_mut(id) {
                held.held = false;
            }
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
    if entity.object == "oSmashTrapLit" {
        entity.animation = (entity.animation + 1.0) % 5.0;
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
            let source_bounds = entity.sprite_bounds();
            let old = if source_bounds.w > 0.0 && source_bounds.h > 0.0 {
                source_bounds
            } else {
                bounds(entity)
            };
            let player = game.player.bounds();
            let lb = player.x.round_ties_even();
            let rb = (player.x + player.w - 1.0).round_ties_even();
            let tb = player.y.round_ties_even();
            let bb = (player.y + player.h).round_ties_even();
            let top_line = Rect::new(lb, bb, rb - lb + 1.0, 1.0);
            let bottom_line = Rect::new(lb, tb - 1.0, rb - lb + 1.0, 1.0);
            let rider = if horizontal {
                viscid && can_carry && old.intersects(top_line)
            } else if step.y > 0.0 {
                viscid && old.intersects(shifted(top_line, Vec2::new(0.0, 1.0)))
            } else {
                old.intersects(top_line)
            };
            let side = if step.x > 0.0 {
                Rect::new(lb - 1.0, tb, 1.0, (bb - 1.0).round_ties_even() - tb + 1.0)
            } else {
                Rect::new(rb + 1.0, tb, 1.0, (bb - 1.0).round_ties_even() - tb + 1.0)
            };
            let touching = if horizontal {
                old.intersects(side)
            } else {
                old.intersects(bottom_line)
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
                if !collides_except(game, obstruction, entity.id) {
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
                if collides_except(game, obstruction, entity.id) {
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

fn trap_damage(game: &mut Game, amount: i32) {
    // Classic solid/crusher kills are outside ordinary enemy invulnerability.
    if game.player.health > 0 {
        game.player.health -= amount;
        game.events.push(GameEvent::Damage { amount });
        if game.player.health <= 0 {
            game.player.state = PlayerState::Dead;
        }
    }
}

/// Smash traps use their own xv/yv, rather than the platform engine's xVel/yVel.
/// Consequently the collision event must see their overlap with a character.
fn move_body(game: &Game, entity: &mut Entity, delta: Vec2) -> bool {
    let steps = delta.x.abs().max(delta.y.abs()).ceil() as u32;
    if steps == 0 {
        return false;
    }
    let step = Vec2::new(delta.x / f64::from(steps), delta.y / f64::from(steps));
    for _ in 0..steps {
        if blocked(game, entity, step) {
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
            entity.sprite = "sCeilingTrapS".into();
            entity.animation = 0.0;
        }
    } else if entity.state == 2 {
        entity.vel.y = 0.0;
        if edge_blocked(game, entity, 1, 1.0) {
            entity.pos.y -= 1.0;
        }
    }
    if entity.sprite == "sCeilingTrapS" {
        entity.animation += 0.4;
        if entity.animation >= 6.0 {
            entity.sprite = "sCeilingTrap".into();
            entity.animation = 0.0;
        }
    }
    if entity.state > 0
        && bounds(entity).intersects(Rect::new(
            game.player.pos.x - 2.0,
            game.player.pos.y - 9.0,
            5.0,
            3.0,
        ))
    {
        trap_damage(game, 10);
    }
}

fn boulder(game: &mut Game, entity: &mut Entity) {
    // Horizontal collisions can break destructible terrain, unlike floor contact.
    let x_distance = scheduled_delta(entity.vel.x, game.engine_time());
    let x_steps = x_distance.abs() as u32;
    if x_steps > 0 {
        let dx = x_distance / f64::from(x_steps);
        for _ in 0..x_steps {
            let next = shifted(bounds(entity), Vec2::new(dx, 0.0));
            let tx = if dx > 0.0 {
                ((next.x + next.w - 0.001) / 16.0).floor() as i32
            } else {
                (next.x / 16.0).floor() as i32
            };
            let mut hard_collision = false;
            for ty in
                (next.y / 16.0).floor() as i32..=((next.y + next.h - 0.001) / 16.0).floor() as i32
            {
                if let Some(tile) = game.world.tile(tx, ty) {
                    if tile.is_solid() {
                        if tile.is_destructible()
                            && entity.vel.x.abs() >= 1.0
                            && f64::from(ty) * 16.0 <= entity.pos.y + 13.0
                        {
                            game.world.set_tile(tx as usize, ty as usize, "");
                            entity.vel.x -= entity.vel.x.signum() * 0.1;
                            game.sound("sndCrunch", entity.pos);
                        } else {
                            hard_collision = true;
                        }
                    }
                }
            }
            if hard_collision {
                entity.vel.x *= -0.5;
                break;
            }
            if move_body(game, entity, Vec2::new(dx, 0.0)) {
                entity.vel.x *= -0.5;
                break;
            }
        }
    }
    let vertical_speed = entity.vel.y;
    let vertical_distance = scheduled_delta(vertical_speed, game.engine_time());
    let vertical_hit = move_body(game, entity, Vec2::new(0.0, vertical_distance));
    entity.vel.y = (entity.vel.y + 0.6).min(8.0);
    if entity.pos.x - 17.0 <= 16.0 && entity.vel.x < 0.0 {
        entity.pos.x += 1.0;
        entity.vel.x = -entity.vel.x;
    }
    if entity.pos.x + 17.0 >= 656.0 && entity.vel.x > 0.0 {
        entity.pos.x -= 1.0;
        entity.vel.x = -entity.vel.x;
    }
    if vertical_hit && vertical_speed < 0.0 {
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
    entity.sprite = if entity.vel.x < 0.0 {
        "sBoulderRotateL"
    } else if entity.vel.x > 0.0 {
        "sBoulderRotateR"
    } else {
        "sBoulder"
    }
    .into();
    entity.animation = (entity.animation + entity.vel.x.abs() / 5.0) % 4.0;
    if entity.sprite_bounds().contains(game.player.pos) {
        trap_damage(game, 99);
    }
}

fn push_block(game: &mut Game, entity: &mut Entity) {
    if touches_lava(
        game,
        Rect::new(entity.pos.x + 8.0, entity.pos.y + 14.0, 1.0, 1.0),
    ) && !blocked(game, entity, Vec2::new(0.0, 1.0))
    {
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
    if entity.sprite_bounds().contains(game.player.pos) {
        trap_damage(game, 99);
    }
    if entity.pos.y > game.world.height as f64 * 16.0 {
        entity.alive = false;
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
    let touching = entity.sprite_bounds().intersects(Rect::new(
        left,
        (player.y + player.h).round_ties_even(),
        right - left + 1.0,
        1.0,
    ));
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
    if collides_except(game, entity.sprite_bounds(), entity.id) {
        entity.alive = false;
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
            let vel = Vec2::new(
                f64::from(game.rng.range(3)) - f64::from(game.rng.range(3)),
                -f64::from(game.rng.range(4)),
            );
            let id = game.world.spawn("oRubbleDark", pos);
            game.world.entity_mut(id).unwrap().vel = vel;
        }
    }
}

fn spring(game: &mut Game, entity: &mut Entity) {
    let active = entity.pos.x > game.camera.x - 16.0
        && entity.pos.x < game.camera.x + 320.0
        && entity.pos.y > game.camera.y - 16.0
        && entity.pos.y < game.camera.y + 240.0;
    if active {
        entity.timer = entity.timer.saturating_sub(1);
    }
    if entity.state == 1 {
        entity.animation += 1.0;
        if entity.animation >= 9.0 {
            entity.state = 0;
            entity.animation = 0.0;
            entity.sprite = "sSpringTrap".into();
        }
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
    if entity.state != 0 || entity.timer != 0 {
        return;
    }
    let contact = crate::catalog::bounds("oSpringTrap", entity.pos);
    let mut sprung = false;
    if (game.player.pos.x - entity.pos.x - 8.0).abs() < 6.0
        && contact.intersects(game.player.sprite_bounds())
        && matches!(
            game.player.state,
            PlayerState::Standing
                | PlayerState::Running
                | PlayerState::Ducking
                | PlayerState::LookingUp
                | PlayerState::Stunned
        )
        && !matches!(game.player.sprite.as_str(), "sPExit" | "sDamselExit")
    {
        game.player.pos.y -= 16.0;
        game.player.vel.y = -16.0;
        game.player.grounded = false;
        sprung = true;
    } else {
        for other in &mut game.world.entities {
            if !other.alive || other.held || other.id == entity.id || other.solid {
                continue;
            }
            let enemy = is_enemy(&other.object);
            if !enemy && !is_item(&other.object) {
                continue;
            }
            let aligned = if enemy {
                (other.pos.x - entity.pos.x).abs() < 6.0
            } else {
                (other.pos.x - entity.pos.x - 8.0).abs() < 6.0
            };
            if aligned && contact.intersects(other.sprite_bounds()) {
                if (enemy && other.value("flying") != 0.0)
                    || (!enemy && other.data.get("active") == Some(&0.0))
                {
                    continue;
                }
                other.pos.y -= if enemy { 16.0 } else { 24.0 };
                other.vel.y = -8.0;
                if enemy || other.object == "oDamsel" {
                    other.vel.x += f64::from(other.facing);
                }
                sprung = true;
                break;
            }
        }
    }
    if sprung {
        entity.state = 1;
        entity.timer = 10;
        entity.animation = 0.0;
        entity.sprite = "sSpringTrapSprung".into();
        game.sound("sndBoing", entity.pos);
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
        trap.vel.x = 2.0;
        trap.data.insert("xa".into(), 0.5);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.x, 400.0);
        assert_eq!(game.world.entity(id).unwrap().vel.x, 2.0);
    }

    #[test]
    fn smash_uses_raw_velocity_then_source_contact_correction() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(6, 5, "oHardBlock");
        let id = game.world.spawn("oSmashTrap", Vec2::new(81.0, 80.0));
        let trap = game.world.entity_mut(id).unwrap();
        trap.state = 1;
        trap.vel.x = 4.0;
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
        assert_eq!(game.player.vel.y, -16.0);
        assert_eq!(game.player.pos.y, 72.0);
        assert_eq!(game.world.entity(id).unwrap().timer, 10);
        game.player.pos = Vec2::new(88.0, 88.0);
        tick(&mut game);
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
        assert_eq!(game.world.entity(id).unwrap().vel.x, 0.5);
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
        tick(&mut game);
        assert_eq!(game.player.health, -6);
        assert_eq!(game.player.state, PlayerState::Dead);
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
