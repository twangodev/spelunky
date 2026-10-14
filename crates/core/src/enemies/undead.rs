//! Classic 1.1 Skeleton, Zombie and Vampire Create/Step event translations.
//! Source instance origins and the separate movement/collision masks are retained.
//! Automatic animation and paired Collision events belong to the engine dispatcher.
use super::{bounds, move_body};
use crate::{CollisionPrecision, Entity, Game, PlayerState, Rect, Vec2, World};

pub fn matches(object: &str) -> bool {
    matches!(object, "oSkeleton" | "oZombie" | "oVampire")
}

pub fn init(e: &mut Entity) {
    e.state = 0;
    e.timer = if e.object == "oSkeleton" { 20 } else { 0 };
    e.health = if e.object == "oVampire" { 6 } else { 1 };
    e.facing = 1;
    e.vel = Vec2::new(if e.object == "oVampire" { 2.5 } else { 0.0 }, 0.0);
    e.set_image_speed(if e.object == "oZombie" { 0.4 } else { 0.5 });
    e.data.insert(
        "stun_time".into(),
        if e.object == "oVampire" { 60.0 } else { 200.0 },
    );
    e.data.insert(
        "bloodless".into(),
        if e.object == "oZombie" { 0.0 } else { 1.0 },
    );
    e.data.insert("source_facing_pending".into(), 1.0);
    // Skeleton Create leaves sSkeletonCreateL; only its Step selects idle/walk.
    if e.object != "oSkeleton" {
        e.set_sprite(if e.object == "oZombie" {
            "sZombieLeft"
        } else {
            "sVampireLeft"
        });
    }
}

pub(super) fn create_local(ctx: &mut crate::creation::EntityCreateContext<'_>, e: &mut Entity) {
    if e.object == "oZombie" && e.value("source_facing_pending") > 0.0 {
        e.facing = if ctx.rng.range(2) == 0 { -1 } else { 1 };
        // Create uses collision_point(x, y, oWater, 0, 0), including descendants.
        if ctx.world.tiles_point_active(e.pos, "oWater")
            || ctx.world.entities.iter().any(|water| {
                water.alive
                    && water.engine_active
                    && crate::catalog::is_a(&water.object, "oWater")
                    && water.mask_contains(e.pos, CollisionPrecision::BoundingBox)
            })
        {
            e.data.insert("swimming".into(), 1.0);
        }
        e.data.insert("source_facing_pending".into(), 0.0);
    }
}

pub(super) fn create(game: &mut Game, e: &mut Entity) {
    create_local(
        &mut crate::creation::EntityCreateContext {
            world: &game.world,
            rng: &mut game.rng,
        },
        e,
    );
    if e.value("source_facing_pending") > 0.0 {
        e.facing = match e.object.as_str() {
            "oSkeleton" => {
                if game.player.pos.x < e.pos.x + 8.0 {
                    -1
                } else {
                    1
                }
            }
            _ => 1,
        };
        e.data.insert("source_facing_pending".into(), 0.0);
    }
}

pub fn tick(game: &mut Game, e: &mut Entity) {
    match e.object.as_str() {
        "oSkeleton" => skeleton(game, e),
        "oZombie" => zombie(game, e),
        "oVampire" => vampire(game, e),
        _ => {}
    }
}

pub(super) fn alarm(_game: &mut Game, e: &mut Entity, index: u8) {
    // oVampire Alarm 0 overrides the inherited event with one variable write.
    if e.object == "oVampire" && index == 0 {
        e.data.insert("whipped".into(), 0.0);
    }
}

/// The inherited enemy event pins held bodies to the player before Step.
/// Keep that position while running the Vampire's recovery/death state machine.
#[cfg(test)]
pub(super) fn tick_held(game: &mut Game, e: &mut Entity) {
    let position = e.pos;
    e.vel = Vec2::default();
    tick(game, e);
    if e.held {
        e.pos = position;
    }
}

fn random_facing(game: &mut Game) -> i8 {
    if game.rng.range(2) == 0 { -1 } else { 1 }
}
// Source collision helpers probe boundary lines, not the translated full body.
fn side(world: &World, e: &Entity, direction: i8, distance: f64) -> bool {
    let r = bounds(e);
    let x = if direction < 0 {
        r.x - distance
    } else {
        r.x + r.w + distance - 1.0
    };
    super::solid_mask(world, Rect::new(x, r.y, 1.0, r.h), e.id)
}
fn bottom(world: &World, e: &Entity) -> bool {
    let r = bounds(e);
    super::solid_mask(world, Rect::new(r.x, r.y + r.h, r.w, 1.0), e.id)
}
fn top(world: &World, e: &Entity) -> bool {
    let r = bounds(e);
    super::solid_mask(world, Rect::new(r.x, r.y - 1.0, r.w, 1.0), e.id)
}
fn solid_at(world: &World, e: &Entity, x: f64, y: f64) -> bool {
    world.solid_point_except(Vec2::new(x, y), e.id)
}
fn source_move(world: &World, e: &mut Entity, gravity: f64) {
    // moveTo only moves; unlike move_body it does not zero vertical velocity.
    // Step applies gravity afterward, and scrCheckCollisions needs the impact speed.
    let y_velocity = e.vel.y;
    move_body(world, e, 0.0);
    let gravity = if gravity > 0.0 && !(e.object == "oVampire" && e.state == 98) {
        super::inherited::gravity(e, gravity)
    } else {
        gravity
    };
    e.vel.y = (y_velocity + gravity).min(10.0);
}
fn water(world: &World, p: Vec2) -> bool {
    world
        .tile((p.x / 16.0).floor() as i32, (p.y / 16.0).floor() as i32)
        .is_some_and(|t| matches!(t.object.as_str(), "oWater" | "oWaterSwim" | "oWaterTop"))
        || world.entities.iter().any(|e| {
            e.alive
                && matches!(e.object.as_str(), "oWater" | "oWaterSwim" | "oWaterTop")
                && e.bounds().contains(p)
        })
}
fn distance_to_player(game: &Game, e: &Entity) -> f64 {
    // Zombie's literal target is oPlayer1, not the wider Character family.
    // Native queries read cached integer envelopes and retain the million
    // sentinel when no active target exists, even for a destroyed receiver.
    let receiver = e.collision_mask().point_bounds();
    let mut distance = 1_000_000.0_f64;
    if game.player.engine_active {
        distance = distance.min(crate::world_events::mask_distance(
            receiver,
            game.player.collision_mask().point_bounds(),
        ));
    }
    for target in &game.world.entities {
        if target.id != e.id
            && target.alive
            && target.engine_active
            && crate::catalog::is_a(&target.object, "oPlayer1")
        {
            distance = distance.min(crate::world_events::mask_distance(
                receiver,
                target.collision_mask().point_bounds(),
            ));
        }
    }
    distance
}
fn distance_to_point(e: &Entity, point: Vec2) -> f64 {
    // Canonical55B380 reads the receiver's cached integer envelope. The
    // point arguments stay real-valued; this intrinsic has no family/alive
    // gate, no bbox refresh, and no distance_to_object million-distance cap.
    let r = e.collision_mask().point_bounds();
    let dx = (r.x - point.x).max(point.x - (r.x + r.w - 1.0)).max(0.0);
    let dy = (r.y - point.y).max(point.y - (r.y + r.h - 1.0)).max(0.0);
    dx.hypot(dy)
}
fn player_available(game: &Game) -> bool {
    super::player_alive(game)
        && game.player.state != PlayerState::Dead
        && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
}
fn debris(game: &mut Game, e: &mut Entity) {
    if e.value("undead_debris") != 0.0 {
        return;
    }
    e.data.insert("undead_debris".into(), 1.0);
    let p = Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0);
    for _ in 0..3 {
        crate::world_events::spawn_detritus(game, "oBone", p);
    }
    let id = game.spawn("oSkull", p);
    let y_velocity = -f64::from(1 + game.rng.range(3));
    let velocity = Vec2::new(
        f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
        y_velocity,
    );
    if let Some(skull) = game.world.entity_mut(id) {
        skull.vel = velocity;
    }
}
fn create_blood(game: &mut Game, e: &Entity) {
    super::create_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 1);
}
fn skeleton(game: &mut Game, e: &mut Entity) {
    let swimming = game
        .world
        .tile(
            ((e.pos.x + 8.0) / 16.0).floor() as i32,
            ((e.pos.y + 8.0) / 16.0).floor() as i32,
        )
        .is_some_and(|t| t.object == "oWaterSwim")
        || game.world.entities.iter().any(|water| {
            water.alive
                && water.object == "oWaterSwim"
                && water
                    .bounds()
                    .contains(Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0))
        });
    source_move(&game.world, e, if swimming { 0.2 } else { 0.6 });
    if solid_at(&game.world, e, e.pos.x + 8.0, e.pos.y + 8.0) {
        e.health = 0;
    }
    if e.health < 1 {
        debris(game, e);
        super::record_death(game, e);
        e.alive = false;
        // Own instance_destroy finishes Enemy Destroy before this Step resumes.
        super::destroyed(game, e);
    }
    if bottom(&game.world, e) && e.state != 98 {
        e.vel.y = 0.0;
    }
    if e.state == 0 {
        e.timer = e.timer.saturating_sub(1);
        if e.timer == 0 {
            e.state = 1;
        }
    } else if e.state == 1 {
        let left = side(&game.world, e, -1, 1.0);
        let right = side(&game.world, e, 1, 1.0);
        if !(side(&game.world, e, -1, 4.0) && side(&game.world, e, 1, 4.0)) && (left || right) {
            e.facing *= -1;
        }
        e.vel.x = f64::from(e.facing);
    }
    if super::body_overlaps_solid(&game.world, e) {
        e.pos.y -= 2.0;
    }
    if e.state != 98 {
        e.set_sprite(if e.state == 1 {
            "sSkeletonWalkLeft"
        } else {
            "sSkeletonLeft"
        });
    }
}
fn zombie(game: &mut Game, e: &mut Entity) {
    source_move(&game.world, e, 0.2);
    if solid_at(&game.world, e, e.pos.x + 8.0, e.pos.y + 8.0) {
        e.health = -999;
    }
    if e.health < 1 {
        create_blood(game, e);
        debris(game, e);
        super::record_death(game, e);
        e.alive = false;
        // Own instance_destroy finishes Enemy Destroy before this Step resumes.
        super::destroyed(game, e);
    }
    if side(&game.world, e, 1, 1.0) {
        e.vel.x = -1.0;
    }
    if side(&game.world, e, -1, 1.0) {
        e.vel.x = 1.0;
    }
    let grounded = bottom(&game.world, e);
    let distance = distance_to_player(game, e);
    match e.state {
        0 => {
            e.vel.x = 0.0;
            if e.timer > 0 {
                e.timer -= 1;
            } else if distance < 64.0 {
                e.state = 1;
            }
            if distance < 48.0 {
                e.state = 1;
            }
            if game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0 {
                e.state = 0;
            }
            if e.state == 1 {
                game.sound("sndZombie", e.pos);
            }
        }
        2 if grounded => {
            e.state = 0;
            e.vel = Vec2::default();
            e.timer = 40 + game.rng.range(61);
        }
        1 => {
            if grounded {
                let big = game.rng.chance(1, 4);
                e.vel.y = -f64::from(if big {
                    2 + game.rng.range(3)
                } else {
                    1 + game.rng.range(2)
                });
                e.facing = if game.player.pos.x < e.pos.x { -1 } else { 1 };
                e.vel.x = f64::from(e.facing) * if big { 3.0 } else { 1.0 };
            } else {
                e.state = 2;
            }
        }
        2 | 4 => {}
        _ => {
            e.state = 0;
            e.vel.x = 0.0;
        }
    }
    if top(&game.world, e) {
        e.vel.y = 1.0;
    }
    e.set_sprite(if grounded {
        "sZombieLeft"
    } else {
        "sZombieJumpL"
    });
}
fn vampire(game: &mut Game, e: &mut Entity) {
    let gravity = if e.held || matches!(e.state, 6 | 7) {
        0.0
    } else if e.state == 98 {
        0.6
    } else {
        0.2
    };
    source_move(&game.world, e, gravity);
    let grounded = bottom(&game.world, e);
    let ceiling = top(&game.world, e);
    let left = side(&game.world, e, -1, 1.0);
    let right = side(&game.world, e, 1, 1.0);
    let crushed = solid_at(
        &game.world,
        e,
        e.pos.x + 8.0,
        e.pos.y + if e.state >= 98 { 12.0 } else { 8.0 },
    );
    if crushed && (e.state >= 98 || !e.held) {
        game.spawn("oSmokePuff", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        game.sound("sndCavemanDie", e.pos);
        e.alive = false;
        // Own instance_destroy finishes Enemy Destroy before this Step resumes.
        super::destroyed(game, e);
    }
    if grounded && e.state != 98 {
        e.vel.y = 0.0;
    }
    if e.state < 98 && e.health < 1 {
        e.state = 99;
    }
    let burning = e.value("burning");
    if burning > 0.0 {
        if game.rng.chance(1, 5) {
            let p = Vec2::new(
                e.pos.x + f64::from(4 + game.rng.range(9)),
                e.pos.y + f64::from(4 + game.rng.range(9)),
            );
            game.spawn("oBurn", p);
        }
        e.data.insert("burning".into(), burning - 1.0);
    }
    let target = game.player.pos;
    let distance = distance_to_point(e, Vec2::new(target.x - 8.0, target.y - 8.0));
    match e.state {
        0 => {
            e.data.insert("bounced".into(), 0.0);
            if grounded
                && (solid_at(&game.world, e, e.pos.x - 1.0, e.pos.y)
                    || solid_at(&game.world, e, e.pos.x + 16.0, e.pos.y))
            {
                e.vel = Vec2::new(f64::from(e.facing), -6.0);
                e.timer = e.timer.saturating_sub(10);
            }
            if e.vel.y < 0.0 && ceiling {
                e.vel.y = 0.0;
            }
            if grounded {
                e.timer = e.timer.saturating_sub(1);
            }
            if e.timer == 0 {
                e.facing = random_facing(game);
                e.state = 1;
            }
            if distance < 96.0 {
                if target.x < e.pos.x + 8.0 {
                    e.facing = -1;
                } else if target.x > e.pos.x + 8.0 {
                    e.facing = 1;
                }
                e.state = 2;
            }
        }
        1 => {
            if left || right {
                e.facing *= -1;
            }
            if grounded {
                let stop = game.rng.chance(1, 100);
                let ahead_x = e.pos.x + if e.facing < 0 { -1.0 } else { 16.0 };
                if stop || !solid_at(&game.world, e, ahead_x, e.pos.y + 16.0) {
                    e.state = 0;
                    e.timer = 20 + game.rng.range(31);
                    e.vel.x = 0.0;
                    if !stop {
                        e.vel.y = 0.0;
                    }
                } else {
                    e.vel.x = f64::from(e.facing) * 1.5;
                }
            }
            if distance < 96.0 {
                e.state = 2;
            }
        }
        2 => {
            e.set_image_speed(1.0);
            if side(&game.world, e, e.facing, 4.0) {
                if ceiling {
                    e.facing *= -1;
                } else {
                    e.state = 5;
                }
            } else if !solid_at(
                &game.world,
                e,
                e.pos.x + f64::from(e.facing) * 8.0,
                e.pos.y + 16.0,
            ) {
                e.state = 5;
            }
            if (target.x - e.pos.x).abs() < 32.0
                && target.y < e.pos.y + 8.0
                && !water(&game.world, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0))
            {
                e.state = 7;
            }
            e.vel.x = f64::from(e.facing) * 4.0;
        }
        4 => {
            if (e.facing < 0 && left) || (e.facing > 0 && right) {
                e.facing *= -1;
                e.vel.x = -e.vel.x;
            } else if grounded {
                e.state = 0;
                e.vel = Vec2::default();
                e.timer = 40 + game.rng.range(61);
            } else if !water(&game.world, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0))
                && (game.rng.chance(1, 100)
                    || water(&game.world, Vec2::new(e.pos.x + 8.0, e.pos.y + 24.0)))
            {
                e.state = 7;
            }
        }
        5 => {
            if grounded {
                e.vel.y = -f64::from(3 + game.rng.range(4));
            } else {
                e.state = 4;
            }
        }
        6 => {
            e.vel = Vec2::default();
            if player_available(game)
                && ((distance < 90.0 && target.y > e.pos.y + 16.0)
                    || !solid_at(&game.world, e, e.pos.x + 8.0, e.pos.y - 1.0))
            {
                e.state = 7;
                game.sound("sndBat", e.pos);
            }
        }
        7 => {
            e.vel = Vec2::default();
            let at_ceiling = solid_at(&game.world, e, e.pos.x + 8.0, e.pos.y - 1.0);
            if player_available(game) {
                if distance < 160.0 {
                    let delta = Vec2::new(target.x - (e.pos.x + 8.0), target.y - (e.pos.y + 8.0));
                    let length = delta.x.hypot(delta.y);
                    if length > 0.0 {
                        e.vel = Vec2::new(delta.x / length * 2.0, delta.y / length * 2.0);
                    }
                    if (right && target.x > e.pos.x + 8.0) || (left && target.x < e.pos.x + 8.0) {
                        e.vel = Vec2::new(0.0, if target.y < e.pos.y + 8.0 { -2.0 } else { 2.0 });
                    }
                    if ((ceiling && target.y < e.pos.y + 8.0)
                        || (grounded && target.y > e.pos.y + 8.0))
                        && (target.x - e.pos.x).abs() > 8.0
                    {
                        e.vel = Vec2::new(if target.x < e.pos.x + 8.0 { -2.0 } else { 2.0 }, 0.0);
                    }
                    if water(&game.world, Vec2::new(e.pos.x + 8.0, e.pos.y + 16.0)) && e.vel.y > 0.0
                    {
                        e.vel = Vec2::new(0.0, -2.0);
                    }
                    if water(&game.world, Vec2::new(e.pos.x, e.pos.y + 12.0)) && target.y >= e.pos.y
                    {
                        e.vel = Vec2::default();
                    }
                } else if at_ceiling {
                    e.state = 6;
                } else {
                    e.vel.y = -2.0;
                }
                e.facing = if target.x < e.pos.x + 8.0 { -1 } else { 1 };
                if grounded || water(&game.world, e.pos) {
                    e.state = 0;
                }
            } else if at_ceiling {
                e.state = 6;
            } else {
                e.vel.y = -1.0;
            }
        }
        98 => {
            if !grounded {
                e.set_sprite(if e.vel.x == 0.0 && e.health > 0 {
                    "sVampireStunL"
                } else if e.value("bounced") > 0.0 {
                    if e.vel.y < 0.0 {
                        "sVampireBounceL"
                    } else {
                        "sVampireFallL"
                    }
                } else if e.vel.x.abs() > 0.0 {
                    "sVampireDieLL"
                } else {
                    "sVampireDieLR"
                });
            }
            if grounded && e.value("bounced") == 0.0 {
                e.data.insert("bounced".into(), 1.0);
                create_blood(game, e);
            }
            if grounded || e.held {
                if e.timer > 0 {
                    e.timer -= 1;
                } else if e.health > 0 {
                    e.state = 0;
                    e.held = false;
                    if game.player.held == Some(e.id) {
                        game.player.held = None;
                        game.player.pickup_item_type.clear();
                    }
                }
            }
        }
        99 => {
            debris(game, e);
            super::record_death(game, e);
            game.sound("sndCavemanDie", e.pos);
            let id = game.spawn("oCapePickup", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
            if let Some(cape) = game.world.entity_mut(id) {
                cape.data.insert("cost".into(), 0.0);
                cape.data.insert("forSale".into(), 0.0);
            }
            e.alive = false;
            // The DEAD branch still reaches scrCheckCollisions and its art tail.
            super::destroyed(game, e);
        }
        // THROW is declared in the source but no Step branch assigns behavior.
        _ => {}
    }
    if e.state >= 98 {
        // scrCheckCollisions.gml: shorter corpse body, wall reflection, floor bounce
        // and strong floor friction occur before the stationary DEAD transition.
        e.data.insert("collision_left".into(), 2.0);
        e.data.insert("collision_top".into(), 6.0);
        e.data.insert("collision_right".into(), 14.0);
        e.data.insert("collision_bottom".into(), 16.0);
        if left && !right {
            e.pos.x += 1.0;
        } else if right {
            e.pos.x -= 1.0;
        }
        if left || right {
            e.vel.x *= -0.5;
        }
        if ceiling && !grounded {
            e.pos.y += 1.0;
        } else if grounded {
            if e.vel.y > 1.0 {
                e.vel.y *= -0.5;
            } else if e.vel.y.abs() < 1.0 {
                e.vel.y = 0.0;
            }
            if e.vel.x.abs() < 0.1 {
                e.vel.x = 0.0;
            } else if e.vel.x != 0.0 {
                e.vel.x *= 0.3;
            }
        }
    }
    if e.state >= 98 && e.vel.x == 0.0 && e.vel.y == 0.0 && e.health < 1 {
        e.state = 99;
    }
    // Source applies friction even in bat form.
    if e.vel.x > 0.0 {
        e.vel.x -= 0.1;
    }
    if e.vel.x < 0.0 {
        e.vel.x += 0.1;
    }
    if e.vel.x.abs() < 0.5 {
        e.vel.x = 0.0;
    }
    if e.state == 6 {
        e.set_sprite("sVampireBatHang");
    } else if e.state < 98 && e.state != 3 {
        e.set_sprite(match e.state {
            7 => "sVampireBatLeft",
            4 => "sVampireJumpL",
            _ if e.vel.x.abs() > 0.0 => "sVampireRunL",
            _ => "sVampireLeft",
        });
    }
    if e.held {
        e.set_sprite(if e.health > 0 {
            "sVampireHeldL"
        } else {
            "sVampireDHeldL"
        });
    }
}

/// An already selected Collision pair executes independently of the AI Step
/// guard. The source Vampire/oBlood handler has no health cap or state guard.
pub(super) fn collision(
    game: &mut Game,
    e: &mut Entity,
    other: crate::dispatcher::Instance,
) -> bool {
    if e.object != "oVampire" {
        return false;
    }
    if other == crate::dispatcher::Instance::Player {
        vampire_character_collision(game, e);
        return true;
    }
    if let crate::dispatcher::Instance::Entity(id) = other {
        let weapon = game.world.entity(id).filter(|weapon| {
            (crate::catalog::is_a(&weapon.object, "oWhip")
                || crate::catalog::is_a(&weapon.object, "oWhipPre"))
                && crate::dispatcher::collision_owner(&e.object, &weapon.object) == Some("oVampire")
        });
        if let Some(weapon) = weapon.cloned() {
            vampire_melee_collision(game, e, &weapon);
            // The selected own event overrides the parent even when ignored.
            return true;
        }
    }
    if other.object(game) != "oBlood" {
        return false;
    }
    let crate::dispatcher::Instance::Entity(id) = other else {
        return true;
    };
    if e.alive {
        if let Some(blood) = game.world.entity_mut(id) {
            e.health += 1;
            blood.alive = false;
        }
    }
    true
}

/// Vampire's front and rear events have the same literal body. The Machete
/// type allows another hit while stunned; puncture and invincibility are absent.
fn vampire_melee_collision(game: &mut Game, e: &mut Entity, weapon: &Entity) {
    let machete = weapon.text.get("type").map_or_else(
        || {
            crate::catalog::is_a(&weapon.object, "oSlash")
                || crate::catalog::is_a(&weapon.object, "oMachetePre")
        },
        |kind| kind == "Machete",
    );
    if e.state >= 98 && !machete {
        return;
    }
    let default_damage = if crate::catalog::is_a(&weapon.object, "oSlash")
        || crate::catalog::is_a(&weapon.object, "oMachetePre")
        || crate::catalog::is_a(&weapon.object, "oMattockHit")
        || crate::catalog::is_a(&weapon.object, "oMattockPre")
    {
        2.0
    } else {
        1.0
    };
    e.health -= weapon.data.get("damage").copied().unwrap_or(default_damage) as i32;
    // This script call is unconditional: source bloodless/cap guards belong to
    // scrCreateBlood, and neither event reads or decrements bloodLeft.
    super::create_blood(game, e, super::sprite_blood_center(e), 1);
    super::set_counts_as_kill(e, true);
    e.state = 98;
    e.timer = e.value("stun_time") as u32;
    e.vel = Vec2::new(
        if weapon.pos.x < e.pos.x + 8.0 {
            2.0
        } else {
            -2.0
        },
        -3.0,
    );
    e.set_image_speed(0.5);
    game.sound("sndHit", e.pos);
}

fn vampire_character_collision(game: &mut Game, e: &mut Entity) {
    if (game.player.pos.x - (e.pos.x + 8.0)).abs() > 8.0 {
        return;
    }
    let jumped_on = game.player.state != PlayerState::Dead
        && game.state != crate::RunState::Dead
        && matches!(
            game.player.state,
            PlayerState::Jumping | PlayerState::Falling
        )
        && game.player.pos.y < e.pos.y + 5.0
        && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0;
    if jumped_on {
        if e.state < 98 {
            game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
            let fall = game.player.data.get("fall_timer").copied().unwrap_or(0.0);
            let multiplier = if game.player.has("SpikeShoes") { 3 } else { 1 };
            e.health -= multiplier * ((fall / 16.0).floor() as i32 + 1);
            if multiplier == 3 && e.value("bloodless") == 0.0 {
                game.spawn(
                    "oBlood",
                    Vec2::new(game.player.pos.x, game.player.pos.y + 8.0),
                );
            }
            game.player.data.insert("fall_timer".into(), 0.0);
            super::set_counts_as_kill(e, true);
            e.state = 98;
            e.timer = e.value("stun_time") as u32;
            e.vel.y = -6.0;
            e.vel.x += if game.player.pos.x < e.pos.x + 8.0 {
                1.0
            } else {
                -1.0
            };
            e.set_image_speed(0.5);
            game.sound("sndHit", e.pos);
        }
    } else if game.player.invincible == 0 && e.state < 98 {
        game.player.data.insert("blink".into(), 30.0);
        game.player.invincible = 30;
        if game.player.pos.y < e.pos.y {
            game.player.vel.y = -6.0;
        }
        game.player.vel.x = if game.player.pos.x < e.pos.x {
            -6.0
        } else {
            6.0
        };
        game.spawn("oBlood", game.player.pos);
        if game.player.health > 0 {
            game.player.health -= 1;
            game.events.push(crate::GameEvent::Damage { amount: 1 });
        }
        // This particular source override has no enemyDeaths[7] producer.
        game.sound("sndHurt", game.player.pos);
    }
}

/// Compatibility for direct family fixtures; the engine dispatches individual
/// pairs through `collision` rather than scanning neighbors during Step.
#[cfg(test)]
pub(super) fn absorb_blood(game: &mut Game, e: &mut Entity) {
    if !e.alive || e.object != "oVampire" {
        return;
    }
    // Collision events use sprite bounds, separately from the shorter move body.
    let body = e.sprite_bounds();
    let blood_ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|blood| blood.alive && blood.object == "oBlood" && body.intersects(blood.bounds()))
        .map(|blood| blood.id)
        .collect();
    for id in blood_ids {
        collision(game, e, crate::dispatcher::Instance::Entity(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RulesPreset;
    #[test]
    fn zombie_local_create_consumes_one_source_draw_and_runtime_preserves_it() {
        for seed in [1, 7, 42, 129] {
            let mut game = Game::shell(seed, RulesPreset::Classic);
            let mut zombie = Entity::new(1, "oZombie", Vec2::new(100.0, 40.0));
            init(&mut zombie);
            let mut source_rng = game.rng.clone();
            let facing = if source_rng.range(2) == 0 { -1 } else { 1 };
            for _ in 0..2 {
                create_local(
                    &mut crate::creation::EntityCreateContext {
                        world: &game.world,
                        rng: &mut game.rng,
                    },
                    &mut zombie,
                );
            }
            assert_eq!(zombie.facing, facing);
            assert_eq!(zombie.value("source_facing_pending"), 0.0);
            assert_eq!(game.rng.clone().next_u64(), source_rng.clone().next_u64());
            zombie.facing = -facing;
            create(&mut game, &mut zombie);
            create(&mut game, &mut zombie);
            assert_eq!(zombie.facing, -facing);
            assert_eq!(game.rng.next_u64(), source_rng.next_u64());
        }
    }

    #[test]
    fn zombie_local_create_preserves_skeleton_and_vampire_runtime_create() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos.x = 40.0;
        let source_rng = game.rng.clone();
        for (object, facing) in [("oSkeleton", -1), ("oVampire", 1)] {
            let mut actor = Entity::new(1, object, Vec2::new(80.0, 80.0));
            init(&mut actor);
            create_local(
                &mut crate::creation::EntityCreateContext {
                    world: &game.world,
                    rng: &mut game.rng,
                },
                &mut actor,
            );
            assert_eq!(actor.value("source_facing_pending"), 1.0, "{object}");
            assert_eq!(actor.facing, 1, "local Zombie hook must ignore {object}");
            create(&mut game, &mut actor);
            assert_eq!(actor.facing, facing, "{object}");
            assert_eq!(actor.value("source_facing_pending"), 0.0, "{object}");
        }
        assert_eq!(game.rng.next_u64(), source_rng.clone().next_u64());
    }

    #[test]
    fn zombie_local_create_latches_origin_water_family_once() {
        for object in ["oWaterSwim", "oLava"] {
            for dynamic in [false, true] {
                for (origin, wet) in [
                    (Vec2::new(48.0, 48.0), true),
                    (Vec2::new(47.0, 48.0), false),
                    (Vec2::new(64.0, 48.0), false),
                ] {
                    let mut game = Game::shell(42, RulesPreset::Classic);
                    if dynamic {
                        game.world.spawn(object, Vec2::new(48.0, 48.0));
                    } else {
                        game.world.set_tile(3, 3, object);
                    }
                    let mut zombie = Entity::new(999, "oZombie", origin);
                    init(&mut zombie);
                    let mut expected_rng = game.rng.clone();
                    expected_rng.range(2);
                    create_local(
                        &mut crate::creation::EntityCreateContext {
                            world: &game.world,
                            rng: &mut game.rng,
                        },
                        &mut zombie,
                    );
                    assert_eq!(
                        zombie.value("swimming"),
                        f64::from(wet),
                        "{object}, dynamic={dynamic}, origin={origin:?}"
                    );
                    // Runtime bootstrap must preserve caller writes after local Create.
                    zombie.data.insert("swimming".into(), f64::from(!wet));
                    create(&mut game, &mut zombie);
                    assert_eq!(zombie.value("swimming"), f64::from(!wet));
                    assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
                }
            }
        }
    }

    #[test]
    fn source_health_and_skeleton_delay() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let mut e = Entity::new(99, "oSkeleton", Vec2::new(40.0, 40.0));
        init(&mut e);
        for _ in 0..19 {
            tick(&mut game, &mut e);
            assert_eq!(e.state, 0);
        }
        tick(&mut game, &mut e);
        assert_eq!(e.state, 1);
        let mut vampire = Entity::new(100, "oVampire", Vec2::new(40.0, 40.0));
        init(&mut vampire);
        assert_eq!(vampire.health, 6);
    }
    #[test]
    fn source_create_facing_is_synchronous_and_does_not_repeat_in_step() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(40.0, 80.0);
        let initial_rng = game.rng.clone();
        let skeleton = game.spawn("oSkeleton", Vec2::new(80.0, 80.0));
        assert_eq!(game.world.entity(skeleton).unwrap().facing, -1);
        assert_eq!(game.rng.clone().next_u64(), initial_rng.clone().next_u64());

        let mut source_rng = game.rng.clone();
        let zombie_facing = if source_rng.range(2) == 0 { -1 } else { 1 };
        let zombie = game.spawn("oZombie", Vec2::new(120.0, 80.0));
        assert_eq!(game.world.entity(zombie).unwrap().facing, zombie_facing);
        assert_eq!(game.rng.clone().next_u64(), source_rng.clone().next_u64());
        game.player.pos = Vec2::new(250.0, 160.0);

        game.step(&crate::InputFrame::default());

        assert_eq!(game.world.entity(skeleton).unwrap().facing, -1);
        assert_eq!(game.world.entity(zombie).unwrap().facing, zombie_facing);
        assert_eq!(game.rng.clone().next_u64(), source_rng.next_u64());
    }
    #[test]
    fn actual_vampire_stomp_restores_credit_and_stunned_control_does_nothing() {
        for stunned in [false, true] {
            let mut game = Game::shell(42, RulesPreset::Classic);
            // oPlayer1.active gates its controller, not automatic collisions.
            // This isolates the child collision from unrelated air movement.
            game.player.data.insert("active".into(), 0.0);
            game.player.pos = Vec2::new(88.0, 80.0);
            game.player.state = PlayerState::Falling;
            game.player.vel.y = 3.0;
            let id = game.spawn("oVampire", Vec2::new(80.0, 80.0));
            let vampire = game.world.entity_mut(id).unwrap();
            vampire.state = if stunned { 98 } else { 3 };
            vampire.vel = Vec2::default();
            super::super::set_counts_as_kill(vampire, false);

            game.step(&crate::InputFrame::default());

            let vampire = game.world.entity(id).unwrap();
            assert_eq!(game.player.health, 4);
            assert_eq!(game.player.invincible, 0);
            assert_eq!(vampire.state, 98);
            if stunned {
                assert_eq!(vampire.health, 6);
                assert_eq!(game.player.vel.y, 3.0);
                assert!(!super::super::counts_as_kill(vampire));
            } else {
                assert_eq!(vampire.health, 5);
                assert_eq!(vampire.vel.y, -6.0);
                assert_eq!(vampire.timer, 60);
                assert_eq!(game.player.vel.y, -6.6);
                assert!(super::super::counts_as_kill(vampire));
            }
        }
    }
    #[test]
    fn actual_vampire_contact_has_blink_and_knockback_without_stun_or_death_stat() {
        for health in [4, 1] {
            let mut game = Game::shell(42, RulesPreset::Classic);
            game.player.data.insert("active".into(), 0.0);
            game.player.pos = Vec2::new(88.0, 78.0);
            game.player.state = PlayerState::Standing;
            game.player.health = health;
            let id = game.spawn("oVampire", Vec2::new(80.0, 80.0));
            let vampire = game.world.entity_mut(id).unwrap();
            vampire.state = 3;
            vampire.vel = Vec2::default();

            game.step(&crate::InputFrame::default());

            assert_eq!(game.player.health, health - 1);
            assert_eq!(game.player.invincible, 30);
            assert_eq!(game.player.data["blink"], 30.0);
            assert_eq!(game.player.stun, 0);
            assert_eq!(game.player.vel, Vec2::new(6.0, -6.0));
            assert_eq!(game.lifetime_stats.enemy_deaths[7], 0);
            assert_eq!(game.world.entity(id).unwrap().health, 6);
        }
    }
    #[test]
    fn family_step_preserves_valid_animation_index_for_engine_endframe() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 160.0);
        for object in ["oSkeleton", "oZombie", "oVampire"] {
            let mut enemy = Entity::new(99, object, Vec2::new(80.0, 80.0));
            crate::enemies::init_entity(&mut enemy);
            enemy.animation = 0.25;
            enemy.timer = 20;
            enemy.vel = Vec2::default();
            tick(&mut game, &mut enemy);
            assert_eq!(enemy.animation, 0.25, "{object} advanced during Step");
            assert_eq!(
                enemy.image_speed(),
                if object == "oZombie" { 0.4 } else { 0.5 }
            );
            assert_eq!(
                enemy.value("stun_time"),
                if object == "oVampire" { 60.0 } else { 200.0 }
            );
        }
    }
    #[test]
    fn actual_death_steps_credit_source_counters_and_lifetime_once() {
        for (object, category, index) in [
            ("oSkeleton", "skeletons", 5),
            ("oZombie", "zombies", 6),
            ("oVampire", "vampires", 7),
        ] {
            for (custom, counts_as_kill) in [(false, true), (true, true), (false, false)] {
                let mut game = Game::shell(42, RulesPreset::Classic);
                game.player.pos = Vec2::new(200.0, 160.0);
                if custom {
                    game.flags.insert("custom".into());
                }
                let id = game.world.spawn(object, Vec2::new(80.0, 80.0));
                let enemy = game.world.entity_mut(id).unwrap();
                enemy.health = 0;
                enemy.state = 99;
                enemy.vel = Vec2::default();
                enemy
                    .data
                    .insert("counts_as_kill".into(), f64::from(counts_as_kill));

                game.step(&crate::InputFrame::default());

                let expected = f64::from(counts_as_kill);
                assert!(game.world.entity(id).is_none(), "{object}");
                assert_eq!(
                    game.data.get(category).copied().unwrap_or(0.0),
                    expected,
                    "{object}"
                );
                assert_eq!(
                    game.data.get("kills").copied().unwrap_or(0.0),
                    expected,
                    "{object}"
                );
                assert_eq!(
                    game.lifetime_stats.enemy_kills[index],
                    u64::from(counts_as_kill && !custom),
                    "{object}"
                );

                game.step(&crate::InputFrame::default());

                assert_eq!(
                    game.data.get(category).copied().unwrap_or(0.0),
                    expected,
                    "{object} repeated"
                );
                assert_eq!(
                    game.data.get("kills").copied().unwrap_or(0.0),
                    expected,
                    "{object} repeated"
                );
                assert_eq!(
                    game.lifetime_stats.enemy_kills[index],
                    u64::from(counts_as_kill && !custom),
                    "{object} repeated"
                );
            }
        }
    }
    #[test]
    fn vampire_credit_and_cape_wait_for_explicit_dead_branch() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 160.0);
        let id = game.world.spawn("oVampire", Vec2::new(80.0, 80.0));
        let vampire = game.world.entity_mut(id).unwrap();
        vampire.health = 0;
        vampire.state = 98;
        vampire.vel = Vec2::new(2.0, -1.0);
        vampire.data.insert("counts_as_kill".into(), 1.0);

        game.step(&crate::InputFrame::default());

        assert_eq!(game.world.entity(id).unwrap().state, 98);
        assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
        assert_eq!(game.lifetime_stats.enemy_kills[7], 0);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.object == "oCapePickup")
        );

        game.world.entity_mut(id).unwrap().state = 99;
        game.step(&crate::InputFrame::default());

        assert!(game.world.entity(id).is_none());
        assert_eq!(game.data["kills"], 1.0);
        assert_eq!(game.data["vampires"], 1.0);
        assert_eq!(game.lifetime_stats.enemy_kills[7], 1);
        let capes: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oCapePickup")
            .collect();
        assert_eq!(capes.len(), 1);
        assert_eq!(capes[0].value("cost"), 0.0);
        assert_eq!(capes[0].value("forSale"), 0.0);
    }
    #[test]
    fn vampire_crush_destroys_without_dead_branch_credit_or_cape() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 160.0);
        game.world.set_tile(5, 5, "oBlock");
        let id = game.world.spawn("oVampire", Vec2::new(80.0, 80.0));
        game.world.entity_mut(id).unwrap().vel = Vec2::default();

        game.step(&crate::InputFrame::default());

        assert!(game.world.entity(id).is_none());
        assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
        assert_eq!(game.lifetime_stats.enemy_kills[7], 0);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.object == "oCapePickup")
        );
    }
    #[test]
    fn zombie_does_not_chase_swimming_player() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(48.0, 48.0);
        game.player.data.insert("swimming".into(), 1.0);
        let mut e = Entity::new(99, "oZombie", Vec2::new(40.0, 40.0));
        init(&mut e);
        tick(&mut game, &mut e);
        assert_eq!(e.state, 0);
        game.player.data.insert("swimming".into(), 0.0);
        tick(&mut game, &mut e);
        assert_eq!(e.state, 1);
    }
    #[test]
    fn vampire_hangs_and_reactivates() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(48.0, 80.0);
        let mut e = Entity::new(99, "oVampire", Vec2::new(40.0, 40.0));
        init(&mut e);
        e.state = 6;
        tick(&mut game, &mut e);
        assert_eq!(e.state, 7);
        assert_eq!(e.vel, Vec2::default());
        tick(&mut game, &mut e);
        assert!(e.vel.y > 0.0);
    }
    #[test]
    fn skeleton_death_spawns_source_debris_once() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let mut e = Entity::new(99, "oSkeleton", Vec2::new(40.0, 40.0));
        init(&mut e);
        e.health = 0;
        tick(&mut game, &mut e);
        assert!(!e.alive);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBone")
                .count(),
            3
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oSkull")
                .count(),
            1
        );
    }
    #[test]
    fn death_bone_creates_precede_skull_launch_draws_in_actual_steps() {
        // oSkeleton/oVampire Step creates three oBones before assigning
        // Skull yVel/xVel. Each Bone Create consumes random(4), random(4),
        // random(2), then rand(1,6), synchronously inside instance_create.
        for object in ["oSkeleton", "oVampire"] {
            let mut game = Game::shell(42, RulesPreset::Classic);
            game.player.pos = Vec2::new(200.0, 160.0);
            let id = game.world.spawn(object, Vec2::new(80.0, 80.0));
            let enemy = game.world.entity_mut(id).unwrap();
            enemy.health = 0;
            enemy.state = 99;
            enemy.vel = Vec2::default();

            let mut source_rng = game.rng.clone();
            let bone_launches: Vec<_> = (0..3)
                .map(|_| {
                    let velocity = Vec2::new(
                        source_rng.unit() * 4.0 - source_rng.unit() * 4.0,
                        -1.0 - source_rng.unit() * 2.0,
                    );
                    let gravity = f64::from(1 + source_rng.range(6)) * 0.1;
                    (velocity, gravity)
                })
                .collect();
            let skull_y = -f64::from(1 + source_rng.range(3));
            let skull_velocity = Vec2::new(
                f64::from(source_rng.range(4)) - f64::from(source_rng.range(4)),
                skull_y,
            );

            game.step(&crate::InputFrame::default());

            assert!(game.world.entity(id).is_none(), "{object}");
            // Source resource order visits Skeleton (652), Bone (654),
            // Skull (663), then Vampire (915). Skeleton's newborn debris
            // receives Step this frame; Vampire's waits one frame.
            let newborn_step_ran = object == "oSkeleton";
            let bones: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBone")
                .collect();
            assert_eq!(bones.len(), 3, "{object}");
            for (bone, (velocity, gravity)) in bones.iter().zip(&bone_launches) {
                assert_eq!(bone.value("world_effect_initialized"), 1.0);
                assert_eq!(
                    bone.value("life"),
                    if newborn_step_ran { 59.0 } else { 60.0 }
                );
                assert_eq!(bone.value("gravity"), *gravity);
                assert_eq!(
                    bone.vel,
                    Vec2::new(
                        velocity.x,
                        velocity.y + if newborn_step_ran { *gravity } else { 0.0 }
                    )
                );
            }
            let skull = game
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oSkull")
                .unwrap();
            assert_eq!(
                skull.vel,
                Vec2::new(
                    skull_velocity.x,
                    skull_velocity.y + if newborn_step_ran { 0.6 } else { 0.0 }
                ),
                "{object}"
            );
            assert_eq!(
                skull.pos,
                if newborn_step_ran {
                    Vec2::new(88.0 + skull_velocity.x, 88.0 + skull_velocity.y)
                } else {
                    Vec2::new(88.0, 88.0)
                }
            );
            assert_eq!(game.rng.clone().next_u64(), source_rng.clone().next_u64());

            game.step(&crate::InputFrame::default());

            // Already initialized bones only run Step on the next frame;
            // neither their Create RNG nor the death producer runs again.
            assert_eq!(game.rng.clone().next_u64(), source_rng.next_u64());
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBone")
                    .count(),
                3,
                "{object}"
            );
            assert!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBone")
                    .all(|e| e.value("life") == if newborn_step_ran { 58.0 } else { 59.0 })
            );
        }
    }
    #[test]
    fn vampire_absorbs_external_blood_without_health_cap() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let mut e = Entity::new(99, "oVampire", Vec2::new(40.0, 40.0));
        init(&mut e);
        e.health = 12;
        e.state = 6;
        let id = game.world.spawn("oBlood", Vec2::new(48.0, 48.0));
        tick(&mut game, &mut e);
        absorb_blood(&mut game, &mut e);
        assert_eq!(e.health, 13);
        assert!(
            !game
                .world
                .entities
                .iter()
                .find(|e| e.id == id)
                .unwrap()
                .alive
        );
    }
    #[test]
    fn bloodless_vampire_does_not_create_own_healing_blood() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let mut e = Entity::new(99, "oVampire", Vec2::new(40.0, 40.0));
        init(&mut e);
        create_blood(&mut game, &e);
        assert!(game.world.entities.is_empty());
    }
    #[test]
    fn integrated_dead_vampire_bounces_before_disintegrating() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        for x in 0..12 {
            game.world.set_tile(x, 4, "oBlock");
        }
        let id = game.world.spawn("oVampire", Vec2::new(40.0, 48.0));
        let e = game.world.entity_mut(id).unwrap();
        e.health = 0;
        e.state = 98;
        e.vel = Vec2::new(2.0, 8.0);
        e.timer = 60;
        crate::enemies::tick(&mut game);
        let e = game.world.entity(id).unwrap();
        assert_eq!(e.state, 98);
        assert!((e.vel.y + 4.3).abs() < 1e-9);
        assert!((e.vel.x - 0.5).abs() < 1e-9);
        assert_eq!(e.value("collision_top"), 6.0);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.object == "oCapePickup")
        );
        for time in 1..150 {
            game.tick = time;
            game.level_tick = time;
            crate::enemies::tick(&mut game);
            if game.world.entity(id).is_none() {
                break;
            }
        }
        assert!(game.world.entity(id).is_none());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oCapePickup")
                .count(),
            1
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBone")
                .count(),
            3
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oSkull")
                .count(),
            1
        );
    }
    #[test]
    fn integrated_zombie_jumps_from_dynamic_solid_and_recovers() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(80.0, 48.0);
        game.world.spawn("oPushBlock", Vec2::new(32.0, 64.0));
        let id = game.world.spawn("oZombie", Vec2::new(36.0, 48.0));
        let e = game.world.entity_mut(id).unwrap();
        e.state = 1;
        crate::enemies::tick(&mut game);
        let e = game.world.entity(id).unwrap();
        assert_eq!(e.state, 1);
        assert!(e.vel.y < 0.0);
        assert!(matches!(e.vel.x, 1.0 | 3.0));
        game.tick = 1;
        game.level_tick = 1;
        crate::enemies::tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().state, 2);
        let e = game.world.entity_mut(id).unwrap();
        e.pos = Vec2::new(36.0, 48.0);
        e.vel = Vec2::default();
        game.tick = 2;
        game.level_tick = 2;
        crate::enemies::tick(&mut game);
        let e = game.world.entity(id).unwrap();
        assert_eq!(e.state, 0);
        assert!((40..=100).contains(&e.timer));
        assert_eq!(e.vel, Vec2::default());
    }
    #[test]
    fn integrated_skeleton_reverses_at_dynamic_wall() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        for x in 0..12 {
            game.world.set_tile(x, 4, "oBlock");
        }
        game.world.spawn("oPushBlock", Vec2::new(64.0, 48.0));
        let id = game.world.spawn("oSkeleton", Vec2::new(50.0, 48.0));
        let e = game.world.entity_mut(id).unwrap();
        e.state = 1;
        e.facing = 1;
        e.data.insert("source_facing_pending".into(), 0.0);
        crate::enemies::tick(&mut game);
        let e = game.world.entity(id).unwrap();
        assert_eq!(e.facing, -1);
        assert_eq!(e.vel.x, -1.0);
    }
    #[test]
    fn integrated_zombie_activation_uses_sprite_bbox_distance() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(120.0, 48.0);
        let id = game.world.spawn("oZombie", Vec2::new(40.0, 40.0));
        crate::enemies::tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().state, 1);
    }
    #[test]
    fn stunned_vampire_blood_collision_uses_sprite_not_shortened_move_bounds() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        let mut vampire = Entity::new(99, "oVampire", Vec2::new(40.0, 40.0));
        crate::enemies::init_entity(&mut vampire);
        vampire.state = 98;
        vampire.sprite = "sVampireStunL".into();
        vampire.data.insert("collision_top".into(), 6.0);
        let blood = game.world.spawn("oBlood", Vec2::new(48.0, 40.0));
        tick(&mut game, &mut vampire);
        absorb_blood(&mut game, &mut vampire);
        assert_eq!(vampire.health, 7);
        assert!(game.world.entity(blood).is_none());
    }
    #[test]
    fn integrated_vampire_attack_uses_bbox_distance_to_offset_player_point() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(150.0, 48.0);
        let id = game.world.spawn("oVampire", Vec2::new(40.0, 40.0));
        let e = game.world.entity_mut(id).unwrap();
        e.timer = 20;
        e.vel = Vec2::default();
        crate::enemies::tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().state, 2);
    }
    #[test]
    fn held_vampire_recovers_on_tick_after_counter_reaches_zero() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let mut e = Entity::new(99, "oVampire", Vec2::new(40.0, 40.0));
        crate::enemies::init_entity(&mut e);
        e.state = 98;
        e.timer = 1;
        e.held = true;
        e.vel = Vec2::new(8.0, 8.0);
        game.player.held = Some(e.id);
        tick_held(&mut game, &mut e);
        assert_eq!(e.state, 98);
        assert_eq!(e.timer, 0);
        assert!(e.held);
        assert_eq!(e.pos, Vec2::new(40.0, 40.0));
        assert_eq!(e.sprite, "sVampireHeldL");
        tick_held(&mut game, &mut e);
        assert_eq!(e.state, 0);
        assert!(!e.held);
        assert_eq!(game.player.held, None);
    }
    #[test]
    fn collision_blood_can_revive_stunned_zero_health_vampire() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let mut e = Entity::new(99, "oVampire", Vec2::new(40.0, 40.0));
        crate::enemies::init_entity(&mut e);
        e.state = 98;
        e.health = 0;
        e.held = true;
        game.world.spawn("oBlood", Vec2::new(48.0, 48.0));
        absorb_blood(&mut game, &mut e);
        assert_eq!(e.health, 1);
    }
}

#[cfg(test)]
mod vampire_own_destroy_tail_tests {
    use super::*;
    use crate::{Rng, RngAlgorithm, RulesPreset};

    #[derive(Clone, Copy)]
    enum OracleMode {
        Legacy,
        Canonical,
    }
    impl OracleMode {
        fn game(self, seed: u64, rules: RulesPreset) -> Game {
            match self {
                Self::Legacy => Game::legacy_shell(seed, rules),
                Self::Canonical => Game::shell(u32::try_from(seed).unwrap(), rules),
            }
        }
    }

    // Legacy preserves the frozen raw-adapter oracle. Canonical never calls a
    // production draw while constructing the independent expected sequence.
    enum DirectSourceTape {
        Legacy(Rng),
        Canonical(u32),
    }
    impl DirectSourceTape {
        fn at(game: &Game) -> Self {
            match game.rng.algorithm() {
                RngAlgorithm::LegacySplitMix64V1 => Self::Legacy(game.rng.clone()),
                RngAlgorithm::Gm8Lcg32V1 => {
                    Self::Canonical(u32::try_from(game.rng.state_bits()).unwrap())
                }
            }
        }
        fn next_u64(&mut self) -> u64 {
            match self {
                Self::Legacy(t) => t.next_u64(),
                Self::Canonical(state) => {
                    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
                    u64::from(*state)
                }
            }
        }
        fn rand(&mut self, width: u64) -> u64 {
            let legacy = matches!(self, Self::Legacy(_));
            let raw = self.next_u64();
            if legacy {
                raw % width
            } else {
                (raw * width) >> 32
            }
        }
    }

    fn own_dead_destroy_finishes_before_retired_collision_and_held_art_tail_case(mode: OracleMode) {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = mode.game(42, rules);
            game.world = World::new(30, 20, crate::Area::Mines, 1);
            game.player.pos = Vec2::new(240.0, 152.0);
            game.player.previous_pos = game.player.pos;
            game.world.set_tile(3, 3, "oBlock");
            for x in 0..30 {
                game.world.set_tile(x, 4, "oBlock");
            }
            let mut actor = Entity::new(99, "oVampire", Vec2::new(34.0, 48.0));
            crate::enemies::init_entity(&mut actor);
            // Initial event-scoped DEAD/held contact intentionally exposes the
            // retired local actor that Game.step removes before public lookup.
            actor.state = 99;
            actor.health = 0;
            actor.held = true;
            actor.vel = Vec2::new(2.0, 0.0);
            game.player.held = Some(actor.id);
            game.player.pickup_item_type = "Vampire".into();
            assert!(bottom(&game.world, &actor));
            assert!(side(&game.world, &actor, 1, 1.0));
            assert!(!side(&game.world, &actor, -1, 1.0));
            vampire(&mut game, &mut actor);
            assert!(!actor.alive);
            assert_eq!(
                actor.value("enemy_destroyed"),
                1.0,
                "SYNCHRONOUS_ENEMY_DESTROY"
            );
            assert_eq!(game.player.held, None);
            assert_eq!(
                game.player.pickup_item_type, "Vampire",
                "SOURCE_WRITES_PICKUPITEM_NOT_TYPE"
            );
            assert!(actor.held, "DESTROY_RETAINS_RECEIVER_HELD_FIELD");
            assert_eq!(
                actor.pos,
                Vec2::new(33.0, 48.0),
                "POST_DEAD_SCRCHECKCOLLISIONS"
            );
            assert_eq!(actor.vel, Vec2::default(), "POST_DEAD_FRICTION");
            assert_eq!(actor.sprite, "sVampireDHeldL", "POST_DEAD_HELD_ART");
            let bones: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oBone")
                .collect();
            assert_eq!(bones.len(), 3);
            assert!(
                bones.iter().all(|e| e.pos == Vec2::new(42.0, 56.0)),
                "PRE_TAIL_OTHER_SELF_ORIGIN"
            );
            assert_eq!(game.data.get("kills"), Some(&1.0));
            // No second public event or artificial re-entry is dispatched on
            // this retired actor; callback idempotence is the shared contract.
        }
    }

    fn own_stunned_crush_keeps_burning_timer_draws_and_recovery_body_case(mode: OracleMode) {
        let mut saw_burn = false;
        let mut saw_skip = false;
        for seed in 0..16 {
            for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
                let mut game = mode.game(seed, rules);
                game.world = World::new(30, 20, crate::Area::Mines, 1);
                game.player.pos = Vec2::new(240.0, 152.0);
                for y in 5..=6 {
                    for x in 6..=7 {
                        game.world.set_tile(x, y, "oBlock");
                    }
                }
                let mut actor = Entity::new(99, "oVampire", Vec2::new(96.0, 80.0));
                crate::enemies::init_entity(&mut actor);
                actor.state = 98;
                actor.timer = 60;
                actor.vel = Vec2::default();
                actor.data.insert("burning".into(), 2.0);
                let mut expected = DirectSourceTape::at(&game);
                // Literal phase oracle: preserve Legacy raw adapter; canonical
                // uses independent LCG/multiply-high math. Burn Create no RNG.
                let burn = expected.rand(5) == 0;
                let position = if burn {
                    Some(Vec2::new(
                        96.0 + (4 + expected.rand(9)) as f64,
                        80.0 + (4 + expected.rand(9)) as f64,
                    ))
                } else {
                    None
                };
                vampire(&mut game, &mut actor);
                assert!(!actor.alive);
                assert_eq!(actor.value("burning"), 1.0, "OWN_BURNING_AFTER_CRUSH");
                assert_eq!(actor.timer, 59, "STUNNED_BODY_AFTER_CRUSH");
                assert_eq!(actor.value("bounced"), 1.0);
                let burns: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBurn")
                    .collect();
                assert_eq!(burns.len(), usize::from(burn));
                if let Some(position) = position {
                    assert_eq!(burns[0].pos, position);
                }
                assert_eq!(
                    game.rng.next_u64(),
                    expected.next_u64(),
                    "OWN_BURN_SOURCE_DRAW_ORDER"
                );
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|e| e.object == "oCapePickup")
                );
                saw_burn |= burn;
                saw_skip |= !burn;
            }
        }
        assert!(saw_burn && saw_skip, "BOTH_SOURCE_BURN_BRANCHES_REQUIRED");
    }

    // Each mode is a distinct test, not a second iteration hidden by old failure.
    #[test]
    fn legacy_own_dead_destroy_finishes_before_retired_collision_and_held_art_tail() {
        own_dead_destroy_finishes_before_retired_collision_and_held_art_tail_case(
            OracleMode::Legacy,
        );
    }

    #[test]
    fn canonical_own_dead_destroy_finishes_before_retired_collision_and_held_art_tail() {
        own_dead_destroy_finishes_before_retired_collision_and_held_art_tail_case(
            OracleMode::Canonical,
        );
    }

    #[test]
    fn legacy_own_stunned_crush_keeps_burning_timer_draws_and_recovery_body() {
        own_stunned_crush_keeps_burning_timer_draws_and_recovery_body_case(OracleMode::Legacy);
    }

    #[test]
    fn canonical_own_stunned_crush_keeps_burning_timer_draws_and_recovery_body() {
        own_stunned_crush_keeps_burning_timer_draws_and_recovery_body_case(OracleMode::Canonical);
    }
}
#[cfg(test)]
mod undead_own_destroy_tail_tests {
    use super::*;
    use crate::{Area, RulesPreset};

    // Controlled detached receiver: Game.step removes the tombstone before a
    // public observer can inspect these otherwise unobservable Skeleton fields.
    fn skeleton_idle_tail_case(canonical: bool) {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = if canonical {
                Game::shell(42, rules)
            } else {
                Game::legacy_shell(42, rules)
            };
            game.world = World::new(30, 20, Area::Mines, 1);
            for x in 0..30 {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.player.pos = Vec2::new(240.0, 152.0);
            let id = game.spawn("oSkeleton", Vec2::new(96.0, 144.0));
            let mut actor = game.world.entity(id).unwrap().clone();
            actor.health = 0;
            actor.state = 0;
            actor.timer = 1;
            actor.vel = Vec2::new(0.0, 2.0);
            actor.held = true;
            game.player.held = Some(id);
            let bomb = game.spawn("oBomb", Vec2::new(32.0, 32.0));
            game.world
                .entity_mut(bomb)
                .unwrap()
                .data
                .insert("attached_id".into(), id as f64);
            actor.data.insert("bomb_id".into(), bomb as f64);
            skeleton(&mut game, &mut actor);
            assert!(!actor.alive);
            assert_eq!(
                actor.value("enemy_destroyed"),
                1.0,
                "SYNCHRONOUS_ENEMY_DESTROY"
            );
            assert_eq!(game.player.held, None);
            assert!(actor.held, "SOURCE_DESTROY_LEAVES_RECEIVER_HELD_FLAG");
            assert_eq!(actor.value("bomb_id"), bomb as f64);
            assert_eq!(game.world.entity(bomb).unwrap().value("attached_id"), 0.0);
            assert_eq!(
                (actor.state, actor.timer),
                (1, 0),
                "POST_DESTROY_IDLE_TIMER_AND_WALK_TRANSITION"
            );
            assert_eq!(actor.vel.y, 0.0, "POST_DESTROY_GROUNDED_VELOCITY");
            assert_eq!(
                actor.sprite, "sSkeletonWalkLeft",
                "POST_DESTROY_SPRITE_EPILOGUE"
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oBone")
                    .count(),
                3
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oSkull")
                    .count(),
                1
            );
            assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
            assert_eq!(game.data.get("skeletons"), Some(&1.0));
            assert_eq!(game.data.get("kills"), Some(&1.0));
        }
    }
    #[test]
    fn legacy_controlled_skeleton_destroy_preserves_idle_and_sprite_tail() {
        skeleton_idle_tail_case(false)
    }
    #[test]
    fn canonical_controlled_skeleton_destroy_preserves_idle_and_sprite_tail() {
        skeleton_idle_tail_case(true)
    }
}

#[cfg(test)]
#[path = "zombie_cached_distance_tests.rs"]
mod zombie_cached_distance_tests;

#[cfg(test)]
#[path = "vampire_cached_point_tests.rs"]
mod vampire_cached_point_tests;
