//! Temple enemies translated from Classic 1.1's individual object events.
//! Instances retain the original GameMaker origins, including top-left boss origins.
use super::{center, move_body};
use crate::dispatcher::Instance;
use crate::model::{Entity, Game, GameEvent, Rect, Vec2, World};

pub fn matches(object: &str) -> bool {
    matches!(
        object,
        "oHawkman"
            | "oHawkmanWorship"
            | "oTombLord"
            | "oFly"
            | "oOlmec"
            | "oOlmecSlam"
            | "oOlmecDebris"
            | "oYellowBall"
            | "oMagmaMan"
            | "oMagma"
            | "oMagmaTrail"
    )
}

pub fn init(e: &mut Entity) {
    e.set_image_speed(match e.object.as_str() {
        "oHawkman" | "oHawkmanWorship" | "oMagmaMan" => 0.5,
        "oTombLord" => 0.25,
        "oOlmec" | "oMagmaTrail" => 0.4,
        "oOlmecDebris" | "oMagma" => 0.3,
        _ => 1.0,
    });
    match e.object.as_str() {
        "oHawkman" | "oHawkmanWorship" => {
            e.health = 4;
            e.vel.x = 2.5;
            e.set_sprite("sHawkLeft");
            e.data.insert("contact_damage".into(), 0.0);
        }
        "oTombLord" => {
            e.health = 20;
            e.data.insert("bloodless".into(), 1.0);
            e.vel.x = 2.5;
            e.set_sprite("sTombLordLeft");
            e.data.insert("contact_damage".into(), 2.0);
            e.set_image_speed(0.25);
        }
        "oOlmec" => {
            e.health = 1;
            e.solid = true;
            e.state = -1;
            e.timer = 0;
            e.set_sprite("sOlmecStart1");
            e.data.insert("intro_stage".into(), 0.0);
            e.data.insert("invincible".into(), 1.0);
            e.data.insert("no_stomp".into(), 1.0);
            e.data.insert("no_whip".into(), 1.0);
        }
        "oOlmecSlam" => {
            e.alarms[0] = 1;
            e.set_sprite("");
        }
        "oMagmaMan" | "oMagma" => {
            e.health = 200;
            if e.object == "oMagma" {
                e.alarms[0] = 2;
                e.alarms[1] = 50;
            }
            if e.object == "oMagmaMan" {
                e.vel.x = 2.5;
            }
            // oMagmaMan Create sets invincible=0; its whip collision only
            // plays a sound, but other projectile damage remains possible.
            e.data.insert("no_stomp".into(), 1.0);
            e.data.insert("no_whip".into(), 1.0);
            e.data.insert("contact_damage".into(), 2.0);
            // Airborne oMagma inherits oDetritus, so oEnemy projectile collision
            // handlers do not apply until it becomes oMagmaMan again.
            if e.object == "oMagma" {
                e.data.insert("invincible".into(), 1.0);
            }
        }
        "oMagmaTrail" => {}
        "oFly" => {}
        "oYellowBall" => {
            e.data.insert("launch".into(), 1.0);
            e.alarms[1] = 1;
        }
        _ => {}
    }
}

pub fn tick(game: &mut Game, e: &mut Entity) {
    match e.object.as_str() {
        "oHawkman" => hawk(game, e),
        "oHawkmanWorship" => {}
        "oTombLord" => tomb_lord(game, e),
        "oFly" => fly(game, e),
        "oOlmec" => olmec(game, e),
        "oOlmecSlam" => slam(game, e),
        "oOlmecDebris" => debris(game, e),
        "oYellowBall" => yellow_ball(game, e),
        "oMagmaMan" => magma_man(game, e),
        "oMagma" => magma(game, e),
        "oMagmaTrail" => {}
        _ => {}
    }
}

pub(super) fn animation_end(game: &mut Game, e: &mut Entity, sprite: &str) {
    match (e.object.as_str(), sprite) {
        ("oHawkman", "sHawkThrowL") => {
            e.state = 0;
            e.set_sprite("sHawkLeft");
        }
        ("oTombLord", "sTombLordTurnL") => {
            e.facing = -1;
            e.state = 1;
        }
        ("oTombLord", "sTombLordTurnR") => {
            e.facing = 1;
            e.state = 1;
        }
        ("oTombLord", "sTombLordAttackL" | "sTombLordAttackR") => {
            e.state = 0;
            e.timer = 30;
            e.set_image_speed(0.25);
        }
        ("oMagmaMan", "sMagmaManDie") | ("oMagmaTrail", _) => e.alive = false,
        ("oMagma", _) if e.value("dying") != 0.0 => {
            let id = spawn(game, "oMagmaMan", Vec2::new(e.pos.x - 8.0, e.pos.y - 8.0));
            game.world.entity_mut(id).unwrap().health = e.health;
            e.alive = false;
        }
        _ => {}
    }
}

fn hurt_player(game: &mut Game, amount: i32, cause: Option<crate::statistics::EnemyDeath>) {
    if game.player.health > 0 {
        game.player.health -= amount;
        game.events.push(GameEvent::Damage { amount });
        if game.player.health <= 0 && crate::statistics::is_real_level(game) {
            if let Some(cause) = cause {
                game.lifetime_stats.enemy_death(cause);
            }
        }
    }
}

fn character_collision(game: &mut Game, e: &mut Entity) {
    use crate::statistics::EnemyDeath;
    if matches!(e.object.as_str(), "oMagmaMan" | "oMagma") {
        if game.player.invincible != 0 {
            return;
        }
        game.player.invincible = 30;
        game.player.data.insert("blink".into(), 30.0);
        game.player.state = crate::PlayerState::Stunned;
        game.player.stun = 20;
        game.player.data.insert("burning".into(), 100.0);
        game.player.vel = Vec2::new(
            if game.player.pos.x < e.pos.x {
                -6.0
            } else {
                6.0
            },
            -4.0,
        );
        crate::world_events::spawn_detritus(game, "oBlood", game.player.pos);
        hurt_player(
            game,
            2,
            (e.object == "oMagmaMan").then_some(EnemyDeath::MagmaMan),
        );
        game.sound("sndHurt", game.player.pos);
        game.sound("sndFlame", game.player.pos);
        return;
    }
    if e.object == "oFly" {
        if game.player.is_exiting() {
            return;
        }
        hurt_player(game, 2, Some(EnemyDeath::TombLord));
        game.player.vel = Vec2::new(e.vel.x, -4.0);
        game.player.state = crate::PlayerState::Stunned;
        game.player.stun = 20;
        crate::world_events::spawn_detritus(game, "oBlood", game.player.pos);
        game.sound("sndHurt", game.player.pos);
        e.alive = false;
        return;
    }
    let hawk = e.object == "oHawkman";
    if hawk && e.state >= 98 {
        return;
    }
    let (offset, width, top) = if hawk {
        (8.0, 8.0, 5.0)
    } else {
        (16.0, 16.0, 8.0)
    };
    if (game.player.pos.x - (e.pos.x + offset)).abs() > width {
        return;
    }
    let stomp = game.player.state != crate::PlayerState::Dead
        && (!hawk || !game.player.is_stunned())
        && matches!(
            game.player.state,
            crate::PlayerState::Jumping | crate::PlayerState::Falling
        )
        && game.player.pos.y < e.pos.y + top
        && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0;
    if stomp {
        game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
        let fall = game.player.data.get("fall_timer").copied().unwrap_or(0.0);
        let spiked = ["SpikeShoes", "spike_shoes", "Spike Shoes", "oSpikeShoes"]
            .iter()
            .any(|name| game.player.has(name));
        e.health -= ((fall / 16.0).floor() as i32 + 1) * if spiked { 3 } else { 1 };
        game.player.data.insert("fall_timer".into(), 0.0);
        super::set_counts_as_kill(e, true);
        if hawk {
            if spiked && e.value("bloodless") == 0.0 {
                crate::world_events::spawn_detritus(
                    game,
                    "oBlood",
                    Vec2::new(game.player.pos.x, game.player.pos.y + 8.0),
                );
            }
            e.state = 98;
            e.timer = e.value("stun_time") as u32;
            e.vel.y = -6.0;
            e.vel.x += if game.player.pos.x < e.pos.x + 8.0 {
                1.0
            } else {
                -1.0
            };
            e.set_image_speed(0.5);
        } else {
            spawn(game, "oBone", Vec2::new(e.pos.x + 16.0, e.pos.y));
        }
        game.sound("sndHit", e.pos);
    } else if game.player.invincible == 0 {
        if hawk && !game.world.solid_at(e.pos.x + 8.0, e.pos.y - 4.0) {
            if e.state == 3 {
                return;
            }
            e.facing = if game.player.pos.x > e.pos.x + 8.0 {
                1
            } else {
                -1
            };
            e.state = 3;
            e.vel.x = 0.0;
            e.set_sprite("sHawkThrowL");
            game.player.pos = Vec2::new(e.pos.x + if e.facing > 0 { 0.0 } else { 16.0 }, e.pos.y);
            game.player.previous_pos = game.player.pos;
            game.player.vel = Vec2::new(6.0 * e.facing as f64, -6.0);
            game.player.state = crate::PlayerState::Stunned;
            game.player.data.insert("bounced".into(), 0.0);
            super::drop_from_throw(game);
            game.player.data.insert("wall_hurt".into(), 1.0);
        } else {
            game.player.invincible = 30;
            game.player.data.insert("blink".into(), 30.0);
            game.player.vel.x = if game.player.pos.x < e.pos.x {
                -6.0
            } else {
                6.0
            };
            if hawk {
                crate::world_events::spawn_detritus(game, "oBlood", game.player.pos);
            } else if game.player.pos.y < e.pos.y {
                game.player.vel.y = -6.0;
            }
            hurt_player(
                game,
                if hawk { 1 } else { 2 },
                Some(if hawk {
                    EnemyDeath::Hawkman
                } else {
                    EnemyDeath::TombLord
                }),
            );
            game.sound("sndHurt", game.player.pos);
        }
    }
}

pub(super) fn collision(game: &mut Game, e: &mut Entity, other: Instance) -> bool {
    let object = other.object(game).to_owned();
    if other == Instance::Player
        && matches!(
            e.object.as_str(),
            "oHawkman" | "oTombLord" | "oFly" | "oMagmaMan" | "oMagma"
        )
    {
        character_collision(game, e);
        return true;
    }
    if e.object == "oOlmecSlam" && crate::catalog::is_a(&object, "oSolid") {
        match other {
            Instance::Entity(id) => {
                if let Some(target) = game.world.entity_mut(id) {
                    target.data.insert("cleanDeath".into(), 0.0);
                    target.alive = false;
                }
            }
            Instance::Terrain(index) => {
                game.world
                    .set_tile(index % game.world.width, index / game.world.width, "")
            }
            _ => {}
        }
        return true;
    }
    if e.object == "oFly" {
        if object == "oDamsel" {
            if let Instance::Entity(id) = other {
                fly_hits_damsel(game, e, id);
            }
            return true;
        }
        if super::is_enemy(&object) {
            if object != "oTombLord" {
                if let Instance::Entity(id) = other {
                    let target = game.world.entity_mut(id).unwrap();
                    target.health -= 2;
                    target.vel = Vec2::new(e.vel.x, -4.0);
                    target.state = 98;
                    target.timer = 20;
                    let pos = target.pos;
                    fly_blood(game, id);
                    game.sound("sndHit", pos);
                    e.alive = false;
                }
            }
            return true;
        }
        if crate::catalog::is_a(&object, "oSolid") {
            game.world.spawn("oSmokePuff", e.pos);
            game.sound("sndHit", e.pos);
            e.alive = false;
            return true;
        }
    }
    if matches!(e.object.as_str(), "oMagmaMan" | "oMagma") {
        if e.object == "oMagma" && crate::catalog::is_a(&object, "oWater") {
            game.world.spawn("oSmokePuff", e.pos);
            e.alive = false;
            return true;
        }
        if e.object == "oMagmaMan" && object == "oBomb" {
            if let Instance::Entity(id) = other {
                let bomb = game.world.entity_mut(id).unwrap();
                if bomb.sprite != "sBombArmed" {
                    bomb.set_sprite("sBombArmed");
                    bomb.set_image_speed(1.0);
                    bomb.data.insert("armed".into(), 1.0);
                    bomb.alarms[1] = 8 + game.rng.range(5) as i32;
                    bomb.timer = bomb.alarms[1] as u32;
                }
                let vx = 2.0 + game.rng.range(3) as f64;
                bomb.vel.x = if bomb.pos.x < e.pos.x { -vx } else { vx };
                if bomb.pos.y < e.pos.y {
                    bomb.vel.y = -(2.0 + game.rng.range(3) as f64);
                }
                if bomb.held {
                    game.player.held = None;
                }
            }
            return true;
        }
        if super::is_enemy(&object) {
            if object != "oMagmaMan" {
                if let Instance::Entity(id) = other {
                    let target = game.world.entity_mut(id).unwrap();
                    let flame = target.state != 98;
                    let pos = target.pos;
                    target.health -= 2;
                    target.vel = Vec2::new(if e.pos.x < target.pos.x { -3.0 } else { 3.0 }, -4.0);
                    target.data.insert("burning".into(), 100.0);
                    if e.object == "oMagma"
                        || !matches!(target.object.as_str(), "oTombLord" | "oYetiKing")
                    {
                        target.state = 98;
                        target.timer = 50;
                    }
                    if flame {
                        game.sound("sndFlame", pos);
                    }
                }
            }
            return true;
        }
    }
    false
}

fn fly_blood(game: &mut Game, id: u64) {
    let target = game.world.entity_mut(id).unwrap();
    let bloodless = target.value("bloodless") != 0.0;
    let blood_left = target.data.entry("blood_left".into()).or_insert(4.0);
    let blood = *blood_left > 0.0 && !bloodless;
    if *blood_left > 0.0 && target.health < 0 {
        *blood_left -= 1.0;
    }
    let (half_width, half_height) = match target.object.as_str() {
        "oJaws" => (24.0, 16.0),
        "oGiantSpider" | "oGiantSpiderHang" | "oYetiKing" | "oTombLord" | "oAlienBoss" => {
            (16.0, 16.0)
        }
        _ => (8.0, 8.0),
    };
    let pos = Vec2::new(target.pos.x + half_width, target.pos.y + half_height);
    if blood {
        let source = target.clone();
        super::create_blood(game, &source, pos, 1);
    }
}

fn fly_hits_damsel(game: &mut Game, e: &mut Entity, id: u64) {
    if game.world.entity(id).unwrap().value("invincible") != 0.0 {
        return;
    }
    fly_blood(game, id);
    let target = game.world.entity_mut(id).unwrap();
    if target.held {
        target.held = false;
        game.player.held = None;
    }
    target.health -= 2;
    target.vel = Vec2::new(e.vel.x * 0.3, -6.0);
    target.state = 2;
    target.timer = 120;
    let pos = target.pos;
    game.sound("sndDamsel", pos);
    e.alive = false;
}

fn ground(world: &World, e: &Entity) -> bool {
    let b = e.bounds();
    super::solid_mask(world, Rect::new(b.x, b.y + b.h, b.w, 1.0), e.id)
}
fn countdown(e: &mut Entity, key: &str) {
    let v = e.value(key);
    if v > 0.0 {
        e.data.insert(key.into(), v - 1.0);
    }
}
fn side(e: &Entity) -> &'static str {
    if e.facing < 0 { "L" } else { "R" }
}
fn spawn(game: &mut Game, object: &str, pos: Vec2) -> u64 {
    let id = game.world.spawn(object, pos);
    if let Some(e) = game.world.entity_mut(id) {
        super::init_entity(e);
        if object == "oOlmecDebris" {
            // Source instance_create finishes Create before the caller writes
            // the directional velocities used by Olmec's intro alarms.
            initialize_debris(&mut game.rng, e);
        } else if object == "oYellowBall" {
            initialize_yellow_ball(&mut game.rng, e);
        } else if object == "oFly" {
            e.vel.y = 0.5 - 3.0 * game.rng.unit();
        }
    }
    if object == "oOlmecSlam" {
        game.sound("sndSlam", pos);
    }
    id
}

fn hawk_idle_wall(world: &World, e: &Entity) -> bool {
    let point_is_solid = |point| {
        world.tiles_point_precision(point, "oSolid", crate::CollisionPrecision::Precise)
            || world.entities.iter().any(|other| {
                other.id != e.id
                    && other.alive
                    && other.engine_active
                    && crate::catalog::is_a(&other.object, "oSolid")
                    && other.mask_contains(point, crate::CollisionPrecision::Precise)
            })
    };
    point_is_solid(Vec2::new(e.pos.x - 1.0, e.pos.y))
        || point_is_solid(Vec2::new(e.pos.x + 16.0, e.pos.y))
}

fn hawk(game: &mut Game, e: &mut Entity) {
    let gravity = if e.held {
        0.0
    } else {
        super::inherited::gravity(e, 0.6)
    };
    let impact_y = (e.vel.y + gravity).min(10.0);
    let (wall, landed) = move_body(&game.world, e, gravity);
    let grounded = landed || ground(&game.world, e);
    // oHawkman Step checks the lower corpse origin after moveTo, and the
    // center for a live, unheld actor. This must not run before movement.
    let crush_y = if e.state >= 98 { 12.0 } else { 8.0 };
    if (e.state >= 98 || !e.held) && game.world.solid_at(e.pos.x + 8.0, e.pos.y + crush_y) {
        super::create_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 3);
        e.alive = false;
        game.sound("sndCavemanDie", e.pos);
        return;
    }
    if e.state != 98 && e.state != 99 && e.health < 1 {
        e.state = 99;
    }
    let body_step = e.state >= 98;
    if e.state == 99 {
        // A moving corpse may return to STUNNED, but the source DEAD entry
        // credits the kill and plays its sound only once.
        if e.value("dead") == 0.0 {
            super::record_death(game, e);
            game.sound("sndCavemanDie", e.pos);
            e.data.insert("dead".into(), 1.0);
        }
        e.set_sprite("sHawkDeadL");
        if e.vel.x != 0.0 || e.vel.y != 0.0 {
            e.state = 98;
        }
    } else if e.state == 98 {
        e.set_sprite(if e.vel.x == 0.0 && e.health > 0 {
            "sHawkStunL"
        } else if e.value("bounced") != 0.0 {
            if e.vel.y < 0.0 {
                "sHawkBounceL"
            } else {
                "sHawkFallL"
            }
        } else if e.vel.x.abs() > 0.0 {
            "sHawkDieLL"
        } else {
            "sHawkDieLR"
        });
        if grounded && e.value("bounced") == 0.0 {
            e.data.insert("bounced".into(), 1.0);
            super::create_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 1);
        }
        if e.held || grounded {
            if e.timer > 0 {
                e.timer -= 1;
            } else if e.health > 0 {
                e.state = 0;
                if e.held {
                    e.held = false;
                    game.player.held = None;
                    game.player.pickup_item_type.clear();
                }
            }
        }
    }
    if body_step {
        if e.state >= 98 {
            super::settle_body(&game.world, e, impact_y);
            if e.vel.x == 0.0 && e.vel.y == 0.0 && e.health < 1 {
                e.state = 99;
            }
        }
        e.vel.x = (e.vel.x.abs() - 0.1).max(0.0) * e.vel.x.signum();
        if e.vel.x.abs() < 0.5 {
            e.vel.x = 0.0;
        }
        if e.held {
            e.set_sprite(if e.health > 0 {
                "sHawkHeldL"
            } else {
                "sHawkDHeldL"
            });
        } else if e.state < 98 {
            e.set_sprite(if e.vel.x != 0.0 {
                "sHawkRunLeft"
            } else {
                "sHawkLeft"
            });
        }
        return;
    }
    // IDLE/WALK cast after their own facing and status changes, even if WALK
    // selects IDLE at a ledge. Other incoming states do not cast this Step.
    let casts_sight = matches!(e.state, 0 | 1);
    match e.state {
        0 => {
            if grounded && hawk_idle_wall(&game.world, e) {
                e.vel.y = -6.0;
                e.vel.x = e.facing as f64;
                e.timer = e.timer.saturating_sub(10);
            }
            if grounded && e.timer > 0 {
                e.timer -= 1;
            }
            if e.timer == 0 {
                e.facing = if game.rng.chance(1, 2) { -1 } else { 1 };
                e.state = 1;
            }
        }
        1 => {
            if wall {
                e.facing = -e.facing;
            }
            e.vel.x = 1.5 * e.facing as f64;
            if !game.world.solid_at(
                e.pos.x + if e.facing < 0 { -1.0 } else { 16.0 },
                e.pos.y + 16.0,
            ) {
                e.state = 0;
                e.timer = 20 + game.rng.range(31);
                e.vel.x = 0.0;
            }
            // Source WALK still rolls after a ledge changes status to IDLE.
            // A successful roll draws and overwrites the first counter.
            if game.rng.chance(1, 100) {
                e.state = 0;
                e.timer = 20 + game.rng.range(31);
                e.vel.x = 0.0;
            }
        }
        2 => {
            e.set_image_speed(1.0);
            if wall {
                e.facing = -e.facing;
            }
            e.vel.x = 3.0 * e.facing as f64;
            if grounded && !game.world.solid_at(e.pos.x, e.pos.y - 16.0) {
                e.vel.y = -6.0;
            }
        }
        3 => {
            e.vel.x = 0.0;
        }
        _ => {}
    }
    if casts_sight {
        let timer = e.value("sight_timer");
        if timer <= 0.0 {
            let id = game.spawn("oEnemySight", e.pos);
            if let Some(sight) = game.world.entity_mut(id) {
                // Source direction/speed are engine motion, not xVel/yVel.
                sight
                    .data
                    .insert("hspeed".into(), f64::from(e.facing) * 10.0);
                sight.data.insert("vspeed".into(), 0.0);
                sight.data.insert("owner".into(), e.id as f64);
            }
            e.data.insert("sight_timer".into(), 5.0);
        } else {
            e.data.insert("sight_timer".into(), timer - 1.0);
        }
    }
    // oHawkman Step applies friction after choosing state velocity.
    if e.vel.x > 0.0 {
        e.vel.x -= 0.1;
    } else if e.vel.x < 0.0 {
        e.vel.x += 0.1;
    }
    if e.vel.x.abs() < 0.5 {
        e.vel.x = 0.0;
    }
    if e.state != 3 {
        e.set_sprite(if e.vel.x.abs() > 0.0 {
            "sHawkRunLeft"
        } else {
            "sHawkLeft"
        });
    }
}

fn tomb_lord(game: &mut Game, e: &mut Entity) {
    let (wall, _) = move_body(&game.world, e, 0.6);
    // oTombLord Step: the source center-in-solid death check follows moveTo.
    if game.world.solid_at(e.pos.x + 16.0, e.pos.y + 16.0) {
        e.health = 0;
    }
    if e.health <= 0 {
        // The caller evaluates both rand operands even when scrCreateBlood
        // returns immediately for the Tomb Lord's bloodless Create default.
        let blood_pos = Vec2::new(
            e.pos.x + 14.0 + f64::from(game.rng.range(5)),
            e.pos.y + 14.0 + f64::from(game.rng.range(5)),
        );
        super::create_blood(game, e, blood_pos, 4);
        for _ in 0..4 {
            let pos = Vec2::new(
                e.pos.x + 14.0 + f64::from(game.rng.range(5)),
                e.pos.y + 12.0 + f64::from(game.rng.range(7)),
            );
            // Complete inherited/own Bone Create before the next origin rolls.
            game.spawn("oBone", pos);
        }
        if game.world.level == 13 {
            game.spawn("oSceptre", Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
        }
        super::record_death(game, e);
        e.alive = false;
        // instance_destroy retires the receiver, but does not return from its
        // current Step. In particular, an ATTACK can still create a Fly below.
    }
    let bounds = e.bounds();
    let left = bounds.x.round_ties_even();
    let right = (bounds.x + bounds.w - 1.0).round_ties_even();
    let bottom = (bounds.y + bounds.h).round_ties_even();
    if e.state != 98
        && super::solid_mask(
            &game.world,
            Rect::new(left, bottom, right - left + 1.0, 1.0),
            e.id,
        )
    {
        e.vel.y = 0.0;
    }
    countdown(e, "attack_cooldown");
    countdown(e, "hit_cooldown");
    if e.state >= 98 {
        e.state = 1;
    }
    match e.state {
        0 => {
            e.vel.x = 0.0;
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 1;
            }
        }
        1 => {
            let d = game.player.pos.x - center(e).x;
            if wall
                || (d * (e.facing as f64) < 0.0
                    && (game.player.pos.y - (e.pos.y + 32.0)).abs() < 16.0
                    && e.timer == 0)
            {
                e.state = 2;
                e.timer = 30;
                e.vel.x = 0.0;
                e.set_sprite(if e.facing < 0 {
                    "sTombLordTurnR"
                } else {
                    "sTombLordTurnL"
                });
            } else if d * e.facing as f64 > 0.0
                && (game.player.pos.y - center(e).y).abs() < 32.0
                && e.value("attack_cooldown") == 0.0
            {
                e.state = 3;
                e.animation = 0.0;
                e.vel.x = 0.0;
                e.set_sprite(&format!("sTombLordAttack{}", side(e)));
            } else {
                e.vel.x = e.facing as f64;
                e.set_sprite(&format!("sTombLordWalk{}", side(e)));
            }
        }
        2 => {
            e.vel.x = 0.0;
        }
        3 => {
            e.vel.x = 0.0;
            e.set_image_speed(0.5);
            // Step refreshes attackTimer throughout the attack animation.
            e.data.insert("attack_cooldown".into(), 100.0);
            if (7.0..=12.0).contains(&e.animation) {
                let pos = Vec2::new(
                    e.pos.x + if e.facing < 0 { 8.0 } else { 24.0 },
                    e.pos.y + 12.0 + game.rng.range(5) as f64,
                );
                let id = spawn(game, "oFly", pos);
                let vx = (3 + game.rng.range(3)) as f64 * e.facing as f64;
                if let Some(f) = game.world.entity_mut(id) {
                    f.vel.x = vx;
                }
            }
        }
        _ => {}
    }
    if e.state == 1 && e.timer > 0 {
        e.timer -= 1;
    }
    // The source's post-state correction also executes after destruction.
    if super::solid_mask(&game.world, e.bounds(), e.id) {
        e.pos.y -= 2.0;
    }
    if e.state == 1 {
        e.set_sprite(&format!("sTombLordWalk{}", side(e)));
    } else if e.state == 0 {
        e.set_sprite(if e.facing < 0 {
            "sTombLordLeft"
        } else {
            "sTombLordRight"
        });
    }
}

fn fly(_game: &mut Game, e: &mut Entity) {
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    e.set_sprite(if e.vel.x < 0.0 {
        "sFlyLeft"
    } else {
        "sFlyRight"
    });
}

fn olmec_rect(e: &Entity) -> Rect {
    Rect::new(e.pos.x + 2.0, e.pos.y, 60.0, 64.0)
}
fn olmec_ground(world: &World, e: &Entity) -> bool {
    super::solid_mask(
        world,
        Rect::new(e.pos.x + 2.0, e.pos.y + 64.0, 60.0, 1.0),
        e.id,
    )
}
fn olmec_move(world: &World, e: &mut Entity) -> bool {
    // Platform Engine/moveTo.gml samples fractional displacement against the
    // global simulation tick, never by accumulating fractional world positions.
    fn displacement(v: f64, time: u64) -> i32 {
        let f = v.abs().fract();
        let extra = f > 0.0 && time.is_multiple_of((1.0 / f).round_ties_even().max(1.0) as u64);
        (v.abs().floor() as i32 + i32::from(extra)) * if v < 0.0 { -1 } else { 1 }
    }
    let mut ceiling = false;
    for horizontal in [true, false] {
        let v = if horizontal { e.vel.x } else { e.vel.y };
        let n = displacement(v, e.value("simulation_tick") as u64);
        for _ in 0..n.unsigned_abs() {
            let b = olmec_rect(e);
            let edge = if horizontal {
                Rect::new(
                    if n < 0 { b.x - 1.0 } else { b.x + b.w },
                    b.y + 5.0,
                    1.0,
                    b.h - 5.0,
                )
            } else {
                Rect::new(b.x, if n < 0 { b.y - 1.0 } else { b.y + b.h }, b.w, 1.0)
            };
            if super::solid_mask(world, edge, e.id) {
                if horizontal {
                    e.vel.x = 0.0;
                } else if v < 0.0 {
                    ceiling = true;
                }
                break;
            }
            if horizontal {
                e.pos.x += f64::from(n.signum());
            } else {
                e.pos.y += f64::from(n.signum());
            }
        }
    }
    ceiling
}

/// Classic alarms precede player/object Step events. Keep the intro's alarm
/// chain separate from Olmec's movement/counter states, especially Alarm 5,
/// which restores player control before that frame's player Step.
pub(super) fn alarm(game: &mut Game, e: &mut Entity, index: u8) {
    match (e.object.as_str(), index) {
        ("oOlmec", 1..=6) => olmec_alarm(game, e, index),
        ("oOlmecSlam", 0) => e.alive = false,
        ("oHawkman" | "oMagmaMan", 0) => {
            e.data.insert("whipped".into(), 0.0);
        }
        ("oMagma", 0) => {
            spawn(game, "oMagmaTrail", e.pos);
            e.alarms[0] = 2;
        }
        ("oYellowBall", 0) => {
            e.data.insert("invincible".into(), 0.0);
        }
        ("oYellowBall", 1) => {
            if game
                .world
                .entities
                .iter()
                .filter(|trail| trail.alive && trail.object == "oYellowTrail")
                .count()
                < 12
            {
                game.world.spawn("oYellowTrail", e.pos);
            }
            e.alarms[1] = 4;
        }
        _ => {}
    }
}

fn olmec_alarm(game: &mut Game, boss: &mut Entity, index: u8) {
    match index {
        1..=4 => {
            let stage = u32::from(index - 1);
            boss.data.insert("intro_stage".into(), f64::from(stage + 1));
            if stage < 3 {
                boss.set_sprite(match stage {
                    0 => "sOlmecStart2",
                    1 => "sOlmecStart3",
                    _ => "sOlmec",
                });
                for _ in 0..if stage < 2 { 6 } else { 12 } {
                    let pos = match stage {
                        0 => Vec2::new(
                            boss.pos.x + 32.0 + f64::from(game.rng.range(33)),
                            boss.pos.y + f64::from(game.rng.range(33)),
                        ),
                        1 => Vec2::new(
                            boss.pos.x + f64::from(game.rng.range(33)),
                            boss.pos.y + f64::from(game.rng.range(33)),
                        ),
                        _ => Vec2::new(
                            boss.pos.x + f64::from(game.rng.range(65)),
                            boss.pos.y + 32.0 + f64::from(game.rng.range(33)),
                        ),
                    };
                    let debris = spawn(game, "oOlmecDebris", pos);
                    // instance_create consumes the debris Create RNG before
                    // the caller writes either directional launch override.
                    let velocity = match stage {
                        0 => Vec2::new(
                            1.0 + f64::from(game.rng.range(4)),
                            -(1.0 + f64::from(game.rng.range(3))),
                        ),
                        1 => Vec2::new(
                            -(1.0 + f64::from(game.rng.range(4))),
                            -(1.0 + f64::from(game.rng.range(3))),
                        ),
                        _ => Vec2::new(
                            f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
                            -(1.0 + f64::from(game.rng.range(3))),
                        ),
                    };
                    game.world.entity_mut(debris).unwrap().vel = velocity;
                }
                game.sound("sndThump", boss.pos);
                boss.alarms[usize::from(index + 1)] = 50;
                if stage == 0 {
                    let worshippers: Vec<_> = game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oHawkmanWorship")
                        .map(|e| (e.id, e.pos))
                        .collect();
                    for (old_id, pos) in worshippers {
                        game.world.entity_mut(old_id).unwrap().alive = false;
                        let new_id = spawn(game, "oHawkman", pos);
                        let hawk = game.world.entity_mut(new_id).unwrap();
                        hawk.health = 1;
                        hawk.state = 98;
                        hawk.timer = 300;
                        hawk.vel = Vec2::new(-3.0, -5.0);
                    }
                    for caveman in game
                        .world
                        .entities
                        .iter_mut()
                        .filter(|e| e.alive && e.object == "oCavemanWorship")
                    {
                        caveman.animation = 0.0;
                        caveman.data.insert("animation_speed".into(), 0.0);
                    }
                }
            } else {
                boss.state = 1;
                boss.timer = 0;
                boss.data.insert("toggle".into(), 0.0);
                boss.alarms[6] = 20;
                game.sound("sndBigJump", boss.pos);
                game.sound("sndAlert", boss.pos);
            }
        }
        6 => {
            let worshippers: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oCavemanWorship")
                .map(|e| (e.id, e.pos))
                .collect();
            for (old_id, pos) in worshippers {
                game.world.entity_mut(old_id).unwrap().alive = false;
                let new_id = spawn(game, "oCaveman", pos);
                let caveman = game.world.entity_mut(new_id).unwrap();
                caveman.facing = 1;
                caveman.state = 2;
            }
        }
        5 => {
            game.player.data.insert("active".into(), 1.0);
            game.camera.x = 0.0;
            boss.state = 0;
            boss.timer = 100;
            crate::music::play(game, "musBoss", true);
        }
        _ => {}
    }
}

fn olmec(game: &mut Game, e: &mut Entity) {
    // Olmec has an additional raw-velocity carry in its own Step, after the
    // oGame moving-solid pass. This deliberately differs from moveTo's integer
    // displacement, and includes the source's persistent carryPlayer flag.
    if e.value("carryPlayer") != 0.0
        || Rect::new(e.pos.x - 1.0, e.pos.y, 68.0, 63.0).intersects(game.player.sprite_bounds())
    {
        game.player.pos.x += e.vel.x;
        game.player.pos.y += e.vel.y;
    }
    let ceiling = olmec_move(&game.world, e);
    if ceiling {
        spawn(game, "oOlmecSlam", Vec2::new(e.pos.x, e.pos.y - 16.0));
        if e.vel.y < 0.0 {
            e.vel.y = -e.vel.y * 0.8;
        }
    }
    let grounded = olmec_ground(&game.world, e);
    e.data.insert(
        "carryPlayer".into(),
        f64::from(
            Rect::new(e.pos.x, e.pos.y - 2.0, 65.0, 67.0).intersects(game.player.sprite_bounds()),
        ),
    );
    // Classic tests the left edge at the feet/head. Authored arenas store
    // lava as water-family entities, while custom levels can also use tiles.
    let lava = |world: &World, point: Vec2| {
        world
            .tile(
                (point.x / 16.0).floor() as i32,
                (point.y / 16.0).floor() as i32,
            )
            .is_some_and(|tile| tile.object == "oLava")
            || world
                .entities
                .iter()
                .any(|lava| lava.alive && lava.object == "oLava" && lava.bounds().contains(point))
    };
    if lava(&game.world, Vec2::new(e.pos.x, e.pos.y - 2.0)) {
        super::record_death(game, e);
        e.alive = false;
        game.flags.insert("olmec_dead".into());
        game.events
            .push(GameEvent::Message("Olmec has fallen!".into()));
        return;
    }
    if lava(&game.world, Vec2::new(e.pos.x, e.pos.y + 64.0)) {
        e.state = 4;
    }
    if e.state != 5 && e.state != 4 {
        e.vel.y = (e.vel.y + 0.2).min(6.0);
    }
    match e.state {
        -1 => {
            if game.camera.x < 176.0 {
                game.camera.x += 2.0;
            } else {
                e.state = -2;
                e.alarms[1] = 100;
            }
            if grounded {
                e.vel.y = 0.0;
            }
        }
        -2 => {
            if grounded {
                e.vel.y = 0.0;
            }
        }
        0 => {
            e.data.insert("toggle".into(), 0.0);
            if grounded {
                e.vel.y = 0.0;
            }
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 1;
            }
        }
        1 => {
            if grounded {
                e.vel.y = -4.0;
            } else {
                e.state = 2;
                e.timer = 10;
                game.sound("sndBigJump", e.pos);
            }
        }
        2 => {
            if grounded {
                e.state = 0;
                e.timer = 40 + game.rng.range(61);
                e.vel = Vec2::default();
                game.sound("sndThump", e.pos);
            } else {
                if e.timer > 1 {
                    e.timer -= 1;
                } else if e.timer == 1 {
                    e.vel.x = if game.player.pos.x < e.pos.x {
                        -0.25
                    } else if game.player.pos.x > e.pos.x + 64.0 {
                        0.25
                    } else {
                        0.0
                    };
                    e.timer = 0;
                } else {
                    let a = if e.value("toggle") == 0.0 {
                        0.25
                    } else {
                        -0.25
                    };
                    e.vel.x += a * e.vel.x.signum();
                    if e.vel.x.abs() >= 2.0 {
                        e.data
                            .insert("toggle".into(), if a > 0.0 { 1.0 } else { 0.0 });
                    }
                }
                let inactive = game.player.data.get("active") == Some(&0.0);
                if (inactive && e.vel.y >= 0.0)
                    || (game.player.pos.y > e.pos.y
                        && (game.player.pos.x - (e.pos.x + 32.0)).abs() < 32.0
                        && e.vel.x > -1.0)
                {
                    e.state = 5;
                    e.timer = 20;
                    e.vel = Vec2::default();
                }
            }
        }
        4 => {
            e.vel = Vec2::new(0.0, 0.1);
            crate::camera_shake::request(game, 10.0);
            game.sound_if_idle("sndFlame", e.pos);
        }
        5 => {
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 6;
                e.vel.y = 5.0;
                e.data.insert("slammed".into(), 0.0);
            }
        }
        6 => {
            if grounded {
                if e.value("slammed") == 0.0 {
                    spawn(game, "oOlmecSlam", Vec2::new(e.pos.x, e.pos.y + 64.0));
                    e.data.insert("slammed".into(), 1.0);
                    crate::camera_shake::request(game, 5.0);
                } else {
                    let idle = game.rng.chance(1, 2);
                    let inactive = game.player.data.get("active") == Some(&0.0);
                    e.state = if idle || inactive { 0 } else { 7 };
                    e.vel = Vec2::default();
                    e.timer = 60;
                    if inactive {
                        e.alarms[5] = 50;
                    }
                }
            }
        }
        7 => {
            let target = Vec2::new(e.pos.x + 32.0, e.pos.y + 16.0);
            for _ in 0..6 {
                let pos = Vec2::new(
                    e.pos.x + 32.0 + f64::from(game.rng.range(33)) - f64::from(game.rng.range(33)),
                    e.pos.y + 14.0 + f64::from(game.rng.range(33)) - f64::from(game.rng.range(33)),
                );
                crate::world_events::spawn_psychic_create_2(game, pos, target);
            }
            for _ in 0..3 {
                spawn(
                    game,
                    "oYellowBall",
                    Vec2::new(e.pos.x + 32.0, e.pos.y + 16.0),
                );
            }
            game.sound("sndPsychic", e.pos);
            e.state = 0;
        }
        _ => {}
    }
}

fn slam(_game: &mut Game, _e: &mut Entity) {}

fn initialize_debris(rng: &mut crate::Rng, e: &mut Entity) {
    if e.value("started") == 0.0 {
        e.vel = Vec2::new(rng.unit() * 4.0 - rng.unit() * 4.0, -1.0 - rng.unit() * 2.0);
        e.data.insert("started".into(), 1.0);
        e.data.insert("gravity".into(), 0.6);
        e.data.insert("invincible".into(), 1.0);
        for (key, value) in [
            ("collision_left", -4.0),
            ("collision_top", -4.0),
            ("collision_right", 4.0),
            ("collision_bottom", 4.0),
        ] {
            e.data.insert(key.into(), value);
        }
        e.set_sprite(match rng.range(3) {
            0 => "sOlmecDebris2",
            1 => "sOlmecDebris3",
            _ => "sOlmecDebris",
        });
    }
}
fn debris(game: &mut Game, e: &mut Entity) {
    initialize_debris(&mut game.rng, e);
    // Unlike oDetritus, this object moves directly through other solids.
    // Its only bounce probe is a bounding-box point against oTemple.
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    if e.vel.y < 6.0 {
        e.vel.y += 0.6;
    }
    if super::inherited::point(&game.world, Vec2::new(e.pos.x, e.pos.y + 4.0), "oTemple") {
        if e.vel.y > 1.0 {
            e.vel.y *= -0.4;
        } else {
            game.world.spawn("oSmokePuff", e.pos);
            e.alive = false;
        }
        e.vel.x = if e.vel.x.abs() < 0.1 {
            0.0
        } else {
            e.vel.x * 0.3
        };
    }
}
fn initialize_yellow_ball(rng: &mut crate::Rng, e: &mut Entity) {
    if e.value("launch") != 0.0 {
        // Source Create consumes yVel, xVel, then the sign draw, immediately
        // after its caller's preceding emissions have been created.
        let vy = -4.0 - rng.unit() * 3.0;
        let mut vx = f64::from(2 + rng.range(4));
        if rng.chance(1, 2) {
            vx = -vx;
        }
        e.vel = Vec2::new(vx, vy);
        e.data.insert("launch".into(), 0.0);
    }
}
fn yellow_ball(game: &mut Game, e: &mut Entity) {
    initialize_yellow_ball(&mut game.rng, e);
    let old = e.pos;
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    e.vel.y = (e.vel.y + 0.15).min(6.0);
    let boss_overlaps = game
        .world
        .entities
        .iter()
        .any(|o| o.object == "oOlmec" && olmec_rect(o).intersects(e.bounds()));
    if game.world.collides(e.bounds()) && !boss_overlaps {
        let n = if game.rng.chance(1, 2) {
            game.rng.range(4)
        } else {
            game.rng.range(5)
        };
        spawn(
            game,
            ["oBat", "oSpider", "oSnake", "oFrog", "oFireFrog"][n as usize],
            Vec2::new(old.x - 8.0, old.y - 8.0),
        );
        e.alive = false;
    }
}

fn magma_man(game: &mut Game, e: &mut Entity) {
    if e.health > 0 {
        e.health -= 1;
    }
    if e.health <= 0 || game.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0) {
        e.state = 99;
        e.vel = Vec2::default();
        e.set_sprite("sMagmaManDie");
    }
    let grounded = ground(&game.world, e);
    if !grounded {
        // Unlike the voluntary IDLE transformation below, the source falling
        // branch creates a fresh 200-hp magma projectile without copying hp.
        crate::world_events::spawn_detritus(
            game,
            "oMagma",
            Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0),
        );
        e.alive = false;
        // instance_destroy continues the source event, including its burn roll.
    }
    if game.rng.chance(1, 20) {
        let pos = Vec2::new(
            e.pos.x + f64::from(4 + game.rng.range(9)),
            e.pos.y + f64::from(4 + game.rng.range(9)),
        );
        game.spawn("oBurn", pos);
    }
    e.data.insert("burning".into(), e.value("burning") - 1.0);
    let b = e.bounds();
    let wall = super::solid_mask(
        &game.world,
        Rect::new(
            if e.facing < 0 { b.x - 1.0 } else { b.x + b.w },
            b.y,
            1.0,
            b.h,
        ),
        e.id,
    );
    e.vel.y = 0.0;
    if e.state == 0 {
        if grounded && e.timer > 0 {
            e.timer -= 1;
        }
        if e.timer == 0 {
            e.facing = if game.rng.chance(1, 2) { -1 } else { 1 };
            e.state = 1;
            if game.rng.chance(1, 6) {
                let id = crate::world_events::spawn_detritus(
                    game,
                    "oMagma",
                    Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0),
                );
                if let Some(m) = game.world.entity_mut(id) {
                    m.health = e.health;
                }
                e.alive = false;
            }
        }
    } else if e.state == 1 {
        if wall {
            e.facing = -e.facing;
        }
        e.vel.x = 1.5 * e.facing as f64;
        if !game.world.solid_at(
            e.pos.x + if e.facing < 0 { -1.0 } else { 16.0 },
            e.pos.y + 16.0,
        ) {
            e.state = 0;
            e.timer = 20 + game.rng.range(31);
            e.vel.x = 0.0;
        }
        // This is independent of the preceding ledge branch in the source.
        if game.rng.chance(1, 100) {
            e.state = 0;
            e.timer = 20 + game.rng.range(31);
            e.vel.x = 0.0;
        }
    }
    move_body(&game.world, e, 0.0);
    if e.state < 98 && e.state != 3 {
        e.set_sprite(if e.vel.x == 0.0 {
            "sMagmaManLeft"
        } else {
            "sMagmaManWalkL"
        });
    }
}
fn magma(game: &mut Game, e: &mut Entity) {
    if e.value("started") == 0.0 {
        crate::world_events::initialize_detritus(game, e);
        e.data.insert("started".into(), 1.0);
    } else if e.value("world_effect_initialized") == 0.0 {
        // Existing launch overrides are already post-Create. Preserve them.
        e.data.insert("world_effect_initialized".into(), 1.0);
        e.data.entry("life".into()).or_insert(60.0);
    }
    crate::world_events::detritus(game, e);
    if !e.alive {
        return;
    }
    if super::solid_mask(
        &game.world,
        Rect::new(e.pos.x - 8.0, e.pos.y + 8.0, 16.0, 1.0),
        e.id,
    ) {
        e.vel = Vec2::default();
        e.set_sprite("sMagmaManCreate");
        e.data.insert("dying".into(), 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Area, RulesPreset};

    // Independent static prediction from canonical EXE bytes: one unsigned
    // LCG state per draw, scaled by 2^-32. All source widths here are <=2^21,
    // so the integer product is exact even in a binary64 real wrapper.
    #[derive(Clone)]
    struct CanonicalOracle(u32);
    impl CanonicalOracle {
        fn checkpoint(rng: &crate::Rng) -> Self {
            assert_eq!(rng.algorithm(), crate::RngAlgorithm::Gm8Lcg32V1);
            Self(u32::try_from(rng.state_bits()).unwrap())
        }
        fn next_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(0x0808_8405).wrapping_add(1);
            u64::from(self.0)
        }
        fn draw(&mut self, width: u64) -> u64 {
            assert!(width <= 1 << 21, "oracle width must stay source-exact");
            (self.next_u64() * width) >> 32
        }
    }

    // Literal WALK statements from oHawkman Step.xml 106-139 and
    // oMagmaMan Step.xml 66-99. Each rand(a,b) consumes one draw. This oracle
    // intentionally does not call production range/chance or WALK helpers.
    // MagmaMan controls separately include the preceding literal burn prefix.
    fn literal_magma_burn_prefix(rng: &mut CanonicalOracle) {
        if 1 + rng.draw(20) == 1 {
            rng.next_u64(); // rand(4,12), x
            rng.next_u64(); // rand(4,12), y
        }
    }

    fn literal_temple_walk(
        rng: &mut CanonicalOracle,
        ledge: bool,
        facing: i8,
    ) -> (i32, u32, f64, bool) {
        let mut state = 1;
        let mut counter = 37;
        let mut x_vel = 1.5 * f64::from(facing);
        if ledge {
            state = 0;
            counter = 20 + (rng.draw(31)) as u32;
            x_vel = 0.0;
        }
        let idle_roll = 1 + rng.draw(100);
        if idle_roll == 1 {
            state = 0;
            counter = 20 + (rng.draw(31)) as u32;
            x_vel = 0.0;
        }
        (state, counter, x_vel, idle_roll == 1)
    }

    fn temple_walk_fixture(object: &str, facing: i8, ledge: bool) -> (Game, u64) {
        let mut game = room();
        if ledge {
            // Feet remain supported by tile 5, while x-1 and x+16 have no
            // forward support. This reaches WALK, not MagmaMan's falling exit.
            game.world = World::new(24, 20, Area::Temple, 13);
            game.world.set_tile(5, 10, "oTemple");
        }
        let id = game.spawn(object, Vec2::new(80.0, 144.0));
        let actor = game.world.entity_mut(id).unwrap();
        actor.state = 1;
        actor.timer = 37;
        actor.facing = facing;
        actor.vel = Vec2::default();
        actor.data.insert("sight_timer".into(), 5.0);
        (game, id)
    }

    fn assert_temple_walk_actor_step(object: &str) {
        // Actor-specific canonical controls account for MagmaMan's preceding
        // burn roll and its two conditional coordinate draws. Hawkman seed 6
        // draws ledge counter 25, idle 1, then counter 48. MagmaMan seed 18 first
        // skips burning, then draws ledge counter 49, idle 1, then counter 34.
        // The later counter must win for both actors, in both directions.
        let cases = match object {
            "oHawkman" => [
                (false, false, 1),
                (false, true, 0),
                (true, false, 0),
                (true, true, 6),
            ],
            "oMagmaMan" => [
                (false, false, 0),
                (false, true, 6),
                (true, false, 0),
                (true, true, 18),
            ],
            _ => panic!("unexpected WALK fixture actor {object}"),
        };
        for (ledge, idle_success, seed) in cases {
            for facing in [-1, 1] {
                let (mut game, id) = temple_walk_fixture(object, facing, ledge);
                game.rng = crate::model::Rng::new(seed);
                let mut expected_rng = CanonicalOracle::checkpoint(&game.rng);
                if object == "oMagmaMan" {
                    literal_magma_burn_prefix(&mut expected_rng);
                }
                let (state, counter, mut x_vel, actual_idle) =
                    literal_temple_walk(&mut expected_rng, ledge, facing);
                assert_eq!(
                    actual_idle, idle_success,
                    "{object} ledge={ledge} seed={seed}"
                );
                if object == "oHawkman" {
                    x_vel = (x_vel.abs() - 0.1).max(0.0) * x_vel.signum();
                }
                super::super::step_instance(&mut game, id);
                let actor = game.world.entity(id).expect("WALK actor survives");
                let case = format!("{object} ledge={ledge} idle={idle_success} facing={facing}");
                assert_eq!(actor.state, state, "{case}");
                assert_eq!(actor.timer, counter, "{case}");
                assert_eq!(actor.facing, facing, "{case}");
                assert!((actor.vel.x - x_vel).abs() < 1e-12, "{case}");
                for _ in 0..8 {
                    assert_eq!(game.rng.next_u64(), expected_rng.next_u64(), "{case}");
                }
            }
        }
    }

    #[test]
    fn actual_hawkman_step_temple_walk_ledge_rng() {
        assert_temple_walk_actor_step("oHawkman");
    }

    #[test]
    fn actual_magma_man_step_temple_walk_ledge_rng() {
        assert_temple_walk_actor_step("oMagmaMan");
    }

    #[test]
    fn actual_airborne_hawkman_step_temple_walk_ledge_rng() {
        let (mut game, id) = temple_walk_fixture("oHawkman", 1, true);
        game.world.entity_mut(id).unwrap().pos.y = 112.0;
        game.rng = crate::model::Rng::new(6);
        let mut expected_rng = CanonicalOracle::checkpoint(&game.rng);
        let (state, counter, x_vel, idle_success) = literal_temple_walk(&mut expected_rng, true, 1);
        assert!(idle_success);
        super::super::step_instance(&mut game, id);
        let actor = game.world.entity(id).unwrap();
        assert_eq!(
            (actor.state, actor.timer, actor.vel.x),
            (state, counter, x_vel)
        );
        assert!(actor.vel.y > 0.0, "this control is airborne");
        for _ in 0..8 {
            assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
        }
    }

    #[test]
    fn actual_sidewall_turn_temple_walk_ledge_rng() {
        for object in ["oHawkman", "oMagmaMan"] {
            for facing in [-1, 1] {
                let (mut game, id) = temple_walk_fixture(object, facing, false);
                let wall_x = if facing < 0 { 4 } else { 6 };
                game.world.set_tile(wall_x, 9, "oTemple");
                let actor = game.world.entity_mut(id).unwrap();
                // The body bounds touch the wall; Hawkman moveTo encounters
                // it before the WALK block. Neither actor starts embedded.
                actor.pos.x = if facing < 0 { 78.0 } else { 82.0 };
                actor.vel.x = 2.5 * f64::from(facing);
                game.rng = crate::model::Rng::new(1);
                let mut expected_rng = CanonicalOracle::checkpoint(&game.rng);
                if object == "oMagmaMan" {
                    literal_magma_burn_prefix(&mut expected_rng);
                }
                let (state, counter, mut x_vel, idle_success) =
                    literal_temple_walk(&mut expected_rng, false, -facing);
                if object == "oHawkman" {
                    assert!(!idle_success);
                }
                if object == "oHawkman" {
                    x_vel = (x_vel.abs() - 0.1).max(0.0) * x_vel.signum();
                }
                super::super::step_instance(&mut game, id);
                let actor = game.world.entity(id).unwrap();
                assert_eq!(actor.facing, -facing, "{object}");
                assert_eq!((actor.state, actor.timer), (state, counter), "{object}");
                assert!((actor.vel.x - x_vel).abs() < 1e-12, "{object}");
                for _ in 0..8 {
                    assert_eq!(game.rng.next_u64(), expected_rng.next_u64(), "{object}");
                }
            }
        }
    }

    #[test]
    fn actual_idle_entry_does_not_fall_through_temple_walk_ledge_rng() {
        for object in ["oHawkman", "oMagmaMan"] {
            let (mut game, id) = temple_walk_fixture(object, 1, true);
            let actor = game.world.entity_mut(id).unwrap();
            actor.state = 0;
            actor.timer = 0;
            // A newly selected WALK waits until the next Step. MagmaMan's
            // seed also accounts for its separate preceding burn roll.
            game.rng = crate::model::Rng::new(if object == "oMagmaMan" { 2 } else { 1 });
            let mut expected_rng = CanonicalOracle::checkpoint(&game.rng);
            if object == "oMagmaMan" {
                literal_magma_burn_prefix(&mut expected_rng);
            }
            let facing = if expected_rng.draw(2) == 0 { -1 } else { 1 };
            if object == "oMagmaMan" {
                assert_ne!(1 + expected_rng.draw(6), 1);
            }
            super::super::step_instance(&mut game, id);
            let actor = game.world.entity(id).unwrap();
            assert_eq!((actor.state, actor.timer, actor.facing), (1, 0, facing));
            assert_eq!(actor.vel.x, 0.0);
            for _ in 0..8 {
                assert_eq!(game.rng.next_u64(), expected_rng.next_u64(), "{object}");
            }
        }
    }

    #[test]
    fn actual_step_tomb_lord_bullet_damage_preserves_source_bloodless_rng() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = room();
            game.rules = rules;
            let lord = game.spawn("oTombLord", Vec2::new(80.0, 128.0));
            let bullet = game.spawn("oBullet", Vec2::new(100.0, 144.0));
            let mut expected_rng = game.rng.clone();

            game.step(&crate::InputFrame::default());

            let target = game.world.entity(lord).unwrap();
            assert_eq!(target.health, 16, "real bullet collision subtracts four HP");
            assert!(game.world.entity(bullet).is_none());
            // oTombLord Create sets bloodless=true. The bullet's inherited
            // enemy collision still deals damage, but scrCreateBlood returns
            // before creating a drop or consuming its four random draws.
            assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
            assert_eq!(target.value("bloodless"), 1.0);
            assert_eq!(target.value("blood_left"), 4.0);
            assert_eq!(target.vel, Vec2::new(0.0, -2.0));
            assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
        }
    }

    #[test]
    fn paired_hawk_and_tomb_stomps_restore_prior_environmental_kill_credit() {
        for object in ["oHawkman", "oTombLord"] {
            let mut game = room();
            let mut e = Entity::new(100, object, Vec2::new(80.0, 144.0));
            init(&mut e);
            super::super::set_counts_as_kill(&mut e, false);
            assert!(!super::super::counts_as_kill(&e));
            game.player.pos = Vec2::new(
                e.pos.x + if object == "oHawkman" { 8.0 } else { 16.0 },
                e.pos.y,
            );
            game.player.state = crate::PlayerState::Falling;
            game.player.vel.y = 2.0;
            let health = e.health;

            assert!(collision(&mut game, &mut e, Instance::Player));

            assert_eq!(e.health, health - 1, "{object}");
            assert!(game.player.vel.y < 0.0, "{object}");
            assert!(super::super::counts_as_kill(&e), "{object}");
            assert_eq!(e.value("environmental_death"), 0.0, "{object}");
        }
    }
    #[test]
    fn paired_temple_character_deaths_use_literal_lifetime_rows_and_guards() {
        use crate::statistics::EnemyDeath;
        for (object, cause) in [
            ("oHawkman", Some(EnemyDeath::Hawkman)),
            ("oTombLord", Some(EnemyDeath::TombLord)),
            ("oFly", Some(EnemyDeath::TombLord)),
            ("oMagmaMan", Some(EnemyDeath::MagmaMan)),
            ("oMagma", None),
        ] {
            for (custom, invincible) in [(false, 0), (true, 0), (false, 1)] {
                let mut game = room();
                if custom {
                    game.flags.insert("custom_level".into());
                }
                game.player.pos = Vec2::new(88.0, 152.0);
                game.player.state = crate::PlayerState::Standing;
                game.player.health = 1;
                game.player.invincible = invincible;
                // The low ceiling selects Hawkman's damaging event branch.
                game.world.set_tile(5, 8, "oTemple");
                let mut e = Entity::new(100, object, Vec2::new(80.0, 144.0));
                init(&mut e);
                assert!(collision(&mut game, &mut e, Instance::Player));
                let hit = invincible == 0 || object == "oFly";
                assert_eq!(game.player.health <= 0, hit, "{object}");
                if let Some(cause) = cause {
                    assert_eq!(
                        game.lifetime_stats.enemy_deaths[cause as usize],
                        u64::from(hit && !custom),
                        "{object}"
                    );
                }
                assert_eq!(
                    game.lifetime_stats.enemy_deaths.iter().sum::<u64>(),
                    u64::from(hit && !custom && cause.is_some())
                );
            }
        }
    }
    #[test]
    fn actual_step_olmec_alarm_runs_debris_create_before_launch_overrides() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.world = World::new(40, 30, Area::Mines, 1);
        game.player.pos = Vec2::new(48.0, 152.0);
        for x in 0..40 {
            game.world.set_tile(x, 10, "oBlock");
        }
        let origin = Vec2::new(160.0, 96.0);
        let boss = spawn(&mut game, "oOlmec", origin);
        let e = game.world.entity_mut(boss).unwrap();
        e.state = -2;
        e.alarms[1] = 1;
        let mut rng = game.rng.clone();
        let expected: Vec<_> = (0..6)
            .map(|_| {
                let pos = Vec2::new(
                    origin.x + 32.0 + rng.range(33) as f64,
                    origin.y + rng.range(33) as f64,
                );
                let _ = rng.unit();
                let _ = rng.unit();
                let _ = rng.unit();
                let sprite = match rng.range(3) {
                    0 => "sOlmecDebris2",
                    1 => "sOlmecDebris3",
                    _ => "sOlmecDebris",
                };
                let velocity = Vec2::new(1.0 + rng.range(4) as f64, -(1.0 + rng.range(3) as f64));
                (pos, sprite, velocity)
            })
            .collect();
        game.step(&crate::InputFrame::default());
        let debris: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oOlmecDebris")
            .collect();
        assert_eq!(debris.len(), 6);
        for (debris, (pos, sprite, velocity)) in debris.into_iter().zip(expected) {
            assert_eq!(debris.previous_pos, pos);
            assert_eq!(debris.sprite, sprite);
            assert_eq!(
                debris.pos,
                Vec2::new(pos.x + velocity.x, pos.y + velocity.y)
            );
            assert!((debris.vel.y - (velocity.y + 0.6)).abs() < 1e-9);
        }
        assert_eq!(game.rng.next_u64(), rng.next_u64());
    }
    #[test]
    fn actual_step_hawk_credits_only_dead_entry_once_and_crush_never_credits() {
        use crate::statistics::EnemyKill;
        for (counts, custom) in [(true, false), (false, false), (true, true)] {
            let mut game = room();
            if custom {
                game.flags.insert("custom_level".into());
            }
            let id = spawn(&mut game, "oHawkman", Vec2::new(80.0, 144.0));
            let hawk = game.world.entity_mut(id).unwrap();
            // An already wounded moving body is a controlled incoming actor;
            // all settling, DEAD entry and later corpse motion use Game.step.
            hawk.health = 0;
            hawk.state = 98;
            hawk.vel = Vec2::new(3.0, -4.0);
            hawk.data.insert("counts_as_kill".into(), f64::from(counts));
            game.step(&crate::InputFrame::default());
            assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
            assert_eq!(game.world.entity(id).unwrap().value("dead"), 0.0);
            for _ in 0..180 {
                game.step(&crate::InputFrame::default());
                if game.world.entity(id).unwrap().value("dead") != 0.0 {
                    break;
                }
            }
            assert_eq!(game.world.entity(id).unwrap().value("dead"), 1.0);
            assert_eq!(
                game.data.get("kills").copied().unwrap_or(0.0),
                f64::from(counts)
            );
            assert_eq!(
                game.data.get("hawkmen").copied().unwrap_or(0.0),
                f64::from(counts)
            );
            assert_eq!(
                game.lifetime_stats.enemy_kills[EnemyKill::Hawkman as usize],
                u64::from(counts && !custom)
            );
            let hawk = game.world.entity_mut(id).unwrap();
            hawk.vel = Vec2::new(2.0, -3.0);
            for _ in 0..90 {
                game.step(&crate::InputFrame::default());
            }
            assert_eq!(
                game.data.get("kills").copied().unwrap_or(0.0),
                f64::from(counts)
            );
            assert_eq!(
                game.lifetime_stats.enemy_kills[EnemyKill::Hawkman as usize],
                u64::from(counts && !custom)
            );
        }
        for health in [4, 0] {
            let mut game = room();
            game.world.set_tile(5, 9, "oTemple");
            let id = spawn(&mut game, "oHawkman", Vec2::new(80.0, 144.0));
            let hawk = game.world.entity_mut(id).unwrap();
            hawk.health = health;
            hawk.state = if health > 0 { 0 } else { 98 };
            hawk.vel = Vec2::default();
            game.step(&crate::InputFrame::default());
            assert!(game.world.entity(id).is_none());
            assert_eq!(game.data.get("kills"), None);
            assert_eq!(
                game.lifetime_stats.enemy_kills[EnemyKill::Hawkman as usize],
                0
            );
        }
        for health in [0, 1] {
            let mut game = room();
            game.player.pos = Vec2::new(230.0, 152.0);
            game.player.invincible = 999;
            let id = spawn(&mut game, "oHawkman", Vec2::new(220.0, 140.0));
            let hawk = game.world.entity_mut(id).unwrap();
            hawk.health = health;
            hawk.state = 98;
            hawk.timer = 0;
            hawk.held = true;
            game.player.held = Some(id);
            game.step(&crate::InputFrame::default());
            let hawk = game.world.entity(id).unwrap();
            assert_eq!(hawk.vel, Vec2::default(), "held bodies receive no gravity");
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.object == "oEnemySight"),
                "STUNNED recovery cannot also execute IDLE in the same source Step"
            );
            if health == 0 {
                assert_eq!(hawk.state, 99);
                assert_eq!(hawk.sprite, "sHawkDHeldL");
                assert_eq!(game.player.held, Some(id));
                assert_eq!(game.data.get("kills"), None);
                game.step(&crate::InputFrame::default());
                assert_eq!(game.data.get("kills"), Some(&1.0));
                assert_eq!(
                    game.lifetime_stats.enemy_kills[EnemyKill::Hawkman as usize],
                    1
                );
            } else {
                assert_eq!(hawk.state, 0);
                assert!(!hawk.held);
                assert_eq!(game.player.held, None);
            }
        }
    }
    #[test]
    fn actual_step_tomb_death_and_olmec_lava_follow_distinct_credit_gates() {
        use crate::statistics::EnemyKill;
        for (counts, custom, crushed) in [
            (true, false, false),
            (false, false, false),
            (true, true, true),
        ] {
            let mut game = room();
            if custom {
                game.flags.insert("custom_level".into());
            }
            let id = spawn(&mut game, "oTombLord", Vec2::new(80.0, 128.0));
            let tomb = game.world.entity_mut(id).unwrap();
            tomb.vel = Vec2::default();
            tomb.health = if crushed { 20 } else { 0 };
            tomb.data.insert("counts_as_kill".into(), f64::from(counts));
            if crushed {
                game.world.set_tile(6, 9, "oTemple");
            }
            game.step(&crate::InputFrame::default());
            assert!(game.world.entity(id).is_none());
            assert_eq!(
                game.data.get("kills").copied().unwrap_or(0.0),
                f64::from(counts)
            );
            assert_eq!(
                game.data.get("tomblords").copied().unwrap_or(0.0),
                f64::from(counts)
            );
            assert_eq!(
                game.lifetime_stats.enemy_kills[EnemyKill::TombLord as usize],
                u64::from(counts && !custom)
            );
            assert!(game.world.entities.iter().any(|e| e.object == "oSceptre"));
        }
        let mut game = room();
        game.flags.insert("custom_level".into());
        game.world.set_tile(5, 5, "oLava");
        let id = spawn(&mut game, "oOlmec", Vec2::new(80.0, 96.0));
        let boss = game.world.entity_mut(id).unwrap();
        boss.state = 0;
        boss.vel = Vec2::default();
        boss.data.insert("counts_as_kill".into(), 0.0);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(
            game.lifetime_stats.enemy_kills[EnemyKill::Olmec as usize],
            1
        );
        assert!(game.flags.contains("olmec_dead"));
    }
    #[test]
    fn natural_olmec_intro_locks_actions_pans_and_releases_after_slam_alarm() {
        let mut game = Game::new(42, RulesPreset::Classic);
        while game.world.level < 16 {
            game.next_level();
        }
        let boss_id = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oOlmec")
            .unwrap()
            .id;
        let start = game.player.pos;
        let inventory = (game.player.bombs, game.player.ropes);
        assert_eq!(start, Vec2::new(24.0, 552.0));
        assert_eq!(game.camera, Vec2::new(0.0, 400.0));
        assert_eq!(game.player.data.get("active"), Some(&0.0));
        let controls = crate::InputFrame {
            right: true,
            jump: true,
            jump_pressed: true,
            bomb_pressed: true,
            rope_pressed: true,
            attack_pressed: true,
            ..crate::InputFrame::default()
        };
        for tick in 1..=339 {
            game.step(&controls);
            assert_eq!(game.player.pos, start, "locked player, tick {tick}");
            assert_eq!((game.player.bombs, game.player.ropes), inventory);
            assert_eq!(game.player.health, 4);
            let boss = game.world.entity(boss_id).unwrap();
            match tick {
                20 => assert_eq!(game.camera, Vec2::new(40.0, 400.0)),
                88 => {
                    assert_eq!(game.camera.x, 176.0);
                    assert_eq!(boss.state, -1);
                }
                89 => {
                    assert_eq!(boss.state, -2);
                    assert_eq!(boss.alarms[1], 100);
                }
                189 => {
                    assert_eq!(boss.sprite, "sOlmecStart2");
                    assert!(
                        !game
                            .world
                            .entities
                            .iter()
                            .any(|e| e.alive && e.object == "oHawkmanWorship")
                    );
                    let cavemen: Vec<_> = game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oCavemanWorship")
                        .collect();
                    assert_eq!(cavemen.len(), 3);
                    assert!(
                        cavemen
                            .iter()
                            .all(|e| e.animation == 0.0 && e.value("animation_speed") == 0.0)
                    );
                }
                238 => assert_eq!(boss.sprite, "sOlmecStart3"),
                287 => assert_eq!(boss.sprite, "sOlmec"),
                336 => {
                    assert_eq!(boss.state, 1);
                    assert!(boss.vel.y < 0.0);
                }
                339 => {
                    // oLevel's96 border follows the naturally jumping boss
                    // before control returns to the player (view_object=Olmec).
                    assert!(boss.pos.y < 496.0);
                    assert_eq!(game.camera.y, boss.pos.y.round_ties_even() - 96.0);
                    assert_eq!(game.data.get("camera_vborder"), Some(&96.0));
                }
                _ => {}
            }
        }
        let mut saw_prepare = false;
        let mut saw_slam = false;
        let mut scheduled_at = None;
        let mut released_at = None;
        for tick in 340..=600 {
            // Hold ordinary Right through the natural introduction; never edit
            // actor state, terrain, timers, or health to make it complete.
            game.step(&crate::InputFrame {
                right: true,
                ..crate::InputFrame::default()
            });
            let boss = game.world.entity(boss_id).unwrap();
            saw_prepare |= boss.state == 5;
            saw_slam |= boss.state == 6;
            if boss.alarms[5] == 50 {
                scheduled_at = Some(tick);
            }
            if tick == 355 {
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|e| e.alive && e.object == "oCavemanWorship")
                );
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oCaveman")
                        .count(),
                    3
                );
            }
            if game.player.data.get("active") == Some(&1.0) {
                released_at = Some(tick);
                assert_eq!(boss.state, 0);
                assert_eq!(boss.timer, 99);
                // Alarm5 sets border64, then this same frame's oLevel
                // Step restores96 before following playerY552:552-(240-96).
                assert_eq!(game.camera, Vec2::new(0.0, 408.0));
                assert_eq!(game.data.get("camera_vborder"), Some(&96.0));
                break;
            }
            assert_eq!(game.player.pos, start, "locked player, tick {tick}");
            assert_eq!(game.player.health, 4);
        }
        assert!(saw_prepare && saw_slam);
        assert_eq!(released_at, scheduled_at.map(|tick| tick + 50));
        assert!(released_at.is_some(), "natural intro must finish");
        for _ in 0..20 {
            game.step(&crate::InputFrame {
                right: true,
                ..crate::InputFrame::default()
            });
        }
        assert!(game.player.pos.x > start.x + 20.0);
        assert_eq!(game.player.health, 4);
        game.step(&crate::InputFrame {
            jump: true,
            jump_pressed: true,
            ..crate::InputFrame::default()
        });
        assert!(game.player.pos.y < start.y);
        assert!(game.player.vel.y < 0.0);
    }

    #[test]
    fn inactive_olmec_player_still_takes_lava_damage_and_runs_dead_physics() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(80.0, 80.0);
        game.player.data.insert("active".into(), 0.0);
        game.world.spawn("oLava", Vec2::new(72.0, 72.0));
        game.step(&crate::InputFrame::default());
        assert_eq!(game.state, crate::RunState::Dead);
        let before = game.player.pos;
        for _ in 0..3 {
            game.step(&crate::InputFrame::default());
        }
        assert!(game.player.pos.y > before.y);
    }

    #[test]
    fn leaving_olmec_room_clears_player_inactive_state_and_intro_alarms() {
        let mut game = Game::new(42, RulesPreset::Classic);
        while game.world.level < 16 {
            game.next_level();
        }
        for _ in 0..100 {
            game.step(&crate::InputFrame::default());
        }
        assert_eq!(game.player.data.get("active"), Some(&0.0));
        game.load_hub(crate::hub::HubKind::Title);
        assert_ne!(game.player.data.get("active"), Some(&0.0));
        assert!(!game.world.entities.iter().any(|e| e.object == "oOlmec"));
        let before = game.player.pos;
        for _ in 0..20 {
            game.step(&crate::InputFrame {
                right: true,
                ..crate::InputFrame::default()
            });
        }
        assert!(game.player.pos.x > before.x);
    }
    #[test]
    fn actual_door_exit_excludes_olmec_crush_while_boss_keeps_moving() {
        use crate::characters::{self, Character};
        use crate::{InputFrame, RunState};
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            let mut level = crate::level::Level::default();
            level.set_tile(6, 7, '@').unwrap();
            level.set_tile(7, 7, 'X').unwrap();
            for x in 0..40 {
                level.set_tile(x, 8, '1').unwrap();
            }
            let mut game = Game::shell(11, RulesPreset::Classic);
            game.load_custom(&level, "olmec-exit", "");
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
                // Only the arriving active boss is controlled. The player's
                // Exit sprite, inactivity and invincibility come from the real
                // door/input path, with no player-state injection.
                let pos = Vec2::new(target.player.pos.x - 32.0, target.player.pos.y - 32.0);
                let id = spawn(target, "oOlmec", pos);
                let boss = target.world.entity_mut(id).unwrap();
                boss.state = 2;
                boss.timer = 10;
                boss.vel = Vec2::new(1.0, 0.0);
                target.step(&InputFrame::default());
                let boss = target.world.entity(id).unwrap();
                assert_eq!(boss.age, 1);
                assert_eq!(boss.pos.x, pos.x + 1.0);
                assert!(olmec_rect(boss).contains(target.player.pos));
                assert!((target.player.pos.y + 8.0 - pos.y).abs() >= 3.0);
                if exiting {
                    assert_eq!(target.player.health, health, "{character:?}");
                    assert!(target.player.is_exiting(), "{character:?}");
                    assert_eq!(target.state, RunState::Ending);
                } else {
                    assert!(target.player.health < health, "{character:?}");
                    assert_eq!(target.state, RunState::Dead);
                }
            }
        }
    }
    // Test-only additions for enemies/temple.rs::tests. This oracle does not call
    // production chance/range, MagmaMan Step, or any movement helper.
    //
    // Source: Objects/Enemies/oMagmaMan.events/Create.xml:32-56 (hp200,
    // IDLE0/counter0/facingRIGHT), Step.xml:16 (hp decay), 42-43 (burn first),
    // 52-61 (grounded IDLE decrement + facing/transform), 94-99 (WALK idle roll).
    //
    // Fixture preconditions: one initially default MagmaMan at (160,144), continuous
    // solid row10, and vertical walls at columns4/16 over rows0..9. Its body stays
    // supported and outside those walls. Left/right wall turns only reverse facing
    // and consume no RNG. Both forward feet probes stay over the continuous floor,
    // so the separate ledge rand(20,50) branch is ineligible. No interacting actors,
    // player burning, lava spurts, or other random producers occur during this
    // isolated patrol. Burn Create/Step/AnimationEnd consume no RNG and have no cap.
    // This oracle proves the RNG eligibility precondition; the existing Game.step
    // test still proves actual support, survival, aging, contact, and animation.
    #[derive(Debug)]
    struct FlatFloorMagmaOracle {
        health: i32,
        state: i32,
        counter: u32,
        initial_facing: i8,
        // Wall reflections are intentionally outside this RNG-only oracle.
        last_random_facing: i8,
        transform_step: Option<u32>,
        burns: u32,
        random_idle_entries: u32,
        draws: u32,
    }

    trait LiteralMagmaStream {
        fn select(&mut self, width: u64) -> u64;
    }
    impl LiteralMagmaStream for CanonicalOracle {
        fn select(&mut self, width: u64) -> u64 {
            self.draw(width)
        }
    }
    // Historical control retains its original full SplitMix arithmetic and
    // literal expected values; it is not a canonical source prediction.
    struct LegacyMagmaOracle(u64);
    impl LegacyMagmaOracle {
        fn next_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut value = self.0;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            value ^ (value >> 31)
        }
    }
    impl LiteralMagmaStream for LegacyMagmaOracle {
        fn select(&mut self, width: u64) -> u64 {
            self.next_u64() % width
        }
    }

    fn literal_flatfloor_magma_199(rng: &mut impl LiteralMagmaStream) -> FlatFloorMagmaOracle {
        fn rand(rng: &mut impl LiteralMagmaStream, draws: &mut u32, low: u64, high: u64) -> u64 {
            *draws += 1;
            low + rng.select(high - low + 1)
        }
        let mut result = FlatFloorMagmaOracle {
            health: 200,
            state: 0, // IDLE
            counter: 0,
            initial_facing: 1, // RIGHT; Create has no own random draw.
            last_random_facing: 1,
            transform_step: None,
            burns: 0,
            random_idle_entries: 0,
            draws: 0,
        };
        for step in 1..=199 {
            result.health -= 1;
            // Source's unconditional emission roll precedes BOTH state branches.
            if rand(rng, &mut result.draws, 1, 20) == 1 {
                rand(rng, &mut result.draws, 4, 12); // x offset
                rand(rng, &mut result.draws, 4, 12); // y offset
                result.burns += 1;
            }
            if result.state == 0 {
                // The whole authored floor supports the source feet probe.
                if result.counter > 0 {
                    result.counter -= 1;
                }
                // This is a second `if`, not `else`: counter1 transitions now.
                if result.counter == 0 {
                    result.last_random_facing = if rand(rng, &mut result.draws, 0, 1) == 0 {
                        -1
                    } else {
                        1
                    };
                    result.state = 1; // WALK
                    if rand(rng, &mut result.draws, 1, 6) == 1 {
                        result.transform_step = Some(step);
                        // Stop at the fixture-disqualifying branch. Source then
                        // creates Magma (four Create draws), copies HP, destroys
                        // this receiver, and continues. This eligibility oracle
                        // deliberately makes no post-transform RNG/state claim.
                        break;
                    }
                }
            } else {
                assert_eq!(result.state, 1);
                // Source wall turns consume zero draws, and the forward floor
                // point is supported; only the independent WALK idle roll remains.
                if rand(rng, &mut result.draws, 1, 100) == 1 {
                    result.state = 0;
                    result.counter = rand(rng, &mut result.draws, 20, 50) as u32;
                    result.random_idle_entries += 1;
                }
            }
        }
        result
    }

    #[test]
    fn literal_flatfloor_magma_historical_legacy_vectors_remain_frozen() {
        let mut source_rng = LegacyMagmaOracle(0);
        let source = literal_flatfloor_magma_199(&mut source_rng);
        assert_eq!(source.initial_facing, 1);
        assert_eq!(source.transform_step, None);
        assert_eq!((source.health, source.state, source.counter), (1, 1, 0));
        assert_eq!(
            (source.burns, source.random_idle_entries, source.draws),
            (9, 1, 373)
        );
        assert_eq!(source_rng.next_u64(), 9_593_559_228_881_912_482);

        // Frozen historical SplitMix negative control: seed1 transforms immediately.
        let mut source_rng = LegacyMagmaOracle(1);
        let source = literal_flatfloor_magma_199(&mut source_rng);
        assert_eq!(source.initial_facing, 1);
        assert_eq!(source.last_random_facing, 1);
        assert_eq!(source.transform_step, Some(1));
        assert_eq!((source.health, source.burns, source.draws), (199, 0, 3));
    }

    #[test]
    fn literal_flatfloor_magma_seed_proves_natural_lifetime_fixture_precondition() {
        // Independently computed by the archived Python source-state machine,
        // using the frozen executable recurrence rather than Rust RNG helpers.
        let mut source_rng = CanonicalOracle(0);
        let source = literal_flatfloor_magma_199(&mut source_rng);
        assert_eq!(source.initial_facing, 1);
        assert_eq!(source.last_random_facing, -1);
        assert_eq!(source.transform_step, None);
        assert_eq!((source.health, source.state, source.counter), (1, 0, 24));
        assert_eq!(
            (source.burns, source.random_idle_entries, source.draws),
            (12, 2, 394)
        );
        assert_eq!(source_rng.next_u64(), 251_700_279);

        // Canonical seed3 reaches the disqualifying transform at step75.
        let mut source_rng = CanonicalOracle(3);
        let source = literal_flatfloor_magma_199(&mut source_rng);
        assert_eq!(source.transform_step, Some(75));
        assert_eq!(
            (
                source.health,
                source.burns,
                source.random_idle_entries,
                source.draws
            ),
            (125, 2, 1, 137)
        );
        assert_eq!(source_rng.next_u64(), 3_383_530_281);
    }

    #[test]
    fn actual_step_naturally_aged_magma_man_hurts_until_die_animation_finishes() {
        // The independent canonical flat-floor oracle proves seed 0 avoids voluntary
        // transformation through tick 199, including the source burn RNG prefix.
        let mut game = Game::shell(0, RulesPreset::Classic);
        game.world = World::new(40, 30, Area::Mines, 1);
        game.flags.insert("custom".into());
        game.player.pos = Vec2::new(32.0, 152.0);
        for x in 0..40 {
            game.world.set_tile(x, 10, "oBlock");
        }
        for y in 0..10 {
            game.world.set_tile(4, y, "oBlock");
            game.world.set_tile(16, y, "oBlock");
        }
        let id = game.world.spawn("oMagmaMan", Vec2::new(160.0, 144.0));
        for _ in 0..198 {
            game.step(&crate::InputFrame::default());
            assert!(
                game.world.entity(id).is_some(),
                "bounded natural patrol must survive to its final life ticks"
            );
        }
        assert_eq!(game.world.entity(id).unwrap().health, 2);
        let touch = |game: &mut Game, invincible| {
            let pos = game.world.entity(id).unwrap().pos;
            game.player.pos = Vec2::new(pos.x + 8.0, pos.y + 8.0);
            game.player.vel = Vec2::default();
            game.player.state = crate::PlayerState::Standing;
            game.player.invincible = invincible;
        };
        let assert_hit = |game: &Game| {
            assert_eq!(game.player.health, 2);
            assert_eq!(game.player.invincible, 30);
            assert_eq!(game.player.data.get("blink"), Some(&30.0));
            assert_eq!(game.player.stun, 20);
            assert!(game.player.is_stunned());
            assert_eq!(game.player.data.get("burning"), Some(&100.0));
            assert_eq!(game.player.vel, Vec2::new(6.0, -4.0));
            assert_eq!(
                game.events
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Damage { amount: 2 }))
                    .count(),
                1,
                "independent contact and shared combat must not both deal damage"
            );
        };
        let mut living = game.clone();
        touch(&mut living, 0);
        living.step(&crate::InputFrame::default());
        assert_eq!(living.world.entity(id).unwrap().health, 1);
        assert_hit(&living);

        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().health, 1);
        let mut immune = game.clone();
        let die_start = game.world.entity(id).unwrap().animation;
        touch(&mut game, 0);
        game.step(&crate::InputFrame::default());
        let dying = game
            .world
            .entity(id)
            .expect("hp0 changes sprite without destroying the instance");
        assert_eq!((dying.health, dying.state), (0, 99));
        assert_eq!(dying.animation, die_start + 0.5);
        assert_eq!(dying.sprite, "sMagmaManDie");
        assert_hit(&game);

        touch(&mut immune, 2);
        immune.step(&crate::InputFrame::default());
        assert_eq!(immune.player.health, 4);
        assert_eq!(immune.player.invincible, 1);
        assert_eq!(immune.player.stun, 0);
        assert_eq!(
            immune.player.data.get("burning").copied().unwrap_or(0.0),
            0.0
        );
        while immune.world.entity(id).unwrap().animation + 0.5 < 13.0 {
            immune.player.invincible = 999;
            immune.step(&crate::InputFrame::default());
        }
        assert_eq!(immune.world.entity(id).unwrap().animation, 12.5);
        let mut last_visible = immune.clone();
        touch(&mut last_visible, 0);
        last_visible.step(&crate::InputFrame::default());
        assert!(last_visible.world.entity(id).is_none());
        assert_hit(&last_visible);
        // Automatic collision precedes endframe Animation End. The same Die
        // completion frame still hurts; the following frame has no instance.
        last_visible.player.invincible = 0;
        last_visible.step(&crate::InputFrame::default());
        assert_eq!(last_visible.player.health, 2);
        assert!(
            !last_visible
                .events
                .iter()
                .any(|event| matches!(event, GameEvent::Damage { .. }))
        );

        touch(&mut immune, 999);
        immune.step(&crate::InputFrame::default());
        assert!(immune.world.entity(id).is_none());
        assert_eq!(immune.player.health, 4);
        assert_eq!(immune.player.stun, 0);
        assert!(
            !immune
                .events
                .iter()
                .any(|event| matches!(event, GameEvent::Damage { .. }))
        );
    }
    #[test]
    fn actual_step_olmec_create_emits_source_positions_headings_and_rng_order() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.world = World::new(40, 30, Area::Mines, 1);
        game.flags.insert("custom".into());
        game.player.pos = Vec2::new(48.0, 152.0);
        for x in 0..40 {
            game.world.set_tile(x, 10, "oBlock");
        }
        let origin = Vec2::new(160.0, 96.0);
        let target = Vec2::new(origin.x + 32.0, origin.y + 16.0);
        let boss_id = spawn(&mut game, "oOlmec", origin);
        game.world.entity_mut(boss_id).unwrap().state = 7;
        let mut expected_rng = game.rng.clone();
        let positions: Vec<_> = (0..6)
            .map(|_| {
                Vec2::new(
                    origin.x + 32.0 + f64::from(expected_rng.range(33))
                        - f64::from(expected_rng.range(33)),
                    origin.y + 14.0 + f64::from(expected_rng.range(33))
                        - f64::from(expected_rng.range(33)),
                )
            })
            .collect();
        let launches: Vec<_> = (0..3)
            .map(|_| {
                let vy = -4.0 - expected_rng.unit() * 3.0;
                let mut vx = f64::from(2 + expected_rng.range(4));
                if expected_rng.chance(1, 2) {
                    vx = -vx;
                }
                Vec2::new(vx, vy)
            })
            .collect();
        game.step(&crate::InputFrame::default());
        let effects: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oPsychicCreate2")
            .collect();
        let balls: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oYellowBall")
            .collect();
        assert_eq!(effects.len(), 6);
        assert_eq!(balls.len(), 3);
        for (effect, pos) in effects.iter().zip(positions) {
            assert_eq!(effect.previous_pos, pos, "source Create origin");
            let angle = (target.y - pos.y).atan2(target.x - pos.x);
            let velocity = Vec2::new(2.0 * angle.cos(), 2.0 * angle.sin());
            assert_eq!(
                effect.vel, velocity,
                "heading must be fixed while the boss is detached"
            );
            assert_eq!(
                effect.pos,
                Vec2::new(pos.x + velocity.x, pos.y + velocity.y)
            );
            assert_eq!(effect.value("psychic_initialized"), 1.0);
            assert!(effect.id < balls[0].id, "all six effects precede the balls");
        }
        let ball_ids: Vec<_> = balls.iter().map(|e| e.id).collect();
        for (ball, velocity) in balls.iter().zip(&launches) {
            assert_eq!(ball.previous_pos, target);
            assert_eq!(
                ball.pos,
                Vec2::new(target.x + velocity.x, target.y + velocity.y)
            );
            assert_eq!(
                ball.vel,
                Vec2::new(velocity.x, velocity.y + 0.15),
                "resource579 runs after its source559 producer in the same Step"
            );
            assert_eq!(ball.value("launch"), 0.0);
        }
        assert_eq!(game.world.entity(boss_id).unwrap().state, 0);
        assert_eq!(
            game.rng.clone().next_u64(),
            expected_rng.clone().next_u64(),
            "six four-draw positions must precede three three-draw launches"
        );
        game.step(&crate::InputFrame::default());
        for (id, launch) in ball_ids.into_iter().zip(launches) {
            let ball = game.world.entity(id).unwrap();
            assert_eq!(
                ball.pos,
                Vec2::new(
                    (target.x + launch.x) + launch.x,
                    (target.y + launch.y) + (launch.y + 0.15)
                )
            );
            assert_eq!(ball.vel, Vec2::new(launch.x, launch.y + 0.15 + 0.15));
        }
        assert_eq!(
            game.rng.clone().next_u64(),
            expected_rng.next_u64(),
            "first YellowBall Step must not rerun Create draws"
        );
    }
    #[test]
    fn actual_step_olmec_intro_debris_preserves_alarm_launches_inside_boss() {
        let mut game = Game::new(42, RulesPreset::Classic);
        while game.world.level < 16 {
            game.next_level();
        }
        let mut previous_stage = 0.0;
        for _ in 0..260 {
            game.step(&crate::InputFrame::default());
            let boss = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oOlmec")
                .unwrap();
            let stage = boss.value("intro_stage");
            if stage > previous_stage && stage <= 2.0 {
                let fragments: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oOlmecDebris" && e.age == 0)
                    .collect();
                assert_eq!(fragments.len(), 6, "intro stage {stage}");
                for fragment in &fragments {
                    let directional_speed = fragment.vel.x * if stage == 1.0 { 1.0 } else { -1.0 };
                    assert!((1.0..=4.0).contains(&directional_speed));
                    assert_eq!(
                        directional_speed.fract(),
                        0.0,
                        "Create replaced alarm velocity"
                    );
                    assert!((-2.4..=-0.4).contains(&fragment.vel.y));
                }
                if stage == 1.0 {
                    assert!(
                        fragments
                            .iter()
                            .any(|e| e.sprite_bounds().intersects(boss.sprite_bounds())),
                        "fragments must pass through the solid boss"
                    );
                } else {
                    return;
                }
            }
            previous_stage = stage;
        }
        panic!("natural intro did not reach the second debris alarm");
    }
    #[test]
    fn actual_step_olmec_debris_moves_fractionally_and_bounces_only_on_temple() {
        for object in ["oTemple", "oBlock"] {
            let mut game = room();
            game.world.set_tile(6, 10, object);
            let id = spawn(&mut game, "oOlmecDebris", Vec2::new(100.0, 156.0));
            game.world.entity_mut(id).unwrap().vel = Vec2::new(1.25, 0.75);
            game.step(&crate::InputFrame::default());
            let fragment = game.world.entity(id).unwrap();
            assert_eq!(fragment.pos, Vec2::new(101.25, 156.75));
            if object == "oTemple" {
                assert_eq!(fragment.vel.x, 0.375);
                assert!((fragment.vel.y + 0.54).abs() < 1e-9);
            } else {
                assert_eq!(fragment.vel.x, 1.25);
                assert!((fragment.vel.y - 1.35).abs() < 1e-9);
            }
        }
        let mut game = room();
        let id = spawn(&mut game, "oOlmecDebris", Vec2::new(100.0, 156.0));
        game.world.entity_mut(id).unwrap().vel = Vec2::default();
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert!(game.world.entities.iter().any(|e| e.object == "oSmokePuff"));
    }
    #[test]
    fn actual_step_yellow_ball_outside_room_uses_full_sprite_at_top_and_side() {
        for above in [true, false] {
            let mut game = Game::shell(if above { 42 } else { 2 }, RulesPreset::Classic);
            game.world = World::new(40, 30, Area::Mines, 1);
            game.flags.insert("custom".into());
            for x in 0..40 {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.player.pos = if above {
                Vec2::new(136.0, 152.0)
            } else {
                Vec2::new(620.0, 152.0)
            };
            if !above {
                game.camera.x = 320.0;
            }
            let id = game.world.spawn(
                "oYellowBall",
                if above {
                    Vec2::new(180.0, 8.0)
                } else {
                    Vec2::new(638.0, 100.0)
                },
            );
            game.step(&crate::InputFrame::default());
            let ball = game
                .world
                .entity(id)
                .expect("partially outside sprite must remain active");
            let room = Rect::new(0.0, 0.0, 640.0, 480.0);
            assert!(ball.sprite_bounds().intersects(room));
            if !above {
                assert!(
                    ball.pos.x > 640.0,
                    "normal launch must cross the side by its origin first"
                );
            }
            for _ in 0..10 {
                game.step(&crate::InputFrame::default());
                if game.world.entity(id).is_none() {
                    break;
                }
            }
            assert!(game.world.entity(id).is_none(), "above={above}");
            assert!(
                !game.world.entities.iter().any(|e| {
                    matches!(
                        e.object.as_str(),
                        "oBat" | "oSpider" | "oSnake" | "oFrog" | "oFireFrog"
                    )
                }),
                "Outside Room must not hatch an enemy"
            );
        }
    }
    fn room() -> Game {
        let mut g = Game::shell(9, RulesPreset::Classic);
        g.world = World::new(24, 20, Area::Temple, 13);
        for x in 0..24 {
            g.world.set_tile(x, 10, "oTemple");
        }
        g.player.pos = Vec2::new(230.0, 120.0);
        g
    }
    #[test]
    fn actual_step_fly_hits_unheld_and_held_damsel_but_respects_invincibility() {
        for (held, invincible) in [(false, false), (true, false), (true, true)] {
            let mut g = room();
            let damsel_pos = if held {
                g.player.pos = Vec2::new(136.0, 152.0);
                Vec2::new(140.0, 152.0)
            } else {
                Vec2::new(96.0, 144.0)
            };
            let damsel_id = g.world.spawn("oDamsel", damsel_pos);
            let damsel = g.world.entity_mut(damsel_id).unwrap();
            damsel.held = held;
            damsel
                .data
                .insert("invincible".into(), f64::from(invincible));
            if held {
                g.player.held = Some(damsel_id);
            }
            // The held fixture touches the Damsel's exposed right edge while
            // clearing the player's sprite, so the dedicated event must run.
            let fly_id = g.world.spawn(
                "oFly",
                if held {
                    Vec2::new(150.0, 148.0)
                } else {
                    Vec2::new(100.0, 148.0)
                },
            );
            let vx = if held { -1.0 } else { 1.0 };
            g.world.entity_mut(fly_id).unwrap().vel = Vec2::new(vx, 0.0);
            g.step(&crate::InputFrame::default());
            let damsel = g.world.entity(damsel_id).unwrap();
            assert_eq!(g.player.health, 4);
            if invincible {
                assert_eq!(damsel.health, 4);
                assert!(damsel.held);
                assert_eq!(g.player.held, Some(damsel_id));
                assert!(g.world.entity(fly_id).is_some());
            } else {
                assert_eq!(damsel.health, 2, "held={held}");
                assert_eq!(damsel.state, 2);
                assert_eq!(damsel.timer, 120);
                assert_eq!(damsel.vel, Vec2::new(vx * 0.3, -6.0));
                assert!(!damsel.held);
                assert!(g.player.held.is_none());
                assert!(g.world.entity(fly_id).is_none());
            }
        }
    }
    #[test]
    fn actual_step_natural_yellow_ball_spawns_enemy_at_rewound_top_left() {
        let mut g = Game::shell(42, RulesPreset::Classic);
        g.world = World::new(40, 30, Area::Mines, 1);
        g.flags.insert("custom".into());
        g.player.pos = Vec2::new(136.0, 136.0);
        for x in 0..40 {
            g.world.set_tile(x, 10, "oBlock");
        }
        for y in 0..10 {
            g.world.set_tile(4, y, "oBlock");
            g.world.set_tile(16, y, "oBlock");
        }
        let ball_id = g.world.spawn("oYellowBall", Vec2::new(180.0, 128.0));
        for _ in 0..100 {
            let before = g.world.entity(ball_id).unwrap().pos;
            g.step(&crate::InputFrame::default());
            if g.world.entity(ball_id).is_none() {
                let spawned = g
                    .world
                    .entities
                    .iter()
                    .find(|entity| {
                        matches!(
                            entity.object.as_str(),
                            "oBat" | "oSpider" | "oSnake" | "oFrog" | "oFireFrog"
                        )
                    })
                    .expect("natural wall impact must hatch an enemy");
                assert_eq!(spawned.pos, Vec2::new(before.x - 8.0, before.y - 8.0));
                return;
            }
        }
        panic!("natural YellowBall trajectory never reached either wall");
    }
    #[test]
    fn hawk_stomp_wins_over_grab_in_integrated_combat() {
        // oHawkman Collision with oCharacter: descending above y+5 stomps.
        let mut g = room();
        g.player.pos = Vec2::new(88.0, 146.0);
        g.player.vel.y = 1.0;
        let id = g.world.spawn("oHawkman", Vec2::new(80.0, 144.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::default();
        g.step(&crate::InputFrame::default());
        assert_eq!(g.player.health, 4);
        assert_eq!(g.world.entity(id).unwrap().health, 3);
        assert_eq!(g.world.entity(id).unwrap().state, 98);
    }
    #[test]
    fn hawk_throw_preserves_sprite_and_drops_held_item() {
        // oHawkman collision + animation-end events, seven frames at 0.5.
        let mut g = room();
        g.player.pos = Vec2::new(90.0, 152.0);
        let held = g.world.spawn("oRock", g.player.pos);
        g.world.entity_mut(held).unwrap().held = true;
        g.player.held = Some(held);
        let id = g.world.spawn("oHawkman", Vec2::new(80.0, 144.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::default();
        g.step(&crate::InputFrame::default());
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.pos, Vec2::new(80.0, 144.0));
        assert_eq!(g.player.held, None);
        assert!(!g.world.entity(held).unwrap().held);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sHawkThrowL");
        // Keep the animation check independent of the source-correct thrown
        // item collision: a dropped, fast Rock can itself stun Hawkman.
        g.world.entity_mut(held).unwrap().pos = Vec2::new(250.0, 64.0);
        g.step(&crate::InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 3);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sHawkThrowL");
    }
    #[test]
    fn source_olmec_move_quantizes_fractional_steps() {
        // Scripts/Platform Engine/moveTo.gml: 0.25 moves on tick multiples of 4.
        let world = World::new(24, 20, Area::Boss, 16);
        let mut e = Entity::new(1, "oOlmec", Vec2::new(80.0, 80.0));
        init(&mut e);
        e.vel.x = 0.25;
        e.data.insert("simulation_tick".into(), 3.0);
        olmec_move(&world, &mut e);
        assert_eq!(e.pos.x, 80.0);
        e.data.insert("simulation_tick".into(), 4.0);
        olmec_move(&world, &mut e);
        assert_eq!(e.pos.x, 81.0);
    }
    #[test]
    fn offset_slam_clears_every_intersecting_source_solid() {
        // oOlmecSlam's source mask is [2,0,60,7]; offset origins can touch five columns.
        let mut g = room();
        let mut e = Entity::new(1, "oOlmecSlam", Vec2::new(107.0, 155.0));
        init(&mut e);
        for x in 6..=10 {
            let index = 10 * g.world.width + x;
            assert!(collision(&mut g, &mut e, Instance::Terrain(index)));
            assert!(g.world.tile(x as i32, 10).unwrap().object.is_empty());
        }
        assert_eq!(g.world.tile(11, 10).unwrap().object, "oTemple");
    }
    #[test]
    fn source_fly_bypasses_and_preserves_player_invincibility() {
        // oFly Collision with oCharacter deliberately has no invincible guard.
        let mut g = room();
        g.player.pos = Vec2::new(88.0, 120.0);
        g.player.invincible = 27;
        let mut e = Entity::new(1, "oFly", g.player.pos);
        init(&mut e);
        assert!(collision(&mut g, &mut e, Instance::Player));
        assert_eq!(g.player.health, 2);
        assert_eq!(g.player.invincible, 27);
        assert!(!e.alive);
    }
    #[test]
    fn magma_extinguishes_in_authored_water_entities() {
        // oMagma Collision with oWater; authored water is an entity rather than a tile.
        let mut g = room();
        let water = g.world.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
        let mut e = Entity::new(100, "oMagma", Vec2::new(88.0, 88.0));
        init(&mut e);
        e.data.insert("started".into(), 1.0);
        assert!(collision(&mut g, &mut e, Instance::Entity(water)));
        assert!(!e.alive);
        assert!(g.world.entities.iter().any(|e| e.object == "oSmokePuff"));
    }
    #[test]
    fn hawk_sight_uses_source_six_tick_counter_and_moving_instances() {
        // oHawkman Step decrements a positive sightCounter, else creates and sets 5.
        let mut g = room();
        let mut e = Entity::new(100, "oHawkman", Vec2::new(80.0, 144.0));
        init(&mut e);
        e.timer = 100;
        for _ in 0..7 {
            tick(&mut g, &mut e);
        }
        let rays: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|o| o.object == "oEnemySight")
            .collect();
        assert_eq!(rays.len(), 2);
        assert_eq!(rays[0].value("hspeed"), 10.0);
        assert_eq!(rays[0].value("vspeed"), 0.0);
        assert_eq!(rays[0].value("owner"), 100.0);
    }
    // Source-default actors and authored floor/wall geometry. After birth,
    // these regressions use ordinary idle input only, without actor edits or
    // manually spawned sight. EnemySight has no source expiry timer.
    fn natural_sight_room(rules: RulesPreset, seed: u32, wall: bool) -> Game {
        let mut game = Game::shell(seed, rules);
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
        game
    }

    #[test]
    fn natural_hawk_sight_moves_and_wall_retires_it_with_caveman_control() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oHawkman", "oCaveman"] {
                for wall in [false, true] {
                    // Seed16's first facing selector chooses RIGHT in both
                    // Legacy and canonical, toward the authored right wall.
                    let mut game = natural_sight_room(rules, 16, wall);
                    let actor = game.spawn(object, Vec2::new(96.0, 144.0));
                    game.step(&crate::InputFrame::default());
                    let ray = game
                        .world
                        .entities
                        .iter()
                        .find(|e| {
                            e.alive && e.object == "oEnemySight" && e.value("owner") == actor as f64
                        })
                        .expect("the real actor Step must create its sight");
                    let ray_id = ray.id;
                    assert_eq!(game.world.entity(actor).unwrap().facing, 1, "{object}");
                    assert_eq!(ray.pos.x - ray.previous_pos.x, 10.0, "{object}");
                    assert_eq!(ray.pos.y, ray.previous_pos.y, "{object}");
                    game.step(&crate::InputFrame::default());
                    assert_eq!(
                        game.world.entity(ray_id).is_none(),
                        wall,
                        "only the wall retires the naturally moving ray: {object}"
                    );
                    let mut alerted = game.events.iter().any(|event| {
                        matches!(event, GameEvent::Sound { name, .. } if name == "sndAlert")
                    });
                    for _ in 2..6 {
                        game.step(&crate::InputFrame::default());
                        alerted |= game.events.iter().any(|event| {
                            matches!(event, GameEvent::Sound { name, .. } if name == "sndAlert")
                        });
                    }
                    assert_eq!(alerted, !wall, "{object}");
                    assert_eq!(game.world.entity(actor).unwrap().state == 2, !wall);
                    assert_eq!(game.player.health, 4);
                }
            }
        }
    }

    #[test]
    fn natural_hawk_idle_sight_uses_facing_chosen_this_step() {
        let mut saw_left = false;
        let mut saw_right = false;
        // Canonical first words for seeds0..15 choose LEFT; seed16 is the
        // first RIGHT witness. Retain all earlier cases and both directions.
        for seed in 0..=16 {
            let mut game = natural_sight_room(RulesPreset::Classic, seed, false);
            let actor = game.spawn("oHawkman", Vec2::new(96.0, 144.0));
            assert_eq!(game.world.entity(actor).unwrap().facing, 1);
            game.step(&crate::InputFrame::default());
            let hawk = game.world.entity(actor).unwrap();
            saw_left |= hawk.facing < 0;
            saw_right |= hawk.facing > 0;
            let ray = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oEnemySight" && e.value("owner") == actor as f64)
                .expect("IDLE must cast after choosing its facing");
            assert_eq!(
                ray.pos.x - ray.previous_pos.x,
                f64::from(hawk.facing) * 10.0
            );
        }
        assert!(
            saw_left && saw_right,
            "exercise both natural facing choices"
        );
    }

    #[test]
    fn natural_hawk_walk_wall_turn_precedes_due_sight() {
        let mut exercised = false;
        // Vary only initial placement on the same authored floor. This finds a
        // natural wall turn on the six-tick sight cadence without injecting a
        // WALK state, facing, sight timer, velocity, or random stream.
        for initial_x in 84..114 {
            // The first facing choice is RIGHT under both RNG versions,
            // so the ordinary walk approaches the same authored right wall.
            let mut game = natural_sight_room(RulesPreset::Classic, 16, true);
            let actor = game.spawn("oHawkman", Vec2::new(f64::from(initial_x), 144.0));
            for _ in 0..32 {
                let before = game.world.entity(actor).unwrap().clone();
                let old_rays: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oEnemySight")
                    .map(|e| e.id)
                    .collect();
                game.step(&crate::InputFrame::default());
                let after = game.world.entity(actor).unwrap();
                if before.state == 1
                    && before.value("sight_timer") == 0.0
                    && before.facing != after.facing
                {
                    let ray = game
                        .world
                        .entities
                        .iter()
                        .find(|e| {
                            e.object == "oEnemySight"
                                && e.value("owner") == actor as f64
                                && !old_rays.contains(&e.id)
                        })
                        .expect("incoming WALK casts even after its wall turn");
                    assert_eq!(
                        ray.pos.x - ray.previous_pos.x,
                        f64::from(after.facing) * 10.0
                    );
                    exercised = true;
                    break;
                }
            }
            if exercised {
                break;
            }
        }
        assert!(exercised, "natural wall turn must coincide with a due ray");
    }

    #[test]
    fn direct_source_control_hawk_idle_counter_precedes_facing_and_sight() {
        // These are explicitly injected incoming-state controls, not a claim
        // that ordinary input produced each boundary counter. The signed
        // oracle follows source counter-=10, then positive-only decrement.
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for seed in [2, 42] {
                for wall in [false, true] {
                    for incoming in [0, 1, 5, 10, 11, 12, 37] {
                        let mut game = natural_sight_room(rules, seed, wall);
                        let actor = game.spawn("oHawkman", Vec2::new(112.0, 144.0));
                        let hawk = game.world.entity_mut(actor).unwrap();
                        hawk.timer = incoming;
                        hawk.vel = Vec2::default();
                        let mut source_counter = i64::from(incoming);
                        if wall {
                            source_counter -= 10;
                        }
                        if source_counter > 0 {
                            source_counter -= 1;
                        }
                        let expected_state = if source_counter < 1 { 1 } else { 0 };
                        let mut source_rng = game.rng.clone();
                        let expected_facing = if expected_state == 1 {
                            if (source_rng.next_u64() * 2) >> 32 == 0 {
                                -1
                            } else {
                                1
                            }
                        } else {
                            1
                        };
                        super::super::step_instance(&mut game, actor);
                        let hawk = game.world.entity(actor).unwrap();
                        assert_eq!(hawk.timer, source_counter.max(0) as u32);
                        assert_eq!(hawk.state, expected_state);
                        assert_eq!(hawk.facing, expected_facing);
                        assert_eq!(hawk.vel.y, if wall { -6.0 } else { 0.0 });
                        let ray = game
                            .world
                            .entities
                            .iter()
                            .find(|e| e.object == "oEnemySight" && e.value("owner") == actor as f64)
                            .expect("incoming IDLE emits after its counter/facing branch");
                        assert_eq!(ray.value("hspeed"), f64::from(expected_facing) * 10.0);
                        assert_eq!(game.rng.next_u64(), source_rng.next_u64());
                    }
                }
            }
        }
    }

    #[test]
    fn natural_hawk_stationary_idle_wall_jump_shortens_counter_before_walking() {
        let mut exercised = false;
        // Two authored walls bound a corridor. A normal WALK random pause
        // produces IDLE with vx=0 and counter20..50. No actor state, timer,
        // velocity, or random stream is changed after source-default birth.
        for seed in 0..32 {
            let mut game = natural_sight_room(RulesPreset::Classic, seed, true);
            for y in 0..10 {
                game.world.set_tile(5, y, "oBlock");
            }
            let actor = game.spawn("oHawkman", Vec2::new(96.0, 144.0));
            let mut saw_walk = false;
            for _ in 0..480 {
                let before = game.world.entity(actor).unwrap().clone();
                saw_walk |= before.state == 1;
                // Independent geometry of the authored walls at x80..95
                // and128..143; y144 is the top of the supported actor.
                let at_wall = (80.0..96.0).contains(&(before.pos.x - 1.0))
                    || (128.0..144.0).contains(&(before.pos.x + 16.0));
                let source_jump = saw_walk
                    && before.state == 0
                    && before.timer > 0
                    && before.vel == Vec2::default()
                    && before.pos.y == 144.0
                    && at_wall;
                game.step(&crate::InputFrame::default());
                if source_jump {
                    let after = game.world.entity(actor).unwrap();
                    assert_eq!(after.vel.y, -6.0);
                    assert_eq!(
                        after.timer,
                        before.timer.saturating_sub(10).saturating_sub(1)
                    );
                    let mut returned_to_walk = after.state == 1;
                    for _ in 0..240 {
                        if returned_to_walk {
                            break;
                        }
                        let before = game.world.entity(actor).unwrap().clone();
                        game.step(&crate::InputFrame::default());
                        let after = game.world.entity(actor).unwrap();
                        if before.state == 0 && after.state == 1 {
                            assert_eq!(after.timer, 0);
                            assert!(before.timer <= 11);
                            returned_to_walk = true;
                        }
                    }
                    assert!(
                        returned_to_walk,
                        "corrected counter reaches facing/WALK branch"
                    );
                    exercised = true;
                    break;
                }
            }
            if exercised {
                break;
            }
        }
        assert!(
            exercised,
            "ordinary WALK pause must reach stationary IDLE wall interaction"
        );
    }

    #[test]
    fn tomb_attack_refreshes_cooldown_until_animation_end() {
        // oTombLord Step ATTACK assigns attackTimer=100 on every step.
        let mut g = room();
        let id = g.world.spawn("oTombLord", Vec2::new(80.0, 128.0));
        let e = g.world.entity_mut(id).unwrap();
        e.state = 3;
        e.sprite = "sTombLordAttackL".into();
        e.animation = 13.5;
        e.data.insert("animation_speed".into(), 0.5);
        e.data.insert("attack_cooldown".into(), 10.0);
        g.step(&crate::InputFrame::default());
        let e = g.world.entity(id).unwrap();
        assert_eq!(e.state, 0);
        assert_eq!(e.value("attack_cooldown"), 100.0);
    }
    #[test]
    fn temple_embedded_actor_checks_follow_source_points() {
        let mut g = room();
        g.world.set_tile(5, 9, "oTemple");
        let mut hawk = Entity::new(100, "oHawkman", Vec2::new(80.0, 144.0));
        init(&mut hawk);
        tick(&mut g, &mut hawk);
        assert!(!hawk.alive);
        let mut tomb = Entity::new(101, "oTombLord", Vec2::new(80.0, 128.0));
        init(&mut tomb);
        g.world.set_tile(6, 9, "oTemple");
        tick(&mut g, &mut tomb);
        assert!(!tomb.alive);
        let sceptre = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oSceptre")
            .unwrap();
        assert_eq!(sceptre.pos, Vec2::new(96.0, 144.0));
    }
    #[test]
    fn source_stats_match() {
        for (name, hp) in [("oHawkman", 4), ("oTombLord", 20), ("oMagmaMan", 200)] {
            let mut e = Entity::new(1, name, Vec2::new(50.0, 50.0));
            init(&mut e);
            assert_eq!(e.health, hp);
        }
    }
    #[test]
    fn tomb_attack_spawns_fly_with_source_velocity() {
        let mut g = room();
        let mut e = Entity::new(1, "oTombLord", Vec2::new(80.0, 128.0));
        init(&mut e);
        e.state = 3;
        e.animation = 7.0;
        tick(&mut g, &mut e);
        let f = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oFly")
            .unwrap();
        assert!((3.0..=5.0).contains(&f.vel.x));
        assert!((-2.5..=0.5).contains(&f.vel.y));
    }
    #[test]
    fn slam_destroys_source_solid_family_even_when_bomb_proof() {
        let mut g = room();
        g.world.set_tile(7, 10, "oBlock");
        let mut e = Entity::new(1, "oOlmecSlam", Vec2::new(96.0, 160.0));
        init(&mut e);
        for x in 6..=9 {
            let index = 10 * g.world.width + x;
            assert!(collision(&mut g, &mut e, Instance::Terrain(index)));
        }
        assert!(
            e.alive,
            "the source Solid event does not destroy its emitter"
        );
        alarm(&mut g, &mut e, 0);
        assert!(g.world.tile(7, 10).unwrap().object.is_empty());
        assert!(g.world.tile(6, 10).unwrap().object.is_empty());
        assert!(g.world.tile(8, 10).unwrap().object.is_empty());
        assert!(!e.alive);
    }
    #[test]
    fn magma_man_falls_into_magma() {
        let mut g = room();
        let mut e = Entity::new(1, "oMagmaMan", Vec2::new(80.0, 80.0));
        init(&mut e);
        tick(&mut g, &mut e);
        assert!(!e.alive);
        assert_eq!(g.world.entities[0].object, "oMagma");
        assert_eq!(g.world.entities[0].health, 200);
    }
    #[test]
    fn magma_arms_bomb_once_without_extending_fuse() {
        let mut g = room();
        let mut magma = Entity::new(100, "oMagmaMan", Vec2::new(80.0, 144.0));
        init(&mut magma);
        let id = g.world.spawn("oBomb", Vec2::new(88.0, 152.0));
        assert!(collision(&mut g, &mut magma, Instance::Entity(id)));
        let bomb = g.world.entity_mut(id).unwrap();
        assert_eq!(bomb.value("armed"), 1.0);
        assert!((8..=12).contains(&bomb.timer));
        bomb.timer = 3;
        assert!(collision(&mut g, &mut magma, Instance::Entity(id)));
        assert_eq!(g.world.entity(id).unwrap().timer, 3);
    }
    #[test]
    fn olmec_prepares_for_twenty_ticks() {
        let mut g = room();
        let mut e = Entity::new(1, "oOlmec", Vec2::new(80.0, 100.0));
        init(&mut e);
        e.state = 5;
        e.timer = 20;
        for _ in 0..20 {
            tick(&mut g, &mut e);
            assert_eq!(e.state, 5);
        }
        tick(&mut g, &mut e);
        assert_eq!(e.state, 6);
        assert_eq!(e.vel.y, 5.0);
    }

    #[test]
    fn olmec_move_reciprocal_rounding_uses_gml_nearest_even_vectors() {
        // Isolated moveTo arithmetic. The authored intro's grounded state does
        // not demonstrate all these synthetic fractional velocities naturally.
        let world = World::new(40, 40, Area::Boss, 16);
        for (velocity, tick, expected) in [
            (0.4_f64, 2_u64, 1),
            (0.4, 3, 0),
            (-0.4, 2, -1),
            (-0.4, 3, 0),
            (2.0 / 9.0, 4, 1),
            (2.0 / 9.0, 5, 0),
            (-2.0 / 9.0, 4, -1),
            (2.0 / 7.0, 4, 1),
            (2.0 / 11.0, 6, 1),
            (1.4, 2, 1),
            (1.4, 3, 2),
            (2.4, 2, 2),
            (2.4, 3, 3),
            (1.5, 2, 2),
            (0.0, 2, 0),
        ] {
            let origin = Vec2::new(128.0, 128.0);
            let mut actor = Entity::new(1, "oOlmec", origin);
            actor.vel = Vec2::new(velocity, velocity);
            actor.data.insert("simulation_tick".into(), tick as f64);
            assert!(!olmec_move(&world, &mut actor));
            assert_eq!(
                actor.pos,
                Vec2::new(
                    origin.x + f64::from(expected),
                    origin.y + f64::from(expected),
                ),
                "velocity={velocity:?}, tick={tick}",
            );
        }
    }

    #[test]
    fn actual_step_olmec_naturally_drowns_with_gated_flame_each_step() {
        for lava_at_feet in [false, true] {
            let mut g = Game::legacy_shell(9, RulesPreset::Classic);
            g.world = World::new(24, 20, Area::Temple, 13);
            for x in 0..24 {
                g.world.set_tile(x, 10, "oTemple");
            }
            g.player.pos = Vec2::new(230.0, 120.0);
            if lava_at_feet {
                g.world.set_tile(5, 9, "oLava");
            }
            let id = g.spawn("oOlmec", Vec2::new(80.0, 80.0));
            // Only the authored room differs; Create supplies all boss state.
            for _ in 0..6 {
                g.step(&crate::InputFrame::default());
                let boss = g.world.entity(id).expect("head remains above lava");
                assert_eq!(boss.state == 4, lava_at_feet);
                let flames: Vec<_> = g
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::SoundIfIdle { name, pos } if name == "sndFlame" => Some(*pos),
                        _ => None,
                    })
                    .collect();
                assert_eq!(flames, if lava_at_feet { vec![boss.pos] } else { vec![] });
                assert!(!g.events.iter().any(|event| {
                    matches!(event, GameEvent::Sound { name, .. } if name == "sndFlame")
                }));
            }
        }
    }
}
