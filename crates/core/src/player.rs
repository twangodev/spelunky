//! Classic's 30 Hz character controller.
//!
//! Constants and ordering follow `characterCreateEvent`, `characterStepEvent`,
//! `moveTo`, and `oPlayer1`'s Step event in the archived 1.1 source. In particular,
//! ordinary gravity is 1 (the separate stunned/dead controller starts at 0.6),
//! and fractional movement is scheduled against the global tick, not accumulated.
use crate::model::*;
mod platform_state;
pub use platform_state::source_climbing;
pub(crate) use platform_state::{
    assign_platform, enter_dead, enter_stunned, revive, source_airborne, source_ducking,
    source_falling, source_grounded as source_platform_grounded, source_hanging,
};

const JUMP_TICKS: u32 = 10;
const ASSIST_TICKS: u32 = 3;

fn value(p: &Player, key: &str, default: f64) -> f64 {
    p.data.get(key).copied().unwrap_or(default)
}
fn set(p: &mut Player, key: &str, n: f64) {
    p.data.insert(key.into(), n);
}
/// The character controller samples this before movement; item actions read
/// the same source variable later in oPlayer1's Step, without another query.
pub(crate) fn cached_col_top(player: &Player) -> bool {
    value(player, "colTop", 0.0) != 0.0
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
/// Source ON_GROUND is a platform state, independently of geometric support.
pub(crate) fn source_grounded(p: &Player) -> bool {
    if matches!(p.state, PlayerState::Stunned | PlayerState::Dead) {
        value(p, "source_platform_grounded", 0.0) != 0.0
    } else {
        grounded_state(p.state) && value(p, "duck_to_hang", 0.0) == 0.0
    }
}
fn airborne(s: PlayerState) -> bool {
    matches!(s, PlayerState::Jumping | PlayerState::Falling)
}
fn remember_platform_traits(p: &mut Player) {
    set(
        p,
        "source_platform_grounded",
        f64::from(grounded_state(p.state) && value(p, "duck_to_hang", 0.0) == 0.0),
    );
    set(
        p,
        "source_platform_climbing",
        f64::from(p.state == PlayerState::Climbing),
    );
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
    collision_rect_precision(w, r, group, CollisionPrecision::BoundingBox)
}

fn precise_rect(w: &World, r: Rect, group: &str) -> bool {
    w.rectangle_family_except(
        Vec2::new(r.x, r.y),
        Vec2::new(r.x + r.w - 1.0, r.y + r.h - 1.0),
        group,
        None,
        CollisionPrecision::Precise,
    )
}

fn precise_line(w: &World, r: Rect, group: &str) -> bool {
    let first = Vec2::new(r.x, r.y);
    let second = Vec2::new(r.x + r.w - 1.0, r.y + r.h - 1.0);
    w.tiles_line_active_precision(first, second, group, CollisionPrecision::Precise)
        || w.entities.iter().any(|e| {
            e.alive
                && e.engine_active
                && !e.held
                && (crate::catalog::is_a(&e.object, group) || (group == "oSolid" && e.solid))
                && e.collision_mask().intersects_axis_line(
                    first,
                    second,
                    CollisionPrecision::Precise,
                )
        })
}

fn first_solid_line(
    w: &World,
    r: Rect,
    order: &[crate::dispatcher::Instance],
) -> Option<crate::dispatcher::Instance> {
    let first = Vec2::new(r.x, r.y);
    let second = Vec2::new(r.x + r.w - 1.0, r.y + r.h - 1.0);
    // A false broad-phase answer can avoid resolving every ordered mask.
    // Keep Entity(0) eligible even though this helper excludes ID zero.
    if !w.solid_line_except(first, second, 0)
        && !order.contains(&crate::dispatcher::Instance::Entity(0))
    {
        return None;
    }
    order.iter().copied().find(|instance| match *instance {
        crate::dispatcher::Instance::Player => false,
        crate::dispatcher::Instance::Terrain(index) => w.tiles.get(index).is_some_and(|tile| {
            tile.engine_active
                && w.tile_collision_mask((index % w.width) as i32, (index / w.width) as i32)
                    .intersects_axis_line(first, second, CollisionPrecision::Precise)
        }),
        crate::dispatcher::Instance::Entity(id) => w.entity(id).is_some_and(|entity| {
            entity.alive
                && entity.engine_active
                && entity.collision_mask().intersects_axis_line(
                    first,
                    second,
                    CollisionPrecision::Precise,
                )
        }),
    })
}

fn collision_rect_precision(
    w: &World,
    r: Rect,
    group: &str,
    precision: CollisionPrecision,
) -> bool {
    // GameMaker has no implicit room-edge collider. In particular, the Ice
    // Caves abyss must let the player leave the room and trigger falling death.
    w.tiles_collide_active_precision(r, group, precision)
        || w.entities.iter().any(|e| {
            e.alive
                && e.engine_active
                && !e.held
                && (crate::catalog::is_a(&e.object, group) || (group == "oSolid" && e.solid))
                && e.mask_intersects(r, precision)
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
    precise_line(w, edge_rect(p, side, distance, false), "oSolid")
}

fn solid_point(w: &World, pos: Vec2) -> bool {
    // A literal collision_point has no area: a point just left of a wall must
    // stay outside, even when a tiny rectangle would overlap the wall.
    w.tiles_point_active(pos, "oSolid")
        || w.entities.iter().any(|e| {
            e.alive
                && e.engine_active
                && !e.held
                && (crate::catalog::is_a(&e.object, "oSolid") || e.solid)
                && e.mask_contains(pos, CollisionPrecision::BoundingBox)
        })
}
fn object_at(w: &World, pos: Vec2, names: &[&str]) -> bool {
    w.tile((pos.x / 16.0).floor() as i32, (pos.y / 16.0).floor() as i32)
        .is_some_and(|t| t.engine_active && names.contains(&t.object.as_str()))
        || w.entities.iter().any(|e| {
            e.alive
                && e.engine_active
                && names.contains(&e.object.as_str())
                && e.sprite_bounds().contains(pos)
        })
}
/// Literal ledge collision_point(..., family, 0, 0): active instances,
/// source family membership, and each instance's cached bounding box.
fn ledge_point(w: &World, point: Vec2, family: &str) -> bool {
    w.tiles.iter().enumerate().any(|(index, tile)| {
        tile.engine_active
            && crate::catalog::is_a(&tile.object, family)
            && w.tile_collision_mask((index % w.width) as i32, (index / w.width) as i32)
                .contains(point, CollisionPrecision::BoundingBox)
    }) || w.entities.iter().any(|entity| {
        entity.alive
            && entity.engine_active
            && crate::catalog::is_a(&entity.object, family)
            && entity.mask_contains(point, CollisionPrecision::BoundingBox)
    })
}

fn arrow_ledge(w: &World, point: Vec2, arrow_order: &[u64]) -> bool {
    let upper = Vec2::new(point.x, point.y - 5.0);
    if !(ledge_point(w, upper, "oArrow")
        || ledge_point(w, Vec2::new(point.x, point.y - 6.0), "oArrow"))
        || ledge_point(w, Vec2::new(point.x, point.y - 9.0), "oArrow")
        || ledge_point(w, Vec2::new(point.x, point.y + 9.0), "oArrow")
    {
        return false;
    }
    // The point queries gate the attempt; nearest searches ALL active Arrows.
    // Strict comparison retains the source parent-array order for equal ties.
    let squared_distance = |entity: &Entity| {
        let dx = entity.pos.x - upper.x;
        let dy = entity.pos.y - upper.y;
        dx * dx + dy * dy
    };
    arrow_order
        .iter()
        .filter_map(|id| w.entity(*id))
        .filter(|entity| {
            entity.alive && entity.engine_active && crate::catalog::is_a(&entity.object, "oArrow")
        })
        .fold(None, |nearest: Option<&Entity>, entity| {
            if nearest.is_none_or(|current| squared_distance(entity) < squared_distance(current)) {
                Some(entity)
            } else {
                nearest
            }
        })
        .is_some_and(|entity| entity.value("stuck") != 0.0)
}

/// isCollisionLadder samples the cached controller bounds, before movement
/// and the later falling-body update. Its precise rectangle allows reversed
/// corners and equal endpoints; scan its rounded inclusive rows, retaining
/// the source raw cached-bbox rejection before selected-mask rasterization.
fn cached_ladder_collision(w: &World, p: &Player) -> bool {
    let bounds = bounds_at(p, p.pos);
    let first = Vec2::new(bounds.x + 8.0, bounds.y + 8.0);
    let last = Vec2::new(bounds.x + bounds.w - 8.0, bounds.y + bounds.h - 8.0);
    ["oLadderTop", "oLadder"].into_iter().any(|family| {
        w.rectangle_family_except(first, last, family, None, CollisionPrecision::Precise)
    })
}

fn ladder_at(w: &World, pos: Vec2) -> Option<Vec2> {
    ladder_match(w, pos, true)
}
/// characterStepEvent's climbing continuation uses collision_point for both
/// ladder families in SourceClassicV2. HistoricalV1 keeps its older tile-first
/// adapter, while the source query excludes deactivated instances.
fn climbing_ladder_point(w: &World, pos: Vec2, source_collision_point: bool) -> bool {
    if source_collision_point {
        ledge_point(w, pos, "oLadder") || ledge_point(w, pos, "oLadderTop")
    } else {
        ladder_at(w, pos).is_some()
    }
}
fn ladder_segment_at(w: &World, pos: Vec2) -> Option<Vec2> {
    ladder_match(w, pos, false)
}
/// Climbing's collision_point returns the first source-family receiver,
/// independently of whether its storage happens to be terrain or an entity.
fn ordered_ladder_segment_at(
    w: &World,
    point: Vec2,
    order: &[crate::dispatcher::Instance],
) -> Option<Vec2> {
    order.iter().find_map(|instance| {
        let (mask, origin) = match *instance {
            crate::dispatcher::Instance::Terrain(index) => {
                let tile = w.tiles.get(index)?;
                if !tile.engine_active || !crate::catalog::is_a(&tile.object, "oLadder") {
                    return None;
                }
                let x = (index % w.width) as i32;
                let y = (index / w.width) as i32;
                (
                    w.tile_collision_mask(x, y),
                    Vec2::new(f64::from(x) * 16.0, f64::from(y) * 16.0),
                )
            }
            crate::dispatcher::Instance::Entity(id) => {
                let entity = w.entity(id)?;
                if !entity.alive
                    || !entity.engine_active
                    || !crate::catalog::is_a(&entity.object, "oLadder")
                {
                    return None;
                }
                (entity.collision_mask(), entity.pos)
            }
            crate::dispatcher::Instance::Player => return None,
        };
        mask.contains(point, CollisionPrecision::BoundingBox)
            .then_some(Vec2::new(origin.x + 8.0, origin.y))
    })
}
/// Literal instance_place: translate the current Player sprite mask without
/// changing its transform or frame, then return the first active source-family
/// instance in registration order. This is separate from ladder point probes.
fn ladder_place(
    w: &World,
    p: &Player,
    pos: Vec2,
    order: &[crate::dispatcher::Instance],
) -> Option<Vec2> {
    let caller = p.collision_mask_at(pos);
    order.iter().find_map(|instance| {
        let (target, origin) = match *instance {
            crate::dispatcher::Instance::Terrain(index) => {
                let tile = w.tiles.get(index)?;
                if !tile.engine_active || !crate::catalog::is_a(&tile.object, "oLadder") {
                    return None;
                }
                let x = (index % w.width) as i32;
                let y = (index / w.width) as i32;
                (
                    w.tile_collision_mask(x, y),
                    Vec2::new(f64::from(x) * 16.0, f64::from(y) * 16.0),
                )
            }
            crate::dispatcher::Instance::Entity(id) => {
                let entity = w.entity(id)?;
                if !entity.alive
                    || !entity.engine_active
                    || !crate::catalog::is_a(&entity.object, "oLadder")
                {
                    return None;
                }
                (entity.collision_mask(), entity.pos)
            }
            crate::dispatcher::Instance::Player => return None,
        };
        caller
            .intersects_mask(&target)
            .then_some(Vec2::new(origin.x + 8.0, origin.y))
    })
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
    // Source's finite -1 precise/notme arguments both convert to false.
    ledge_point(w, pos, "oWater")
}

fn swimming_without_lava(w: &World, p: &Player) -> bool {
    value(p, "swimming", 0.0) > 0.0 && !ledge_point(w, p.pos, "oLava")
}

/// oPlayer1 runs these effects before entering its dead/stunned controller.
/// The swimming flag starts only in oWaterSwim; ordinary movement clears it
/// after leaving water and webs. Bubble emission belongs to the active branch.
fn environmental_prelude(game: &mut Game) {
    let red = value(&game.player, "red_color", 0.0);
    if red > 0.0 {
        if value(&game.player, "red_toggle", 0.0) != 0.0 {
            set(&mut game.player, "red_color", red - 5.0);
        } else if red < 20.0 {
            set(&mut game.player, "red_color", red + 5.0);
        } else {
            set(&mut game.player, "red_toggle", 1.0);
        }
    } else {
        set(&mut game.player, "red_color", 0.0);
    }
    // oPlayer1 Step78-86 resets hidden corpses before environmental and
    // dead/stunned motion. The later bounced branch still adds one pixel.
    let dead = game.player.state == PlayerState::Dead || game.state == RunState::Dead;
    let invisible =
        value(&game.player, "hidden", 0.0) != 0.0 || value(&game.player, "visible", 1.0) == 0.0;
    if dead && invisible {
        game.player.vel = Vec2::default();
        set(&mut game.player, "gravity", 0.0);
        set(&mut game.player, "my_gravity", 0.0);
        set(&mut game.player, "bounced", 1.0);
    }
    let pos = game.player.pos;
    if ledge_point(&game.world, pos, "oWaterSwim") && value(&game.player, "swimming", 0.0) == 0.0 {
        set(&mut game.player, "swimming", 1.0);
        game.world.spawn("oSplash", Vec2::new(pos.x, pos.y - 8.0));
        game.sound("sndSplash", pos);
    }
    let burning = value(&game.player, "burning", 0.0);
    if burning > 0.0 {
        if game.rng.range(5) == 0 {
            let x = pos.x - 8.0 + 4.0 + f64::from(game.rng.range(9));
            let y = pos.y - 8.0 + 4.0 + f64::from(game.rng.range(9));
            game.world.spawn("oBurn", Vec2::new(x, y));
        }
        set(&mut game.player, "burning", burning - 1.0);
    }
}

fn swimming_effects(game: &mut Game) {
    let pos = game.player.pos;
    let bubble_max = value(&game.player, "bubble_timer_max", 20.0);
    if swimming_without_lava(&game.world, &game.player) {
        set(&mut game.player, "fall_timer", 0.0);
        set(&mut game.player, "fall_ticks", 0.0);
        let bubble = value(&game.player, "bubble_timer", 0.0);
        if bubble > 0.0 {
            set(&mut game.player, "bubble_timer", bubble - 1.0);
        } else {
            crate::world_events::spawn_effect(game, "oBubble", Vec2::new(pos.x, pos.y - 4.0));
            set(&mut game.player, "bubble_timer", bubble_max);
        }
    } else {
        set(&mut game.player, "bubble_timer", bubble_max);
    }
}

/// The active Player Step prelude runs even when characterStepEvent is skipped
/// by a stun, death, ledge transition, or exit animation. Its fall injury and
/// synchronous Create events precede the swimming/bubble branch.
fn active_prelude(game: &mut Game) {
    let canonical = platform_state::canonical(game);
    let parachute_exists = game
        .world
        .entities
        .iter()
        .any(|entity| entity.alive && entity.engine_active && entity.object == "oParachute");
    let p = &mut game.player;
    let stationary = matches!(
        p.sprite.as_str(),
        "sStunL" | "sDamselStunL" | "sTunnelStunL"
    );
    if p.stun > 0 && stationary {
        p.set_image_speed(0.4);
        p.stun -= 1;
    }
    if p.stun == 0 && stationary {
        // Source clears its independent stunned flag only on the stationary
        // stun sprite, including a zero-timer throw completed during Animation End.
        p.data.remove("source_stunned");
        platform_state::recovered(p, canonical);
    }
    let mut fall = value(p, "fall_timer", 0.0);
    // This is a derived controller cache, not the source existence predicate.
    set(p, "parachute_open", f64::from(parachute_exists));
    if parachute_exists {
        fall = 0.0;
    }
    let mut deploy_parachute = false;
    let mut remove_parachute = false;
    let mut injury = 0;
    let hidden_state = matches!(p.state, PlayerState::Stunned | PlayerState::Dead);
    let climbing = if canonical {
        match platform_state::read(p, true) {
            Some(state) => state == platform_state::State::Climbing,
            None => hidden_state && value(p, "source_platform_climbing", 0.0) != 0.0,
        }
    } else {
        p.state == PlayerState::Climbing
            || (hidden_state && value(p, "source_platform_climbing", 0.0) != 0.0)
    };
    if p.vel.y > 0.0 && !climbing {
        fall += 1.0;
        if fall > 16.0 {
            set(p, "wall_hurt", 0.0);
            set(p, "wallHurt", 0.0);
        }
        if has(p, &["Parachute", "parachute"])
            && !p.is_stunned()
            && fall > 14.0
            && !solid_point(&game.world, Vec2::new(p.pos.x, p.pos.y + 32.0))
        {
            deploy_parachute = true;
        }
    } else if platform_state::prelude_grounded(p, canonical)
        && fall > 16.0
        && !collision_rect(
            &game.world,
            Rect::new(p.pos.x - 8.0, p.pos.y - 8.0, 17.0, 17.0),
            "oSpringTrap",
        )
    {
        injury = if fall > 48.0 {
            10
        } else if fall > 32.0 {
            2
        } else {
            1
        };
        fall = 0.0;
    } else {
        fall = 0.0;
        remove_parachute = true;
    }
    set(p, "fall_timer", fall);
    set(p, "fall_ticks", fall);
    if deploy_parachute {
        // Step276: synchronous Create precedes both the timer reset and the
        // inventory clear; own-Step hurt later in this same Step sees it.
        game.spawn(
            "oParachute",
            Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 16.0),
        );
        set(&mut game.player, "fall_timer", 0.0);
        set(&mut game.player, "fall_ticks", 0.0);
        game.player.equipment.remove("Parachute");
        game.player.equipment.remove("parachute");
        set(&mut game.player, "parachute_open", 1.0);
    } else if remove_parachute {
        // Step309: retire the canopy at the pre-movement Player origin.
        remove_player_parachutes(game);
    }
    if injury > 0 {
        // The source subtracts life directly: fall injury bypasses the hit
        // invulnerability check and does not invent its hurt sound or flash.
        game.player.lose_health(injury);
        game.events.push(GameEvent::Damage { amount: injury });
        if game.player.health < 1 {
            for _ in 0..3 {
                if value(&game.player, "bloodless", 0.0) == 0.0
                    && game
                        .world
                        .entities
                        .iter()
                        .filter(|e| {
                            e.alive
                                && e.engine_active
                                && crate::catalog::is_a(&e.object, "oDetritus")
                        })
                        .count()
                        < 16
                {
                    game.spawn("oBlood", game.player.pos);
                }
            }
            if crate::statistics::is_real_level(game) {
                game.lifetime_stats
                    .misc_death(crate::statistics::MiscDeath::LongFall);
            }
        }
        set(&mut game.player, "bounced", 1.0);
        game.player.stun += 60;
        game.player.vel.y = -3.0;
        for (offset, velocity) in [(-4.0, -0.4), (4.0, 0.4)] {
            let id = crate::world_events::spawn_effect(
                game,
                "oPoof",
                Vec2::new(game.player.pos.x + offset, game.player.pos.y + 6.0),
            );
            game.world.entity_mut(id).unwrap().vel.x = velocity;
        }
        game.sound("sndThud", game.player.pos);
    }
    swimming_effects(game);
}

fn pit_prelude(game: &mut Game) {
    if game.player.pos.y <= game.world.height as f64 * 16.0 + 16.0
        || game.state == RunState::Dead
        || game.player.state == PlayerState::Dead
    {
        return;
    }
    if crate::statistics::is_real_level(game) {
        game.lifetime_stats
            .misc_death(crate::statistics::MiscDeath::Pit);
    }
    game.player.lose_health(99);
    game.player.vel = Vec2::default();
    set(&mut game.player, "gravity", 0.0);
    set(&mut game.player, "my_gravity", 0.0);
    set(&mut game.player, "bounced", 1.0);
    if let Some(id) = game.player.held.take() {
        game.player.pickup_item_type.clear();
        if let Some(held) = game.world.entity_mut(id) {
            held.data.insert("visible".into(), 1.0);
            held.held = false;
        }
    }
    game.sound("sndThud", game.player.pos);
    game.sound("sndDie", game.player.pos);
}

fn level_feature(game: &Game, names: &[&str], room_name: &str) -> bool {
    (!game.flags.contains("custom_level") && game.world.name == room_name)
        || names.iter().any(|name| game.flags.contains(*name))
}

/// scrInitLevel schedules the feature announcement after ten source ticks.
/// Imported editor rooms are rLoadLevel and omit this announcement alarm.
pub(crate) fn initialize_level_alarms(game: &mut Game) {
    if game.flags.contains("custom_level") {
        return;
    }
    if game.world.dark
        || level_feature(game, &["black_market", "blackMarket"], "Black Market")
        || level_feature(game, &["snake_pit", "snakePit"], "Snake Pit")
        || level_feature(game, &["cemetery", "cemetary"], "Cemetery")
        || level_feature(game, &["lake"], "Lake")
        || level_feature(game, &["yeti_lair", "yetiLair"], "Yeti Lair")
        || level_feature(game, &["alien_ship", "alienCraft"], "Alien Ship")
        || level_feature(game, &["sacrifice_pit", "sacrificePit"], "Sacrifice Pit")
        || level_feature(game, &["city_of_gold", "cityOfGold"], "City of Gold")
    {
        game.player.alarms[0] = 10;
    }
}

fn level_message(game: &mut Game, first: &str, second: &str) {
    game.data.insert("message_timer".into(), 200.0);
    game.events.push(GameEvent::Dialogue(SourceMessage {
        first: first.into(),
        second: second.into(),
        ticks: 200,
    }));
}

/// The player's source Alarm events are dispatched independently of its active
/// controller flag, through the engine's indexed Alarm phase.
pub fn alarm(game: &mut Game, number: u8) {
    if matches!(number, 0 | 1 | 4) {
        if number != 4 && game.flags.contains("tutorial") {
            return;
        }
        let dark = game.world.dark;
        let cemetery = level_feature(game, &["cemetery", "cemetary"], "Cemetery");
        let lake = level_feature(game, &["lake"], "Lake");
        let sacrifice = level_feature(game, &["sacrifice_pit", "sacrificePit"], "Sacrifice Pit");
        let message = if number == 0 && dark {
            game.player.alarms[1] = 210;
            if has(&game.player, &["Crown", "crown", "Hedjet"]) {
                Some(("THE HEDJET SHINES BRIGHTLY.", ""))
            } else {
                Some(("I CAN'T SEE A THING!", "I'D BETTER USE THESE FLARES!"))
            }
        } else if number == 0
            && level_feature(game, &["black_market", "blackMarket"], "Black Market")
        {
            game.player.alarms[1] = 210;
            Some(("WELCOME TO THE BLACK MARKET!", ""))
        } else if number != 4 && level_feature(game, &["snake_pit", "snakePit"], "Snake Pit") {
            Some(("I HEAR SNAKES... I HATE SNAKES!", ""))
        } else if cemetery && (number == 0 || (number == 1 && dark)) {
            if lake {
                game.player.alarms[if number == 0 { 1 } else { 4 }] = 210;
            }
            Some(("THE DEAD ARE RESTLESS!", ""))
        } else if lake {
            Some((
                if number == 4 {
                    "YOU HEAR RUSHING WATER..."
                } else {
                    "I CAN HEAR RUSHING WATER..."
                },
                "",
            ))
        } else if number != 4 && level_feature(game, &["yeti_lair", "yetiLair"], "Yeti Lair") {
            Some(("IT SMELLS LIKE WET FUR IN HERE!", ""))
        } else if number != 4 && level_feature(game, &["alien_ship", "alienCraft"], "Alien Ship") {
            Some(("THERE'S A PSYCHIC PRESENCE HERE!", ""))
        } else if number != 4
            && level_feature(game, &["city_of_gold", "cityOfGold"], "City of Gold")
        {
            if sacrifice {
                game.player.alarms[if number == 0 { 1 } else { 4 }] = 210;
            }
            Some(("IT'S THE LEGENDARY CITY OF GOLD!", ""))
        } else if sacrifice {
            Some(("I CAN HEAR PRAYERS TO KALI!", ""))
        } else {
            None
        };
        if let Some((first, second)) = message {
            level_message(game, first, second);
        }
        return;
    }
    match number {
        2 | 3 => {
            let key = if number == 2 {
                "climb_sound_toggle"
            } else {
                "walk_sound_toggle"
            };
            let toggle = value(&game.player, key, 0.0) != 0.0;
            let sound = match (number, toggle) {
                (2, true) => "sndClimb1",
                (2, false) => "sndClimb2",
                (_, true) => "sndStep1",
                (_, false) => "sndStep2",
            };
            game.sound(sound, game.player.pos);
            set(&mut game.player, key, f64::from(!toggle));
        }
        10 => {
            let x = game.player.pos.x + f64::from(game.rng.range(4)) - f64::from(game.rng.range(4));
            let y = game.player.pos.y + f64::from(game.rng.range(4)) - f64::from(game.rng.range(4));
            let id = crate::world_events::spawn_effect(game, "oFlareSpark", Vec2::new(x, y));
            let vy = f64::from(1 + game.rng.range(3));
            let vx = f64::from(game.rng.range(4)) - f64::from(game.rng.range(4));
            if let Some(spark) = game.world.entity_mut(id) {
                spark.vel = Vec2::new(vx, vy);
            }
            game.sound("sndJetpack", game.player.pos);
        }
        11 => {
            if value(&game.player, "bow_armed", 0.0) != 0.0
                || value(&game.player, "bow_bomb_armed", 0.0) != 0.0
            {
                let toggle = value(&game.player, "bow_arrow_toggle", 0.0) == 0.0;
                set(&mut game.player, "bow_arrow_toggle", f64::from(toggle));
                game.player.alarms[11] = 1;
            }
        }
        _ => {}
    }
}

/// oPlayer1 owns the Blood collision event; dead Kapala corpses can collect and
/// heal without clearing the independent dead flag.
pub fn collision(game: &mut Game, other: crate::dispatcher::Instance) {
    let crate::dispatcher::Instance::Entity(id) = other else {
        return;
    };
    if !has(&game.player, &["Kapala", "kapala"]) {
        return;
    }
    let Some(blood) = game.world.entity(id) else {
        return;
    };
    if !crate::catalog::is_a(&blood.object, "oBlood") || blood.value("collectible") == 0.0 {
        return;
    }
    let pos = blood.pos;
    let count = game.data.entry("bloodLevel".into()).or_default();
    *count += 1.0;
    let heal = *count > 8.0;
    crate::world_events::spawn_effect(game, "oBloodSpark", pos);
    if let Some(blood) = game.world.entity_mut(id) {
        blood.alive = false;
    }
    if heal {
        game.data.insert("bloodLevel".into(), 0.0);
        game.player.health += 1;
        crate::world_events::spawn_effect(
            game,
            "oHeart",
            Vec2::new(game.player.pos.x, game.player.pos.y - 8.0),
        );
        game.sound("sndKiss", game.player.pos);
    }
    let red = value(&game.player, "red_color", 0.0);
    if red < 55.0 {
        set(&mut game.player, "red_color", red + 5.0);
    }
    set(&mut game.player, "red_toggle", 0.0);
}
fn platform(w: &World, p: &Player, pos: Vec2, dy: f64) -> bool {
    if dy <= 0.0 {
        return false;
    }
    debug_assert_eq!(pos, p.pos);
    !precise_rect(w, bounds_at(p, pos), "oPlatform")
        && precise_line(w, edge_rect(p, Side::Bottom, dy, false), "oPlatform")
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

fn move_player(
    w: &mut World,
    p: &mut Player,
    tick: u64,
    down: bool,
    solid_order: &[crate::dispatcher::Instance],
    events: &mut Vec<GameEvent>,
) {
    move_by(w, p, tick, down, p.vel, solid_order, events);
}

fn move_by(
    w: &mut World,
    p: &mut Player,
    tick: u64,
    down: bool,
    movement: Vec2,
    solid_order: &[crate::dispatcher::Instance],
    events: &mut Vec<GameEvent>,
) -> (i32, i32) {
    let dx = displacement(movement.x, tick);
    let dy = displacement(movement.y, tick);
    for _ in 0..dx.unsigned_abs() {
        let next = Vec2::new(p.pos.x + f64::from(dx.signum()), p.pos.y);
        let probe = edge_rect(p, if dx > 0 { Side::Right } else { Side::Left }, 1.0, true);
        if let Some(receiver) = first_solid_line(w, probe, solid_order) {
            let crate::dispatcher::Instance::Entity(id) = receiver else {
                break;
            };
            let block = w.entities.iter().position(|entity| {
                entity.id == id
                    && crate::catalog::object(&entity.object)
                        .is_some_and(|object| object.parent == "oMoveableSolid")
            });
            let pushed = block.is_some_and(|index| {
                let destination = Vec2::new(
                    w.entities[index].pos.x + f64::from(dx.signum()),
                    w.entities[index].pos.y,
                );
                // moveTo tests place_meeting in the block's own context.
                // Translate its source mask without changing the live receiver.
                let mut moved = w.entities[index].clone();
                moved.pos = destination;
                let mask = moved.collision_mask();
                let blocked = solid_order.iter().any(|instance| match *instance {
                    crate::dispatcher::Instance::Player => false,
                    crate::dispatcher::Instance::Terrain(index) => {
                        let tile = &w.tiles[index];
                        tile.engine_active
                            && crate::catalog::is_a(&tile.object, "oSolid")
                            && mask.intersects_mask(&w.tile_collision_mask(
                                (index % w.width) as i32,
                                (index / w.width) as i32,
                            ))
                    }
                    crate::dispatcher::Instance::Entity(id) => {
                        id != moved.id
                            && w.entity(id).is_some_and(|other| {
                                other.engine_active
                                    && crate::catalog::is_a(&other.object, "oSolid")
                                    && mask.intersects_mask(&other.collision_mask())
                            })
                    }
                });
                if blocked {
                    return false;
                }
                w.entities[index].pos = destination;
                events.push(GameEvent::SoundIfIdle {
                    name: "sndPush".into(),
                    pos: destination,
                });
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

fn slope_move(
    w: &mut World,
    p: &mut Player,
    time: u64,
    down: bool,
    col_top: bool,
    solid_order: &[crate::dispatcher::Instance],
    events: &mut Vec<GameEvent>,
) {
    if !grounded_state(p.state) || p.vel.x == 0.0 {
        move_player(w, p, time, down, solid_order, events);
        return;
    }
    let original = p.pos;
    // Classic's inclusive `for(y=y; y>=previous-maxSlope; y-=1)` raises
    // maxSlope+1 pixels. colTop is the earlier cached collision result.
    let rise = if col_top { 0.0 } else { 5.0 };
    p.pos.y -= rise;
    let high = p.pos;
    let (ix, iy) = move_by(
        w,
        p,
        time,
        down,
        Vec2::new(p.vel.x, p.vel.y + rise),
        solid_order,
        events,
    );
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
            solid_order,
            events,
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
    // Source distance_to_object uses integer aggregate sprite envelopes,
    // including the previous Draw transform, independently of movement bounds.
    if crate::world_events::mask_distance(
        p.collision_mask().point_bounds(),
        ball.collision_mask().point_bounds(),
    ) < 24.0
    {
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
    // Direct controller callers retain held-transition inference. Game::step
    // already normalized this actual frame before all dispatcher/state guards.
    let frame = game.normalize_direction_releases(input);
    let frame = game.normalize_action_edges(&frame);
    let input = &frame;
    let engine_time = game.engine_time();
    // scrStealItem destroys the Cape when a Jetpack replaces it. This port
    // stores Cape openness on the player, so discard that derived state once
    // its owning equipment is gone, before it can reset fall time or add drag.
    // Pickups run after this controller: their pickup tick retains source drag.
    if !has(&game.player, &["Cape", "cape"]) {
        set(&mut game.player, "cape_open", 0.0);
    } else if value(&game.player, "cape_open", 0.0) > 0.0 {
        set(&mut game.player, "fall_timer", 0.0);
        set(&mut game.player, "fall_ticks", 0.0);
    }
    environmental_prelude(game);
    // Source burning ticks before the separate lava branch resets it to 100.
    // Keeping lava outside this prelude would shorten its lifetime on entry.
    crate::traps::player_lava(game);
    // Source refuels before the active/controller guard, and only an owned
    // Jetpack. A pickup later in this Step cannot retroactively refill it.
    let grounded = platform_state::prelude_grounded(&game.player, platform_state::canonical(game));
    let p = &mut game.player;
    if grounded && has(p, &["Jetpack", "jetpack"]) {
        set(p, "jetpack_fuel", 50.0);
    }
    pit_prelude(game);
    if game.state == RunState::Dead {
        enter_dead(game);
    }
    let damsel = game.flags.contains("character_damsel");
    let tunnelman = game.flags.contains("character_tunnelman");
    let p = &mut game.player;
    set(p, "character_damsel", f64::from(damsel));
    set(p, "character_tunnelman", f64::from(tunnelman));
    let active = value(p, "active", 1.0) != 0.0;
    if active {
        active_prelude(game);
    }
    let solid_order = crate::dispatcher::descendant_instances(game, "oSolid");
    let arrow_order: Vec<u64> = crate::dispatcher::descendant_instances(game, "oArrow")
        .into_iter()
        .filter_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => Some(id),
            _ => None,
        })
        .collect();
    // HistoricalV1 retains its earlier tile-first climbing point adapter.
    let source_ladder_point =
        game.generation_revision == crate::GenerationRevision::SourceClassicV2;
    // Attachment uses instance_place; SourceClassicV2 climbing uses collision_point.
    // Both source receiver queries retain the family's registration order.
    let ladder_order = if (source_ladder_point && game.player.state == PlayerState::Climbing)
        || input.up
        || input.down
    {
        crate::dispatcher::descendant_instances(game, "oLadder")
    } else {
        Vec::new()
    };
    let web_order: Vec<u64> = crate::dispatcher::descendant_instances(game, "oWeb")
        .into_iter()
        .filter_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => Some(id),
            _ => None,
        })
        .collect();
    let p = &mut game.player;
    if p.state == PlayerState::Dead || p.is_stunned() {
        // Source keeps its platform state separate from dead/stunned. The
        // skipped active controller forces STANDING only after the fall
        // prelude, so retain the preceding tick's trait for first-hit onset.
        if active {
            if source_ladder_point {
                let state = platform_state::read(p, true);
                if !matches!(
                    state,
                    Some(platform_state::State::Ducking | platform_state::State::DuckToHang)
                ) {
                    platform_state::assign(p, platform_state::State::Standing, true);
                }
            } else {
                let standing = value(p, "duck_to_hang", 0.0) == 0.0;
                set(p, "source_platform_grounded", f64::from(standing));
                set(p, "source_platform_climbing", 0.0);
            }
        }
        crate::items::release_held_before_stunned_motion(game);
        let retain_ducking = source_ladder_point && source_ducking(game);
        stunned_tick(
            &mut game.world,
            &mut game.player,
            engine_time,
            source_ladder_point,
            retain_ducking,
            &solid_order,
            &mut game.events,
        );
        return;
    }
    // oPlayer1 Step gates characterStepEvent on active, independently of
    // environmental damage and the dead/stunned controller above.
    if !active {
        look_step(game);
        return;
    }
    // Ankh revival can reactivate a player during the existing exit animation.
    // oPlayer1 Step 331-337 still excludes that sprite from characterStepEvent;
    // its Animation End retains ownership of the pending room transition.
    if p.is_exiting() {
        if p.state != PlayerState::Ducking && value(p, "duck_to_hang", 0.0) == 0.0 {
            p.state = PlayerState::Standing;
        }
        let ground = grounded_state(p.state) && value(p, "duck_to_hang", 0.0) == 0.0;
        set(p, "source_platform_grounded", f64::from(ground));
        set(p, "source_platform_climbing", 0.0);
        look_step(game);
        return;
    }
    // Classic keeps its platform state STANDING while the separate `stunned`
    // flag owns movement (oPlayer1 Step 331-337). Our explicit Stunned state
    // must leave that controller when its timer expires; otherwise it is
    // neither ON_GROUND nor IN_AIR and can never jump or select normal art.
    if p.state == PlayerState::Stunned {
        p.state = PlayerState::Standing;
    }
    if value(p, "duck_to_hang", 0.0) > 0.0 {
        p.vel = Vec2::default();
        set(p, "source_platform_grounded", 0.0);
        set(p, "source_platform_climbing", 0.0);
        look_step(game);
        return;
    }
    // characterStepEvent alone samples these action keys. Skipped controllers
    // retain them; a successful container opening separately clears Attack.
    if input.attack_pressed {
        set(p, "source_k_attack_pressed", 1.0);
    } else {
        p.data.remove("source_k_attack_pressed");
    }
    set(p, "source_k_down", f64::from(input.down));
    if input.up {
        set(p, "source_k_up", 1.0);
    } else {
        p.data.remove("source_k_up");
    }
    set(p, "bounced", 0.0);
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
    let col_solid_bot = edge(w, p, Side::Bottom, 1.0);
    let col_bot = col_solid_bot || platform(w, p, p.pos, 1.0);
    let col_platform_bot = precise_line(w, edge_rect(p, Side::Bottom, 1.0, false), "oPlatform");
    let col_platform = precise_rect(w, bounds_at(p, p.pos), "oPlatform");
    let col_left = edge(w, p, Side::Left, 1.0);
    let col_right = edge(w, p, Side::Right, 1.0);
    let col_top = edge(w, p, Side::Top, 1.0);
    let col_ladder = cached_ladder_collision(w, p);
    set(p, "colTop", f64::from(col_top));
    let col_moveable_left = precise_line(w, edge_rect(p, Side::Left, 1.0, false), "oMoveableSolid");
    let col_moveable_right =
        precise_line(w, edge_rect(p, Side::Right, 1.0, false), "oMoveableSolid");
    let water_jump = precise_line(w, edge_rect(p, Side::Top, 1.0, false), "oWater");
    let col_ice_bot = ledge_point(w, Vec2::new(p.pos.x, p.pos.y + 8.0), "oIce");
    let mut fall = value(p, "fall_timer", 0.0);
    let mut gravity = value(p, "gravity", 1.0);
    let mut intensity = value(p, "gravity_intensity", 1.0);
    let mut ladder_timer = value(p, "ladder_timer", 0.0);
    let mut hang_count = value(p, "hang_count", 0.0);
    let mut cape = value(p, "cape_open", 0.0) > 0.0;
    let parachute = value(p, "parachute_open", 0.0) > 0.0;
    let mut fuel = value(p, "jetpack_fuel", 0.0);
    let mut jumped = value(p, "jumped", 0.0) > 0.0;
    let mut ax: f64 = 0.0;
    let mut ay: f64 = 0.0;
    let mut limit_x = value(p, "velocity_limit_x", 16.0);
    let mut limit_y_acc = value(p, "yAccLimit", 6.0);

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
        if input.left_released && p.vel.x.abs() < 0.1 {
            ax -= 0.5;
        }
        if input.right_released && p.vel.x.abs() < 0.1 {
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
        let ladder = if source_ladder_point {
            ordered_ladder_segment_at(w, p.pos, &ladder_order)
        } else {
            ladder_segment_at(w, p.pos)
        };
        if let Some(ladder) = ladder {
            p.pos.x = ladder.x;
        }
        if input.left {
            p.facing = -1;
        } else if input.right {
            p.facing = 1;
        }
        if input.up {
            let above = Vec2::new(p.pos.x, p.pos.y - 8.0);
            if climbing_ladder_point(w, above, source_ladder_point) {
                ay -= 0.6;
                if p.alarms[2] < 1 {
                    p.alarms[2] = 8;
                }
            }
        } else if input.down {
            let below = Vec2::new(p.pos.x, p.pos.y + 8.0);
            if climbing_ladder_point(w, below, source_ladder_point) {
                ay += 0.6;
                if p.alarms[2] < 1 {
                    p.alarms[2] = 8;
                }
            } else {
                p.state = PlayerState::Falling;
            }
            if col_solid_bot {
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
    // characterStepEvent256: overlapping a Platform excludes this stop even
    // when a Solid also supports the body. Keep the pre-controller snapshot.
    if col_bot && !col_platform {
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
    // characterStepEvent295 samples the current origin after Climbing snaps.
    // colWaterTop remains the earlier source collision snapshot.
    let in_web = ledge_point(w, p.pos, "oWeb");
    set(p, "in_web", f64::from(in_web));
    if jump_pressed && (in_web || water_jump) {
        ay -= 4.0;
        p.vel.y -= 3.0;
        ax += p.vel.x / 2.0;
        p.state = PlayerState::Jumping;
        p.jump_time = 0;
        gravity = 1.0;
        set(p, "jump_released", 0.0);
        if in_web {
            // The origin point only gates escape. instance_place separately
            // selects the first registered Web overlapping the current masks.
            let player_mask = p.collision_mask();
            let selected = web_order.iter().copied().find(|id| {
                w.entity(*id).is_some_and(|web| {
                    web.alive
                        && web.engine_active
                        && crate::catalog::is_a(&web.object, "oWeb")
                        && player_mask.intersects_mask(&web.collision_mask())
                })
            });
            if let Some(web) = selected.and_then(|id| w.entity_mut(id)) {
                let life = web.data.entry("life".into()).or_insert(12.0);
                *life -= 1.0;
                // oWeb's Step owns alpha, natural decay, and destruction at
                // life <= 1. Jumping consumes the same lifetime counter.
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
        if p.alarms[10] < 1 {
            p.alarms[10] = 3;
        }
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
        } else {
            limit_y_acc = 6.0;
        }
        p.state = PlayerState::Falling;
        p.jump_time = 0;
        p.coyote_ticks = 0;
        p.buffered_jump_ticks = 0;
        set(p, "push_timer", 0.0);
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

    if input.up && grounded_state(p.state) && !col_ladder && p.vel.x == 0.0 && ax == 0.0 {
        p.state = PlayerState::LookingUp;
    }
    if !input.up && p.state == PlayerState::LookingUp {
        p.state = PlayerState::Standing;
    }

    if !col_top && hang_count == 0.0 && p.pos.y > 16.0 && !grounded_state(p.state) {
        let side_point =
            |direction: f64, offset: f64| Vec2::new(p.pos.x + direction * 9.0, p.pos.y + offset);
        let touches = |direction: f64, family: &str| {
            ledge_point(w, side_point(direction, -5.0), family)
                || ledge_point(w, side_point(direction, -6.0), family)
        };
        let ordinary = |direction: f64| {
            touches(direction, "oSolid")
                && !ledge_point(w, side_point(direction, -9.0), "oSolid")
                && !ledge_point(w, Vec2::new(p.pos.x, p.pos.y + 9.0), "oSolid")
        };
        // Preserve the source's complete predicate order, including its outer
        // Gloves branch. A failed right candidate must still try the left.
        let grab = if has(p, &["Climbing Gloves", "Gloves", "gloves"]) && p.vel.y > 0.0 {
            (input.right && col_right && touches(1.0, "oSolid"))
                || (input.left && col_left && touches(-1.0, "oSolid"))
        } else {
            (input.right && col_right && touches(1.0, "oTree"))
                || (input.left && col_left && touches(-1.0, "oTree"))
                || (input.right && col_right && ordinary(1.0))
                || (input.left && col_left && ordinary(-1.0))
        };
        if grab {
            p.state = PlayerState::Hanging;
            p.pos.x = gm_round(p.pos.x);
            p.pos.y = gm_round(p.pos.y / 8.0) * 8.0;
            p.previous_pos = p.pos;
            p.vel.y = 0.0;
            ay = 0.0;
            gravity = 0.0;
        }
        if p.state == PlayerState::Falling && arrow_ledge(w, p.pos, &arrow_order) {
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
        if col_solid_bot {
            p.state = PlayerState::Ducking;
        } else if col_platform_bot {
            // characterStepEvent550 clears the previous fall before either
            // attaching to the ladder below or beginning a platform drop.
            fall = 0.0;
            if let Some(ladder) =
                ladder_place(w, p, Vec2::new(p.pos.x, p.pos.y + 16.0), &ladder_order)
            {
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
        // characterStepEvent600: incoming yVel, before acceleration/moveTo.
        if p.vel.y > 0.0 && airborne(p.state) {
            -6.0
        } else {
            -8.0
        },
    );
    // characterStepEvent608 snapshots the origin point before attachment.
    let col_point_ladder = if source_ladder_point {
        ledge_point(w, p.pos, "oLadder") || ledge_point(w, p.pos, "oLadderTop")
    } else {
        ladder_at(w, p.pos).is_some()
    };
    let airborne_ladder_above = airborne(p.state)
        && if source_ladder_point {
            ledge_point(w, Vec2::new(p.pos.x, p.pos.y - 8.0), "oLadder")
        } else {
            ladder_segment_at(w, Vec2::new(p.pos.x, p.pos.y - 8.0)).is_some()
        };
    if ladder_timer == 0.0
        && ((input.up && (col_point_ladder || airborne_ladder_above))
            || (input.down
                // characterStepEvent608/612 requires the CURRENT origin point
                // on a Ladder or LadderTop, independently of the mask below.
                && (ledge_point(w, p.pos, "oLadder") || ledge_point(w, p.pos, "oLadderTop"))
                && grounded_state(p.state)
                && p.vel.x == 0.0
                && object_at(
                    w,
                    Vec2::new(p.pos.x, p.pos.y + 9.0),
                    &["oLadderTop"],
                )))
    {
        if let Some(ladder) = ladder_place(w, p, Vec2::new(p.pos.x, p.pos.y - 8.0), &ladder_order) {
            if (p.pos.x - ladder.x).abs() < 4.0 {
                p.pos.x = ladder.x;
                // The source repeats collision_point after snapping X.
                let on_ladder = if source_ladder_point {
                    ledge_point(w, p.pos, "oLadder") || ledge_point(w, p.pos, "oLadderTop")
                } else {
                    ladder_at(w, p.pos).is_some()
                };
                if !on_ladder {
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
            // Classic's default downToRun applies only outside the strict
            // (-2, 2) slow-crawl range. Left wins when both keys are held.
            if p.vel.x > -2.0 && p.vel.x < 2.0 {
                xf = 0.2;
                limit_x = 3.0;
                p.set_image_speed(0.8);
            } else if input.left && game.down_to_run {
                p.vel.x -= 0.1;
                limit_x = 6.0;
                xf = 0.98;
            } else if input.right && game.down_to_run {
                p.vel.x += 0.1;
                limit_x = 6.0;
                xf = 0.98;
            } else {
                p.vel.x *= 0.8;
                if p.vel.x < 0.5 {
                    p.vel.x = 0.0;
                }
                xf = 0.2;
                limit_x = 3.0;
                p.set_image_speed(0.8);
            }
        }
        // characterStepEvent735–770 queries AFTER ladder snapping/attachment.
        // CLIMBING retains swimming; only the dry non-climbing branch clears it.
        if ledge_point(w, p.pos, "oWeb") {
            xf = 0.2;
            yf = 0.2;
            fall = 0.0;
        } else if water(w, p.pos) {
            cape = false;
            yf = if p.state == PlayerState::Falling && p.vel.y > 0.0 {
                0.5
            } else if !water(w, Vec2::new(p.pos.x, p.pos.y - 9.0)) {
                1.0
            } else {
                0.9
            };
        } else {
            set(p, "swimming", 0.0);
        }
    }
    if col_ice_bot
        && p.state != PlayerState::Ducking
        && !has(p, &["Spike Shoes", "SpikeShoes", "spike_shoes"])
    {
        xf = 0.98;
        yf = 1.0;
    }
    if grounded_state(p.state) {
        if p.state == PlayerState::Running
            && ((input.left && col_left) || (input.right && col_right))
        {
            let push = value(p, "push_timer", 0.0);
            set(p, "push_timer", push + 1.0);
        } else {
            set(p, "push_timer", 0.0);
        }
    }
    let push = value(p, "push_timer", 0.0);
    set(p, "push_timer", push.min(100.0));
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
        ax = 0.0;
        ay = 0.0;
        gravity = 0.0;
        let sprite = character_sprite(p, "sDuckToHangL");
        p.set_sprite(sprite);
        p.animation = 0.0;
        p.set_image_speed(0.8);
        set(p, "source_platform_grounded", 0.0);
        set(p, "source_platform_climbing", 0.0);
        let direction = f64::from(p.facing);
        if let Some(id) = p.held {
            if let Some(held) = w.entity_mut(id) {
                // The ledge script adjusts the existing held origin before
                // scrDropItem; Crystal Skull inherits the Gold Idol type.
                if crate::catalog::is_a(&held.object, "oGoldIdol") {
                    held.pos.y -= 8.0;
                }
            }
            crate::items::drop_held_with_velocity(game, Vec2::new(direction, -4.0));
        }
        // `with oMonkey` affects every active GRAB instance, including actors
        // outside their own Step viewport guard. It consumes no random draws.
        for monkey in &mut game.world.entities {
            if monkey.alive
                && monkey.engine_active
                && crate::catalog::is_a(&monkey.object, "oMonkey")
                && monkey.state == 7
            {
                monkey.vel = Vec2::new(direction, -4.0);
                monkey.state = 1;
                monkey.data.insert("vine_counter".into(), 20.0);
                monkey.data.insert("grab_counter".into(), 60.0);
            }
        }
        // Entry still completes characterStepEvent's motion and state tail.
        // Subsequent DUCKTOHANG Steps are excluded by oPlayer1's controller gate.
    }
    let p = &mut game.player;
    let w = &mut game.world;
    if cape || parachute {
        yf = 0.5;
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
    slope_move(
        w,
        p,
        engine_time,
        input.down,
        col_top,
        &solid_order,
        &mut game.events,
    );
    p.grounded = below(w, p)
        && p.vel.y >= 0.0
        && p.state != PlayerState::Climbing
        && value(p, "duck_to_hang", 0.0) == 0.0;
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
    // yAccLimit is an instance field: a Jordans jump retains twelve through
    // climbing and water escape. Keep the unchanged default implicit in saves.
    if limit_y_acc != 6.0 || p.data.contains_key("yAccLimit") {
        set(p, "yAccLimit", limit_y_acc);
    }
    remember_platform_traits(p);
    animate(p, input, w);
    if let Some(name) = sound {
        game.sound(name, game.player.pos);
    }
    look_step(game);
}

/// oPlayer1 Step399-412: the live branch follows its physical controller,
/// including skipped inactive/exit controllers with their cached keys. The
/// dead/stunned movement branch never reaches this helper in the same Step.
fn look_step(game: &mut Game) {
    if !game.is_level() {
        return;
    }
    let grounded = source_platform_grounded(game);
    let hanging = source_hanging(game);
    let p = &mut game.player;
    let eligible = (grounded || hanging)
        && value(p, "previous_left", 0.0) == 0.0
        && value(p, "previous_right", 0.0) == 0.0;
    let direction = if eligible && value(p, "source_k_down", 0.0) != 0.0 {
        1.0
    } else if eligible && value(p, "source_k_up", 0.0) != 0.0 {
        -1.0
    } else {
        p.data.remove("view_count");
        return;
    };
    let count = value(p, "view_count", 0.0);
    if count <= 30.0 {
        set(p, "view_count", count + 1.0);
    } else {
        game.camera.y += direction * 4.0;
    }
}

/// Animation End queries oLadder first, then oLadderTop. Preserve each
/// family's dispatcher order and its complete active instance point mask.
fn duck_to_hang_ladder(game: &Game, point: Vec2) -> Option<Vec2> {
    for family in ["oLadder", "oLadderTop"] {
        let found = crate::dispatcher::descendant_instances(game, family)
            .into_iter()
            .find_map(|instance| {
                let (mask, origin) = match instance {
                    crate::dispatcher::Instance::Terrain(index) => {
                        let tile = game.world.tiles.get(index)?;
                        if !tile.engine_active {
                            return None;
                        }
                        let x = (index % game.world.width) as i32;
                        let y = (index / game.world.width) as i32;
                        (
                            game.world.tile_collision_mask(x, y),
                            Vec2::new(f64::from(x) * 16.0, f64::from(y) * 16.0),
                        )
                    }
                    crate::dispatcher::Instance::Entity(id) => {
                        let entity = game.world.entity(id)?;
                        if !entity.alive || !entity.engine_active {
                            return None;
                        }
                        (entity.collision_mask(), entity.pos)
                    }
                    crate::dispatcher::Instance::Player => return None,
                };
                mask.contains(point, CollisionPrecision::BoundingBox)
                    .then_some(Vec2::new(origin.x + 8.0, origin.y))
            });
        if found.is_some() {
            return found;
        }
    }
    None
}

fn duck_to_hang_end(game: &mut Game) {
    // Source move_snap(1, 8) snaps X before the ladder query and ledge offset.
    game.player.pos.x = gm_round(game.player.pos.x);
    let y = gm_round((game.player.pos.y + 16.0) / 8.0) * 8.0;
    let point = Vec2::new(game.player.pos.x + f64::from(game.player.facing) * 8.0, y);
    let ladder = duck_to_hang_ladder(game, point);
    let canonical = platform_state::canonical(game);
    let p = &mut game.player;
    if canonical {
        set(p, "duck_to_hang", 0.0);
    }
    // Animation End writes the platform state without clearing the independent
    // stunned flag. Preserve a zero-timer automatic throw before replacing its enum.
    let stunned = p.is_stunned();
    p.vel = Vec2::default();
    set(p, "xAcc", 0.0);
    set(p, "yAcc", 0.0);
    set(p, "source_stunned", f64::from(stunned));
    p.pos.y = y;
    if let Some(ladder) = ladder {
        p.pos.x = ladder.x;
        platform_state::assign(p, platform_state::State::Climbing, canonical);
    } else {
        p.pos.x += if p.facing < 0 { -5.0 } else { 6.0 };
        p.facing = -p.facing;
        platform_state::assign(p, platform_state::State::Hanging, canonical);
    }
    p.previous_pos = p.pos;
    if canonical && matches!(p.state, PlayerState::Stunned | PlayerState::Dead) {
        platform_state::capture(p, true);
    } else {
        remember_platform_traits(p);
    }
    // The source callback changes movement state, leaving sprite/index/speed
    // untouched until characterSprite runs during the following Step.
    set(p, "duck_to_hang", 0.0);
    set(p, "gravity", 0.0);
    set(p, "fall_timer", 0.0);
    set(p, "fall_ticks", 0.0);
}

fn stunned_tick(
    w: &mut World,
    p: &mut Player,
    time: u64,
    canonical: bool,
    retain_ducking: bool,
    solid_order: &[crate::dispatcher::Instance],
    events: &mut Vec<GameEvent>,
) {
    let dead = p.state == PlayerState::Dead;
    let bounced = value(p, "bounced", 0.0) > 0.0;
    p.vel.y += if bounced { 1.0 } else { 0.6 };
    if p.vel.y < 0.0 && edge(w, p, Side::Top, 1.0) {
        p.vel.y *= -0.8;
    }
    if edge(w, p, Side::Left, 1.0) || edge(w, p, Side::Right, 1.0) {
        p.vel.x *= -0.5;
    }
    // Source Step376 uses the pure platform-bottom line here. The separate
    // downward moveTo query still rejects platforms overlapping the body.
    if edge(w, p, Side::Bottom, 1.0)
        || precise_line(w, edge_rect(p, Side::Bottom, 1.0, false), "oPlatform")
    {
        p.vel.y = if p.vel.y > 1.0 { -p.vel.y * 0.5 } else { 0.0 };
        // Source Step384 tests the incoming speed before floor friction.
        if p.vel.x.abs() < 0.1 {
            p.vel.x = 0.0;
        } else {
            p.vel.x *= if object_at(w, Vec2::new(p.pos.x, p.pos.y + 16.0), &["oIce"]) {
                0.8
            } else {
                0.3
            };
        }
        set(p, "bounced", 1.0);
    }
    // xVelLimit is a persistent source variable, also read after recovery.
    set(p, "velocity_limit_x", 10.0);
    p.vel.x = p.vel.x.clamp(-10.0, 10.0);
    p.vel.y = p.vel.y.clamp(-10.0, 10.0);
    let down = value(p, "source_k_down", 0.0) != 0.0;
    move_player(w, p, time, down, solid_order, events);
    p.grounded = below(w, p);
    // Timer-only Arrow/Bullet/Rock/explosion hits do not enter the producer
    // collapse helper. Capture every physical-tail collapse as well.
    platform_state::capture(p, canonical);
    p.state = if dead {
        PlayerState::Dead
    } else if p.is_stunned() {
        // oPlayer1 Step338 preserves DUCKING while the independent stunned
        // flag owns motion. Its eventual Down release must still clear xAcc.
        if retain_ducking {
            PlayerState::Ducking
        } else {
            PlayerState::Stunned
        }
    } else {
        PlayerState::Falling
    };
}

pub(crate) fn script_player_blood(game: &mut Game, count: u32) {
    if value(&game.player, "bloodless", 0.0) != 0.0 {
        return;
    }
    for _ in 0..count {
        if game
            .world
            .entities
            .iter()
            .filter(|entity| {
                entity.alive
                    && entity.engine_active
                    && crate::catalog::is_a(&entity.object, "oDetritus")
            })
            .count()
            < 16
        {
            game.spawn("oBlood", game.player.pos);
        }
    }
}

fn player_origin_on_spikes(world: &World, pos: Vec2) -> bool {
    if !pos.x.is_finite() || !pos.y.is_finite() {
        return false;
    }
    if world.width > 0 && world.height > 0 {
        let envelope =
            crate::catalog::group_bounds("oSpikes").unwrap_or(Rect::new(0.0, 0.0, 16.0, 16.0));
        let x0 = (((pos.x - envelope.x - envelope.w) / TILE_SIZE).floor() as i32).max(0);
        let y0 = (((pos.y - envelope.y - envelope.h) / TILE_SIZE).floor() as i32).max(0);
        let x1 = (((pos.x - envelope.x) / TILE_SIZE).floor() as i32).min(world.width as i32 - 1);
        let y1 = (((pos.y - envelope.y) / TILE_SIZE).floor() as i32).min(world.height as i32 - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if world.tile(x, y).is_some_and(|tile| {
                    tile.engine_active && crate::catalog::is_a(&tile.object, "oSpikes")
                }) && world
                    .tile_collision_mask(x, y)
                    .contains(pos, CollisionPrecision::BoundingBox)
                {
                    return true;
                }
            }
        }
    }
    world.entities.iter().any(|entity| {
        entity.alive
            && entity.engine_active
            && crate::catalog::is_a(&entity.object, "oSpikes")
            && entity.mask_contains(pos, CollisionPrecision::BoundingBox)
    })
}

fn wall_injury(game: &mut Game) {
    game.player.lose_health(1);
    let remaining = value(
        &game.player,
        "wall_hurt",
        value(&game.player, "wallHurt", 0.0),
    ) - 1.0;
    set(&mut game.player, "wall_hurt", remaining);
    set(&mut game.player, "wallHurt", remaining);
    game.events.push(GameEvent::Damage { amount: 1 });
}

/// Both source canopy cleanup sites create one used canopy before retiring all
/// active parachutes. The caller supplies the exact phase and Player origin.
fn remove_player_parachutes(game: &mut Game) {
    if game
        .world
        .entities
        .iter()
        .any(|entity| entity.alive && entity.engine_active && entity.object == "oParachute")
    {
        game.spawn(
            "oParaUsed",
            Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 24.0),
        );
        for entity in &mut game.world.entities {
            if entity.alive && entity.engine_active && entity.object == "oParachute" {
                entity.alive = false;
            }
        }
    }
    set(&mut game.player, "parachute_open", 0.0);
}

/// oPlayer1 Step1808-1913, after own-Step hurt and held release, before DIED.
/// Automatic Collision has not run, so its newly assigned stun belongs to the
/// following Player Step. The prior dead flag is independent of current HP.
pub(crate) fn late_hurt_step(game: &mut Game) {
    let dead = game.player.state == PlayerState::Dead || game.state == RunState::Dead;
    if !dead && !game.player.is_stunned() {
        return;
    }
    remove_player_parachutes(game);
    crate::items::cancel_player_attack_after_hurt(game);
    let p = &mut game.player;
    let sprite = character_sprite(
        p,
        if p.vel.x == 0.0 {
            if dead { "sDieL" } else { "sStunL" }
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
    );
    p.set_sprite(sprite);
    // Canonical normal Step enters with other==self; these source other.x/y
    // expressions therefore use the current Player origin, after all withs.
    if dead
        && p.vel.y != 0.0
        && player_origin_on_spikes(&game.world, p.pos)
        && game.rng.range(8) == 0
    {
        script_player_blood(game, 1);
    }
    if (edge(&game.world, &game.player, Side::Right, 1.0)
        || edge(&game.world, &game.player, Side::Left, 1.0)
        || edge(&game.world, &game.player, Side::Bottom, 1.0))
        && value(
            &game.player,
            "wall_hurt",
            value(&game.player, "wallHurt", 0.0),
        ) > 0.0
    {
        // Direct births deliberately bypass scrCreateBlood's bloodless/16 gate.
        for _ in 0..3 {
            game.spawn("oBlood", game.player.pos);
        }
        wall_injury(game);
        game.sound("sndHurt", game.player.pos);
    }
    // This independent test follows sprite selection and can injure again.
    if edge(&game.world, &game.player, Side::Bottom, 1.0)
        && value(&game.player, "bounced", 0.0) == 0.0
    {
        set(&mut game.player, "bounced", 1.0);
        script_player_blood(game, 3);
        if value(
            &game.player,
            "wall_hurt",
            value(&game.player, "wallHurt", 0.0),
        ) > 0.0
        {
            wall_injury(game);
        }
    }
}

/// Dispatch oPlayer1's Animation End after the engine has advanced and wrapped
/// image_index. The controller never advances animation during Step.
pub fn animation_end(game: &mut Game) {
    match game.player.sprite.as_str() {
        "sAttackLeft" | "sDamselAttackL" | "sTunnelAttackL" => {
            game.player.attack_timer = 0;
            game.player.data.remove("whip_elapsed");
            if let Some(held) = game.player.held.and_then(|id| game.world.entity_mut(id)) {
                held.data.insert("visible".into(), 1.0);
                held.data.insert("hidden".into(), 0.0);
            }
            crate::characters::animation_end(game);
        }
        "sDuckToHangL" | "sDamselDtHL" | "sTunnelDtHL" => {
            duck_to_hang_end(game);
        }
        "sPExit" | "sDamselExit" | "sTunnelExit" => {
            crate::ending::player_animation_end(game);
        }
        _ => {}
    }
}

/// characterDrawEvent writes image_xscale in the authoritative Draw phase,
/// after automatic Collision. Physical presentation frames only read it.
pub(crate) fn draw_event(game: &mut Game) {
    let scale = if game.player.facing > 0 { -1.0 } else { 1.0 };
    set(&mut game.player, "image_xscale", scale);
}

/// oPlayer1's separate End Step event runs after automatic Collision. It
/// restores held-item drawing depth and repairs an interrupted ledge animation.
pub(crate) fn end_step(game: &mut Game) {
    let canonical = platform_state::canonical(game);
    let p = &game.player;
    let climbing = if canonical {
        source_climbing(game)
    } else {
        p.state == PlayerState::Climbing
            || (matches!(p.state, PlayerState::Stunned | PlayerState::Dead)
                && value(p, "source_platform_climbing", 0.0) != 0.0)
    };
    let depth = if climbing && has(p, &["Cape", "cape", "Jetpack", "jetpack", "Jet Pack"]) {
        51.0
    } else {
        0.0
    };
    if let Some(held) = p.held.and_then(|id| game.world.entity_mut(id)) {
        held.data.insert("depth".into(), depth);
    }
    let p = &mut game.player;
    if value(p, "duck_to_hang", 0.0) > 0.0
        && !matches!(
            p.sprite.as_str(),
            "sDuckToHangL" | "sDamselDtHL" | "sTunnelDtHL"
        )
    {
        set(p, "duck_to_hang", 0.0);
        // Classic's state=STANDING leaves its independent dead/stunned flags
        // intact. Keep the port's enum representation of those flags as well,
        // including enemy throws whose stun timer is zero.
        if canonical {
            platform_state::assign(p, platform_state::State::Standing, true);
        } else if !matches!(p.state, PlayerState::Stunned | PlayerState::Dead) {
            p.state = PlayerState::Standing;
        }
        set(p, "source_platform_grounded", 1.0);
        set(p, "source_platform_climbing", 0.0);
    }
}

/// Final oPlayer1 Step bookkeeping runs after its hurt/revival actions and
/// before the engine's automatic Collision phase. A fresh late collision keeps
/// its complete blink/invincibility duration until the following Player Step.
pub fn finish_step(game: &mut Game) {
    let p = &mut game.player;
    if p.state != PlayerState::Dead && game.state != RunState::Dead {
        p.invincible = p.invincible.saturating_sub(1);
    }
    let blink = value(p, "blink", 0.0);
    if blink > 0.0 {
        let toggle = value(p, "blink_toggle", -1.0);
        set(p, "blink_toggle", -toggle);
        set(p, "blink", blink - 1.0);
    } else {
        set(p, "blink_toggle", -1.0);
    }
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
    if value(p, "duck_to_hang", 0.0) > 0.0 {
        p.set_sprite(character_sprite(p, "sDuckToHangL"));
        p.animation = 0.0;
        p.set_image_speed(0.8);
        remember_controller_history(p);
        return;
    }
    if p.attack_timer > 0 {
        // characterStepEvent restores this on every whipping Step. The attack
        // start later in that Step overrides it to 0.6 for its first frame.
        p.set_image_speed(1.0);
        // characterSprite skips whipping, but characterStepEvent1008-1009
        // still records the platform state before the following controller.
        remember_controller_history(p);
        return;
    }
    let mut speed = p.image_speed();
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
            if p.vel.x == 0.0 {
                "sDuckLeft"
            } else if p.vel.x.abs() < 3.0 {
                "sCrawlLeft"
            } else {
                "sRunLeft"
            }
        }
        PlayerState::LookingUp => {
            speed = p.vel.x.abs() * 0.1 + 0.1;
            if p.vel.x == 0.0 {
                "sLookLeft"
            } else if value(p, "character_tunnelman", 0.0) > 0.0 {
                // Tunnel Man's LOOKING_UP branch differs from RUNNING with Up.
                "sRunLeft"
            } else {
                "sLookRunL"
            }
        }
        PlayerState::Jumping => "sJumpLeft",
        PlayerState::Falling => {
            let count = value(p, "fall_state_ticks", 2.0);
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
            // characterSprite186 queries the active oRope family only.
            if ledge_point(w, p.pos, "oRope") {
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
    let name = character_sprite(
        p,
        if value(p, "push_timer", 0.0) > 20.0 && p.state != PlayerState::Climbing {
            "sPushLeft"
        } else {
            &name
        },
    )
    .to_owned();
    p.set_sprite(&name);
    p.set_image_speed(speed.min(1.0));
    remember_controller_history(p);
}

/// Equivalent to the source's statePrev/statePrevPrev for its three-FALLING
/// sprite gate. Only completed character controllers record this history;
/// dead, stunned, inactive, exiting and already-DUCKTOHANG Steps leave it alone.
/// A fresh character initializes both previous states to FALLING, hence two
/// completed FALLING states when the cache is absent after Create/reset.
fn remember_controller_history(p: &mut Player) {
    let fall_count = if p.state == PlayerState::Falling {
        value(p, "fall_state_ticks", 2.0) + 1.0
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

    #[test]
    fn source_climb_continuation_uses_active_ladder_family_points() {
        for object in ["oLadderOrange", "oLadderTop"] {
            for terrain in [true, false] {
                let mut world = World::new(4, 4, Area::Mines, 1);
                let index = 2 * world.width + 1;
                if terrain {
                    world.set_tile(1, 2, object);
                } else {
                    world.spawn(object, Vec2::new(16.0, 32.0));
                }
                let point = Vec2::new(24.0, 40.0);
                assert!(
                    climbing_ladder_point(&world, point, true),
                    "{object}/{terrain}"
                );

                if terrain {
                    world.tiles[index].engine_active = false;
                } else {
                    world.entities[0].engine_active = false;
                }
                assert!(
                    !climbing_ladder_point(&world, point, true),
                    "SourceClassicV2 collision_point must skip inactive {object}/{terrain}"
                );
                assert!(
                    climbing_ladder_point(&world, point, false),
                    "HistoricalV1 keeps its legacy predicate for {object}/{terrain}"
                );
            }
        }
    }

    #[test]
    fn source_climb_down_input_drops_when_ladder_below_is_deactivated() {
        for (active, expected_state) in
            [(true, PlayerState::Climbing), (false, PlayerState::Falling)]
        {
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.world = World::new(8, 8, Area::Mines, 1);
            game.world.set_tile(1, 3, "oLadderOrange");
            game.world.tiles[3 * game.world.width + 1].engine_active = active;
            game.player.pos = Vec2::new(24.0, 40.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Climbing;
            game.player.vel = Vec2::default();

            // Exercise the production controller directly so a surrounding
            // oLevel activation event cannot reactivate the deliberately
            // deactivated ladder before characterStepEvent runs.
            tick(
                &mut game,
                &InputFrame {
                    down: true,
                    ..Default::default()
                },
            );

            assert_eq!(
                game.player.state, expected_state,
                "SourceClassicV2 down continuation with active={active}"
            );
        }
    }

    #[test]
    fn source_controller_side_probe_ignores_inactive_solid_tiles_and_entities() {
        for terrain in [true, false] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.world = World::new(8, 8, Area::Mines, 1);
            if terrain {
                game.world.set_tile(2, 2, "oBlock");
                game.world.tiles[2 * game.world.width + 2].engine_active = false;
            } else {
                let id = game.world.spawn("oBlock", Vec2::new(32.0, 32.0));
                game.world.entity_mut(id).unwrap().engine_active = false;
            }
            // The right-edge collision_line is x=32, inside the inactive
            // block's mask. The controller must ignore it and keep momentum.
            game.player.pos = Vec2::new(27.0, 40.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Falling;
            game.player.vel = Vec2::new(3.0, 0.0);
            game.player.facing = 1;

            tick(
                &mut game,
                &InputFrame {
                    right: true,
                    ..Default::default()
                },
            );

            assert_eq!(
                game.player.pos.x, 29.0,
                "inactive solid terrain={terrain} must not stop the player"
            );
        }
    }

    #[test]
    fn source_long_fall_rectangle_ignores_inactive_spring_traps() {
        for terrain in [true, false] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.world = World::new(8, 8, Area::Mines, 1);
            if terrain {
                game.world.set_tile(2, 2, "oSpringTrap");
                game.world.tiles[2 * game.world.width + 2].engine_active = false;
            } else {
                let id = game.world.spawn("oSpringTrap", Vec2::new(32.0, 32.0));
                game.world.entity_mut(id).unwrap().engine_active = false;
            }
            game.player.pos = Vec2::new(40.0, 40.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Standing;
            game.player.data.insert("fall_timer".into(), 20.0);

            tick(&mut game, &InputFrame::default());

            assert_eq!(
                game.player.health, 3,
                "inactive spring trap terrain={terrain} must not suppress long-fall injury"
            );
            assert_eq!(game.player.stun, 60);
        }
    }

    #[test]
    fn game_step_sinking_block_still_accepts_player_ledge_grab() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(11.0, 73.0);
        game.player.state = PlayerState::Falling;
        game.world.spawn("oLava", Vec2::new(16.0, 96.0));
        let id = game.world.spawn("oPushBlock", Vec2::new(16.0, 80.0));
        // Adjacent grid-authored rows: ordinary gravity first drops the block
        // into the lava, then its own Step begins the fractional sinking.
        for _ in 0..5 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.player.pos, Vec2::new(11.0, 88.0));
        assert_eq!(game.player.state, PlayerState::Falling);
        assert!((game.world.entity(id).unwrap().pos.y - 83.2).abs() < 1e-10);
        // At the real controller's next ledge probe, (x+9,y-5)=(20,83)
        // lies in the canonical integer bbox but above the float sprite bbox.
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert_eq!(game.player.state, PlayerState::Hanging);
        assert_eq!(game.player.pos, Vec2::new(11.0, 88.0));
        assert_eq!(game.player.previous_pos, game.player.pos);
        assert!((game.world.entity(id).unwrap().pos.y - 83.25).abs() < 1e-10);
        assert_eq!(game.player.health, 4);
    }

    #[test]
    fn literal_solid_point_does_not_inflate_query_by_one_thousandth() {
        for dynamic in [false, true] {
            let mut world = World::new(8, 8, Area::Mines, 1);
            if dynamic {
                world.spawn("oPushBlock", Vec2::new(16.0, 80.0));
            } else {
                world.set_tile(1, 5, "oBlock");
            }
            assert!(
                !solid_point(&world, Vec2::new(15.9995, 88.0)),
                "dynamic={dynamic}"
            );
            assert!(
                solid_point(&world, Vec2::new(16.0, 88.0)),
                "dynamic={dynamic}"
            );
            assert!(
                !solid_point(&world, Vec2::new(32.0, 88.0)),
                "dynamic={dynamic}"
            );
        }
    }

    #[test]
    fn game_step_lava_sinking_block_uses_integer_point_bbox_and_smooth_position() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(64.0, 40.0);
        game.world.spawn("oLava", Vec2::new(16.0, 80.0));
        // The first global gravity pass moves the integer-authored block down
        // one pixel; its source Step then sinks it by .05 on each real tick.
        let id = game.world.spawn("oPushBlock", Vec2::new(16.0, 79.0));
        for _ in 0..5 {
            game.step(&InputFrame::default());
        }
        let block = game.world.entity(id).unwrap();
        assert!((block.pos.y - 80.25).abs() < 1e-10, "{:?}", block.pos);
        assert!((block.sprite_bounds().y - 80.25).abs() < 1e-10);
        for (point, expected) in [
            (Vec2::new(24.0, 80.1), true),
            (Vec2::new(24.0, 96.1), false),
            (Vec2::new(15.9995, 88.0), false),
        ] {
            assert_eq!(
                solid_point(&game.world, point),
                expected,
                "player point {point:?}"
            );
            assert_eq!(
                game.world.solid_point_except(point, 0),
                expected,
                "world point {point:?}"
            );
        }
    }

    #[test]
    fn natural_yeti_throw_keeps_source_standing_trait_for_long_fall_injury() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        for x in 0..game.world.width {
            game.world.set_tile(x, 31, "oBlock");
        }
        let pos = game.player.pos;
        game.spawn("oYeti", Vec2::new(pos.x - 8.0, pos.y - 8.0));
        game.step(&InputFrame::default());
        assert!(game.player.is_stunned());
        assert_eq!(game.player.stun, 0);
        // The late enemy throw leaves the earlier FALLING trait intact for
        // the following prelude; only the next skipped controller forces it.
        assert_eq!(value(&game.player, "source_platform_grounded", 0.0), 0.0);
        game.step(&InputFrame::default());
        assert_eq!(value(&game.player, "source_platform_grounded", 0.0), 1.0);
        let mut injured = false;
        for _ in 0..120 {
            game.step(&InputFrame::default());
            if game.lifetime_stats.misc_deaths[3] != 0 {
                injured = true;
                break;
            }
        }
        assert!(
            injured,
            "the real throw's long descent must retain source ON_GROUND"
        );
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(game.lifetime_stats.misc_deaths[3], 1);
        assert_eq!(value(&game.player, "wall_hurt", 0.0), 0.0);
        assert_eq!(game.player.stun, 60);
    }

    #[test]
    fn natural_long_fall_creates_source_blood_then_poofs_with_immediate_rng() {
        for (floor, injury, blood_count) in [(20, 1, 0), (31, 10, 3)] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            for x in 0..game.world.width {
                game.world.set_tile(x, floor, "oBlock");
            }
            let mut observed = false;
            for _ in 0..120 {
                let before = game.clone();
                game.step(&InputFrame::default());
                if !game
                    .events
                    .iter()
                    .any(|e| matches!(e, GameEvent::Damage { amount } if *amount == injury))
                {
                    continue;
                }
                observed = true;
                // The later source death checkpoint clamps lethal life to zero.
                assert_eq!(game.player.health, (before.player.health - injury).max(0));
                assert_eq!(game.player.stun, 60);
                assert_eq!(game.player.invincible, 0);
                assert!(
                    game.events
                        .iter()
                        .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndThud"))
                );
                assert!(
                    !game
                        .events
                        .iter()
                        .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndHurt"))
                );
                let particles: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| matches!(e.object.as_str(), "oBlood" | "oPoof"))
                    .collect();
                assert_eq!(particles.len(), blood_count + 2);
                assert!(
                    particles[..blood_count]
                        .iter()
                        .all(|e| e.object == "oBlood")
                );
                assert!(particles[blood_count..].iter().all(|e| e.object == "oPoof"));
                assert_eq!(particles[blood_count].vel.x, -0.4);
                assert_eq!(particles[blood_count + 1].vel.x, 0.4);
                assert!(
                    particles[blood_count..]
                        .iter()
                        .all(|e| { e.vel.y == 0.0 && e.image_speed() == 0.4 })
                );
                // scrCreateBlood's Create draws happen immediately, before the
                // following Poofs or any later Player Step producers. Each
                // inherited Detritus Create draws x twice, y once, gravity once.
                let mut expected_rng = before.rng;
                for blood in &particles[..blood_count] {
                    let vx = expected_rng.unit() * 4.0 - expected_rng.unit() * 4.0;
                    let _vy = -1.0 - expected_rng.unit() * 2.0;
                    let gravity = f64::from(1 + expected_rng.range(6)) * 0.1;
                    assert_eq!(blood.vel.x, vx);
                    assert_eq!(blood.value("gravity"), gravity);
                    assert_eq!(blood.value("world_effect_initialized"), 1.0);
                }
                assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
                break;
            }
            assert!(
                observed,
                "authored floor {floor} must cause its natural fall injury"
            );
        }
    }

    #[test]
    fn natural_landing_in_new_water_runs_fall_injury_before_bubble_create() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        for x in 0..game.world.width {
            game.world.set_tile(x, 31, "oBlock");
        }
        for _ in 0..120 {
            if grounded_state(game.player.state)
                && game.player.vel.y == 0.0
                && value(&game.player, "fall_timer", 0.0) > 48.0
            {
                break;
            }
            game.step(&InputFrame::default());
        }
        assert_eq!(game.player.health, 4);
        assert!(value(&game.player, "fall_timer", 0.0) > 48.0);
        // Add an ordinary water instance at the natural landing, before the
        // next Player Step recognizes the landing injury. Player position,
        // velocity, fall timer, health, and swimming state are never injected.
        let pos = game.player.pos;
        game.spawn(
            "oWaterSwim",
            Vec2::new((pos.x / 16.0).floor() * 16.0, (pos.y / 16.0).floor() * 16.0),
        );
        let mut expected_rng = game.rng.clone();
        for _ in 0..3 {
            expected_rng.unit();
            expected_rng.unit();
            expected_rng.unit();
            expected_rng.range(6);
        }
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 0);
        assert_eq!(value(&game.player, "fall_timer", -1.0), 0.0);
        // Dry Player Steps keep bubbleTimer at its maximum. Water entry ticks
        // it down after the injury; it does not create an immediate bubble.
        assert_eq!(value(&game.player, "bubble_timer", -1.0), 19.0);
        let effects: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| {
                matches!(
                    e.object.as_str(),
                    "oSplash" | "oBlood" | "oPoof" | "oBubble"
                )
            })
            .map(|e| e.object.as_str())
            .collect();
        assert_eq!(
            effects,
            ["oSplash", "oBlood", "oBlood", "oBlood", "oPoof", "oPoof"]
        );
        assert_eq!(game.rng.clone().next_u64(), expected_rng.clone().next_u64());
        let bubble_velocity = -f64::from(1 + expected_rng.range(3)) * 0.1;
        for _ in 0..20 {
            game.step(&InputFrame::default());
        }
        assert_eq!(value(&game.player, "bubble_timer", -1.0), 20.0);
        let bubble = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oBubble")
            .unwrap();
        assert_eq!(bubble.vel.y, bubble_velocity);
        assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
    }

    #[test]
    fn natural_long_fall_and_pit_deaths_record_the_source_counter_once() {
        for pit in [false, true] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            if !pit {
                for x in 0..game.world.width {
                    game.world.set_tile(x, 31, "oBlock");
                }
            }
            for _ in 0..160 {
                game.step(&InputFrame::default());
            }
            assert_eq!(game.state, RunState::Dead);
            assert_eq!(game.lifetime_stats.misc_deaths[if pit { 10 } else { 3 }], 1);
            assert_eq!(game.lifetime_stats.misc_deaths[if pit { 3 } else { 10 }], 0);

            // Authored editor rooms use the same controller but are excluded by
            // the source isRealLevel guard, including a fatal natural pit fall.
            let mut custom = Game::shell(7, RulesPreset::Classic);
            let mut level = crate::level::Level::default();
            level.set_tile(4, 3, '@').unwrap();
            custom.load_custom(&level, "counter-context", "");
            for _ in 0..160 {
                custom.step(&InputFrame::default());
            }
            assert_eq!(custom.state, RunState::Dead);
            assert_eq!(custom.lifetime_stats.misc_deaths[10], 0);
        }
    }

    #[test]
    fn real_running_look_and_crawl_inputs_preserve_source_animation_phase() {
        use crate::characters::{Character, apply};
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (character, running, looking, crawling) in [
                (Character::Explorer, "sRunLeft", "sLookRunL", "sCrawlLeft"),
                (
                    Character::Damsel,
                    "sDamselRunL",
                    "sDamselRunL",
                    "sDamselCrawlL",
                ),
                (
                    Character::TunnelMan,
                    "sTunnelRunL",
                    "sTunnelLookRunL",
                    "sTunnelCrawlL",
                ),
            ] {
                let mut level = crate::level::Level::default();
                level.set_tile(4, 3, '@').unwrap();
                for x in 0..40 {
                    level.set_tile(x, 5, '1').unwrap();
                }
                // Source settings.cfg permits disabling default Down-to-run.
                // This fixture exercises the crawl transition from full-speed
                // walking, so configure that supported option before any Step.
                // Existing serde APIs also keep the old-controller arm usable.
                let mut initial = serde_json::to_value(Game::shell(7, rules)).unwrap();
                initial["down_to_run"] = serde_json::Value::Bool(false);
                let mut game: Game = serde_json::from_value(initial).unwrap();
                game.load_custom(&level, "animation-phase", "");
                apply(&mut game, character);
                for _ in 0..8 {
                    game.step(&InputFrame::default());
                }
                for _ in 0..8 {
                    game.step(&InputFrame {
                        right: true,
                        ..Default::default()
                    });
                }
                assert_eq!(game.player.sprite, running);
                let before = game.player.animation;
                assert!(
                    before.floor() >= 1.0,
                    "fixture must reach a visible later pose"
                );
                game.step(&InputFrame {
                    right: true,
                    up: true,
                    ..Default::default()
                });
                assert_eq!(game.player.sprite, looking);
                let count = crate::catalog::sprite_frame_count(looking) as f64;
                let mut expected = before + 0.4;
                if expected.floor() >= count {
                    expected -= count;
                }
                assert!(
                    (game.player.animation - expected).abs() < 1e-9,
                    "{rules:?}/{character:?}: look transition restarted animation"
                );

                // Incoming speed must reach the source's fast-Duck threshold;
                // disabled Down-to-run then chooses the crawl fallback.
                assert_eq!(game.player.state, PlayerState::Running);
                assert!(game.player.grounded);
                assert!((2.0..=3.0).contains(&game.player.vel.x));
                let before = game.player.animation;
                game.step(&InputFrame {
                    right: true,
                    down: true,
                    ..Default::default()
                });
                assert_eq!(game.player.sprite, crawling);
                assert_eq!(game.player.state, PlayerState::Ducking);
                assert!(game.player.vel.x > 0.0 && game.player.vel.x < 3.0);
                let count = crate::catalog::sprite_frame_count(crawling) as f64;
                let mut expected = before + 0.8;
                if expected.floor() >= count {
                    expected -= count;
                }
                assert!(
                    (game.player.animation - expected).abs() < 1e-9,
                    "{rules:?}/{character:?}: crawl transition restarted animation"
                );
            }
        }
    }

    #[test]
    fn real_ankh_pickup_then_exit_lava_revives_without_interrupting_exit_animation() {
        use crate::characters::{self, Character};
        use crate::ending::EndingStage;
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
                let mut level = crate::level::Level::default();
                level.set_tile(6, 7, '@').unwrap();
                level.set_tile(7, 7, 'X').unwrap();
                for x in 0..40 {
                    level.set_tile(x, 8, '1').unwrap();
                }
                let mut game = Game::shell(11, rules);
                game.load_custom(&level, "exit-ankh", "");
                characters::apply(&mut game, character);
                for _ in 0..8 {
                    game.step(&InputFrame::default());
                }
                let entrance = game.player.pos;
                let ankh = game.world.spawn(
                    "oAnkh",
                    Vec2::new(game.player.pos.x, game.player.pos.y + 4.),
                );
                crate::items::init_entity(game.world.entity_mut(ankh).unwrap());
                let item = game.world.entity_mut(ankh).unwrap();
                item.data.insert("cost".into(), 0.);
                item.data.insert("forSale".into(), 0.);
                game.step(&InputFrame {
                    down: true,
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                });
                assert!(game.player.has("Ankh"));
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
                assert!(game.player.is_exiting());
                assert_eq!(game.state, RunState::Ending);
                let sprite = game.player.sprite.clone();
                let animation = game.player.animation;
                // Controlled arriving lava exercises the real hazard producer.
                // Pickup and door entry use inputs, without player field writes.
                let pos = game.player.pos;
                let lava = game.world.spawn("oLava", Vec2::new(pos.x - 8., pos.y - 8.));
                game.step(&InputFrame::default());
                assert!(game.events.contains(&GameEvent::Damage { amount: 99 }));
                assert_eq!(game.player.health, 4, "{rules:?}/{character:?}");
                assert!(!game.player.has("Ankh"));
                assert_eq!(value(&game.player, "active", 0.), 1.);
                assert_eq!(game.player.pos, entrance);
                assert_eq!(game.player.vel, Vec2::default());
                assert_eq!(game.player.sprite, sprite);
                assert_eq!(game.player.animation, animation + 0.5);
                assert_eq!(game.state, RunState::Ending);
                let age = game.world.entity(lava).unwrap().age;
                game.step(&InputFrame {
                    right: true,
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                });
                assert_eq!(game.player.pos, entrance);
                assert_eq!(game.player.vel, Vec2::default());
                assert_eq!(game.player.state, PlayerState::Standing);
                assert_eq!(game.player.sprite, sprite);
                assert_eq!(game.player.animation, animation + 1.);
                assert!(game.world.entity(lava).unwrap().age > age);
                for _ in 0..32 {
                    if game.ending.as_ref().unwrap().stage != EndingStage::Exit {
                        break;
                    }
                    game.step(&InputFrame::default());
                }
                assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Transition);
                assert_eq!(game.player.health, 4);
            }
        }
    }
    #[test]
    fn natural_cape_open_then_jetpack_pickup_stops_cape_drag_on_following_tick() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut level = crate::level::Level::default();
            level.set_tile(4, 3, '@').unwrap();
            for x in 0..40 {
                level.set_tile(x, 20, '1').unwrap();
            }
            let mut g = Game::shell(7, rules);
            g.load_custom(&level, "cape-swap", "");
            let spawn = g.player.pos;
            let cape = g.spawn("oCapePickup", Vec2::new(spawn.x, spawn.y + 2.0));
            let jetpack = g.spawn("oJetpack", Vec2::new(spawn.x + 8.0, spawn.y + 4.0));
            assert_eq!(g.world.entity(cape).unwrap().value("cost"), 12000.0);
            assert_eq!(g.world.entity(jetpack).unwrap().value("cost"), 20000.0);
            // Source Create assigns shop costs. With no Shopkeeper, oItem
            // clears them in its later Step, after the player's pickup event.
            // Begin a real whip while that Item Step clears the costs. Source
            // Player Step1219 gives an airborne empty-hand attack priority over
            // Down+Attack pickup; the following attack reaches its pickup
            // else-if because the first whip is still in progress.
            g.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert!(g.player.attack_timer > 0);
            // Release before the next ordinary press; keep the whip in progress.
            g.step(&InputFrame {
                attack_released: true,
                ..Default::default()
            });
            assert!(g.player.attack_timer > 0);
            assert_eq!(g.world.entity(cape).unwrap().value("cost"), 0.0);
            assert_eq!(g.world.entity(jetpack).unwrap().value("cost"), 0.0);
            g.step(&InputFrame {
                down: true,
                attack_pressed: true,
                jump: true,
                ..Default::default()
            });
            assert!(g.player.has("Cape"));
            g.step(&InputFrame {
                jump_released: true,
                ..Default::default()
            });
            let falling_before_open = g.player.vel.y;
            assert!(falling_before_open > 0.0);
            g.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                down: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert!(g.player.has("Jetpack"));
            assert!(!g.player.has("Cape"));
            // The controller ran before item pickup: the source Cape was
            // still present for this tick's drag and its falling-time reset.
            let cape_drag = (falling_before_open + 1.0) * 0.5;
            assert_eq!(g.player.vel.y, cape_drag);
            assert_eq!(value(&g.player, "cape_open", 0.0), 1.0);
            g.step(&InputFrame::default());
            assert_eq!(value(&g.player, "cape_open", -1.0), 0.0);
            assert_eq!(g.player.vel.y, cape_drag + 1.0);
            assert!(value(&g.player, "fall_timer", 0.0) > 0.0);
            g.step(&InputFrame::default());
            assert_eq!(g.player.vel.y, cape_drag + 2.0);
        }
    }
    #[test]
    fn source_precise_floor_probe_rejects_only_dark_fall_transparent_corner() {
        for (x, supported) in [(76.0, false), (77.0, true)] {
            let mut g = room(RulesPreset::Classic);
            g.world.spawn("oDarkFall", Vec2::new(80.0, 80.0));
            g.player.pos = Vec2::new(x, 72.0);
            g.step(&InputFrame::default());
            if supported {
                assert_eq!(g.player.pos.y, 72.0);
            } else {
                // The next image row is opaque: fall past the transparent
                // corner, then gain support from the real image pixels.
                assert_eq!(g.player.pos.y, 73.0);
                assert_eq!(g.player.state, PlayerState::Falling);
            }
        }
    }
    #[test]
    fn kapala_healing_a_real_corpse_preserves_latched_death_and_suppresses_movement() {
        use crate::characters::{self, Character};
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            let mut g = room(RulesPreset::Classic);
            characters::apply(&mut g, character);
            g.player.health = 4;
            g.player.equipment.insert("Kapala".into());
            let bullet = g.world.spawn("oBullet", Vec2::new(80.0, 152.0));
            g.world.entity_mut(bullet).unwrap().vel.x = 8.0;
            g.step(&InputFrame::default());
            assert_eq!(g.state, RunState::Playing);
            assert!(g.player.health <= 0);
            // Automatic Bullet collision follows Player Step; its independent
            // death flag is latched during the next Player Step checkpoint.
            for _ in 0..40 {
                g.step(&InputFrame::default());
            }
            assert_eq!(g.state, RunState::Dead);
            g.data.insert("bloodLevel".into(), 8.0);
            // Let real Blood Create, movement and Alarm 2 establish collectible
            // state. No particle age/velocity/collectible field is injected.
            for _ in 0..9 {
                let pos = g.player.pos;
                crate::world_events::spawn_detritus(&mut g, "oBlood", pos);
            }
            for _ in 0..40 {
                g.step(&InputFrame::default());
                if g.player.health > 0 {
                    break;
                }
            }
            assert_eq!(g.player.health, 1, "legitimate Kapala heal {character:?}");
            assert_eq!(g.state, RunState::Dead);
            let mut neutral = g.clone();
            for _ in 0..5 {
                g.step(&InputFrame {
                    right: true,
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                });
                neutral.step(&InputFrame::default());
                assert_eq!(
                    g.player.pos, neutral.player.pos,
                    "input changed corpse motion {character:?}"
                );
                assert_eq!(g.player.vel, neutral.player.vel);
                assert_eq!(g.player.state, PlayerState::Dead);
                assert_eq!(g.player.health, 1);
            }
        }
    }
    fn fill_bath(g: &mut Game, start_x: usize) {
        for x in start_x..20 {
            for y in 6..10 {
                g.world.set_tile(x, y, "oWaterSwim");
            }
        }
    }
    #[test]
    fn actual_water_entry_splashes_once_and_bubbles_on_source_twenty_one_tick_cycle() {
        let mut g = room(RulesPreset::Classic);
        fill_bath(&mut g, 0);
        g.step(&InputFrame::default());
        assert_eq!(value(&g.player, "swimming", 0.0), 1.0);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oSplash")
                .count(),
            1
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oBubble")
                .count(),
            1
        );
        assert_eq!(value(&g.player, "bubble_timer", -1.0), 20.0);
        assert!(
            g.events
                .iter()
                .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndSplash"))
        );
        for _ in 0..20 {
            g.step(&InputFrame::default());
            assert!(
                !g.events
                    .iter()
                    .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndSplash"))
            );
        }
        assert_eq!(value(&g.player, "bubble_timer", -1.0), 0.0);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oBubble")
                .count(),
            1
        );
        g.step(&InputFrame::default());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oBubble")
                .count(),
            2
        );
        assert_eq!(value(&g.player, "fall_timer", -1.0), 0.0);
    }
    #[test]
    fn actual_projectile_knockback_into_water_runs_effects_before_stunned_return() {
        let mut g = room(RulesPreset::Classic);
        g.player.pos.x = 95.0;
        g.player.health = 20;
        fill_bath(&mut g, 6);
        let bullet = g.world.spawn("oBullet", Vec2::new(87.0, 152.0));
        g.world.entity_mut(bullet).unwrap().vel.x = 8.0;
        g.step(&InputFrame::default());
        assert!(g.player.is_stunned());
        assert_eq!(value(&g.player, "swimming", 0.0), 0.0);
        g.step(&InputFrame::default());
        assert!(g.player.pos.x >= 96.0);
        g.step(&InputFrame::default());
        assert!(g.player.is_stunned());
        assert_eq!(value(&g.player, "swimming", 0.0), 1.0);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oSplash")
                .count(),
            1
        );
        assert!(
            g.events
                .iter()
                .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndSplash"))
        );
        assert_eq!(value(&g.player, "fall_timer", -1.0), 0.0);
    }
    #[test]
    fn lava_death_burns_after_leaving_lava_and_counter_eventually_expires() {
        let mut g = room(RulesPreset::Classic);
        g.world.set_tile(5, 9, "oLava");
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Dead);
        assert_eq!(value(&g.player, "swimming", 0.0), 0.0);
        assert_eq!(value(&g.player, "burning", 0.0), 100.0);
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.object == "oSplash" || e.object == "oBubble")
        );
        // Isolate the lingering source effect from lava reapplying burning100.
        g.world.set_tile(5, 9, "");
        let mut produced = false;
        for n in 1..=100 {
            g.step(&InputFrame::default());
            produced |= g.world.entities.iter().any(|e| e.object == "oBurn");
            assert_eq!(value(&g.player, "burning", -1.0), f64::from(100 - n));
        }
        assert!(produced);
        for _ in 0..30 {
            g.step(&InputFrame::default());
        }
        assert!(!g.world.entities.iter().any(|e| e.object == "oBurn"));
    }
    #[test]
    fn real_contact_blink_alternates_every_authoritative_tick_and_stops_at_expiry() {
        use crate::characters::{self, Character};
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            let mut g = room(RulesPreset::Classic);
            characters::apply(&mut g, character);
            g.player.health = 20;
            let snake = g.world.spawn("oSnake", Vec2::new(80.0, 144.0));
            g.step(&InputFrame::default());
            assert_eq!(g.player.health, 19, "real contact {character:?}");
            assert_eq!(value(&g.player, "blink", -1.0), 30.0);
            assert_eq!(value(&g.player, "blink_toggle", 0.0), -1.0);
            g.world.entity_mut(snake).unwrap().alive = false;
            for n in 1..=30 {
                g.step(&InputFrame::default());
                assert_eq!(value(&g.player, "blink", -1.0), f64::from(30 - n));
                assert_eq!(
                    value(&g.player, "blink_toggle", 0.0),
                    if n % 2 == 1 { 1.0 } else { -1.0 }
                );
            }
            for _ in 0..5 {
                g.step(&InputFrame::default());
                assert_eq!(value(&g.player, "blink_toggle", 0.0), -1.0);
            }
        }
    }
    #[test]
    fn invulnerability_without_blink_stays_visible_and_does_not_count_down_on_corpses() {
        let mut g = room(RulesPreset::Classic);
        g.player.invincible = 60;
        g.step(&InputFrame::default());
        assert_eq!(g.player.invincible, 59);
        assert_eq!(value(&g.player, "blink_toggle", 0.0), -1.0);
        let bullet = g.world.spawn("oBullet", Vec2::new(80.0, 152.0));
        g.world.entity_mut(bullet).unwrap().vel.x = 8.0;
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Playing);
        assert!(g.player.health <= 0);
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Dead);
        let remaining = g.player.invincible;
        for _ in 0..10 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.player.invincible, remaining);
        assert_eq!(value(&g.player, "blink_toggle", 0.0), -1.0);
    }
    #[test]
    fn repeated_actual_web_jumps_consume_source_twelve_life_until_eleven_jumps() {
        let mut g = room(RulesPreset::Classic);
        let web = g.world.spawn("oWeb", Vec2::new(80.0, 144.0));
        for n in 1..=11 {
            assert!(
                g.world.entity(web).is_some(),
                "web vanished before jump {n}"
            );
            g.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            });
            if n == 11 {
                assert!(g.world.entity(web).is_none());
            } else {
                assert_eq!(
                    g.world.entity(web).unwrap().value("life"),
                    f64::from(12 - n)
                );
                for _ in 0..60 {
                    g.step(&InputFrame {
                        jump_released: true,
                        ..Default::default()
                    });
                    if g.player.grounded && grounded_state(g.player.state) && g.player.vel.y == 0.0
                    {
                        break;
                    }
                }
                assert!(g.player.grounded);
            }
        }
    }
    #[test]
    fn web_jump_uses_instance_mask_at_far_corner_instead_of_origin_radius() {
        let mut g = room(RulesPreset::Classic);
        g.player.pos.x = 95.0;
        let web = g.world.spawn("oWeb", Vec2::new(80.0, 144.0));
        assert!(
            g.world
                .entity(web)
                .unwrap()
                .sprite_bounds()
                .contains(g.player.pos)
        );
        assert!(g.world.entity(web).unwrap().pos.distance(g.player.pos) > 12.0);
        g.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.world.entity(web).unwrap().value("life"), 11.0);
        assert!(g.player.vel.y < 0.0);
    }
    #[test]
    fn sustained_wall_push_uses_selected_pose_and_clears_on_release_or_jump() {
        use crate::characters::{self, Character};
        for (character, sprite) in [
            (Character::Explorer, "sPushLeft"),
            (Character::Damsel, "sDamselPushL"),
            (Character::TunnelMan, "sTunnelPushL"),
        ] {
            let mut g = room(RulesPreset::Classic);
            characters::apply(&mut g, character);
            g.world.set_tile(6, 9, "oBlock");
            g.player.pos.x = 91.0;
            for _ in 0..25 {
                g.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            assert_eq!(g.player.sprite, sprite);
            assert!(value(&g.player, "push_timer", 0.0) > 20.0);
            g.step(&InputFrame::default());
            assert_eq!(value(&g.player, "push_timer", -1.0), 0.0);
            assert_ne!(g.player.sprite, sprite);
            for _ in 0..130 {
                g.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            assert_eq!(value(&g.player, "push_timer", -1.0), 100.0);
            g.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            });
            assert_eq!(value(&g.player, "push_timer", -1.0), 0.0);
            assert!(g.player.vel.y < 0.0);
            g.step(&InputFrame {
                jump: true,
                ..Default::default()
            });
            assert_ne!(g.player.sprite, sprite);
        }
    }
    #[test]
    fn moveable_solid_push_reaches_pose_after_two_ticks_on_either_side() {
        use crate::characters::{self, Character};
        for (character, sprite) in [
            (Character::Explorer, "sPushLeft"),
            (Character::Damsel, "sDamselPushL"),
            (Character::TunnelMan, "sTunnelPushL"),
        ] {
            for right in [false, true] {
                let mut g = room(RulesPreset::Classic);
                characters::apply(&mut g, character);
                g.player.pos.x = if right { 91.0 } else { 117.0 };
                g.world.spawn("oPushBlock", Vec2::new(96.0, 144.0));
                for _ in 0..2 {
                    g.step(&InputFrame {
                        left: !right,
                        right,
                        ..Default::default()
                    });
                }
                assert_eq!(value(&g.player, "push_timer", -1.0), 22.0);
                assert_eq!(g.player.sprite, sprite);
            }
        }
    }
    #[test]
    fn projectile_stun_eventually_recovers_on_authored_support_without_new_hits() {
        use crate::characters::{self, Character};
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for support in ["oBlock", "oIce", "oTreeBranchTile", "oPushBlock"] {
                for environment in ["", "oWaterSwim", "oWeb", "oLadder"] {
                    let mut g = room(RulesPreset::Classic);
                    characters::apply(&mut g, character);
                    g.player.health = 20;
                    for x in 0..20 {
                        g.world.set_tile(x, 10, "");
                        g.world.set_tile(x, 11, "oBlock");
                        if support == "oPushBlock" {
                            g.world.spawn(support, Vec2::new(x as f64 * 16.0, 160.0));
                        } else {
                            g.world.set_tile(x, 10, support);
                        }
                        if !environment.is_empty() {
                            g.world.set_tile(x, 9, environment);
                        }
                    }
                    let bullet = g.world.spawn("oBullet", Vec2::new(80.0, 152.0));
                    g.world.entity_mut(bullet).unwrap().vel.x = 8.0;
                    g.step(&InputFrame::default());
                    assert!(
                        g.player.stun > 0,
                        "impact did not stun: {character:?}/{support}/{environment}"
                    );
                    for _ in 0..300 {
                        g.step(&InputFrame::default());
                        if g.player.stun == 0 {
                            break;
                        }
                    }
                    assert_eq!(
                        g.player.stun, 0,
                        "stuck: {character:?}/{support}/{environment} pos={:?} vel={:?} sprite={} state={:?}",
                        g.player.pos, g.player.vel, g.player.sprite, g.player.state
                    );
                    assert_eq!(
                        g.state,
                        RunState::Playing,
                        "died: {character:?}/{support}/{environment}"
                    );
                    g.step(&InputFrame {
                        jump: true,
                        jump_pressed: true,
                        ..Default::default()
                    });
                    assert!(
                        g.player.vel.y < 0.0,
                        "cannot jump after recovery: {character:?}/{support}/{environment} state={:?}",
                        g.player.state
                    );
                    assert_ne!(g.player.state, PlayerState::Stunned);
                }
            }
        }
    }
    #[test]
    fn yeti_throw_expires_wall_injury_during_long_fall() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.world = World::new(128, 100, Area::Ice, 9);
        for x in 0..128 {
            g.world.set_tile(x, 80, "oIce");
        }
        g.world.set_tile(8, 9, "oIce");
        g.player.pos = Vec2::new(136.0, 136.0);
        g.player.state = PlayerState::Standing;
        g.player.grounded = true;
        g.player.health = 20;
        g.world.spawn("oYeti", Vec2::new(128.0, 128.0));
        g.step(&InputFrame::default());
        assert_eq!(g.player.stun, 0, "Classic throws do not add a stun timer");
        assert!(
            g.player.is_stunned(),
            "fixture must enter the real Yeti grab event"
        );
        assert_eq!(value(&g.player, "wall_hurt", 0.0), 1.0);
        for _ in 0..40 {
            g.step(&InputFrame::default());
        }
        assert!(value(&g.player, "fall_timer", 0.0) > 16.0);
        assert_eq!(value(&g.player, "wall_hurt", -1.0), 0.0);
        assert_eq!(g.player.health, 20);
    }
    #[test]
    fn zero_timer_throws_remain_stunned_in_flight_then_recover_for_all_characters() {
        use crate::characters::{self, Character};
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for actor in ["oYeti", "oHawkman", "oShopkeeper"] {
                let mut g = room(RulesPreset::Classic);
                characters::apply(&mut g, character);
                g.player.health = 20;
                g.player.pos = Vec2::new(264.0, 136.0);
                g.world.set_tile(16, 9, "oIce");
                let id = g.world.spawn(actor, Vec2::new(256.0, 128.0));
                let enemy = g.world.entity_mut(id).unwrap();
                crate::enemies::init_entity(enemy);
                if actor == "oShopkeeper" {
                    enemy.state = 2;
                    enemy.data.insert("has_gun".into(), 0.0);
                }
                g.step(&InputFrame::default());
                assert_eq!(g.player.stun, 0);
                assert!(g.player.is_stunned(), "real throw {character:?}/{actor}");
                // Isolate recovery from subsequent attacks by the same enemy.
                g.world.entity_mut(id).unwrap().alive = false;
                g.step(&InputFrame {
                    attack: true,
                    attack_pressed: true,
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                });
                assert!(
                    g.player.is_stunned(),
                    "must still tumble {character:?}/{actor}"
                );
                assert_eq!(g.player.attack_timer, 0);
                for _ in 0..120 {
                    if !g.player.is_stunned() {
                        break;
                    }
                    g.step(&InputFrame::default());
                }
                assert!(
                    !g.player.is_stunned(),
                    "eventual recovery {character:?}/{actor}"
                );
                assert!(
                    g.player.health > 0,
                    "survive {character:?}/{actor} pos={:?} state={:?}",
                    g.player.pos,
                    g.player.state
                );
                for _ in 0..30 {
                    if g.player.grounded && grounded_state(g.player.state) && g.player.vel.y == 0.0
                    {
                        break;
                    }
                    g.step(&InputFrame::default());
                }
                assert!(
                    g.player.grounded,
                    "land after recovery {character:?}/{actor}"
                );
                g.step(&InputFrame {
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                });
                assert!(
                    g.player.vel.y < 0.0,
                    "jump after throw {character:?}/{actor} state={:?} vel={:?} stun={}",
                    g.player.state,
                    g.player.vel,
                    g.player.stun
                );
            }
        }
    }
    #[test]
    fn projectile_stun_interrupts_attack_and_held_bow_then_restores_controls() {
        use crate::characters::{self, Character};
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for bow in [false, true] {
                let mut g = room(RulesPreset::Classic);
                characters::apply(&mut g, character);
                g.player.health = 20;
                if bow {
                    let id = g.world.spawn("oBow", g.player.pos);
                    g.player.held = Some(id);
                    g.world.entity_mut(id).unwrap().held = true;
                }
                g.step(&InputFrame {
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                });
                let bullet = g.world.spawn("oBullet", Vec2::new(80.0, 152.0));
                g.world.entity_mut(bullet).unwrap().vel.x = 8.0;
                g.step(&InputFrame {
                    attack: true,
                    ..Default::default()
                });
                assert_eq!(g.player.stun, 20);
                let pre_ids: Vec<_> = g
                    .world
                    .entities
                    .iter()
                    .filter(|e| {
                        e.alive && e.engine_active && crate::catalog::is_a(&e.object, "oWhipPre")
                    })
                    .map(|e| e.id)
                    .collect();
                assert_eq!(pre_ids.len(), usize::from(!bow), "{character:?}/bow={bow}");
                g.step(&InputFrame::default());
                assert!(g.player.held.is_none());
                assert_eq!(g.player.attack_timer, 0);
                // The late hurt branch destroys only oWhip descendants.
                // Pre parts survive this Step; the following non-whipping
                // branch destroys oWhipPre descendants (or their Alarm0 does).
                assert!(!g.world.entities.iter().any(|e| e.alive
                    && e.engine_active
                    && crate::catalog::is_a(&e.object, "oWhip")));
                let surviving_pre_ids: Vec<_> = g
                    .world
                    .entities
                    .iter()
                    .filter(|e| {
                        e.alive && e.engine_active && crate::catalog::is_a(&e.object, "oWhipPre")
                    })
                    .map(|e| e.id)
                    .collect();
                assert_eq!(surviving_pre_ids, pre_ids, "{character:?}/bow={bow}");
                g.step(&InputFrame::default());
                assert!(!g.world.entities.iter().any(|e| e.alive
                    && e.engine_active
                    && (crate::catalog::is_a(&e.object, "oWhip")
                        || crate::catalog::is_a(&e.object, "oWhipPre"))));
                for _ in 0..120 {
                    if g.player.stun == 0 {
                        break;
                    }
                    g.step(&InputFrame::default());
                }
                assert_eq!(g.player.stun, 0);
                assert_eq!(value(&g.player, "bounced", -1.0), 0.0);
                g.step(&InputFrame {
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                });
                assert!(g.player.vel.y < 0.0, "{character:?}/bow={bow}");
            }
        }
    }
    #[test]
    fn natural_ledge_grab_projectile_hit_recovers_on_both_sides_for_each_character() {
        use crate::characters::{self, Character};
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for facing in [-1, 1] {
                let mut g = room(RulesPreset::Classic);
                characters::apply(&mut g, character);
                g.player.health = 20;
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
                g.step(&InputFrame::default());
                g.step(&InputFrame::default());
                let mut held = false;
                for _ in 0..60 {
                    let press = !held && !g.player.grounded;
                    held |= press;
                    g.step(&InputFrame {
                        left: facing < 0,
                        right: facing > 0,
                        jump: held,
                        jump_pressed: press,
                        ..Default::default()
                    });
                    if g.player.state == PlayerState::Hanging {
                        break;
                    }
                }
                assert_eq!(
                    g.player.state,
                    PlayerState::Hanging,
                    "natural grab {character:?}/{facing}"
                );
                let bullet = g.world.spawn(
                    "oBullet",
                    Vec2::new(g.player.pos.x - f64::from(facing) * 8.0, g.player.pos.y),
                );
                g.world.entity_mut(bullet).unwrap().vel.x = f64::from(facing) * 8.0;
                g.step(&InputFrame::default());
                assert_eq!(g.player.stun, 20, "real impact {character:?}/{facing}");
                for _ in 0..180 {
                    if !g.player.is_stunned() {
                        break;
                    }
                    g.step(&InputFrame::default());
                }
                assert!(!g.player.is_stunned(), "recovery {character:?}/{facing}");
                g.step(&InputFrame {
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                });
                assert!(g.player.vel.y < 0.0, "jump {character:?}/{facing}");
            }
        }
    }
    #[test]
    fn actual_ladder_climb_projectile_stun_releases_climb_and_recovers() {
        use crate::characters::{self, Character};
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            let mut g = room(RulesPreset::Classic);
            characters::apply(&mut g, character);
            g.player.health = 20;
            for y in 5..10 {
                g.world.set_tile(5, y, "oLadderOrange");
            }
            g.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(g.player.state, PlayerState::Climbing);
            let bullet = g.world.spawn("oBullet", Vec2::new(80.0, 152.0));
            g.world.entity_mut(bullet).unwrap().vel.x = 8.0;
            g.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(g.player.stun, 20);
            for _ in 0..120 {
                if !g.player.is_stunned() {
                    break;
                }
                g.step(&InputFrame::default());
            }
            assert!(!g.player.is_stunned());
            assert_ne!(g.player.state, PlayerState::Climbing);
            for _ in 0..30 {
                if g.player.grounded && grounded_state(g.player.state) && g.player.vel.y == 0.0 {
                    break;
                }
                g.step(&InputFrame::default());
            }
            g.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            });
            assert!(
                g.player.vel.y < 0.0,
                "{character:?} pos={:?} state={:?} data={:?}",
                g.player.pos,
                g.player.state,
                g.player.data
            );
        }
    }
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
        crate::dispatcher::register_instances(g);
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
        g.world.set_tile(5, 9, "oLadderOrange");
        g.world.set_tile(5, 8, "oLadderOrange");
        g.player.pos.x = 89.0;
        crate::dispatcher::register_instances(&mut g);
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
            crate::dispatcher::register_instances(&mut g);
            let solid_order = crate::dispatcher::descendant_instances(&g, "oSolid");
            slope_move(
                &mut g.world,
                &mut g.player,
                1,
                false,
                false,
                &solid_order,
                &mut g.events,
            );
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
        crate::dispatcher::register_instances(&mut g);
        let solid_order = crate::dispatcher::descendant_instances(&g, "oSolid");
        slope_move(
            &mut g.world,
            &mut g.player,
            1,
            false,
            false,
            &solid_order,
            &mut g.events,
        );
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
        crate::dispatcher::register_instances(&mut g);
        let solid_order = crate::dispatcher::descendant_instances(&g, "oSolid");
        slope_move(
            &mut g.world,
            &mut g.player,
            1,
            false,
            false,
            &solid_order,
            &mut g.events,
        );
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
            g.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            });
            g.step(&InputFrame {
                jump: true,
                ..Default::default()
            });
            assert_eq!(g.player.sprite, jump_sprite);
            assert!(crate::catalog::sprite_def(&g.player.sprite).is_some());
            g.player.stun = 10;
            g.player.vel.x = 0.0;
            g.step(&InputFrame::default());
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

    #[test]
    fn actual_step_successful_push_pixels_gate_sound_at_block_origin_both_directions() {
        for direction in [-1.0, 1.0] {
            let mut g = Game::legacy_shell(1, RulesPreset::Classic);
            g.world = World::new(20, 20, Area::Mines, 1);
            for x in 0..20 {
                g.world.set_tile(x, 10, "oBlock");
            }
            g.player.pos = Vec2::new(88.0, 152.0);
            g.player.previous_pos = g.player.pos;
            g.player.state = PlayerState::Standing;
            g.player.grounded = true;
            g.player.pos.x = if direction > 0.0 { 91.0 } else { 117.0 };
            g.player.previous_pos = g.player.pos;
            let id = g.world.spawn("oPushBlock", Vec2::new(96.0, 144.0));
            g.world
                .set_tile(if direction > 0.0 { 8 } else { 4 }, 9, "oBlock");
            let input = InputFrame {
                left: direction < 0.0,
                right: direction > 0.0,
                ..Default::default()
            };
            let mut total_pixels = 0;
            let mut blocked_steps = 0;
            for _ in 0..80 {
                let before = g.world.entity(id).unwrap().pos;
                g.step(&input);
                let after = g.world.entity(id).unwrap().pos;
                let pixels = ((after.x - before.x) * direction) as usize;
                let sounds: Vec<_> = g
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::SoundIfIdle { name, pos } if name == "sndPush" => Some(*pos),
                        _ => None,
                    })
                    .collect();
                let expected: Vec<_> = (1..=pixels)
                    .map(|pixel| Vec2::new(before.x + direction * pixel as f64, before.y))
                    .collect();
                assert_eq!(sounds, expected);
                assert!(!g.events.iter().any(|event| {
                    matches!(event, GameEvent::Sound { name, .. } if name == "sndPush")
                }));
                total_pixels += pixels;
                if total_pixels == 16 && pixels == 0 {
                    blocked_steps += 1;
                }
            }
            assert_eq!(total_pixels, 16);
            assert!(
                blocked_steps > 10,
                "holding toward the wall never emits another push"
            );
        }
    }
}

#[cfg(test)]
#[path = "player/ledge_tests.rs"]
mod ledge_tests;

#[cfg(test)]
#[path = "player/movement_top_tests.rs"]
mod movement_top_tests;

#[cfg(test)]
mod native_ball_tether_boundary_tests {
    use super::*;
    #[test]
    fn player_tether_uses_previous_draw_mirror_before_motion() {
        for (scale, x, expected) in [
            (1., 114., 3.),
            (1., 115., 0.),
            (-1., 115., 3.),
            (-1., 116., 0.),
        ] {
            let mut g = Game::shell(29, crate::RulesPreset::Classic);
            g.world.spawn("oBall", Vec2::new(80., 104.));
            g.player.pos = Vec2::new(x, 104.);
            g.player.set_sprite("sRunLeft");
            g.player.data.insert("image_xscale".into(), scale);
            g.player.vel.x = 3.;
            let mut fall = 0.;
            apply_ball_tether(&g.world, &mut g.player, &InputFrame::default(), &mut fall);
            assert_eq!(
                g.player.vel.x, expected,
                "SOURCE_INCLUSIVE_GAP24 scale={scale} x={x}"
            );
        }
    }
    #[test]
    fn jumping_tether_uses_ball_sprite_top_not_movement_top() {
        for (y, expected) in [(70., -3.), (68., 0.), (69., 0.)] {
            let mut g = Game::shell(29, crate::RulesPreset::Classic);
            g.world.spawn("oBall", Vec2::new(80., 104.));
            g.player.pos = Vec2::new(80., y);
            g.player.set_sprite("sStandLeft");
            g.player.vel.y = -3.;
            let mut fall = 0.;
            apply_ball_tether(&g.world, &mut g.player, &InputFrame::default(), &mut fall);
            assert_eq!(
                g.player.vel.y,
                expected,
                "SOURCE_BALL_SPRITE_TOP100 PLAYER_BOTTOM={} y={y}",
                y + 7.
            );
        }
    }
}

#[cfg(test)]
#[path = "player/source_animation_callback_tests.rs"]
mod source_animation_callback_tests;

#[cfg(test)]
#[path = "player/swimming_raw_point_tests.rs"]
mod swimming_raw_point_tests;
