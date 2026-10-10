//! Cave enemy state machines translated from the Classic 1.1 event sources.
//!
//! Positions retain original GameMaker instance origins. Collision uses the
//! shared source sprite rectangles rather than per-pixel masks. Moving sight
//! instances alert all nearby cavemen and hawkmen, as the source collision event
//! does. New sight instances first move on the next dispatcher tick; GameMaker's
//! within-frame instance scheduling has not been reproduced. Animation-end transitions use the
//! source sprite frame count and speed. Combat, death drops, and activation
//! distance are owned by the parent enemy dispatcher.
//!
//! Source references: `Objects/Enemies/oCaveman.events/Step.xml` and
//! `oEnemySight.events/Collision with oCharacter.xml` define moving sight and
//! group alerts; `oBat.events/Step.xml` defines water steering; spider
//! `Alarm 0.xml`/`Alarm 1.xml` and `Step.xml` define recovery and drowning.

use super::{bounds, center, move_body, steer_toward};
use crate::{Entity, Game, PlayerState, Rect, Vec2, World};

pub fn matches(object: &str) -> bool {
    matches!(
        object,
        "oSnake"
            | "oBat"
            | "oSpider"
            | "oSpiderHang"
            | "oGiantSpider"
            | "oGiantSpiderHang"
            | "oCaveman"
            | "oCavemanWorship"
            | "oEnemySight"
    )
}

pub fn init(e: &mut Entity) {
    e.state = 0;
    e.facing = 1;
    e.health = if e.object == "oCaveman" { 3 } else { 1 };
    e.vel = Vec2::default();
    e.data.insert("animation_speed".into(), 0.4);
    let (width, height) = match e.object.as_str() {
        "oBat" => (12.0, 12.0),
        "oSpider" => (14.0, 11.0),
        "oGiantSpider" => (28.0, 16.0),
        _ => (12.0, 16.0),
    };
    e.data.insert("size_width".into(), width);
    e.data.insert("size_height".into(), height);
    e.data.insert(
        "hit_cooldown".into(),
        if e.object == "oGiantSpider" {
            10.0
        } else {
            0.0
        },
    );
    match e.object.as_str() {
        "oSnake" => {
            e.vel.x = 2.5;
            e.sprite = "sSnakeWalkL".into();
        }
        "oBat" => {
            e.sprite = "sBatHang".into();
            e.data.insert("animation_speed".into(), 0.5);
        }
        "oSpider" => e.sprite = "sSpider".into(),
        "oSpiderHang" => {
            e.sprite = "sSpiderHang".into();
            set_collision(e, [4.0, 0.0, 12.0, 12.0]);
        }
        "oGiantSpiderHang" => {
            e.sprite = "sGiantSpiderHang".into();
            e.health = 10;
            e.data.insert("contact_damage".into(), 2.0);
            set_collision(e, [0.0, 0.0, 32.0, 16.0]);
        }
        "oGiantSpider" => {
            e.data.insert("contact_damage".into(), 2.0);
            e.sprite = "sGiantSpider".into();
            e.data.insert("animation_speed".into(), 0.8);
            // Assigned using the game's RNG on the first active tick.
            e.data.insert("squirt_timer".into(), -1.0);
        }
        "oCaveman" => {
            e.vel.x = 2.5;
            e.sprite = "sCavemanLeft".into();
            e.data.insert("animation_speed".into(), 0.5);
            e.data.insert("stun_time".into(), 200.0);
        }
        "oCavemanWorship" => {
            // This source object has no enemy parent or gameplay events.
            e.sprite = "sCavemanWorshipR".into();
            e.data.insert("animation_speed".into(), 0.25);
            e.data.insert("enemy".into(), 0.0);
        }
        "oEnemySight" => {
            e.sprite = "sSight".into();
            e.data.insert("visible".into(), 0.0);
            e.data.insert("animation_speed".into(), 0.0);
            e.data.insert("enemy".into(), 0.0);
        }
        _ => {}
    }
}

pub fn tick(game: &mut Game, e: &mut Entity) {
    match e.object.as_str() {
        "oSnake" => snake(game, e),
        "oBat" => bat(game, e),
        "oSpider" => spider(game, e, false),
        "oGiantSpider" => spider(game, e, true),
        "oSpiderHang" => hanging_spider(game, e, false),
        "oGiantSpiderHang" => hanging_spider(game, e, true),
        "oCaveman" => caveman(game, e),
        "oEnemySight" => enemy_sight(game, e),
        _ => {}
    }
}

/// GameMaker alarms keep counting even when a Step event's viewport guard
/// prevents AI/movement. The dispatcher calls this only for inactive actors.
pub(super) fn tick_inactive_alarm(game: &mut Game, e: &mut Entity) {
    if matches!(e.object.as_str(), "oSpider" | "oGiantSpider") {
        spider_alarms(game, e);
    }
    if e.alive
        && e.object == "oSpider"
        && e.state != 4
        && in_water(game, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0))
    {
        e.state = 4;
        e.timer = 30;
        e.sprite = "sSpiderDrowning".into();
        e.vel = Vec2::new(0.0, 0.2);
    }
}

fn spider_alarms(game: &mut Game, e: &mut Entity) -> bool {
    if e.state == 2 {
        e.timer = e.timer.saturating_sub(1);
        if e.timer == 0 {
            e.state = 1;
            if bottom(&game.world, e) {
                launch_spider(game, e, 2, true);
            }
        }
    } else if e.object == "oSpider" && e.state == 4 {
        e.timer = e.timer.saturating_sub(1);
        if e.timer == 0 {
            e.alive = false;
        }
    }
    e.alive
}

fn crushed(game: &Game, e: &Entity, offset: Vec2) -> bool {
    game.world
        .solid_point_except(Vec2::new(e.pos.x + offset.x, e.pos.y + offset.y), e.id)
}

fn distance_to_player(game: &Game, e: &Entity) -> f64 {
    // distance_to_object uses sprite bounds, not the platform-engine body.
    let a = e.sprite_bounds();
    let b = game.player.sprite_bounds();
    let x = (a.x - (b.x + b.w - 1.0))
        .max(b.x - (a.x + a.w - 1.0))
        .max(0.0);
    let y = (a.y - (b.y + b.h - 1.0))
        .max(b.y - (a.y + a.h - 1.0))
        .max(0.0);
    x.hypot(y)
}

fn enemy_sight(game: &mut Game, e: &mut Entity) {
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    if game.world.collides_except(e.sprite_bounds(), e.id) {
        e.alive = false;
        return;
    }
    if e.sprite_bounds().intersects(game.player.sprite_bounds()) {
        let alerted: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|other| {
                other.alive
                    && matches!(other.object.as_str(), "oCaveman" | "oHawkman")
                    && other.state < 98
                    && distance_to_player(game, other) < 100.0
            })
            .map(|other| other.id)
            .collect();
        for id in alerted {
            let pos = if let Some(other) = game.world.entity_mut(id) {
                other.state = 2;
                other.pos
            } else {
                continue;
            };
            game.sound("sndAlert", pos);
        }
    }
}

fn set_collision(e: &mut Entity, values: [f64; 4]) {
    for (key, value) in [
        "collision_left",
        "collision_top",
        "collision_right",
        "collision_bottom",
    ]
    .into_iter()
    .zip(values)
    {
        e.data.insert(key.into(), value);
    }
}

fn hanging_spider(game: &mut Game, e: &mut Entity, giant: bool) {
    // Original Create places these immediately. The Game-free initializer
    // defers world spawning until the first active Step instead.
    if giant && e.value("hanging_webs_created") == 0.0 {
        game.world.spawn("oWeb", Vec2::new(e.pos.x, e.pos.y + 16.0));
        game.world
            .spawn("oWeb", Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
        e.data.insert("hanging_webs_created".into(), 1.0);
    }
    let half = if giant { 16.0 } else { 8.0 };
    let crush_y = if giant { 8.0 } else { 4.0 };
    if game.world.solid_at(e.pos.x + half, e.pos.y + crush_y) {
        e.health = 0;
    }
    if e.health <= 0 {
        return;
    }
    let beneath = distance_to_player(game, e) < 90.0
        && game.player.pos.y > e.pos.y
        && (game.player.pos.x - (e.pos.x + half)).abs() < 8.0;
    if (giant && e.health < 10) || !game.world.solid_at(e.pos.x, e.pos.y - 16.0) || beneath {
        // Both hanging Step events create a fresh actor and copy only hp.
        // Reusing the ID would also incorrectly retain whip-hit bookkeeping.
        let id = game
            .world
            .spawn(if giant { "oGiantSpider" } else { "oSpider" }, e.pos);
        game.world.entity_mut(id).unwrap().health = e.health;
        e.alive = false;
    }
}

fn bottom(world: &World, e: &Entity) -> bool {
    let r = bounds(e);
    world.collides_except(Rect::new(r.x, r.y + r.h, r.w, 1.0), e.id)
}

fn top(world: &World, e: &Entity) -> bool {
    let r = bounds(e);
    world.collides_except(Rect::new(r.x, r.y - 1.0, r.w, 1.0), e.id)
}

fn side(world: &World, e: &Entity, direction: i8) -> bool {
    let r = bounds(e);
    world.collides_except(
        Rect::new(
            if direction < 0 { r.x - 1.0 } else { r.x + r.w },
            r.y,
            1.0,
            r.h,
        ),
        e.id,
    )
}

fn floor_ahead(world: &World, e: &Entity, direction: i8) -> bool {
    world.solid_at(
        e.pos.x + if direction < 0 { -1.0 } else { 16.0 },
        e.pos.y + 16.0,
    )
}

fn random_facing(game: &mut Game) -> i8 {
    if game.rng.range(2) == 0 { -1 } else { 1 }
}

fn snake(game: &mut Game, e: &mut Entity) {
    move_body(&game.world, e, 0.6);
    if crushed(game, e, Vec2::new(8.0, 8.0)) {
        e.health = 0;
    }
    if e.health <= 0 {
        return;
    }
    if e.state == 0 {
        if e.timer > 0 {
            e.timer -= 1;
        } else {
            e.facing = random_facing(game);
            e.state = 1;
        }
    } else if e.state == 1 {
        if side(&game.world, e, -1) || side(&game.world, e, 1) {
            e.facing *= -1;
        }
        if !floor_ahead(&game.world, e, e.facing) {
            e.facing *= -1;
        }
        let blocked_left =
            !floor_ahead(&game.world, e, -1) || game.world.solid_at(e.pos.x - 1.0, e.pos.y);
        let blocked_right =
            !floor_ahead(&game.world, e, 1) || game.world.solid_at(e.pos.x + 16.0, e.pos.y);
        e.vel.x = if blocked_left && blocked_right {
            0.0
        } else {
            f64::from(e.facing)
        };
        if game.rng.chance(1, 100) {
            e.state = 0;
            e.timer = 20 + game.rng.range(31);
            e.vel.x = 0.0;
        }
    }
    e.sprite = "sSnakeWalkL".into();
    if game.world.collides_except(e.bounds(), e.id) {
        e.pos.y -= 2.0;
    }
    e.data.insert(
        "animation_speed".into(),
        if e.vel.x == 0.0 { 0.2 } else { 0.4 },
    );
}

fn player_available(game: &Game) -> bool {
    super::player_alive(game)
        && game.player.state != PlayerState::Dead
        && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
}

fn bat(game: &mut Game, e: &mut Entity) {
    move_body(&game.world, e, 0.0);
    if crushed(game, e, Vec2::new(8.0, 8.0)) {
        e.health = -999;
    }
    if e.health <= 0 {
        return;
    }
    let body_center = center(e);
    let distance = body_center.distance(game.player.pos);
    let ceiling = game.world.solid_at(e.pos.x + 8.0, e.pos.y - 1.0);
    if e.state == 0 {
        e.sprite = "sBatHang".into();
        if player_available(game)
            && ((distance < 90.0 && game.player.pos.y > e.pos.y + 16.0) || !ceiling)
        {
            e.state = 1;
            game.sound("sndBat", e.pos);
        }
        return;
    }
    if player_available(game) && distance < 160.0 {
        let previous_velocity = e.vel;
        let target = game.player.pos;
        steer_toward(e, target, 1.0);
        let target_side = if target.x < body_center.x { -1 } else { 1 };
        if side(&game.world, e, target_side) {
            e.vel = Vec2::new(0.0, if target.y < body_center.y { -1.0 } else { 1.0 });
        }
        if ((top(&game.world, e) && target.y < body_center.y)
            || (bottom(&game.world, e) && target.y > body_center.y))
            && (target.x - e.pos.x).abs() > 8.0
        {
            e.vel = Vec2::new(f64::from(target_side), 0.0);
        }
        // The original avoids chasing players into water.
        if in_water(game, Vec2::new(e.pos.x + 8.0, e.pos.y + 16.0)) && e.vel.y > 0.0 {
            e.vel = Vec2::new(0.0, -1.0);
        }
        if in_water(game, Vec2::new(e.pos.x, e.pos.y + 12.0)) && target.y >= e.pos.y {
            e.vel = previous_velocity;
        }
    } else if ceiling {
        e.state = 0;
        e.vel = Vec2::default();
    } else {
        e.vel = Vec2::new(0.0, -1.0);
    }
    e.facing = if game.player.pos.x < body_center.x {
        -1
    } else {
        1
    };
    e.sprite = if e.facing < 0 {
        "sBatLeft"
    } else {
        "sBatRight"
    }
    .into();
}

fn in_water(game: &Game, pos: Vec2) -> bool {
    game.world.entities.iter().any(|other| {
        other.alive
            && crate::catalog::is_a(&other.object, "oWater")
            && other.sprite_bounds().contains(pos)
    }) || game
        .world
        .tile((pos.x / 16.0).floor() as i32, (pos.y / 16.0).floor() as i32)
        .is_some_and(|tile| crate::catalog::is_a(&tile.object, "oWater"))
}

fn launch_spider(game: &mut Game, e: &mut Entity, low: u32, alarm: bool) {
    e.vel.y = -f64::from(low + game.rng.range(4));
    // oSpider/Alarm 0 compares against x; its Step compares against x+8.
    // Both giant-spider events compare against x+16.
    let target_x = e.pos.x
        + if e.object == "oGiantSpider" {
            16.0
        } else if alarm {
            0.0
        } else {
            8.0
        };
    e.facing = if game.player.pos.x < target_x { -1 } else { 1 };
    e.vel.x = f64::from(e.facing) * 2.5;
}

fn spider(game: &mut Game, e: &mut Entity, giant: bool) {
    // Alarm 0 runs before Step/moveTo in Classic. Jumping here allows the
    // recovery-expiry frame to move upwards rather than standing still once.
    if !spider_alarms(game, e) {
        return;
    }
    let (_, grounded) = move_body(&game.world, e, if giant { 0.3 } else { 0.2 });
    if crushed(
        game,
        e,
        if giant {
            Vec2::new(16.0, 24.0)
        } else {
            Vec2::new(8.0, 8.0)
        },
    ) {
        e.health = 0;
    }
    if e.health <= 0 {
        return;
    }
    let grounded = grounded || bottom(&game.world, e);
    let ceiling = top(&game.world, e);
    if !giant && e.state != 4 && in_water(game, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0)) {
        e.state = 4;
        e.timer = 30;
        e.sprite = "sSpiderDrowning".into();
        e.vel = Vec2::new(0.0, 0.2);
        game.world
            .spawn("oSplash", Vec2::new(e.pos.x + 8.0, e.pos.y));
        game.sound("sndSplash", e.pos);
        return;
    }
    if e.state == 4 {
        return;
    }
    if side(&game.world, e, 1) {
        e.vel.x = 1.0;
    }
    if side(&game.world, e, -1) {
        e.vel.x = -1.0;
    }

    let mut squirt = e.value("squirt_timer");
    if giant {
        if squirt < 0.0 {
            squirt = f64::from(100 + game.rng.range(901));
            game.sound("sndGiantSpider", e.pos);
        }
        squirt = (squirt - 1.0).max(0.0);
        if ceiling && grounded && e.state != 3 {
            e.state = 3;
            e.vel.x = if game.player.pos.x < center(e).x {
                -1.0
            } else {
                1.0
            };
        }
    }

    match e.state {
        0 => {
            e.timer = 5 + game.rng.range(16);
            e.state = if giant && squirt == 0.0 { 5 } else { 2 };
            if giant {
                e.sprite = "sGiantSpider".into();
            }
            if e.state == 5 {
                e.animation = 0.0;
            }
        }
        1 if distance_to_player(game, e) < if giant { 120.0 } else { 90.0 } => {
            if giant {
                e.sprite = "sGiantSpiderJump".into();
            }
            if grounded {
                launch_spider(game, e, if giant { 3 } else { 2 }, false);
                if giant {
                    e.sprite = "sGiantSpider".into();
                    game.sound("sndSpiderJump", e.pos);
                }
                if game.rng.chance(1, 4) {
                    e.state = 0;
                    e.vel = Vec2::default();
                }
            }
        }
        2 => {
            if grounded {
                e.vel.x = 0.0;
            }
        }
        3 if giant => {
            e.sprite = "sGiantSpiderCrawl".into();
            if !ceiling || !grounded {
                e.state = 0;
            } else if side(&game.world, e, 1) {
                e.vel.x = -1.0;
            } else if side(&game.world, e, -1) {
                e.vel.x = 1.0;
            }
        }
        5 if giant => {
            e.sprite = "sGiantSpiderSquirt".into();
            if e.animation >= 5.0 && squirt == 0.0 {
                let id = game
                    .world
                    .spawn("oWebBall", Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
                let velocity = Vec2::new(
                    f64::from(1 + game.rng.range(3)) * f64::from(random_facing(game)),
                    -(game.rng.unit() * 3.0 + 1.0),
                );
                let life = 20 + game.rng.range(81);
                if let Some(web) = game.world.entity_mut(id) {
                    web.vel = velocity;
                    web.timer = life;
                    web.data.insert("life".into(), f64::from(life));
                }
                squirt = f64::from(100 + game.rng.range(901));
            }
            // Eight frames at the source's 0.4 image_speed.
            if e.animation >= 8.0 {
                e.state = 0;
                e.animation = 0.0;
                e.data.insert("animation_speed".into(), 0.4);
            }
        }
        _ => {
            e.state = 0;
        }
    }
    if ceiling {
        e.vel.y = 1.0;
    }
    if giant {
        e.data.insert("squirt_timer".into(), squirt);
    }
}

fn caveman(game: &mut Game, e: &mut Entity) {
    let impact_y = (e.vel.y
        + if e.held {
            0.0
        } else {
            super::inherited::gravity(e, 0.6)
        })
    .min(10.0);
    let (_, grounded) = move_body(&game.world, e, if e.held { 0.0 } else { 0.6 });
    let crush_y = if e.state >= 98 { 12.0 } else { 8.0 };
    if (!e.held || e.state >= 98) && crushed(game, e, Vec2::new(8.0, crush_y)) {
        e.alive = false;
        game.sound("sndCavemanDie", e.pos);
        return;
    }
    let grounded = grounded || bottom(&game.world, e);
    let hit_side = side(&game.world, e, -1) || side(&game.world, e, 1);
    if e.health < 1 && e.state < 98 {
        e.state = 99;
    }
    let casts_sight = e.state < 2;
    match e.state {
        0 => {
            if grounded && hit_side {
                e.vel.y = -6.0;
                e.vel.x = f64::from(e.facing);
                e.timer = e.timer.saturating_sub(10);
            }
            if grounded {
                e.timer = e.timer.saturating_sub(1);
            }
            if e.timer == 0 {
                e.facing = random_facing(game);
                e.state = 1;
            }
        }
        1 => {
            if hit_side {
                e.facing *= -1;
            }
            if !floor_ahead(&game.world, e, e.facing) {
                e.state = 0;
                e.timer = 20 + game.rng.range(31);
            }
            // Source assigns walking speed even after its ledge state change.
            e.vel.x = f64::from(e.facing) * 1.5;
            if game.rng.chance(1, 100) {
                e.state = 0;
                e.timer = 20 + game.rng.range(31);
                e.vel.x = 0.0;
            }
        }
        2 => {
            e.data.insert("animation_speed".into(), 1.0);
            if hit_side {
                e.facing *= -1;
            }
            e.vel.x = f64::from(e.facing) * 3.0;
        }
        98 => {
            e.sprite = if e.vel.x == 0.0 && e.health > 0 {
                "sCavemanStunL"
            } else if e.vel.y < 0.0 {
                "sCavemanBounceL"
            } else {
                "sCavemanFallL"
            }
            .into();
            if grounded || e.held {
                if e.timer > 0 {
                    e.timer -= 1;
                } else if e.health > 0 {
                    e.state = 0;
                    e.held = false;
                    if game.player.held == Some(e.id) {
                        game.player.held = None;
                    }
                }
            }
            if grounded && e.vel.x.abs() < 0.5 && e.health < 1 {
                e.state = 99;
            }
        }
        99 => {
            e.sprite = "sCavemanDeadL".into();
            if e.value("death_sound") == 0.0 {
                game.sound("sndCavemanDie", e.pos);
                e.data.insert("death_sound".into(), 1.0);
            }
            if e.vel.x.abs() > 0.0 || e.vel.y.abs() > 0.0 {
                e.state = 98;
            }
        }
        _ => {}
    }
    if casts_sight {
        let sight_timer = e.value("sight_timer");
        if sight_timer <= 0.0 {
            let id = game.world.spawn("oEnemySight", e.pos);
            if let Some(sight) = game.world.entity_mut(id) {
                sight.vel.x = f64::from(e.facing) * 10.0;
                sight.data.insert("owner".into(), e.id as f64);
            }
            e.data.insert("sight_timer".into(), 5.0);
        } else {
            e.data.insert("sight_timer".into(), sight_timer - 1.0);
        }
    }
    if e.state >= 98 {
        super::settle_body(&game.world, e, impact_y);
    }
    e.vel.x = (e.vel.x.abs() - 0.1).max(0.0) * e.vel.x.signum();
    if e.vel.x.abs() < 0.5 {
        e.vel.x = 0.0;
    }
    if e.state < 98 {
        e.sprite = if e.vel.x == 0.0 {
            "sCavemanLeft"
        } else {
            "sCavemanRunLeft"
        }
        .into();
    }
    if e.held {
        e.sprite = if e.health > 0 {
            "sCavemanHeldL"
        } else {
            "sCavemanDHeldL"
        }
        .into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RulesPreset;

    #[test]
    fn cave_family_and_source_hitpoints() {
        for name in ["oSnake", "oBat", "oSpider", "oGiantSpider", "oCaveman"] {
            assert!(matches(name));
            let mut enemy = Entity::new(1, name, Vec2::new(40.0, 40.0));
            init(&mut enemy);
            assert_eq!(enemy.health, if name == "oCaveman" { 3 } else { 1 });
        }
        assert!(!matches("oYeti"));
    }

    #[test]
    fn bat_wakes_below_and_has_unit_flight_speed() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.world.set_tile(2, 1, "oBlock");
        game.player.pos = Vec2::new(40.0, 70.0);
        let mut bat = Entity::new(1, "oBat", Vec2::new(40.0, 39.0));
        init(&mut bat);
        tick(&mut game, &mut bat);
        assert_eq!(bat.state, 1);
        tick(&mut game, &mut bat);
        assert!((bat.vel.x.hypot(bat.vel.y) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn moving_sight_alerts_nearby_cavemen_and_hawkmen_as_a_group() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(88.0, 72.0);
        let ray = game.world.spawn("oEnemySight", Vec2::new(64.0, 64.0));
        game.world.entity_mut(ray).unwrap().vel.x = 10.0;
        let caveman = game.world.spawn("oCaveman", Vec2::new(64.0, 100.0));
        let hawkman = game.world.spawn("oHawkman", Vec2::new(120.0, 100.0));
        let stunned = game.world.spawn("oCaveman", Vec2::new(90.0, 100.0));
        game.world.entity_mut(stunned).unwrap().state = 98;
        game.world.entity_mut(stunned).unwrap().timer = 200;
        super::super::tick(&mut game);
        assert_eq!(game.world.entity(caveman).unwrap().state, 2);
        assert_eq!(game.world.entity(hawkman).unwrap().state, 2);
        assert_eq!(game.world.entity(stunned).unwrap().state, 98);
        assert_eq!(game.world.entity(ray).unwrap().pos.x, 74.0);
        assert_eq!(game.player.health, 4);
    }

    #[test]
    fn moving_sight_dies_at_solids_without_alerting() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(88.0, 72.0);
        game.world.set_tile(5, 4, "oBlock");
        let ray = game.world.spawn("oEnemySight", Vec2::new(64.0, 64.0));
        game.world.entity_mut(ray).unwrap().vel.x = 10.0;
        let caveman = game.world.spawn("oCaveman", Vec2::new(64.0, 100.0));
        super::super::tick(&mut game);
        assert!(game.world.entity(ray).is_none());
        assert_ne!(game.world.entity(caveman).unwrap().state, 2);
        assert_eq!(game.player.health, 4);
    }

    #[test]
    fn caveman_sight_counter_spawns_every_six_ticks() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(250.0, 160.0);
        for x in 0..20 {
            game.world.set_tile(x, 5, "oBlock");
        }
        let id = game.world.spawn("oCaveman", Vec2::new(64.0, 64.0));
        let enemy = game.world.entity_mut(id).unwrap();
        enemy.timer = 100;
        enemy.vel = Vec2::default();
        for tick in 0..13 {
            game.tick = tick;
            super::super::tick(&mut game);
        }
        let rays: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oEnemySight")
            .collect();
        assert_eq!(rays.len(), 3);
        assert!(rays.iter().all(|e| e.vel.x == 10.0));
    }

    #[test]
    fn patrol_ledge_probes_use_original_instance_origin() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let enemy = Entity::new(1, "oSnake", Vec2::new(32.0, 48.0));
        game.world.set_tile(3, 4, "oBlock");
        assert!(floor_ahead(&game.world, &enemy, 1));
        assert!(!floor_ahead(&game.world, &enemy, -1));
    }

    #[test]
    fn caveman_patrol_uses_source_post_friction_fractional_speed() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(250.0, 200.0);
        for x in 0..20 {
            game.world.set_tile(x, 5, "oBlock");
        }
        let id = game.world.spawn("oCaveman", Vec2::new(64.0, 64.0));
        let enemy = game.world.entity_mut(id).unwrap();
        enemy.state = 1;
        enemy.vel = Vec2::new(1.4, 0.0);
        let mut positions = Vec::new();
        for tick in 1..=3 {
            game.tick = tick;
            game.level_tick = tick - 1;
            super::super::tick(&mut game);
            positions.push(game.world.entity(id).unwrap().pos.x);
        }
        // Step chooses 1.5, then subtracts 0.1. moveTo quantizes the
        // 0.4 fraction using global time mod round(1/0.4), i.e. mod 3.
        assert_eq!(positions, vec![65.0, 66.0, 68.0]);
        assert_eq!(game.world.entity(id).unwrap().vel.x, 1.4);
    }

    #[test]
    fn giant_web_spawn_uses_source_offsets_from_original_origin() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let mut enemy = Entity::new(100, "oGiantSpider", Vec2::new(64.0, 64.0));
        init(&mut enemy);
        enemy.state = 5;
        enemy.animation = 5.0;
        enemy.data.insert("squirt_timer".into(), 0.0);
        tick(&mut game, &mut enemy);
        let web = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oWebBall")
            .unwrap();
        assert_eq!(web.pos, Vec2::new(80.0, 80.0));
        assert!((1.0..=3.0).contains(&web.vel.x.abs()));
        assert!((-4.0..=-1.0).contains(&web.vel.y));
        assert!((20..=100).contains(&web.timer));
        assert_eq!(web.value("life"), f64::from(web.timer));
    }

    #[test]
    fn hanging_spider_drops_on_player_alignment_and_lost_ceiling() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(4, 3, "oBlock");
        game.player.pos = Vec2::new(120.0, 110.0);
        let mut enemy = Entity::new(100, "oSpiderHang", Vec2::new(64.0, 64.0));
        super::super::init_entity(&mut enemy);
        tick(&mut game, &mut enemy);
        assert_eq!(enemy.object, "oSpiderHang");
        assert_eq!(enemy.vel, Vec2::default());
        game.player.pos = Vec2::new(72.0, 100.0);
        tick(&mut game, &mut enemy);
        assert!(!enemy.alive);
        let replacement = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oSpider")
            .unwrap();
        assert_ne!(replacement.id, 100);
        assert_eq!(replacement.pos, Vec2::new(64.0, 64.0));
        assert_eq!(replacement.previous_pos, replacement.pos);
        assert_eq!(replacement.bounds(), Rect::new(65.0, 69.0, 14.0, 11.0));

        let mut unsupported = Entity::new(101, "oSpiderHang", Vec2::new(96.0, 64.0));
        super::super::init_entity(&mut unsupported);
        game.player.pos = Vec2::new(200.0, 200.0);
        tick(&mut game, &mut unsupported);
        assert!(!unsupported.alive);
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.object == "oSpider" && e.pos == Vec2::new(96.0, 64.0))
        );
    }

    #[test]
    fn giant_hanger_starts_with_ten_health_and_releases_when_hurt() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(4, 3, "oBlock");
        game.player.pos = Vec2::new(200.0, 200.0);
        let mut enemy = Entity::new(100, "oGiantSpiderHang", Vec2::new(64.0, 64.0));
        super::super::init_entity(&mut enemy);
        assert_eq!(enemy.health, 10);
        tick(&mut game, &mut enemy);
        assert_eq!(enemy.object, "oGiantSpiderHang");
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oWeb")
                .count(),
            2
        );
        tick(&mut game, &mut enemy);
        assert_eq!(game.world.entities.len(), 2);
        enemy.health = 9;
        enemy.data.insert("webbed".into(), 30.0);
        tick(&mut game, &mut enemy);
        assert!(!enemy.alive);
        let replacement = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oGiantSpider")
            .unwrap();
        assert_eq!(replacement.health, 9);
        assert_eq!(replacement.value("webbed"), 0.0);
        assert_eq!(replacement.bounds(), Rect::new(66.0, 80.0, 28.0, 16.0));
    }

    #[test]
    fn caveman_worship_is_stationary_decoration() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let mut decoration = Entity::new(100, "oCavemanWorship", Vec2::new(64.0, 64.0));
        init(&mut decoration);
        tick(&mut game, &mut decoration);
        assert_eq!(decoration.pos, Vec2::new(64.0, 64.0));
        assert_eq!(decoration.sprite, "sCavemanWorshipR");
        assert_eq!(decoration.value("animation_speed"), 0.25);
        assert_eq!(decoration.value("enemy"), 0.0);
        assert!(game.events.is_empty());
    }

    #[test]
    fn spider_recovery_alarm_jumps_before_step_movement() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(88.0, 72.0);
        for x in 0..20 {
            game.world.set_tile(x, 5, "oBlock");
        }
        let id = game.world.spawn("oSpider", Vec2::new(64.0, 64.0));
        let enemy = game.world.entity_mut(id).unwrap();
        enemy.state = 2;
        enemy.timer = 1;
        super::super::tick(&mut game);
        let enemy = game.world.entity(id).unwrap();
        assert!(enemy.pos.y < 64.0);
        assert!(enemy.vel.y < 0.0);
        assert_eq!(enemy.state, 1);
    }

    #[test]
    fn spider_recovery_alarm_uses_raw_x_for_jump_direction() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(68.0, 72.0);
        for x in 0..20 {
            game.world.set_tile(x, 5, "oBlock");
        }
        let id = game.world.spawn("oSpider", Vec2::new(64.0, 64.0));
        let enemy = game.world.entity_mut(id).unwrap();
        enemy.state = 2;
        enemy.timer = 1;
        super::super::tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().vel.x, 2.5);
    }

    #[test]
    fn hanging_trigger_uses_rectangle_distance_instead_of_center_distance() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(72.0, 170.0);
        game.world.set_tile(4, 3, "oBlock");
        let id = game.world.spawn("oSpiderHang", Vec2::new(64.0, 64.0));
        super::super::tick(&mut game);
        assert!(game.world.entity(id).is_none());
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oSpider")
        );
    }

    #[test]
    fn drowning_spider_has_thirty_ticks_after_water_entry() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.spawn("oWaterSwim", Vec2::new(64.0, 64.0));
        let id = game.world.spawn("oSpider", Vec2::new(64.0, 64.0));
        super::super::tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().state, 4);
        assert_eq!(game.world.entity(id).unwrap().timer, 30);
        for _ in 0..29 {
            super::super::tick(&mut game);
        }
        assert!(game.world.entity(id).is_some());
        super::super::tick(&mut game);
        assert!(game.world.entity(id).is_none());
    }

    #[test]
    fn submerged_bat_retains_velocity_when_player_is_below() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(100.0, 110.0);
        game.world.spawn("oWaterSwim", Vec2::new(64.0, 64.0));
        let id = game.world.spawn("oBat", Vec2::new(64.0, 64.0));
        let enemy = game.world.entity_mut(id).unwrap();
        enemy.state = 1;
        enemy.vel = Vec2::new(0.0, -0.2);
        super::super::tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().vel, Vec2::new(0.0, -0.2));
    }

    #[test]
    fn offscreen_spider_alarms_expire_without_moving_the_actor() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.world.set_tile(32, 5, "oBlock");
        let id = game.world.spawn("oSpider", Vec2::new(512.0, 64.0));
        let enemy = game.world.entity_mut(id).unwrap();
        enemy.state = 2;
        enemy.timer = 1;
        super::super::tick(&mut game);
        let enemy = game.world.entity(id).unwrap();
        assert_eq!(enemy.state, 1);
        assert_eq!(enemy.pos, Vec2::new(512.0, 64.0));
        assert!(enemy.vel.y < 0.0);

        let enemy = game.world.entity_mut(id).unwrap();
        enemy.state = 4;
        enemy.timer = 1;
        super::super::tick(&mut game);
        assert!(game.world.entity(id).is_none());
    }

    #[test]
    fn spider_crush_uses_source_point_instead_of_body_center() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.set_tile(4, 4, "oBlock");
        let id = game.world.spawn("oSpider", Vec2::new(64.0, 55.0));
        super::super::tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().health, 1);
    }

    #[test]
    fn caveman_corpse_crush_uses_lower_source_point() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.set_tile(4, 4, "oBlock");
        let id = game.world.spawn("oCaveman", Vec2::new(64.0, 55.0));
        let enemy = game.world.entity_mut(id).unwrap();
        enemy.health = 0;
        enemy.state = 99;
        enemy.vel = Vec2::default();
        super::super::tick(&mut game);
        assert!(game.world.entity(id).is_none());
    }
}
