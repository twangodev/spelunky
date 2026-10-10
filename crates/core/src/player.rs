//! Classic's 30 Hz character controller.
//!
//! Constants and ordering follow `characterCreateEvent`, `characterStepEvent`,
//! `moveTo`, and `oPlayer1`'s Step event in the archived 1.1 source. In particular,
//! ordinary gravity is 1 (the separate stunned/dead controller starts at 0.6),
//! and fractional movement is scheduled against the global tick, not accumulated.
use crate::model::*;

const JUMP_TICKS: u32 = 10;
const ASSIST_TICKS: u32 = 3;

fn value(p: &Player, key: &str, default: f64) -> f64 {
    p.data.get(key).copied().unwrap_or(default)
}
fn set(p: &mut Player, key: &str, n: f64) {
    p.data.insert(key.into(), n);
}
fn has(p: &Player, names: &[&str]) -> bool {
    names.iter().any(|name| p.has(name))
}
fn grounded_state(s: PlayerState) -> bool {
    matches!(
        s,
        PlayerState::Standing
            | PlayerState::Running
            | PlayerState::Ducking
            | PlayerState::LookingUp
    )
}
fn airborne(s: PlayerState) -> bool {
    matches!(s, PlayerState::Jumping | PlayerState::Falling)
}
fn bounds_at(p: &Player, pos: Vec2) -> Rect {
    let half = value(p, "collision_half_width", 5.0);
    let top = value(p, "collision_top", -8.0);
    Rect::new(pos.x - half, pos.y + top, half * 2.0, 8.0 - top)
}
#[cfg(test)]
fn solid(w: &World, p: &Player, pos: Vec2) -> bool {
    let r = bounds_at(p, pos);
    collision_rect(w, r, "oSolid")
}

fn collision_rect(w: &World, r: Rect, group: &str) -> bool {
    // GameMaker has no implicit room-edge collider. In particular, the Ice
    // Caves abyss must let the player leave the room and trigger falling death.
    let mut tile_collision = false;
    for y in (r.y / 16.0).floor() as i32..=((r.y + r.h - 0.001) / 16.0).floor() as i32 {
        for x in (r.x / 16.0).floor() as i32..=((r.x + r.w - 0.001) / 16.0).floor() as i32 {
            tile_collision |= w.tile(x, y).is_some_and(|t| {
                crate::catalog::is_a(&t.object, group)
                    && crate::catalog::bounds(
                        &t.object,
                        Vec2::new(f64::from(x) * 16.0, f64::from(y) * 16.0),
                    )
                    .intersects(r)
            });
        }
    }
    tile_collision
        || w.entities.iter().any(|e| {
            e.alive
                && !e.held
                && (crate::catalog::is_a(&e.object, group) || (group == "oSolid" && e.solid))
                && e.sprite_bounds().intersects(r)
        })
}

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

fn edge_rect(p: &Player, side: Side, distance: f64, move_probe: bool) -> Rect {
    let r = bounds_at(p, p.pos);
    let left = gm_round(r.x);
    let right = gm_round(r.x + r.w - 1.0);
    let top = gm_round(r.y + if move_probe { 5.0 } else { 0.0 });
    let bottom = gm_round(r.y + r.h - 1.0);
    match side {
        Side::Left => Rect::new(gm_round(r.x - distance), top, 1.0, bottom - top + 1.0),
        Side::Right => Rect::new(
            gm_round(r.x + r.w + distance - 1.0),
            top,
            1.0,
            bottom - top + 1.0,
        ),
        Side::Top => Rect::new(left, gm_round(r.y - distance), right - left + 1.0, 1.0),
        Side::Bottom => Rect::new(
            left,
            gm_round(r.y + r.h + distance - 1.0),
            right - left + 1.0,
            1.0,
        ),
    }
}

fn edge(w: &World, p: &Player, side: Side, distance: f64) -> bool {
    collision_rect(w, edge_rect(p, side, distance, false), "oSolid")
}

fn solid_point(w: &World, pos: Vec2) -> bool {
    collision_rect(w, Rect::new(pos.x, pos.y, 0.001, 0.001), "oSolid")
}
fn object_at(w: &World, pos: Vec2, names: &[&str]) -> bool {
    w.tile((pos.x / 16.0).floor() as i32, (pos.y / 16.0).floor() as i32)
        .is_some_and(|t| names.contains(&t.object.as_str()))
        || w.entities.iter().any(|e| {
            e.alive && names.contains(&e.object.as_str()) && e.sprite_bounds().contains(pos)
        })
}
fn ladder_at(w: &World, pos: Vec2) -> Option<Vec2> {
    ladder_match(w, pos, true)
}
fn ladder_segment_at(w: &World, pos: Vec2) -> Option<Vec2> {
    ladder_match(w, pos, false)
}
fn ladder_match(w: &World, pos: Vec2, include_top: bool) -> Option<Vec2> {
    let tx = (pos.x / 16.0).floor() as i32;
    let ty = (pos.y / 16.0).floor() as i32;
    if w.tile(tx, ty).is_some_and(|t| {
        crate::catalog::is_a(&t.object, "oLadder")
            || (include_top && crate::catalog::is_a(&t.object, "oLadderTop"))
    }) {
        return Some(Vec2::new(f64::from(tx) * 16.0 + 8.0, f64::from(ty) * 16.0));
    }
    w.entities
        .iter()
        .find(|e| {
            e.alive
                && (crate::catalog::is_a(&e.object, "oLadder")
                    || (include_top && crate::catalog::is_a(&e.object, "oLadderTop")))
                && e.sprite_bounds().contains(pos)
        })
        .map(|e| Vec2::new(e.pos.x + 8.0, e.pos.y))
}
fn water(w: &World, pos: Vec2) -> bool {
    object_at(w, pos, &["oWater", "oWaterSwim", "oWaterTop"])
}
fn platform(w: &World, p: &Player, pos: Vec2, dy: f64) -> bool {
    if dy <= 0.0 {
        return false;
    }
    debug_assert_eq!(pos, p.pos);
    !collision_rect(w, bounds_at(p, pos), "oPlatform")
        && collision_rect(w, edge_rect(p, Side::Bottom, dy, false), "oPlatform")
}
fn below(w: &World, p: &Player) -> bool {
    edge(w, p, Side::Bottom, 1.0) || platform(w, p, p.pos, 1.0)
}

/// The original reciprocal/modulo rounding deliberately differs from a subpixel
/// accumulator: e.g. 0.3 moves every third tick, while 0.6 moves every second tick.
fn displacement(velocity: f64, tick: u64) -> i32 {
    let magnitude = velocity.abs();
    let fraction = magnitude.fract();
    let period = if fraction > 0.0 {
        gm_round(1.0 / fraction) as u64
    } else {
        0
    };
    let extra = period > 0 && tick.is_multiple_of(period);
    (magnitude.floor() as i32 + i32::from(extra)) * if velocity < 0.0 { -1 } else { 1 }
}

fn move_player(w: &mut World, p: &mut Player, tick: u64, down: bool) {
    move_by(w, p, tick, down, p.vel);
}

fn move_by(w: &mut World, p: &mut Player, tick: u64, down: bool, movement: Vec2) -> (i32, i32) {
    let dx = displacement(movement.x, tick);
    let dy = displacement(movement.y, tick);
    for _ in 0..dx.unsigned_abs() {
        let next = Vec2::new(p.pos.x + f64::from(dx.signum()), p.pos.y);
        let probe = edge_rect(p, if dx > 0 { Side::Right } else { Side::Left }, 1.0, true);
        if collision_rect(w, probe, "oSolid") {
            // Source canPushMoveableSolids checks player ancestry, so a player's
            // remaining airborne momentum can also push a block pixel by pixel.
            let target = probe;
            let block = w.entities.iter().position(|e| {
                e.alive
                    && e.solid
                    && crate::catalog::is_a(&e.object, "oMoveableSolid")
                    && e.sprite_bounds().intersects(target)
            });
            let pushed = block.is_some_and(|index| {
                let destination = Vec2::new(
                    w.entities[index].pos.x + f64::from(dx.signum()),
                    w.entities[index].pos.y,
                );
                let old = w.entities[index].sprite_bounds();
                let rect = Rect::new(old.x + f64::from(dx.signum()), old.y, old.w, old.h);
                if w.collides_except(rect, w.entities[index].id) {
                    return false;
                }
                w.entities[index].pos = destination;
                true
            });
            if !pushed {
                break;
            }
        }
        p.pos = next;
    }
    for _ in 0..dy.unsigned_abs() {
        let next = Vec2::new(p.pos.x, p.pos.y + f64::from(dy.signum()));
        if edge(w, p, if dy > 0 { Side::Bottom } else { Side::Top }, 1.0)
            || (!down && platform(w, p, p.pos, f64::from(dy.signum())))
        {
            break;
        }
        p.pos = next;
    }
    (dx, dy)
}

fn slope_move(w: &mut World, p: &mut Player, time: u64, down: bool, col_top: bool) {
    if !grounded_state(p.state) || p.vel.x == 0.0 {
        move_player(w, p, time, down);
        return;
    }
    let original = p.pos;
    // Classic's inclusive `for(y=y; y>=previous-maxSlope; y-=1)` raises
    // maxSlope+1 pixels. colTop is the earlier cached collision result.
    let rise = if col_top { 0.0 } else { 5.0 };
    p.pos.y -= rise;
    let high = p.pos;
    let (ix, iy) = move_by(w, p, time, down, Vec2::new(p.vel.x, p.vel.y + rise));
    let distance = original.distance(p.pos);
    if distance > f64::from(ix.abs()) {
        let ratio = f64::from(ix.abs()) / distance * 0.9;
        p.pos = high;
        move_by(
            w,
            p,
            time,
            down,
            Vec2::new(
                gm_round(f64::from(ix) * ratio),
                gm_round(f64::from(iy) * ratio + rise),
            ),
        );
    }
    // The original downhill loop checks cached colBot and restores its start
    // when false; it cannot add descent. moveTo above already settles downward.
}

fn gm_round(n: f64) -> f64 {
    // GameMaker rounds exact halves to the nearest even integer.
    n.round_ties_even()
}

fn apply_ball_tether(w: &World, p: &mut Player, input: &InputFrame, fall: &mut f64) {
    let Some(ball) = w.entities.iter().find(|e| e.alive && e.object == "oBall") else {
        return;
    };
    let a = crate::catalog::sprite_def(&p.sprite)
        .map(|s| {
            Rect::new(
                p.pos.x + f64::from(s.bounds[0] - s.origin[0]),
                p.pos.y + f64::from(s.bounds[1] - s.origin[1]),
                f64::from(s.bounds[2] - s.bounds[0] + 1),
                f64::from(s.bounds[3] - s.bounds[1] + 1),
            )
        })
        .unwrap_or_else(|| bounds_at(p, p.pos));
    let b = ball.bounds();
    let dx = (a.x - (b.x + b.w - 1.0))
        .max(b.x - (a.x + a.w - 1.0))
        .max(0.0);
    let dy = (a.y - (b.y + b.h - 1.0))
        .max(b.y - (a.y + a.h - 1.0))
        .max(0.0);
    if dx.hypot(dy) < 24.0 {
        return;
    }
    if (p.vel.x > 0.0 && ball.pos.x < p.pos.x && (ball.pos.x - p.pos.x).abs() > 24.0)
        || (p.vel.x < 0.0 && ball.pos.x > p.pos.x && (ball.pos.x - p.pos.x).abs() > 24.0)
    {
        p.vel.x = 0.0;
    }
    if p.vel.y > 0.0 && ball.pos.y < p.pos.y && (ball.pos.y - p.pos.y).abs() > 24.0 {
        if (ball.pos.x - p.pos.x).abs() < 1.0 {
            p.pos.x = ball.pos.x;
        } else if ball.pos.x < p.pos.x && !input.right {
            if p.vel.x > 0.0 {
                p.vel.x *= -0.25;
            } else if p.vel.x == 0.0 {
                p.vel.x = -1.0;
            }
        } else if ball.pos.x > p.pos.x && !input.left {
            if p.vel.x < 0.0 {
                p.vel.x *= -0.25;
            } else if p.vel.x == 0.0 {
                p.vel.x = 1.0;
            }
        }
        p.vel.y = 0.0;
        *fall = 0.0;
    }
    if p.vel.y < 0.0 && ball.pos.y > p.pos.y && (ball.pos.y - p.pos.y).abs() > 24.0 {
        p.vel.y = 0.0;
    }
}

/// Advance exactly one authoritative Classic tick. Presentation interpolation
/// belongs to the host; this function never accepts a frame delta.
pub fn tick(game: &mut Game, input: &InputFrame) {
    let engine_time = game.engine_time();
    let damsel = game.flags.contains("character_damsel");
    let tunnelman = game.flags.contains("character_tunnelman");
    let p = &mut game.player;
    set(p, "character_damsel", f64::from(damsel));
    set(p, "character_tunnelman", f64::from(tunnelman));
    p.invincible = p.invincible.saturating_sub(1);
    if p.stun > 0
        && matches!(
            p.sprite.as_str(),
            "sStunL" | "sDamselStunL" | "sTunnelStunL"
        )
    {
        p.stun -= 1;
    }
    if p.health <= 0 || p.stun > 0 {
        let impact = stunned_tick(&mut game.world, p, engine_time);
        if impact {
            game.events.push(GameEvent::Damage { amount: 1 });
            game.sound("sndHurt", game.player.pos);
        }
        return;
    }
    if value(p, "duck_to_hang", 0.0) > 0.0 {
        duck_to_hang_tick(&game.world, p);
        return;
    }
    let mut effective = *input;
    let cant_jump = value(p, "cantJump", 0.0);
    if cant_jump > 0.0 || (tunnelman && p.sprite == "sTunnelAttackL" && p.held.is_none()) {
        if cant_jump > 0.0 {
            set(p, "cantJump", cant_jump - 1.0);
        }
        effective.jump = false;
        effective.jump_pressed = false;
        effective.jump_released = false;
        p.buffered_jump_ticks = 0;
    }
    let input = &effective;
    let w = &mut game.world;
    let mut sound = None;
    let mut fall_damage = 0;
    let col_bot = below(w, p);
    let col_left = edge(w, p, Side::Left, 1.0);
    let col_right = edge(w, p, Side::Right, 1.0);
    let col_top = edge(w, p, Side::Top, 1.0);
    let col_moveable_left =
        collision_rect(w, edge_rect(p, Side::Left, 1.0, false), "oMoveableSolid");
    let col_moveable_right =
        collision_rect(w, edge_rect(p, Side::Right, 1.0, false), "oMoveableSolid");
    let in_water = water(w, p.pos);
    let in_web = object_at(w, p.pos, &["oWeb"]);
    let water_jump = collision_rect(w, edge_rect(p, Side::Top, 1.0, false), "oWater");
    let water_surface = in_water && !water(w, Vec2::new(p.pos.x, p.pos.y - 9.0));
    let was_grounded = grounded_state(p.state);
    let mut fall = value(p, "fall_timer", 0.0);
    let mut gravity = value(p, "gravity", 1.0);
    let mut intensity = value(p, "gravity_intensity", 1.0);
    let mut ladder_timer = value(p, "ladder_timer", 0.0);
    let mut hang_count = value(p, "hang_count", 0.0);
    let mut cape = value(p, "cape_open", 0.0) > 0.0;
    let mut parachute = value(p, "parachute_open", 0.0) > 0.0;
    let mut fuel = value(p, "jetpack_fuel", 0.0);
    let mut jumped = value(p, "jumped", 0.0) > 0.0;
    let mut ax: f64 = 0.0;
    let mut ay: f64 = 0.0;
    let mut limit_x = value(p, "velocity_limit_x", 16.0);
    let mut limit_y_acc: f64 = 6.0;

    // These run before characterStepEvent in Classic's oPlayer1 Step event.
    if cape || parachute || in_water || in_web {
        fall = 0.0;
    }
    if p.vel.y > 0.0 && p.state != PlayerState::Climbing {
        fall += 1.0;
        if has(p, &["Parachute", "parachute"])
            && fall > 14.0
            && !solid_point(w, Vec2::new(p.pos.x, p.pos.y + 32.0))
        {
            p.equipment.remove("Parachute");
            p.equipment.remove("parachute");
            parachute = true;
            fall = 0.0;
        }
    } else if was_grounded && fall > 16.0 && !object_at(w, p.pos, &["oSpringTrap"]) {
        fall_damage = if fall > 48.0 {
            10
        } else if fall > 32.0 {
            2
        } else {
            1
        };
        p.vel.y = -3.0;
        fall = 0.0;
    } else {
        fall = 0.0;
        parachute = false;
    }
    if fall_damage > 0 {
        set(p, "fall_timer", 0.0);
        set(p, "fall_ticks", 0.0);
        // This branch precedes characterStepEvent in the original and bypasses
        // ordinary hit invulnerability, including the hurt flash timer.
        let invincible = p.invincible;
        p.invincible = 0;
        game.damage_player(fall_damage);
        game.player.invincible = invincible;
        game.player.stun = 60;
        game.player.vel.y = -3.0;
        set(&mut game.player, "bounced", 1.0);
        game.sound("sndThud", game.player.pos);
        let impact = stunned_tick(&mut game.world, &mut game.player, engine_time);
        if impact {
            game.events.push(GameEvent::Damage { amount: 1 });
            game.sound("sndHurt", game.player.pos);
        }
        return;
    }
    if was_grounded {
        fuel = 50.0;
    }
    if in_water && value(p, "swimming", 0.0) == 0.0 {
        sound = Some("sndSplash");
    }
    set(p, "swimming", f64::from(in_water));
    set(p, "in_web", f64::from(in_web));

    // Classic intentionally waits two ticks before horizontal acceleration.
    let left_steps = if input.left {
        value(p, "left_steps", 0.0) + 1.0
    } else {
        0.0
    };
    let right_steps = if input.right {
        value(p, "right_steps", 0.0) + 1.0
    } else {
        0.0
    };
    if !matches!(p.state, PlayerState::Climbing | PlayerState::Hanging) {
        if value(p, "previous_left", 0.0) > 0.0 && !input.left && p.vel.x.abs() < 0.1 {
            ax -= 0.5;
        }
        if value(p, "previous_right", 0.0) > 0.0 && !input.right && p.vel.x.abs() < 0.1 {
            ax += 0.5;
        }
        if input.left && !input.right {
            if col_moveable_left {
                if grounded_state(p.state) && p.state != PlayerState::Ducking {
                    ax -= 1.0;
                    let push = value(p, "push_timer", 0.0);
                    set(p, "push_timer", push + 10.0);
                }
            } else if left_steps > 2.0 && (p.facing < 0 || p.vel.x.abs() < 0.1) {
                ax -= 3.0;
            }
            p.facing = -1;
        } else if input.right && !input.left {
            if col_moveable_right {
                if grounded_state(p.state) && p.state != PlayerState::Ducking {
                    ax += 1.0;
                    let push = value(p, "push_timer", 0.0);
                    set(p, "push_timer", push + 10.0);
                }
            } else if (right_steps > 2.0 || col_moveable_left)
                && (p.facing > 0 || p.vel.x.abs() < 0.1)
            {
                ax += 3.0;
            }
            p.facing = 1;
        }
    }
    set(p, "previous_left", f64::from(input.left));
    set(p, "previous_right", f64::from(input.right));
    let run_key = input.run || (input.attack && p.attack_timer == 0);
    let run_held = if !run_key || (!input.left && !input.right) {
        0.0
    } else if input.run {
        100.0
    } else {
        value(p, "run_held", 0.0) + 1.0
    };

    if game.rules == RulesPreset::Assisted {
        if col_bot && p.vel.y >= 0.0 {
            p.coyote_ticks = ASSIST_TICKS;
        }
        if input.jump_pressed {
            p.buffered_jump_ticks = ASSIST_TICKS;
        }
    } else {
        p.coyote_ticks = 0;
        p.buffered_jump_ticks = 0;
    }
    let jump_pressed = input.jump_pressed
        || (game.rules == RulesPreset::Assisted && p.buffered_jump_ticks > 0 && col_bot);
    let can_coyote = game.rules == RulesPreset::Assisted
        && p.coyote_ticks > 0
        && !matches!(
            p.state,
            PlayerState::Hanging | PlayerState::Climbing | PlayerState::Stunned | PlayerState::Dead
        );

    if p.state == PlayerState::Climbing {
        cape = false;
        jumped = false;
        ladder_timer = 10.0;
        if let Some(ladder) = ladder_segment_at(w, p.pos) {
            p.pos.x = ladder.x;
        }
        if input.left {
            p.facing = -1;
        } else if input.right {
            p.facing = 1;
        }
        if input.up && ladder_at(w, Vec2::new(p.pos.x, p.pos.y - 8.0)).is_some() {
            ay -= 0.6;
        } else if input.down {
            if ladder_at(w, Vec2::new(p.pos.x, p.pos.y + 8.0)).is_some() {
                ay += 0.6;
            } else {
                p.state = PlayerState::Falling;
            }
            if col_bot {
                p.state = PlayerState::Standing;
            }
        }
        if jump_pressed && p.attack_timer == 0 {
            p.buffered_jump_ticks = 0;
            p.coyote_ticks = 0;
            p.vel.x = if input.left {
                -4.0
            } else if input.right {
                4.0
            } else {
                0.0
            };
            ay -= 4.0;
            p.state = PlayerState::Jumping;
            p.jump_time = 0;
            set(p, "jump_released", 0.0);
            ladder_timer = 5.0;
        }
    } else {
        ladder_timer = (ladder_timer - 1.0).max(0.0);
    }
    if airborne(p.state) {
        ay += intensity;
    }
    if col_bot && airborne(p.state) && p.vel.y >= 0.0 {
        p.vel.y = 0.0;
        ay = 0.0;
        p.state = PlayerState::Running;
    }
    if col_bot {
        p.vel.y = 0.0;
    }
    if !col_bot && grounded_state(p.state) {
        p.state = PlayerState::Falling;
        ay += gravity;
        jumped = true;
        if has(p, &["Climbing Gloves", "Gloves", "gloves"]) {
            hang_count = 5.0;
        }
    }
    if col_top && p.state == PlayerState::Jumping {
        p.vel.y = (p.vel.y * 0.3).abs();
    }
    if (col_left && p.facing < 0) || (col_right && p.facing > 0) {
        p.vel.x = 0.0;
    }

    if input.jump_released && airborne(p.state) {
        jumped = true;
    } else if grounded_state(p.state) {
        cape = false;
        jumped = false;
    }

    let normal_jump =
        jump_pressed && (grounded_state(p.state) || can_coyote) && (fall == 0.0 || can_coyote);
    if jump_pressed && (in_web || water_jump) {
        ay -= 4.0;
        p.vel.y -= 3.0;
        ax += p.vel.x / 2.0;
        p.state = PlayerState::Jumping;
        p.jump_time = 0;
        gravity = 1.0;
        set(p, "jump_released", 0.0);
        if in_web {
            if let Some(web) = w
                .entities
                .iter_mut()
                .find(|e| e.alive && e.object == "oWeb" && e.pos.distance(p.pos) < 12.0)
            {
                let life = web.data.entry("life".into()).or_insert(3.0);
                *life -= 1.0;
                if *life <= 0.0 {
                    web.alive = false;
                }
            }
        }
    } else if has(p, &["Cape", "cape"]) && jump_pressed && jumped && airborne(p.state) {
        cape = !cape;
    } else if has(p, &["Jetpack", "jetpack"])
        && input.jump
        && jumped
        && airborne(p.state)
        && fuel > 0.0
    {
        ay -= 2.0;
        p.vel.y = -1.0;
        fuel -= 1.0;
        gravity = 0.0;
        p.state = PlayerState::Jumping;
        p.jump_time = 0;
        set(p, "jump_released", 0.0);
    } else if normal_jump {
        if can_coyote && !grounded_state(p.state) {
            ay = 0.0;
        }
        ay -= 4.0;
        ax += if p.vel.x.abs() > 3.0 {
            p.vel.x * 2.0
        } else {
            p.vel.x / 2.0
        };
        if has(p, &["Jordans", "jordans"]) {
            ay *= 3.0;
            limit_y_acc = 12.0;
            gravity = 0.5;
        } else if has(p, &["Spring Shoes", "SpringShoes", "spring_shoes"]) {
            ay *= 1.5;
        }
        p.state = PlayerState::Falling;
        p.jump_time = 0;
        p.coyote_ticks = 0;
        p.buffered_jump_ticks = 0;
        set(p, "jump_released", 0.0);
        sound = Some("sndJump");
    }
    p.jump_time = (p.jump_time + 1).min(JUMP_TICKS);
    if !input.jump {
        set(p, "jump_released", 1.0);
    }
    if value(p, "jump_released", 1.0) > 0.0 {
        p.jump_time = JUMP_TICKS;
    }
    intensity = f64::from(p.jump_time) / f64::from(JUMP_TICKS) * gravity;

    if input.up
        && grounded_state(p.state)
        && ladder_at(w, p.pos).is_none()
        && p.vel.x == 0.0
        && ax == 0.0
    {
        p.state = PlayerState::LookingUp;
    }
    if !input.up && p.state == PlayerState::LookingUp {
        p.state = PlayerState::Standing;
    }

    if !col_top
        && hang_count == 0.0
        && p.pos.y > 16.0
        && !grounded_state(p.state)
        && p.state != PlayerState::Climbing
    {
        let dir = if input.right && col_right {
            1.0
        } else if input.left && col_left {
            -1.0
        } else {
            0.0
        };
        if dir != 0.0 {
            let edge = Vec2::new(p.pos.x + dir * 9.0, p.pos.y - 5.0);
            let upper_edge = Vec2::new(edge.x, p.pos.y - 6.0);
            let grip = has(p, &["Climbing Gloves", "Gloves", "gloves"]) && p.vel.y > 0.0;
            if ((solid_point(w, edge) || solid_point(w, upper_edge))
                && (grip
                    || (!solid_point(w, Vec2::new(edge.x, p.pos.y - 9.0))
                        && !solid_point(w, Vec2::new(p.pos.x, p.pos.y + 9.0)))))
                || object_at(w, edge, &["oTree"])
                || object_at(w, upper_edge, &["oTree"])
            {
                p.state = PlayerState::Hanging;
                p.pos.x = gm_round(p.pos.x);
                p.pos.y = gm_round(p.pos.y / 8.0) * 8.0;
                p.previous_pos = p.pos;
                p.vel.y = 0.0;
                ay = 0.0;
                gravity = 0.0;
            }
        }
        if hang_count == 0.0
            && p.state == PlayerState::Falling
            && w.entities.iter().any(|e| {
                e.alive
                    && e.object == "oArrow"
                    && e.value("stuck") > 0.0
                    && (e.pos.x - p.pos.x).abs() < 8.0
                    && (e.pos.y - (p.pos.y - 5.0)).abs() < 2.0
            })
        {
            p.state = PlayerState::Hanging;
            p.vel.y = 0.0;
            ay = 0.0;
            gravity = 0.0;
            set(p, "hanging_arrow", 1.0);
        }
    }
    hang_count = (hang_count - 1.0).max(0.0);
    if p.state == PlayerState::Hanging {
        cape = false;
        jumped = false;
        if jump_pressed {
            p.buffered_jump_ticks = 0;
            p.coyote_ticks = 0;
            gravity = 1.0;
            if input.down {
                p.state = PlayerState::Falling;
                ay -= 1.0;
                hang_count = if has(p, &["Climbing Gloves", "Gloves", "gloves"]) {
                    10.0
                } else {
                    5.0
                };
            } else {
                if (p.facing > 0 && input.left) || (p.facing < 0 && input.right) {
                    p.state = PlayerState::Falling;
                    ay -= 1.0;
                } else {
                    p.state = PlayerState::Jumping;
                    ay -= 4.0;
                    p.pos.x -= f64::from(p.facing) * 2.0;
                }
                hang_count = 3.0;
            }
        }
        if !edge(
            w,
            p,
            if p.facing > 0 {
                Side::Right
            } else {
                Side::Left
            },
            2.0,
        ) {
            gravity = 1.0;
            p.state = PlayerState::Falling;
            ay -= 1.0;
            hang_count = 4.0;
        }
    } else {
        gravity = 1.0;
        set(p, "hanging_arrow", 0.0);
    }

    if input.down && grounded_state(p.state) && p.attack_timer == 0 {
        if edge(w, p, Side::Bottom, 1.0) {
            p.state = PlayerState::Ducking;
        } else if let Some(ladder) = ladder_segment_at(w, Vec2::new(p.pos.x, p.pos.y + 16.0)) {
            if (p.pos.x - ladder.x).abs() < 4.0 {
                p.pos.x = ladder.x;
                p.vel = Vec2::default();
                ax = 0.0;
                ay = 0.0;
                p.state = PlayerState::Climbing;
            }
        } else {
            p.pos.y += 1.0;
            p.state = PlayerState::Falling;
            ay += gravity;
        }
    }
    if !input.down && p.state == PlayerState::Ducking {
        p.state = PlayerState::Standing;
        p.vel.x = 0.0;
        ax = 0.0;
    }
    if p.vel.x == 0.0 && ax == 0.0 && p.state == PlayerState::Running {
        p.state = PlayerState::Standing;
    }
    if ax != 0.0 && p.state == PlayerState::Standing {
        p.state = PlayerState::Running;
    }
    if airborne(p.state) {
        if p.vel.y < 0.0 {
            p.state = PlayerState::Jumping;
        } else if p.vel.y > 0.0 {
            p.state = PlayerState::Falling;
        }
    }
    set(p, "collision_half_width", 5.0);
    set(
        p,
        "collision_top",
        if p.state == PlayerState::Falling {
            -6.0
        } else {
            -8.0
        },
    );
    if ladder_timer == 0.0
        && ((input.up
            && (ladder_at(w, p.pos).is_some()
                || (airborne(p.state)
                    && ladder_segment_at(w, Vec2::new(p.pos.x, p.pos.y - 8.0)).is_some())))
            || (input.down
                && grounded_state(p.state)
                && p.vel.x == 0.0
                && object_at(
                    w,
                    Vec2::new(p.pos.x, p.pos.y + 9.0),
                    &["oLadderTop", "oVineTop"],
                )))
    {
        if let Some(ladder) = ladder_segment_at(w, Vec2::new(p.pos.x, p.pos.y - 8.0)) {
            if (p.pos.x - ladder.x).abs() < 4.0 {
                p.pos.x = ladder.x;
                if ladder_at(w, p.pos).is_none() {
                    p.pos.y = ladder.y + 14.0;
                }
                p.vel = Vec2::default();
                ax = 0.0;
                ay = 0.0;
                p.state = PlayerState::Climbing;
                p.previous_pos = p.pos;
            }
        }
    }
    let mut xf = if airborne(p.state) { 0.8 } else { 0.6 };
    let mut yf = 1.0;
    if p.state == PlayerState::Climbing {
        xf = 0.6;
        yf = 0.6;
    } else {
        if run_key && grounded_state(p.state) && run_held >= 10.0 && (input.left || input.right) {
            p.vel.x += if input.left { -0.1 } else { 0.1 };
            limit_x = 6.0;
            xf = 0.98;
        } else if p.state == PlayerState::Ducking {
            if p.vel.x.abs() >= 2.0 {
                p.vel.x *= 0.8;
                if p.vel.x < 0.5 {
                    p.vel.x = 0.0;
                }
            }
            xf = 0.2;
            limit_x = 3.0;
        }
        if in_web {
            xf = 0.2;
            yf = 0.2;
            fall = 0.0;
        } else if in_water {
            cape = false;
            yf = if p.state == PlayerState::Falling && p.vel.y > 0.0 {
                0.5
            } else if water_surface {
                1.0
            } else {
                0.9
            };
        }
    }
    if object_at(w, Vec2::new(p.pos.x, p.pos.y + 8.0), &["oIce", "oIceBlock"])
        && p.state != PlayerState::Ducking
        && !has(p, &["Spike Shoes", "SpikeShoes", "spike_shoes"])
    {
        xf = 0.98;
        yf = 1.0;
    }
    if grounded_state(p.state) && !input.jump && !input.down && !run_key {
        limit_x = 3.0;
    }
    if p.state == PlayerState::Ducking
        && p.vel.x.abs() < 3.0
        && solid_point(w, Vec2::new(p.pos.x, p.pos.y + 9.0))
        && ((p.facing < 0
            && input.left
            && !solid_point(w, Vec2::new(p.pos.x - 1.0, p.pos.y + 9.0)))
            || (p.facing > 0
                && input.right
                && !solid_point(w, Vec2::new(p.pos.x + 1.0, p.pos.y + 9.0))))
    {
        set(p, "duck_to_hang", 1.0);
        p.vel = Vec2::default();
        p.sprite = character_sprite(p, "sDuckToHangL").into();
        p.animation = 0.0;
        if let Some(id) = p.held.take() {
            if let Some(held) = w.entity_mut(id) {
                held.held = false;
                held.vel = Vec2::new(f64::from(p.facing), -4.0);
            }
        }
        return;
    }
    if cape || parachute {
        yf = 0.5;
        fall = 0.0;
    }
    p.vel.x = (p.vel.x + ax.clamp(-9.0, 9.0)) * xf;
    p.vel.y = (p.vel.y + ay.clamp(-limit_y_acc, limit_y_acc)) * yf;
    apply_ball_tether(w, p, input, &mut fall);
    p.vel.x = p.vel.x.clamp(-limit_x, limit_x);
    p.vel.y = p.vel.y.clamp(-10.0, 10.0);
    if p.vel.x.abs() < 0.1 {
        p.vel.x = 0.0;
    }
    if p.vel.y.abs() < 0.1 {
        p.vel.y = 0.0;
    }
    slope_move(w, p, engine_time, input.down, col_top);
    p.grounded = below(w, p) && p.vel.y >= 0.0 && p.state != PlayerState::Climbing;
    if game.rules == RulesPreset::Assisted {
        if !col_bot {
            p.coyote_ticks = p.coyote_ticks.saturating_sub(1);
        }
        p.buffered_jump_ticks = p.buffered_jump_ticks.saturating_sub(1);
    }
    set(p, "left_steps", left_steps);
    set(p, "right_steps", right_steps);
    set(p, "run_held", run_held);
    set(p, "fall_timer", fall);
    set(p, "fall_ticks", fall);
    set(p, "gravity", gravity);
    set(p, "gravity_intensity", intensity);
    set(p, "ladder_timer", ladder_timer);
    set(p, "hang_count", hang_count);
    set(p, "cape_open", f64::from(cape));
    set(p, "parachute_open", f64::from(parachute));
    set(p, "jetpack_fuel", fuel);
    set(p, "jumped", f64::from(jumped));
    set(p, "velocity_limit_x", limit_x);
    animate(p, input, w);
    if let Some(name) = sound {
        game.sound(name, game.player.pos);
    }
}

fn duck_to_hang_tick(w: &World, p: &mut Player) {
    p.vel = Vec2::default();
    p.animation += 0.8;
    // The original flip sprite has nine poses; its animation-end event performs
    // the displacement and either grabs an adjacent ladder or reverses facing.
    if p.animation < 9.0 {
        return;
    }
    p.pos.y = gm_round((p.pos.y + 16.0) / 8.0) * 8.0;
    if let Some(ladder) = ladder_at(w, Vec2::new(p.pos.x + f64::from(p.facing) * 8.0, p.pos.y)) {
        p.pos.x = ladder.x;
        p.state = PlayerState::Climbing;
    } else {
        p.pos.x += if p.facing < 0 { -5.0 } else { 6.0 };
        p.facing = -p.facing;
        p.state = PlayerState::Hanging;
    }
    p.previous_pos = p.pos;
    p.animation = 0.0;
    p.sprite = character_sprite(
        p,
        if p.state == PlayerState::Hanging {
            "sHangLeft"
        } else {
            "sClimbUp"
        },
    )
    .into();
    set(p, "duck_to_hang", 0.0);
    set(p, "gravity", 0.0);
    set(p, "fall_timer", 0.0);
    set(p, "fall_ticks", 0.0);
}

fn stunned_tick(w: &mut World, p: &mut Player, time: u64) -> bool {
    let bounced = value(p, "bounced", 0.0) > 0.0;
    p.vel.y += if bounced { 1.0 } else { 0.6 };
    if p.vel.y < 0.0 && edge(w, p, Side::Top, 1.0) {
        p.vel.y *= -0.8;
    }
    if edge(w, p, Side::Left, 1.0) || edge(w, p, Side::Right, 1.0) {
        p.vel.x *= -0.5;
    }
    if below(w, p) {
        p.vel.y = if p.vel.y > 1.0 { -p.vel.y * 0.5 } else { 0.0 };
        p.vel.x *= if object_at(w, Vec2::new(p.pos.x, p.pos.y + 16.0), &["oIce"]) {
            0.8
        } else {
            0.3
        };
        if p.vel.x.abs() < 0.1 {
            p.vel.x = 0.0;
        }
        set(p, "bounced", 1.0);
    }
    p.vel.x = p.vel.x.clamp(-10.0, 10.0);
    p.vel.y = p.vel.y.clamp(-10.0, 10.0);
    move_player(w, p, time, false);
    let impact = value(p, "wall_hurt", value(p, "wallHurt", 0.0)) > 0.0
        && (edge(w, p, Side::Left, 1.0)
            || edge(w, p, Side::Right, 1.0)
            || edge(w, p, Side::Bottom, 1.0));
    if impact {
        p.health -= 1;
        let remaining = value(p, "wall_hurt", value(p, "wallHurt", 0.0)) - 1.0;
        set(p, "wall_hurt", remaining);
        set(p, "wallHurt", remaining);
    }
    p.grounded = below(w, p);
    p.state = if p.health <= 0 {
        PlayerState::Dead
    } else if p.stun > 0 {
        PlayerState::Stunned
    } else {
        PlayerState::Falling
    };
    p.sprite = character_sprite(
        p,
        if p.vel.x == 0.0 {
            if p.health <= 0 { "sDieL" } else { "sStunL" }
        } else if value(p, "bounced", 0.0) > 0.0 {
            if p.vel.y < 0.0 {
                "sDieLBounce"
            } else {
                "sDieLFall"
            }
        } else if p.vel.x < 0.0 {
            "sDieLL"
        } else {
            "sDieLR"
        },
    )
    .into();
    p.animation += 0.4;
    impact
}

fn animate(p: &mut Player, input: &InputFrame, w: &World) {
    // The engine restores its collision mask even while characterSprite skips
    // a whipping character. Attack animation ownership must not skip this.
    set(p, "collision_top", -8.0);
    set(
        p,
        "collision_half_width",
        if p.vel.x.abs() >= 4.0 && grounded_state(p.state) {
            8.0
        } else {
            5.0
        },
    );
    if p.attack_timer > 0 {
        return;
    }
    let mut speed = 0.0;
    let name = match p.state {
        PlayerState::Standing => {
            if !solid_point(w, Vec2::new(p.pos.x - 2.0, p.pos.y + 9.0)) {
                speed = 0.6;
                "sWhoaLeft"
            } else {
                "sStandLeft"
            }
        }
        PlayerState::Running => {
            speed = p.vel.x.abs() * 0.1 + 0.1;
            if input.up { "sLookRunL" } else { "sRunLeft" }
        }
        PlayerState::Ducking => {
            speed = 0.8;
            if p.vel.x == 0.0 {
                "sDuckLeft"
            } else if p.vel.x.abs() < 3.0 {
                "sCrawlLeft"
            } else {
                "sRunLeft"
            }
        }
        PlayerState::LookingUp => {
            if p.vel.x == 0.0 {
                "sLookLeft"
            } else {
                speed = p.vel.x.abs() * 0.1 + 0.1;
                "sLookRunL"
            }
        }
        PlayerState::Jumping => "sJumpLeft",
        PlayerState::Falling => {
            let count = value(p, "fall_state_ticks", 0.0);
            if count >= 2.0 {
                "sFallLeft"
            } else if crate::catalog::sprite_def(&p.sprite).is_none() {
                "sJumpLeft"
            } else {
                p.sprite.as_str()
            }
        }
        PlayerState::Hanging => "sHangLeft",
        PlayerState::Climbing => {
            speed = p.vel.x.hypot(p.vel.y) * 0.4;
            if object_at(w, p.pos, &["oRope", "oRopeTop", "oRopeSegment"]) {
                if input.down { "sClimbUp3" } else { "sClimbUp2" }
            } else {
                "sClimbUp"
            }
        }
        PlayerState::Stunned => {
            speed = 0.4;
            "sStunL"
        }
        PlayerState::Dead => "sDieL",
    }
    .to_owned();
    if p.vel.x.abs() >= 4.0 {
        speed = 1.0;
        if grounded_state(p.state) {
            set(p, "collision_half_width", 8.0);
        }
    }
    // characterStepEvent resets the falling upper bound after moving; its -6
    // bound is specifically a movement-phase mask, not the next tick's mask.
    set(p, "collision_top", -8.0);
    let name = character_sprite(p, &name).to_owned();
    if name != p.sprite {
        p.sprite = name;
        p.animation = 0.0;
    }
    p.animation += speed.min(1.0);
    let fall_count = if p.state == PlayerState::Falling {
        value(p, "fall_state_ticks", 0.0) + 1.0
    } else {
        0.0
    };
    set(p, "fall_state_ticks", fall_count);
}

fn character_sprite<'a>(p: &Player, name: &'a str) -> &'a str {
    if value(p, "character_tunnelman", 0.0) > 0.0 {
        match name {
            "sStandLeft" => "sTunnelLeft",
            "sWhoaLeft" => "sTunnelWhoaL",
            "sRunLeft" => "sTunnelRunL",
            "sLookRunL" => "sTunnelLookRunL",
            "sDuckLeft" => "sTunnelDuckL",
            "sCrawlLeft" => "sTunnelCrawlL",
            "sLookLeft" => "sTunnelLookL",
            "sJumpLeft" => "sTunnelJumpL",
            "sFallLeft" | "sDieLFall" => "sTunnelFallL",
            "sHangLeft" => "sTunnelHangL",
            "sPushLeft" => "sTunnelPushL",
            "sDuckToHangL" => "sTunnelDtHL",
            "sClimbUp" => "sTunnelClimb",
            "sClimbUp2" => "sTunnelClimb2",
            "sClimbUp3" => "sTunnelClimb3",
            "sDieL" => "sTunnelDieL",
            "sStunL" => "sTunnelStunL",
            "sDieLBounce" => "sTunnelLBounce",
            "sDieLL" => "sTunnelDieLL",
            "sDieLR" => "sTunnelDieLR",
            _ => name,
        }
    } else if value(p, "character_damsel", 0.0) > 0.0 {
        match name {
            "sStandLeft" => "sDamselLeft",
            "sWhoaLeft" => "sDamselWhoaL",
            "sRunLeft" | "sLookRunL" => "sDamselRunL",
            "sDuckLeft" => "sDamselDuckL",
            "sCrawlLeft" => "sDamselCrawlL",
            "sLookLeft" => "sDamselLookL",
            "sJumpLeft" | "sDieLR" => "sDamselDieLR",
            "sFallLeft" | "sDieLFall" => "sDamselFallL",
            "sHangLeft" => "sDamselHangL",
            "sPushLeft" => "sDamselPushL",
            "sDuckToHangL" => "sDamselDtHL",
            "sClimbUp" => "sDamselClimb",
            "sClimbUp2" => "sDamselClimb2",
            "sClimbUp3" => "sDamselClimb3",
            "sDieL" => "sDamselDieL",
            "sStunL" => "sDamselStunL",
            "sDieLBounce" => "sDamselBounceL",
            "sDieLL" => "sDamselDieLL",
            _ => name,
        }
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn room(rules: RulesPreset) -> Game {
        let mut g = Game::shell(1, rules);
        g.world = World::new(20, 20, Area::Mines, 1);
        for x in 0..20 {
            g.world.set_tile(x, 10, "oBlock");
        }
        g.player.pos = Vec2::new(88.0, 152.0);
        g.player.previous_pos = g.player.pos;
        g.player.state = PlayerState::Standing;
        g.player.grounded = true;
        g
    }
    fn step(g: &mut Game, i: InputFrame) {
        g.tick += 1;
        g.level_tick += 1;
        tick(g, &i);
    }
    #[test]
    fn classic_fraction_schedule_is_not_an_accumulator() {
        assert_eq!(
            (1..=6).map(|t| displacement(0.3, t)).collect::<Vec<_>>(),
            [0, 0, 1, 0, 0, 1]
        );
        assert_eq!(
            (1..=4).map(|t| displacement(-1.6, t)).collect::<Vec<_>>(),
            [-1, -2, -1, -2]
        );
        assert_eq!(
            (1..=4).map(|t| displacement(0.4, t)).collect::<Vec<_>>(),
            [0, 1, 0, 1]
        );
    }
    #[test]
    fn classic_waits_two_ticks_before_running_acceleration() {
        let mut g = room(RulesPreset::Classic);
        let i = InputFrame {
            right: true,
            ..Default::default()
        };
        step(&mut g, i);
        step(&mut g, i);
        assert_eq!(g.player.vel.x, 0.0);
        step(&mut g, i);
        assert!((g.player.vel.x - 1.8).abs() < 1e-12);
        for _ in 0..10 {
            step(&mut g, i);
        }
        assert_eq!(g.player.vel.x, 3.0);
    }
    #[test]
    fn direction_release_uses_original_half_pixel_acceleration() {
        let mut g = room(RulesPreset::Classic);
        step(
            &mut g,
            InputFrame {
                right: true,
                ..Default::default()
            },
        );
        step(&mut g, InputFrame::default());
        assert!((g.player.vel.x - 0.3).abs() < 1e-12);
    }
    #[test]
    fn jump_is_minus_four_then_held_gravity_increases_in_tenths() {
        let mut g = room(RulesPreset::Classic);
        step(
            &mut g,
            InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.vel.y, -4.0);
        assert_eq!(g.player.pos.y, 148.0);
        step(
            &mut g,
            InputFrame {
                jump: true,
                ..Default::default()
            },
        );
        assert!((g.player.vel.y + 3.9).abs() < 1e-12);
        step(
            &mut g,
            InputFrame {
                jump_released: true,
                ..Default::default()
            },
        );
        let before = g.player.vel.y;
        step(&mut g, InputFrame::default());
        assert!((g.player.vel.y - before - 1.0).abs() < 1e-12);
    }
    #[test]
    fn sprint_reaches_original_six_pixel_limit() {
        let mut g = room(RulesPreset::Classic);
        for _ in 0..10 {
            step(
                &mut g,
                InputFrame {
                    right: true,
                    run: true,
                    ..Default::default()
                },
            );
        }
        assert_eq!(g.player.vel.x, 6.0);
    }
    #[test]
    fn assisted_coyote_is_opt_in_and_expires() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut g = room(rules);
            step(&mut g, InputFrame::default());
            for x in 0..20 {
                g.world.set_tile(x, 10, "");
            }
            step(
                &mut g,
                InputFrame {
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                },
            );
            assert_eq!(g.player.vel.y < 0.0, rules == RulesPreset::Assisted);
        }
        let mut g = room(RulesPreset::Assisted);
        step(&mut g, InputFrame::default());
        for x in 0..20 {
            g.world.set_tile(x, 10, "");
        }
        for _ in 0..3 {
            step(&mut g, InputFrame::default());
        }
        step(
            &mut g,
            InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            },
        );
        assert!(g.player.vel.y > 0.0);
    }
    #[test]
    fn assisted_buffers_before_landing_but_classic_does_not() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut g = room(rules);
            g.player.pos.y = 151.0;
            g.player.vel.y = 1.0;
            g.player.state = PlayerState::Falling;
            step(
                &mut g,
                InputFrame {
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                },
            );
            step(
                &mut g,
                InputFrame {
                    jump: true,
                    ..Default::default()
                },
            );
            assert_eq!(g.player.vel.y < 0.0, rules == RulesPreset::Assisted);
        }
    }
    #[test]
    fn collision_never_tunnels_through_one_tile_wall() {
        let mut g = room(RulesPreset::Classic);
        g.world.set_tile(7, 9, "oBlock");
        g.player.pos = Vec2::new(100.0, 140.0);
        g.player.state = PlayerState::Falling;
        g.player.vel.x = 16.0;
        step(&mut g, InputFrame::default());
        assert!(g.player.pos.x <= 107.0);
        assert!(!g.world.collides(bounds_at(&g.player, g.player.pos)));
    }
    #[test]
    fn spring_shoes_multiply_jump_acceleration() {
        let mut g = room(RulesPreset::Classic);
        g.player.equipment.insert("Spring Shoes".into());
        step(
            &mut g,
            InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.vel.y, -6.0);
    }
    #[test]
    fn ladder_grab_snaps_and_climb_uses_point_six_acceleration() {
        let mut g = room(RulesPreset::Classic);
        g.world.set_tile(5, 9, "oLadder");
        g.world.set_tile(5, 8, "oLadder");
        g.player.pos.x = 89.0;
        step(
            &mut g,
            InputFrame {
                up: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.state, PlayerState::Climbing);
        assert_eq!(g.player.pos.x, 88.0);
        step(
            &mut g,
            InputFrame {
                up: true,
                ..Default::default()
            },
        );
        assert!((g.player.vel.y + 0.36).abs() < 1e-12);
    }
    #[test]
    fn fall_injury_uses_descending_ticks_and_bypasses_hit_flash() {
        for (ticks, damage) in [
            (16.0, 0),
            (17.0, 1),
            (32.0, 1),
            (33.0, 2),
            (48.0, 2),
            (49.0, 10),
        ] {
            let mut g = room(RulesPreset::Classic);
            g.player.health = 20;
            g.player.invincible = 30;
            set(&mut g.player, "fall_timer", ticks);
            step(&mut g, InputFrame::default());
            assert_eq!(g.player.health, 20 - damage, "descending ticks={ticks}");
            if damage > 0 {
                assert_eq!(g.player.stun, 60);
            }
        }
    }
    #[test]
    fn open_room_bottom_does_not_act_as_an_invisible_floor() {
        let mut g = room(RulesPreset::Classic);
        g.player.pos = Vec2::new(80.0, 319.0);
        g.player.state = PlayerState::Falling;
        g.player.vel.y = 10.0;
        step(&mut g, InputFrame::default());
        assert!(g.player.pos.y > 320.0);
    }
    #[test]
    fn mattock_prevents_jump_for_twenty_active_ticks_in_both_presets() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut g = room(rules);
            set(&mut g.player, "cantJump", 20.0);
            for _ in 0..20 {
                step(
                    &mut g,
                    InputFrame {
                        jump: true,
                        jump_pressed: true,
                        ..Default::default()
                    },
                );
                assert_eq!(g.player.pos.y, 152.0);
                assert_eq!(g.player.vel.y, 0.0);
            }
            step(
                &mut g,
                InputFrame {
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                },
            );
            assert_eq!(g.player.vel.y, -4.0);
        }
    }
    #[test]
    fn flat_ground_preserves_running_distance_through_slope_path() {
        let mut g = room(RulesPreset::Classic);
        g.player.vel.x = 3.0;
        for _ in 0..4 {
            slope_move(&mut g.world, &mut g.player, 1, false, false);
        }
        assert_eq!(g.player.pos, Vec2::new(100.0, 152.0));
    }
    #[test]
    fn low_step_is_climbed_with_slope_distance_damping() {
        let mut g = room(RulesPreset::Classic);
        g.player.pos.x = 94.0;
        g.player.vel.x = 4.0;
        let id = g.world.spawn("oPushBlock", Vec2::new(101.0, 156.0));
        g.world.entity_mut(id).unwrap().solid = true;
        slope_move(&mut g.world, &mut g.player, 1, false, false);
        assert_eq!(g.player.pos, Vec2::new(97.0, 148.0));
        assert!(!solid(&g.world, &g.player, g.player.pos));
    }
    #[test]
    fn full_height_block_remains_a_wall_under_slope_movement() {
        let mut g = room(RulesPreset::Classic);
        g.player.pos.x = 94.0;
        g.player.vel.x = 4.0;
        g.world.set_tile(6, 9, "oBlock");
        g.player.pos.x = 90.0;
        slope_move(&mut g.world, &mut g.player, 1, false, false);
        assert!(g.player.pos.x <= 91.0);
        assert_eq!(g.player.pos.y, 152.0);
    }
    #[test]
    fn ball_tether_only_limits_movement_when_bounding_gap_reaches_twenty_four() {
        let mut g = room(RulesPreset::Classic);
        g.player.pos = Vec2::new(88.0, 80.0);
        g.player.sprite = "sStandLeft".into();
        g.player.state = PlayerState::Falling;
        g.player.vel.x = -3.0;
        let ball = g.world.spawn("oBall", Vec2::new(120.0, 80.0));
        step(&mut g, InputFrame::default());
        assert!(g.player.vel.x < 0.0);
        g.player.pos = Vec2::new(88.0, 80.0);
        g.player.vel.x = -3.0;
        g.world.entity_mut(ball).unwrap().pos.x = 124.0;
        step(&mut g, InputFrame::default());
        assert_eq!(g.player.vel.x, 0.0);
    }
    #[test]
    fn suspended_ball_tether_swings_toward_ball_and_resets_fall_ticks() {
        let mut g = room(RulesPreset::Classic);
        g.player.pos = Vec2::new(88.0, 100.0);
        g.player.state = PlayerState::Falling;
        g.player.vel.y = 3.0;
        set(&mut g.player, "fall_timer", 12.0);
        g.world.spawn("oBall", Vec2::new(80.0, 40.0));
        step(&mut g, InputFrame::default());
        assert_eq!(g.player.vel.y, 0.0);
        assert_eq!(g.player.vel.x, -1.0);
        assert_eq!(value(&g.player, "fall_ticks", -1.0), 0.0);
    }
    #[test]
    fn branch_tile_is_a_one_way_platform_but_ice_bottom_is_decoration() {
        let mut g = room(RulesPreset::Classic);
        g.world.set_tile(5, 8, "oTreeBranchTile");
        g.player.pos.y = 119.0;
        g.player.state = PlayerState::Falling;
        g.player.vel.y = 3.0;
        step(&mut g, InputFrame::default());
        assert_eq!(g.player.pos.y, 120.0);
        g.world.set_tile(5, 8, "oIceBottom");
        step(&mut g, InputFrame::default());
        assert!(g.player.pos.y > 120.0);
    }
    #[test]
    fn selected_characters_use_original_jump_and_stun_art() {
        for (flag, jump_sprite, stun_sprite) in [
            ("character_damsel", "sDamselDieLR", "sDamselStunL"),
            ("character_tunnelman", "sTunnelJumpL", "sTunnelStunL"),
        ] {
            let mut g = room(RulesPreset::Classic);
            g.flags.insert(flag.into());
            step(
                &mut g,
                InputFrame {
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                },
            );
            step(
                &mut g,
                InputFrame {
                    jump: true,
                    ..Default::default()
                },
            );
            assert_eq!(g.player.sprite, jump_sprite);
            assert!(crate::catalog::sprite_def(&g.player.sprite).is_some());
            g.player.stun = 10;
            g.player.vel.x = 0.0;
            step(&mut g, InputFrame::default());
            assert_eq!(g.player.sprite, stun_sprite);
        }
    }
    #[test]
    fn empty_handed_tunnel_attack_blocks_jump_until_attack_sprite_ends() {
        let mut g = room(RulesPreset::Assisted);
        g.flags.insert("character_tunnelman".into());
        g.player.sprite = "sTunnelAttackL".into();
        g.player.attack_timer = 3;
        step(
            &mut g,
            InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.vel.y, 0.0);
        assert_eq!(g.player.buffered_jump_ticks, 0);
        g.player.sprite = "sTunnelLeft".into();
        g.player.attack_timer = 0;
        step(
            &mut g,
            InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.vel.y, -4.0);
    }
    #[test]
    fn actual_ledge_grab_can_jump_around_corner_and_land_above_both_sides() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for character in [
                crate::characters::Character::Explorer,
                crate::characters::Character::Damsel,
                crate::characters::Character::TunnelMan,
            ] {
                for facing in [-1, 1] {
                    let mut g = room(rules);
                    crate::characters::apply(&mut g, character);
                    // Reach the ledge after ordinary grounded play, which sets
                    // Classic's walking speed limit before the airborne grab.
                    g.step(&InputFrame::default());
                    for y in 5..10 {
                        g.world.set_tile(10, y, "oBlock");
                    }
                    g.player.pos = Vec2::new(if facing > 0 { 155.0 } else { 181.0 }, 86.0);
                    g.player.previous_pos = g.player.pos;
                    g.player.facing = facing;
                    g.player.state = PlayerState::Falling;
                    g.player.grounded = false;
                    g.player.vel.y = 1.0;
                    let toward = InputFrame {
                        left: facing < 0,
                        right: facing > 0,
                        ..Default::default()
                    };
                    g.step(&toward);
                    assert_eq!(
                        g.player.state,
                        PlayerState::Hanging,
                        "{rules:?}/{character:?}/{facing}"
                    );
                    assert_eq!(g.player.pos.y, 88.0);
                    g.step(&InputFrame {
                        jump: true,
                        jump_pressed: true,
                        ..toward
                    });
                    assert_eq!(
                        g.player.vel.y, -5.0,
                        "ledge jump must include detached-support impulse"
                    );
                    assert_eq!(value(&g.player, "hang_count", 0.0), 4.0);
                    let mut landed_above = false;
                    for _ in 0..14 {
                        g.step(&InputFrame {
                            jump: true,
                            ..toward
                        });
                        landed_above |= g.player.grounded
                            && g.player.pos.y == 72.0
                            && g.player.pos.x >= 155.0
                            && g.player.pos.x <= 181.0;
                    }
                    assert!(
                        landed_above,
                        "failed ledge climb {rules:?}/{character:?}/{facing}: {:?} {:?}",
                        g.player.pos, g.player.state
                    );
                }
            }
        }
    }
    #[test]
    fn restored_falling_head_mask_can_escape_ceiling_overlap_without_false_landing() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for facing in [-1, 1] {
                let mut g = room(rules);
                for x in 0..20 {
                    g.world.set_tile(x, 10, "");
                }
                g.world.set_tile(6, 9, "oBlock");
                g.player.pos = Vec2::new(if facing > 0 { 94.0 } else { 113.0 }, 166.0);
                g.player.state = PlayerState::Falling;
                g.player.vel = Vec2::new(0.0, 2.0);
                g.player.facing = facing;
                set(&mut g.player, "collision_top", -8.0);
                g.step(&InputFrame::default());
                assert_eq!(g.player.pos.y, 169.0);
                assert!(!g.player.grounded);
                assert!(g.player.vel.y > 0.0);
                assert!(!solid(&g.world, &g.player, g.player.pos));
            }
        }
    }
    #[test]
    fn movement_head_band_can_skim_ledge_corner_in_both_directions() {
        for facing in [-1, 1] {
            let mut g = room(RulesPreset::Classic);
            g.world.set_tile(6, 5, "oBlock");
            g.player.pos = Vec2::new(if facing > 0 { 91.0 } else { 117.0 }, 100.0);
            g.player.state = PlayerState::Jumping;
            g.player.vel.y = -1.0;
            g.player.facing = facing;
            set(
                &mut g.player,
                if facing > 0 {
                    "right_steps"
                } else {
                    "left_steps"
                },
                2.0,
            );
            let before = g.player.pos;
            g.step(&InputFrame {
                right: facing > 0,
                left: facing < 0,
                jump: true,
                ..Default::default()
            });
            assert!((g.player.pos.x - before.x) * f64::from(facing) > 0.0);
        }
    }
    #[test]
    fn landing_clears_jetpack_release_latch_before_a_fresh_jump() {
        let mut g = room(RulesPreset::Classic);
        g.player.state = PlayerState::Falling;
        g.player.equipment.insert("Jetpack".into());
        set(&mut g.player, "jumped", 1.0);
        set(&mut g.player, "jetpack_fuel", 10.0);
        g.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        assert_eq!(value(&g.player, "jumped", -1.0), 0.0);
        g.step(&InputFrame {
            jump: true,
            ..Default::default()
        });
        assert_eq!(value(&g.player, "jetpack_fuel", -1.0), 10.0);
        assert!((g.player.vel.y + 3.9).abs() < 1e-12);
    }
    #[test]
    fn source_dynamic_solid_ledges_grab_like_authored_tile_ledges() {
        for facing in [-1, 1] {
            let mut g = room(RulesPreset::Classic);
            g.world.set_tile(6, 7, "oBlock");
            g.world.spawn("oPushBlock", Vec2::new(96.0, 96.0));
            g.player.pos = Vec2::new(if facing > 0 { 91.0 } else { 117.0 }, 101.0);
            g.player.facing = facing;
            g.player.state = PlayerState::Falling;
            g.player.vel.y = 1.0;
            g.step(&InputFrame {
                left: facing < 0,
                right: facing > 0,
                ..Default::default()
            });
            assert_eq!(g.player.state, PlayerState::Hanging);
            assert_eq!(g.player.pos.y, 104.0);
            assert_eq!(
                g.player.previous_pos, g.player.pos,
                "snapped grab must reset interpolation"
            );
        }
    }
    #[test]
    fn grabbing_and_jumping_away_uses_three_tick_lockout_not_down_jump_lockout() {
        for down in [false, true] {
            let mut g = room(RulesPreset::Classic);
            g.world.set_tile(6, 6, "oBlock");
            g.player.pos = Vec2::new(91.0, 101.0);
            g.player.state = PlayerState::Falling;
            g.player.vel.y = 1.0;
            g.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            assert_eq!(g.player.state, PlayerState::Hanging);
            g.step(&InputFrame {
                left: !down,
                down,
                jump: true,
                jump_pressed: true,
                ..Default::default()
            });
            assert_eq!(
                value(&g.player, "hang_count", 0.0),
                if down { 5.0 } else { 3.0 }
            );
            assert_eq!(g.player.vel.y, -1.0);
        }
    }
    #[test]
    fn moveable_block_accelerates_immediately_at_push_rate_and_stops_at_wall() {
        let mut g = room(RulesPreset::Classic);
        g.player.pos.x = 91.0;
        let block = g.world.spawn("oPushBlock", Vec2::new(96.0, 144.0));
        g.world.set_tile(8, 9, "oBlock");
        let input = InputFrame {
            right: true,
            ..Default::default()
        };
        g.step(&input);
        assert!((g.player.vel.x - 0.6).abs() < 1e-12);
        assert_eq!(g.world.entity(block).unwrap().pos.x, 97.0);
        for _ in 0..40 {
            g.step(&input);
        }
        assert_eq!(g.world.entity(block).unwrap().pos.x, 112.0);
        assert!(g.player.pos.x <= 107.0);
    }
    #[test]
    fn player_can_push_an_ice_block_while_airborne_and_facing_away() {
        let mut g = room(RulesPreset::Classic);
        g.world.set_tile(6, 7, "oBlock");
        g.world.set_tile(7, 7, "oBlock");
        let block = g.world.spawn("oIceBlock", Vec2::new(96.0, 96.0));
        g.player.pos = Vec2::new(91.0, 104.0);
        g.player.state = PlayerState::Jumping;
        g.player.vel = Vec2::new(6.0, -1.0);
        g.step(&InputFrame {
            left: true,
            jump: true,
            ..Default::default()
        });
        assert_eq!(g.player.facing, -1);
        assert!(g.world.entity(block).unwrap().pos.x > 96.0);
    }
    #[test]
    fn underwater_jump_uses_current_head_edge_mask_instead_of_fixed_center_point() {
        let mut g = room(RulesPreset::Classic);
        g.world.set_tile(5, 5, "oWaterSwim");
        g.player.pos = Vec2::new(88.0, 88.0);
        g.player.state = PlayerState::Falling;
        g.player.vel.y = 1.0;
        set(&mut g.player, "collision_top", -6.0);
        g.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        assert!(g.player.vel.y < 0.0);
        assert!(g.player.pos.y < 88.0);
    }
    #[test]
    fn thrown_wall_injury_bypasses_hit_flash_and_is_consumed_once() {
        let mut g = room(RulesPreset::Classic);
        g.world.set_tile(6, 9, "oBlock");
        g.player.pos = Vec2::new(91.0, 152.0);
        g.player.vel = Vec2::new(6.0, -1.0);
        g.player.stun = 20;
        g.player.invincible = 60;
        g.player.sprite = "sDieLR".into();
        set(&mut g.player, "wall_hurt", 1.0);
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 3);
        assert_eq!(value(&g.player, "wall_hurt", -1.0), 0.0);
        assert!(g.events.contains(&GameEvent::Damage { amount: 1 }));
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 3);
    }
    #[test]
    fn attack_animation_does_not_leave_falling_collision_mask_shrunk_next_tick() {
        let mut g = room(RulesPreset::Classic);
        g.player.pos = Vec2::new(80.0, 80.0);
        g.player.state = PlayerState::Falling;
        g.player.vel.y = 1.0;
        g.player.attack_timer = 5;
        set(&mut g.player, "collision_top", -6.0);
        g.step(&InputFrame::default());
        assert_eq!(value(&g.player, "collision_top", 0.0), -8.0);
    }
    #[test]
    fn natural_grid_spawn_walk_fall_grab_release_repress_sequence_climbs_ledge() {
        for facing in [-1, 1] {
            let mut g = room(RulesPreset::Classic);
            for x in 10..14 {
                for y in 5..10 {
                    g.world.set_tile(x, y, "oBlock");
                }
            }
            g.world
                .set_tile(if facing > 0 { 8 } else { 15 }, 5, "oBlock");
            g.player.pos = Vec2::new(if facing > 0 { 136.0 } else { 248.0 }, 72.0);
            g.player.state = PlayerState::Falling;
            g.player.grounded = false;
            let toward = InputFrame {
                left: facing < 0,
                right: facing > 0,
                ..Default::default()
            };
            g.step(&InputFrame::default());
            g.step(&InputFrame::default());
            assert!(g.player.grounded);
            let mut held = false;
            let mut grabbed = false;
            for _ in 0..60 {
                let press = !held && !g.player.grounded;
                if press {
                    held = true;
                }
                g.step(&InputFrame {
                    jump: held,
                    jump_pressed: press,
                    ..toward
                });
                if g.player.state == PlayerState::Hanging {
                    grabbed = true;
                    break;
                }
            }
            assert!(grabbed, "natural walk/fall did not reach ledge {facing}");
            assert_eq!(g.player.pos.y, 88.0);
            g.step(&InputFrame {
                jump_released: true,
                ..toward
            });
            assert_eq!(g.player.state, PlayerState::Hanging);
            g.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                ..toward
            });
            assert_eq!(g.player.vel.y, -5.0);
            let mut landed = false;
            for _ in 0..20 {
                g.step(&InputFrame {
                    jump: true,
                    ..toward
                });
                landed |= g.player.grounded
                    && g.player.pos.y == 72.0
                    && g.player.pos.x > 155.0
                    && g.player.pos.x < 229.0;
            }
            assert!(landed, "natural sequence did not land atop ledge {facing}");
        }
    }
}
