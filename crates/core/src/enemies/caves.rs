//! Cave enemy state machines translated from the Classic 1.1 event sources.
//!
//! Positions retain original GameMaker instance origins. Explicit solid edge
//! helpers use precise source masks; contact events retain sprite bounds. Moving sight
//! instances alert nearby cavemen and hawkmen in their automatic collision event.
//! Alarm and Animation End callbacks are dispatched by the engine independently
//! of each source Step viewport guard. Death credit belongs to each source death
//! branch; a stunned body with nonpositive HP remains uncredited while moving.
//!
//! Source references: `Objects/Enemies/oCaveman.events/Step.xml` and
//! `oEnemySight.events/Collision with oCharacter.xml` define moving sight and
//! group alerts; `oBat.events/Step.xml` defines water steering; spider
//! `Alarm 0.xml`/`Alarm 1.xml` and `Step.xml` define recovery and drowning.

use super::{bounds, center, move_body, move_body_preserving_velocity, steer_toward};
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
        }
        "oBat" => {
            set_sprite(e, "sBatHang");
            e.data.insert("animation_speed".into(), 0.5);
        }
        "oSpider" => set_sprite(e, "sSpiderFlip"),
        "oSpiderHang" => {
            set_sprite(e, "sSpiderHang");
            set_collision(e, [4.0, 0.0, 12.0, 12.0]);
        }
        "oGiantSpiderHang" => {
            set_sprite(e, "sGiantSpiderHang");
            e.health = 10;
            set_collision(e, [0.0, 0.0, 32.0, 16.0]);
        }
        "oGiantSpider" => {
            e.data.insert("contact_damage".into(), 2.0);
            set_sprite(e, "sGiantSpiderFlip");
            e.data.insert("animation_speed".into(), 0.8);
            // Context-dependent Create supplies the source RNG draw.
            e.data.insert("squirt_timer".into(), -1.0);
        }
        "oCaveman" => {
            e.vel.x = 2.5;
            set_sprite(e, "sCavemanLeft");
            e.data.insert("animation_speed".into(), 0.5);
            e.data.insert("stun_time".into(), 200.0);
        }
        "oEnemySight" => {
            set_sprite(e, "sSight");
            e.data.insert("visible".into(), 0.0);
            e.data.insert("animation_speed".into(), 0.0);
            e.data.insert("enemy".into(), 0.0);
        }
        _ => {}
    }
}

/// World-only Create side effects execute synchronously at instance creation.
pub(super) fn on_spawn(world: &mut World, id: u64) {
    let Some(spider) = world
        .entity_mut(id)
        .filter(|e| e.object == "oGiantSpiderHang")
    else {
        return;
    };
    if spider.value("hanging_webs_created") > 0.0 {
        return;
    }
    spider.data.insert("hanging_webs_created".into(), 1.0);
    let pos = spider.pos;
    world.spawn("oWeb", Vec2::new(pos.x, pos.y + 16.0));
    world.spawn("oWeb", Vec2::new(pos.x + 16.0, pos.y + 16.0));
}

/// Context-dependent source Create actions use the game's authoritative RNG.
pub(super) fn create(game: &mut Game, e: &mut Entity) {
    if e.object == "oGiantSpider" && e.value("cave_create_done") == 0.0 {
        e.data.insert("cave_create_done".into(), 1.0);
        e.data
            .insert("squirt_timer".into(), f64::from(100 + game.rng.range(901)));
        game.sound("sndGiantSpider", e.pos);
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
        // oEnemySight has builtin speed but no source Step event.
        "oEnemySight" => {}
        _ => {}
    }
}

/// Spider's water check is outside its viewport-guarded Step body.
pub(super) fn tick_independent_events(game: &mut Game, e: &mut Entity) {
    if e.object == "oSpider"
        && e.state != 4
        && in_water(game, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0))
    {
        e.state = 4;
        set_sprite(e, "sSpiderDrowning");
        e.alarms[1] = 30;
        e.vel = Vec2::new(0.0, 0.2);
        game.world
            .spawn("oSplash", Vec2::new(e.pos.x + 8.0, e.pos.y));
        game.sound("sndSplash", e.pos);
    }
}

pub(super) fn alarm(game: &mut Game, e: &mut Entity, alarm: u8) {
    match (e.object.as_str(), alarm) {
        ("oSpider", 0) => {
            e.state = 1;
            if bottom(&game.world, e) {
                launch_spider(game, e, 2, true);
            }
        }
        ("oGiantSpider", 0) if e.sprite != "sGiantSpiderSquirt" => {
            e.state = 1;
            set_sprite(e, "sGiantSpiderJump");
            if bottom(&game.world, e) {
                set_sprite(e, "sGiantSpider");
                launch_spider(game, e, 2, true);
            }
        }
        ("oSpider", 1) => e.alive = false,
        _ => {}
    }
}

pub(super) fn animation_end(_game: &mut Game, e: &mut Entity) {
    match e.sprite.as_str() {
        "sSpiderFlip" if e.object == "oSpider" => set_sprite(e, "sSpider"),
        "sGiantSpiderFlip" if e.object == "oGiantSpider" => {
            set_sprite(e, "sGiantSpider");
            e.data.insert("animation_speed".into(), 0.4);
        }
        "sGiantSpiderSquirt" if e.object == "oGiantSpider" => {
            e.state = 0;
            e.data.insert("animation_speed".into(), 0.4);
        }
        _ => {}
    }
}

fn set_sprite(e: &mut Entity, sprite: &str) {
    crate::animation::assign(&mut e.sprite, &mut e.animation, sprite);
}

fn crushed(game: &Game, e: &Entity, offset: Vec2) -> bool {
    game.world
        .solid_point_except(Vec2::new(e.pos.x + offset.x, e.pos.y + offset.y), e.id)
}

// Native distance_to_object reads inclusive cached integer envelopes from
// active target-family members. A destroyed receiver can continue its Step.
fn distance_to_object(game: &Game, e: &Entity, family: &str) -> f64 {
    let receiver = e.collision_mask().point_bounds();
    let mut distance = 1_000_000.0_f64;
    if game.player.engine_active && crate::catalog::is_a("oPlayer1", family) {
        distance = distance.min(crate::world_events::mask_distance(
            receiver,
            game.player.collision_mask().point_bounds(),
        ));
    }
    for target in &game.world.entities {
        if target.id != e.id
            && target.alive
            && target.engine_active
            && crate::catalog::is_a(&target.object, family)
        {
            distance = distance.min(crate::world_events::mask_distance(
                receiver,
                target.collision_mask().point_bounds(),
            ));
        }
    }
    distance
}

pub(super) fn collision(
    game: &mut Game,
    e: &mut Entity,
    other: crate::dispatcher::Instance,
) -> bool {
    if e.object == "oCaveman" && other == crate::dispatcher::Instance::Player {
        caveman_character_collision(game, e);
        return true;
    }
    if e.object == "oGiantSpider" && other == crate::dispatcher::Instance::Player {
        giant_spider_character_collision(game, e);
        return true;
    }
    if e.object != "oEnemySight" {
        return false;
    }
    if crate::catalog::is_a(other.object(game), "oSolid") {
        // The source event is action_kill_object applied to .self.
        e.alive = false;
        return true;
    }
    if !crate::catalog::is_a(other.object(game), "oCharacter") {
        return false;
    }
    // Source executes two separate with statements: all Cavemen, then all
    // Hawkmen. Each family follows its current native array, independent of
    // storage partitioning. Pending raw World births append by creation order
    // without an impure registration/Create call inside this query.
    for object in ["oCaveman", "oHawkman"] {
        let family = crate::dispatcher::descendant_instances(game, object);
        let mut targets: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|target| {
                target.alive && target.engine_active && crate::catalog::is_a(&target.object, object)
            })
            .collect();
        targets.sort_by_key(|target| {
            family
                .iter()
                .position(|instance| *instance == crate::dispatcher::Instance::Entity(target.id))
                .map_or((1_u8, target.creation_serial), |rank| (0, rank as u64))
        });
        let alerted: Vec<_> = targets
            .into_iter()
            .filter(|target| {
                target.state < 98 && distance_to_object(game, target, "oPlayer1") < 100.0
            })
            .map(|target| target.id)
            .collect();
        for id in alerted {
            let Some(target) = game.world.entity_mut(id) else {
                continue;
            };
            target.state = 2;
            let pos = target.pos;
            game.sound("sndAlert", pos);
        }
    }
    true
}

fn jumping_player(game: &Game, ceiling: f64) -> bool {
    super::player_alive(game)
        && matches!(
            game.player.state,
            PlayerState::Jumping | PlayerState::Falling
        )
        && game.player.pos.y < ceiling
        && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
}

fn stomp_damage(game: &Game) -> i32 {
    let fall = game.player.data.get("fall_timer").copied().unwrap_or(0.0);
    ((fall / 16.0).floor() as i32 + 1) * if game.player.has("SpikeShoes") { 3 } else { 1 }
}

fn caveman_character_collision(game: &mut Game, e: &mut Entity) {
    if (game.player.pos.x - (e.pos.x + 8.0)).abs() > 8.0 {
        return;
    }
    if jumping_player(game, e.pos.y + 5.0) {
        if e.state < 98 {
            game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
            e.health -= stomp_damage(game);
            if game.player.has("SpikeShoes") && e.value("bloodless") == 0.0 {
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
            e.data.insert("animation_speed".into(), 0.5);
            game.sound("sndHit", e.pos);
        }
    } else if game.player.invincible == 0 && e.state < 98 {
        game.spawn("oBlood", game.player.pos);
        cave_contact_damage(game, e, 1, crate::statistics::EnemyDeath::Caveman);
    }
}

fn giant_spider_character_collision(game: &mut Game, e: &mut Entity) {
    if (game.player.pos.x - (e.pos.x + 16.0)).abs() > 16.0 {
        return;
    }
    if jumping_player(game, e.pos.y + 16.0) {
        game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
        e.health -= stomp_damage(game);
        if game.player.has("SpikeShoes") && e.value("bloodless") == 0.0 {
            game.spawn(
                "oBlood",
                Vec2::new(game.player.pos.x, game.player.pos.y + 8.0),
            );
        }
        game.player.data.insert("fall_timer".into(), 0.0);
        // This child event deliberately does not restore countsAsKill.
        game.spawn("oBlood", Vec2::new(e.pos.x + 16.0, e.pos.y + 24.0));
        game.sound("sndHit", e.pos);
    } else if game.player.invincible == 0 {
        cave_contact_damage(game, e, 2, crate::statistics::EnemyDeath::GiantSpider);
    }
}

fn cave_contact_damage(
    game: &mut Game,
    e: &Entity,
    amount: i32,
    cause: crate::statistics::EnemyDeath,
) {
    game.player.invincible = 30;
    game.player.data.insert("blink".into(), 30.0);
    if game.player.pos.y < e.pos.y {
        game.player.vel.y = -6.0;
    }
    game.player.vel.x = if game.player.pos.x < e.pos.x {
        -6.0
    } else {
        6.0
    };
    if game.player.health > 0 {
        game.player.health -= amount;
        game.events.push(crate::GameEvent::Damage { amount });
        if game.player.health <= 0 && crate::statistics::is_real_level(game) {
            game.lifetime_stats.enemy_death(cause);
        }
    }
    game.sound("sndHurt", game.player.pos);
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

fn blood(game: &mut Game, e: &Entity, pos: Vec2, count: usize) {
    if e.value("bloodless") > 0.0 {
        return;
    }
    for _ in 0..count {
        if game
            .world
            .entities
            .iter()
            .filter(|other| {
                other.alive
                    && other.engine_active
                    && crate::catalog::is_a(&other.object, "oDetritus")
            })
            .count()
            < 16
        {
            game.spawn("oBlood", pos);
        }
    }
}

fn spider_death(game: &mut Game, e: &mut Entity, giant: bool) {
    if giant {
        let pos = Vec2::new(e.pos.x + 16.0, e.pos.y + 24.0);
        for _ in 0..1 + game.rng.range(3) {
            let object = match game.rng.range(3) {
                0 => "oEmeraldBig",
                1 => "oSapphireBig",
                _ => "oRubyBig",
            };
            let id = game.spawn(object, pos);
            let vx = f64::from(game.rng.range(4)) - f64::from(game.rng.range(4));
            let gem = game.world.entity_mut(id).unwrap();
            gem.vel = Vec2::new(vx, -2.0);
        }
        let paste = game.spawn("oPaste", pos);
        let paste = game.world.entity_mut(paste).unwrap();
        paste.data.insert("cost".into(), 0.0);
        paste.data.insert("forSale".into(), 0.0);
        blood(game, e, pos, 4);
    } else {
        blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 3);
    }
    super::record_death(game, e);
    e.alive = false;
}

fn hanging_spider(game: &mut Game, e: &mut Entity, giant: bool) {
    let half = if giant { 16.0 } else { 8.0 };
    let crush_y = if giant { 8.0 } else { 4.0 };
    if game.world.solid_at(e.pos.x + half, e.pos.y + crush_y) {
        e.health = 0;
    }
    if e.health <= 0 {
        spider_death(game, e, giant);
        return;
    }
    let beneath = distance_to_object(game, e, "oCharacter") < 90.0
        && game.player.pos.y > e.pos.y
        && (game.player.pos.x - (e.pos.x + half)).abs() < 8.0;
    if (giant && e.health < 10) || !game.world.solid_at(e.pos.x, e.pos.y - 16.0) || beneath {
        // Both hanging Step events create a fresh actor and copy only hp.
        // Reusing the ID would also incorrectly retain whip-hit bookkeeping.
        let id = game.spawn(if giant { "oGiantSpider" } else { "oSpider" }, e.pos);
        game.world.entity_mut(id).unwrap().health = e.health;
        e.alive = false;
    }
}

fn bottom(world: &World, e: &Entity) -> bool {
    let r = bounds(e);
    super::solid_mask(world, Rect::new(r.x, r.y + r.h, r.w, 1.0), e.id)
}

fn top(world: &World, e: &Entity) -> bool {
    let r = bounds(e);
    super::solid_mask(world, Rect::new(r.x, r.y - 1.0, r.w, 1.0), e.id)
}

fn side(world: &World, e: &Entity, direction: i8) -> bool {
    let r = bounds(e);
    super::solid_mask(
        world,
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
        blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 3);
        super::record_death(game, e);
        e.alive = false;
        // instance_destroy finishes Enemy Destroy, then the current Snake
        // Step continues through its facing/counter/RNG and sprite tail.
        super::destroyed(game, e);
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
            e.facing = if game.world.solid_at(e.pos.x - 1.0, e.pos.y) {
                1
            } else {
                -1
            };
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
    set_sprite(e, "sSnakeWalkL");
    if super::body_overlaps_solid(&game.world, e) {
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
    // Source moveTo changes position only. In particular, its wet guard can
    // skip the later velocity assignments after a blocked vertical move.
    let incoming_velocity = e.vel;
    move_body(&game.world, e, 0.0);
    e.vel = incoming_velocity;
    if crushed(game, e, Vec2::new(8.0, 8.0)) {
        e.health = -999;
    }
    if e.health <= 0 {
        blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 3);
        super::record_death(game, e);
        e.alive = false;
        // Own Bat Step still evaluates HANG/ATTACK after Enemy Destroy.
        super::destroyed(game, e);
    }
    let body_center = center(e);
    let distance = body_center.distance(game.player.pos);
    let ceiling = game.world.solid_at(e.pos.x + 8.0, e.pos.y - 1.0);
    if e.state == 0 {
        set_sprite(e, "sBatHang");
        if player_available(game)
            && ((distance < 90.0 && game.player.pos.y > e.pos.y + 16.0) || !ceiling)
        {
            e.state = 1;
            game.sound("sndBat", e.pos);
        }
        return;
    }
    let target_available = player_available(game);
    if target_available && distance < 160.0 {
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
        // Both source ATTACK fallbacks assign only status under a ceiling.
        e.state = 0;
    } else {
        e.vel = Vec2::new(0.0, -1.0);
    }
    // Directional sprites belong only to the eligible-target branch. The
    // swimming/dead fallback retains its previous image until HANG's Step.
    if target_available {
        e.facing = if game.player.pos.x < body_center.x {
            -1
        } else {
            1
        };
        set_sprite(
            e,
            if e.facing < 0 {
                "sBatLeft"
            } else {
                "sBatRight"
            },
        );
    }
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
    if giant && e.value("hit_cooldown") > 0.0 {
        e.data
            .insert("hit_cooldown".into(), e.value("hit_cooldown") - 1.0);
    }
    let (_, grounded) =
        move_body_preserving_velocity(&game.world, e, if giant { 0.3 } else { 0.2 });
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
        spider_death(game, e, giant);
        super::destroyed(game, e);
        // Both source Spider events continue after instance_destroy.
    }
    let grounded = grounded || bottom(&game.world, e);
    let ceiling = top(&game.world, e);
    if side(&game.world, e, 1) {
        e.vel.x = 1.0;
    }
    if side(&game.world, e, -1) {
        e.vel.x = -1.0;
    }

    let mut squirt = e.value("squirt_timer");
    if giant {
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
            e.alarms[0] = (5 + game.rng.range(16)) as i32;
            e.state = if giant && squirt == 0.0 { 5 } else { 2 };
            if giant && e.sprite != "sGiantSpiderFlip" {
                set_sprite(e, "sGiantSpider");
            }
        }
        1 if distance_to_object(game, e, "oCharacter") < if giant { 120.0 } else { 90.0 } => {
            if giant {
                set_sprite(e, "sGiantSpiderJump");
            }
            if grounded {
                launch_spider(game, e, if giant { 3 } else { 2 }, false);
                if giant {
                    set_sprite(e, "sGiantSpider");
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
            set_sprite(e, "sGiantSpiderCrawl");
            if !ceiling || !grounded {
                e.state = 0;
            } else if side(&game.world, e, 1) {
                e.vel.x = -1.0;
            } else if side(&game.world, e, -1) {
                e.vel.x = 1.0;
            }
        }
        // DROWNED skips the state transitions, not the unconditional probes.
        4 => {}
        5 if giant => {
            set_sprite(e, "sGiantSpiderSquirt");
            if e.animation >= 5.0 && squirt == 0.0 {
                // Source instance_create completes WebBall Create before
                // the following squirtTimer draw in GiantSpider Step.
                game.spawn("oWebBall", Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
                squirt = f64::from(100 + game.rng.range(901));
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

// The current Step receiver is detached from World while it executes. A live
// sight emitter retains its existing owner; after Destroy, instance_place must
// instead search the remaining live family using the receiver's sprite mask.
pub(super) fn continuation_sight_owner(game: &mut Game, e: &Entity) -> f64 {
    if e.alive {
        return e.id as f64;
    }
    crate::dispatcher::register_instances(game);
    crate::dispatcher::descendant_instances(game, &e.object)
        .into_iter()
        .find_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => game
                .world
                .entity(id)
                .filter(|other| other.engine_active && e.event_intersects(other))
                .map(|other| other.id as f64),
            _ => None,
        })
        .unwrap_or(-4.0) // GameMaker noone
}

// Source IDLE uses raw top collision_point(-1,-1), while WALK/ATTACK
// consume the earlier captured full-height side flags. Native GM8 converts
// each finite -1 flag to false, so these points use the cached bbox.
fn caveman_idle_wall(game: &Game, e: &Entity) -> bool {
    let points = [
        Vec2::new(e.pos.x - 1.0, e.pos.y),
        Vec2::new(e.pos.x + 16.0, e.pos.y),
    ];
    // A false-only coarse query avoids resolving every terrain mask for an
    // empty pair of points; registered source receivers remain authoritative.
    let world = &game.world;
    if !points.iter().any(|point| {
        world.tiles_point(*point, "oSolid")
            || world.entities.iter().any(|other| {
                other.alive
                    && other.engine_active
                    && crate::catalog::is_a(&other.object, "oSolid")
                    && other.mask_contains(*point, crate::CollisionPrecision::BoundingBox)
            })
    }) {
        return false;
    }
    crate::dispatcher::descendant_instances(game, "oSolid")
        .into_iter()
        .any(|instance| match instance {
            crate::dispatcher::Instance::Player => false,
            crate::dispatcher::Instance::Terrain(index) => {
                world
                    .tiles
                    .get(index)
                    .is_some_and(|tile| tile.engine_active)
                    && points.iter().any(|point| {
                        world
                            .tile_collision_mask(
                                (index % world.width) as i32,
                                (index / world.width) as i32,
                            )
                            .contains(*point, crate::CollisionPrecision::BoundingBox)
                    })
            }
            crate::dispatcher::Instance::Entity(id) => world.entity(id).is_some_and(|other| {
                other.engine_active
                    && points.iter().any(|point| {
                        other.mask_contains(*point, crate::CollisionPrecision::BoundingBox)
                    })
            }),
        })
}

fn caveman(game: &mut Game, e: &mut Entity) {
    let mut impact_y = (e.vel.y
        + if e.held {
            0.0
        } else {
            super::inherited::gravity(e, 0.6)
        })
    .min(10.0);
    move_body(&game.world, e, if e.held { 0.0 } else { 0.6 });
    let contacts = super::body_contacts(&game.world, e);
    // The source moveTo resolves displacement without clearing stored yVel.
    e.vel.y = impact_y;
    let crush_y = if e.state >= 98 { 12.0 } else { 8.0 };
    if (!e.held || e.state >= 98) && crushed(game, e, Vec2::new(8.0, crush_y)) {
        blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 3);
        e.alive = false;
        game.sound("sndCavemanDie", e.pos);
        // instance_destroy completes Enemy Destroy, then this Step continues.
        super::destroyed(game, e);
    }
    let grounded = contacts.bottom;
    let hit_side = contacts.left || contacts.right;
    if e.health < 1 && e.state < 98 {
        e.state = 99;
    }
    if contacts.bottom && e.state != 98 {
        e.vel.y = 0.0;
        impact_y = 0.0;
    }
    let casts_sight = e.state < 2;
    match e.state {
        0 => {
            e.data.insert("bounced".into(), 0.0);
            if grounded && caveman_idle_wall(game, e) {
                e.vel.y = -6.0;
                e.vel.x = f64::from(e.facing);
                e.timer = e.timer.saturating_sub(10);
            }
            if e.vel.y < 0.0 && contacts.top {
                e.vel.y = 0.0;
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
            set_sprite(
                e,
                if e.vel.x == 0.0 && e.health > 0 {
                    "sCavemanStunL"
                } else if e.value("bounced") > 0.0 {
                    if e.vel.y < 0.0 {
                        "sCavemanBounceL"
                    } else {
                        "sCavemanFallL"
                    }
                } else if e.vel.x.abs() > 0.0 {
                    "sCavemanDieLL"
                } else {
                    "sCavemanDieLR"
                },
            );
            if grounded && e.value("bounced") == 0.0 {
                e.data.insert("bounced".into(), 1.0);
                blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 1);
            }
            if grounded || e.held {
                if e.timer > 0 {
                    e.timer -= 1;
                } else if e.health > 0 {
                    e.state = 0;
                    // Destroy already cleared Player holdItem, but left this
                    // receiver's held flag intact for the remaining Step.
                    if e.held {
                        e.held = false;
                        game.player.held = None;
                        game.player.pickup_item_type.clear();
                    }
                }
            }
        }
        99 => {
            super::record_death(game, e);
            set_sprite(e, "sCavemanDeadL");
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
            let id = game.spawn("oEnemySight", e.pos);
            let owner = continuation_sight_owner(game, e);
            if let Some(sight) = game.world.entity_mut(id) {
                sight
                    .data
                    .insert("hspeed".into(), f64::from(e.facing) * 10.0);
                sight.data.insert("vspeed".into(), 0.0);
                sight.data.insert("owner".into(), owner);
            }
            e.data.insert("sight_timer".into(), 5.0);
        } else {
            e.data.insert("sight_timer".into(), sight_timer - 1.0);
        }
    }
    if e.state >= 98 {
        super::settle_body(e, contacts, impact_y);
        // Source enters DEAD only after the collision settlement and before
        // ordinary friction. Credit occurs in the next Step's DEAD branch.
        if e.vel.x == 0.0 && e.vel.y == 0.0 && e.health < 1 {
            e.state = 99;
        }
    }
    e.vel.x = (e.vel.x.abs() - 0.1).max(0.0) * e.vel.x.signum();
    if e.vel.x.abs() < 0.5 {
        e.vel.x = 0.0;
    }
    if e.state < 98 {
        set_sprite(
            e,
            if e.vel.x == 0.0 {
                "sCavemanLeft"
            } else {
                "sCavemanRunLeft"
            },
        );
    }
    if e.held {
        set_sprite(
            e,
            if e.health > 0 {
                "sCavemanHeldL"
            } else {
                "sCavemanDHeldL"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RulesPreset;

    #[test]
    fn natural_whip_kills_create_three_source_initialized_blood_drops() {
        for (rules, object) in [
            (RulesPreset::Classic, "oSnake"),
            (RulesPreset::Classic, "oBat"),
            (RulesPreset::Assisted, "oSnake"),
            (RulesPreset::Assisted, "oBat"),
        ] {
            let mut game = Game::shell(42, rules);
            game.world = World::new(20, 16, crate::Area::Mines, 1);
            game.world.name = "rLevel".into();
            for x in 0..20 {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.player.pos = Vec2::new(80.0, 152.0);
            game.player.previous_pos = game.player.pos;
            game.player.facing = 1;
            let animal = game.spawn(object, Vec2::new(104.0, 144.0));
            game.step(&crate::InputFrame {
                right: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            let mut death_rng = None;
            let mut previous_blood = Vec::new();
            let mut snake_tail = None;
            for _ in 0..30 {
                if game.world.entity(animal).is_some_and(|e| e.health <= 0) {
                    death_rng = Some(game.rng.clone());
                    if object == "oSnake" {
                        let actor = game.world.entity(animal).unwrap();
                        snake_tail = Some((actor.state, actor.timer));
                    }
                    previous_blood = game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oBlood")
                        .map(|e| e.id)
                        .collect();
                }
                if game.world.entity(animal).is_none() {
                    break;
                }
                game.step(&crate::InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            let mut expected_rng = death_rng.expect("actual whip must inflict lethal damage");
            assert!(game.world.entity(animal).is_none(), "{object} must die");
            let blood: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBlood" && !previous_blood.contains(&e.id))
                .collect();
            assert_eq!(blood.len(), 3, "source death burst for {object}");
            for drop in blood {
                let expected_velocity = Vec2::new(
                    expected_rng.unit() * 4.0 - expected_rng.unit() * 4.0,
                    -1.0 - expected_rng.unit() * 2.0,
                );
                let expected_gravity = f64::from(1 + expected_rng.range(6)) * 0.1;
                assert_eq!(drop.vel, expected_velocity);
                assert_eq!(drop.value("gravity"), expected_gravity);
                assert!(drop.engine_active);
                assert_eq!(drop.sprite, "sBlood");
                assert_eq!(drop.value("world_effect_initialized"), 1.0);
                assert_eq!(drop.image_speed(), 0.3);
                assert_eq!(drop.alarms[0], 1);
                assert_eq!(drop.alarms[1], 1);
                assert_eq!(drop.alarms[2], 5);
                assert!((-4.0..4.0).contains(&drop.vel.x));
                assert!((-3.0..=-1.0).contains(&drop.vel.y));
                assert!((0.1..=0.6 + f64::EPSILON).contains(&drop.value("gravity")));
            }
            if let Some((state, timer)) = snake_tail {
                // Source Snake Step continues AFTER instance_destroy. This
                // tail follows all three synchronous Blood Create events.
                if state == 0 && timer == 0 {
                    expected_rng.range(2);
                } else if state == 1 && expected_rng.range(100) == 0 {
                    expected_rng.range(31);
                }
            }
            assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
        }
    }

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
        game.world
            .entity_mut(ray)
            .unwrap()
            .data
            .insert("hspeed".into(), 10.0);
        let caveman = game.world.spawn("oCaveman", Vec2::new(64.0, 100.0));
        let hawkman = game.world.spawn("oHawkman", Vec2::new(120.0, 100.0));
        let stunned = game.world.spawn("oCaveman", Vec2::new(90.0, 100.0));
        game.world.entity_mut(stunned).unwrap().state = 98;
        game.world.entity_mut(stunned).unwrap().timer = 200;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(caveman).unwrap().state, 2);
        assert_eq!(game.world.entity(hawkman).unwrap().state, 2);
        assert_eq!(game.world.entity(stunned).unwrap().state, 98);
        assert_eq!(game.world.entity(ray).unwrap().pos.x, 74.0);
        assert_eq!(game.player.health, 4);
    }

    #[test]
    fn natural_caveman_sight_dies_at_a_wall_and_open_path_still_alerts() {
        // oCaveman Step's first idle-to-walk transition draws rand(0,1).
        // Canonical seed 16 first yields raw 2156413009, so multiply-high
        // range(2) selects RIGHT and casts sight toward the authored wall.
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for wall in [false, true] {
                let mut game = Game::shell(16, rules);
                for x in 0..game.world.width {
                    game.world.set_tile(x, 10, "oBlock");
                }
                if wall {
                    for y in 0..10 {
                        game.world.set_tile(8, y, "oBlock");
                    }
                }
                game.player.pos = Vec2::new(160.0, 152.0);
                game.player.previous_pos = game.player.pos;
                let caveman = game.spawn("oCaveman", Vec2::new(96.0, 144.0));
                // The source idle branch casts its own moving sight; there is
                // no manually spawned projectile or actor-state injection.
                game.step(&crate::InputFrame::default());
                let ray = game
                    .world
                    .entities
                    .iter()
                    .find(|e| {
                        e.alive && e.object == "oEnemySight" && e.value("owner") == caveman as f64
                    })
                    .expect("the real Caveman Step must create its sight");
                let ray_id = ray.id;
                assert_eq!(ray.value("hspeed"), 10.0);
                assert_eq!(game.world.entity(caveman).unwrap().state, 1);
                game.step(&crate::InputFrame::default());
                assert_eq!(
                    game.world.entity(ray_id).is_none(),
                    wall,
                    "the naturally moving sight reaches the authored wall on its second tick"
                );
                let mut alerted = game.events.iter().any(|event| {
                    matches!(event, crate::GameEvent::Sound { name, .. } if name == "sndAlert")
                });
                for _ in 2..6 {
                    game.step(&crate::InputFrame::default());
                    alerted |= game.events.iter().any(|event| {
                        matches!(event, crate::GameEvent::Sound { name, .. } if name == "sndAlert")
                    });
                }
                assert_eq!(
                    alerted, !wall,
                    "only the unobstructed sight can alert its owner"
                );
                assert_eq!(game.world.entity(caveman).unwrap().state == 2, !wall);
                assert_eq!(game.player.health, 4);
            }
        }
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
        assert!(rays.iter().all(|e| e.value("hspeed") == 10.0));
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
        assert!((20.0..=100.0).contains(&web.value("life")));
    }

    #[test]
    fn actual_step_giant_spider_runs_web_create_before_squirt_timer_rng() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 152.0);
        game.world.set_tile(4, 6, "oBlock");
        game.world.set_tile(5, 6, "oBlock");
        let id = game.spawn("oGiantSpider", Vec2::new(64.0, 64.0));
        let spider = game.world.entity_mut(id).unwrap();
        spider.state = 5;
        spider.animation = 5.0;
        spider.data.insert("squirt_timer".into(), 0.0);
        let mut expected = game.rng.clone();
        let vy = -(expected.unit() * 3.0 + 1.0);
        let mut vx = f64::from(1 + expected.range(3));
        if expected.chance(1, 2) {
            vx *= -1.0;
        }
        let life = f64::from(20 + expected.range(81));
        let squirt = f64::from(100 + expected.range(901));

        game.step(&crate::InputFrame::default());

        let web = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oWebBall")
            .unwrap();
        // WebBall's later resource Step runs in the same frame as Create.
        assert_eq!(web.vel, Vec2::new(vx, vy + 0.2));
        assert_eq!(web.value("life"), life - 1.0);
        assert_eq!(web.value("web_create_initialized"), 1.0);
        assert_eq!(game.world.entity(id).unwrap().value("squirt_timer"), squirt);
        assert_eq!(game.rng.clone().unit(), expected.clone().unit());

        game.step(&crate::InputFrame::default());

        assert_eq!(game.rng.clone().unit(), expected.unit());
        assert_eq!(
            game.world
                .entities
                .iter()
                .find(|e| e.object == "oWebBall")
                .unwrap()
                .value("life"),
            life - 2.0
        );
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
        let id = game.spawn("oGiantSpiderHang", Vec2::new(64.0, 64.0));
        let mut enemy = game.world.entity(id).unwrap().clone();
        assert_eq!(enemy.health, 10);
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
        assert_eq!(game.world.entities.len(), 3);
        on_spawn(&mut game.world, id);
        assert_eq!(game.world.entities.len(), 3, "Create webs only once");
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
        enemy.alarms[0] = 1;
        game.step(&crate::InputFrame::default());
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
        enemy.alarms[0] = 1;
        game.step(&crate::InputFrame::default());
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
        game.player.pos = Vec2::new(200.0, 120.0);
        for x in 0..20 {
            game.world.set_tile(x, 8, "oBlock");
        }
        game.world.spawn("oWaterSwim", Vec2::new(64.0, 64.0));
        let id = game.world.spawn("oSpider", Vec2::new(64.0, 64.0));
        // No pending recovery alarm; the independent drowning alarm has 30 ticks.
        game.world.entity_mut(id).unwrap().state = 2;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().state, 4);
        assert_eq!(game.world.entity(id).unwrap().alarms[1], 30);
        for _ in 0..29 {
            game.step(&crate::InputFrame::default());
        }
        assert!(game.world.entity(id).is_some());
        game.step(&crate::InputFrame::default());
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
        game.world = World::new(40, 30, crate::Area::Mines, 1);
        game.player.pos = Vec2::new(160.0, 72.0);
        game.world.set_tile(21, 5, "oBlock");
        let id = game.world.spawn("oSpider", Vec2::new(336.0, 64.0));
        let enemy = game.world.entity_mut(id).unwrap();
        enemy.state = 2;
        enemy.alarms[0] = 1;
        game.step(&crate::InputFrame::default());
        let enemy = game.world.entity(id).unwrap();
        assert_eq!(enemy.state, 1);
        assert_eq!(enemy.pos, Vec2::new(336.0, 64.0));
        assert!(enemy.vel.y < 0.0);

        let enemy = game.world.entity_mut(id).unwrap();
        enemy.state = 4;
        enemy.alarms[1] = 1;
        game.step(&crate::InputFrame::default());
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
    #[test]
    fn natural_shotgun_kill_waits_for_caveman_stationary_dead_branch() {
        let mut game = Game::shell(123, RulesPreset::Classic);
        game.world = World::new(40, 30, crate::Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..40 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(80.0, 120.0);
        game.player.previous_pos = game.player.pos;
        game.player.facing = 1;
        let enemy = game.world.spawn("oCaveman", Vec2::new(104.0, 112.0));
        let shotgun = game.world.spawn("oShotgun", game.player.pos);
        game.player.held = Some(shotgun);
        game.world.entity_mut(shotgun).unwrap().held = true;
        game.step(&crate::InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let mut witnessed_airborne_death = false;
        for _ in 0..20 {
            game.step(&crate::InputFrame::default());
            let body = game.world.entity(enemy).unwrap();
            if body.health <= 0 && body.vel.y < 0.0 {
                witnessed_airborne_death = true;
                assert_eq!(body.state, 98, "airborne lethal hit remains STUNNED");
                assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
                assert_eq!(game.lifetime_stats.enemy_kills[4], 0);
                break;
            }
        }
        assert!(
            witnessed_airborne_death,
            "real airborne shotgun kill required"
        );
        for _ in 0..240 {
            game.step(&crate::InputFrame::default());
            if game.data.get("cavemen").copied().unwrap_or(0.0) > 0.0 {
                break;
            }
        }
        assert_eq!(game.data.get("cavemen"), Some(&1.0));
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.lifetime_stats.enemy_kills[4], 1);
        for _ in 0..30 {
            game.step(&crate::InputFrame::default());
        }
        assert_eq!(game.data.get("cavemen"), Some(&1.0));
        assert_eq!(game.lifetime_stats.enemy_kills[4], 1);
    }

    #[test]
    fn giant_spider_squirt_finishes_only_at_animation_end_callback() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let mut spider = Entity::new(100, "oGiantSpider", Vec2::new(64.0, 64.0));
        init(&mut spider);
        spider.state = 5;
        set_sprite(&mut spider, "sGiantSpiderSquirt");
        spider.animation = 7.9;
        spider.data.insert("squirt_timer".into(), 50.0);
        tick(&mut game, &mut spider);
        assert_eq!(spider.state, 5, "Step does not dispatch Animation End");
        animation_end(&mut game, &mut spider);
        assert_eq!(spider.state, 0);
        assert_eq!(spider.value("animation_speed"), 0.4);
    }
}

#[cfg(test)]
#[path = "caves_cached_distance_tests.rs"]
mod caves_cached_distance_tests;

#[cfg(test)]
#[path = "enemy_sight_family_order_tests.rs"]
mod enemy_sight_family_order_tests;

#[cfg(test)]
#[path = "snake_create_sprite_source_tests.rs"]
mod snake_create_sprite_source_tests;
