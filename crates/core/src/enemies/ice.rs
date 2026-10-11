//! Ice enemies translated from Classic 1.1 Create/Step/Animation end events.
//! Positions retain the original GameMaker sprite origins and source offsets.
//! The engine owns automatic collision, alarms, and sprite playback phases.
//! Decorative blood and dynamic tile-layer effects remain approximate.
//! Source contains no oAlienUFO object; this module does not invent that alias.
use super::move_body;
use crate::{Entity, Game, GameEvent, PlayerState, Rect, Vec2};

#[cfg(test)]
#[path = "player_hurt_rectangle_contract_tests.rs"]
mod player_hurt_rectangle_contract_tests;

pub fn matches(object: &str) -> bool {
    matches!(
        object,
        "oYeti"
            | "oYetiKing"
            | "oAlien"
            | "oUFO"
            | "oAlienBoss"
            | "oAlienEject"
            | "oUFOCrash"
            | "oLaser"
            | "oPsychicWave"
            | "oPsychicCreate"
    )
}

pub fn init(e: &mut Entity) {
    e.health = match e.object.as_str() {
        "oYeti" => 5,
        "oYetiKing" => 30,
        "oAlienBoss" => 10,
        _ => 1,
    };
    e.state = 0;
    e.set_image_speed(match e.object.as_str() {
        "oAlienEject" => 0.6,
        "oPsychicCreate" => 0.4,
        "oPsychicWave" | "oYetiKing" | "oAlienBoss" => 0.25,
        "oLaser" | "oUFOCrash" => 1.0,
        _ => 0.5,
    });
    if matches!(e.object.as_str(), "oYetiKing" | "oAlienBoss") {
        e.data.insert(
            "size_width".into(),
            if e.object == "oYetiKing" { 20.0 } else { 32.0 },
        );
        e.data.insert("size_height".into(), 32.0);
        e.set_image_speed(0.25);
    }
    if e.object == "oYetiKing" {
        e.data.insert("contact_damage".into(), 2.0);
    }
    match e.object.as_str() {
        "oAlienBoss" => {
            e.facing = -1;
            e.data.insert("psychic_recover".into(), 100.0);
        }
        "oYeti" | "oYetiKing" | "oAlien" => e.vel.x = 2.5,
        "oAlienEject" => {
            e.vel.y = -6.0;
            e.data.insert("counts_as_kill".into(), 1.0);
        }
        "oUFO" => {
            e.data.insert("shift_toggle".into(), -1.0);
        }
        _ => {}
    }
    if e.object == "oUFOCrash" {
        e.alarms[0] = 3;
    }
    if e.object == "oLaser" {
        e.alarms[0] = 1;
    }
    if e.object == "oAlienEject" {
        e.data.insert("drift_unset".into(), 1.0);
        for (key, value) in [
            ("collision_left", -4.0),
            ("collision_top", -4.0),
            ("collision_right", 4.0),
            ("collision_bottom", 4.0),
        ] {
            e.data.insert(key.into(), value);
        }
    }
    let sprite = match e.object.as_str() {
        "oYeti" => "sYetiLeft",
        "oYetiKing" => "sYetiKingRight",
        "oAlienBoss" => "sAlienBoss",
        "oAlienEject" => "sAlienEject",
        "oUFOCrash" => "sUFOCrash",
        "oPsychicWave" => "sPsychicWave",
        "oPsychicCreate" => "sPsychicCreate",
        "oLaser" => "sLaser",
        "oUFO" => "sUFO",
        _ => "sAlien",
    };
    e.set_sprite(sprite);
}

/// Source Create fields requiring only the generated room and its RNG.
pub(super) fn create_local(ctx: &mut crate::creation::EntityCreateContext<'_>, e: &mut Entity) {
    if e.object == "oUFO" && e.value("ice_ufo_local_created") == 0.0 {
        e.data
            .insert("shift_toggle".into(), f64::from(ctx.rng.range(2)));
        e.data.insert("ice_ufo_local_created".into(), 1.0);
    }
}

/// Context-dependent Create code, called synchronously before caller writes.
pub(super) fn create(game: &mut Game, e: &mut Entity) {
    create_local(
        &mut crate::creation::EntityCreateContext {
            world: &game.world,
            rng: &mut game.rng,
        },
        e,
    );
    if e.value("ice_context_created") != 0.0 {
        return;
    }
    match e.object.as_str() {
        "oAlienEject" => {
            e.facing = if game.rng.range(2) == 0 { -1 } else { 1 };
            e.data.remove("drift_unset");
        }
        "oPsychicCreate" => {
            let supplied = e
                .data
                .get("create_target_x")
                .copied()
                .zip(e.data.get("create_target_y").copied());
            let target = supplied
                .map(|(x, y)| Vec2::new(x, y))
                .or_else(|| first_boss_center(game, None));
            if let Some(target) = target {
                let delta = Vec2::new(target.x - e.pos.x, target.y - e.pos.y);
                // point_direction at coincident points is zero degrees.
                let angle = delta.y.atan2(delta.x);
                e.vel = Vec2::new(2.0 * angle.cos(), 2.0 * angle.sin());
            }
        }
        _ => {}
    }
    e.data.insert("ice_context_created".into(), 1.0);
}

fn first_boss_center(game: &Game, caller: Option<&Entity>) -> Option<Vec2> {
    // oAlienBoss.x/y reads the first live member of the current family array.
    // The calling Boss stays in that array while with_actor detaches its body.
    // Reading it must not synchronize births while Create's receiver is absent.
    game.dispatcher
        .registration_order
        .iter()
        .find_map(|instance| {
            let crate::dispatcher::Instance::Entity(id) = *instance else {
                return None;
            };
            let boss = caller
                .filter(|actor| actor.id == id)
                .or_else(|| game.world.entity(id))?;
            (boss.alive && boss.engine_active && crate::catalog::is_a(&boss.object, "oAlienBoss"))
                .then_some(Vec2::new(boss.pos.x + 16.0, boss.pos.y + 16.0))
        })
}

fn spawn_psychic_create(game: &mut Game, pos: Vec2, caller: &Entity) -> u64 {
    // Capture the source family read before nested Create; the absent caller
    // must retain its actual place rather than automatically winning the query.
    let target = first_boss_center(game, Some(caller));
    let id = game.world.spawn("oPsychicCreate", pos);
    if let Some(target) = target {
        if let Some(effect) = game.world.entity_mut(id) {
            effect.data.insert("create_target_x".into(), target.x);
            effect.data.insert("create_target_y".into(), target.y);
        }
    }
    crate::dispatcher::create(game, id);
    id
}

fn counter(e: &mut Entity, key: &str) -> f64 {
    let value = (e.value(key) - 1.0).max(0.0);
    e.data.insert(key.into(), value);
    value
}

fn enter(e: &mut Entity, state: i32) {
    // Source status writes do not restart sprite playback. Only explicit
    // image_index writes at attack entry reset the animation.
    e.state = state;
}

fn distance_to_player(game: &Game, e: &Entity) -> f64 {
    let a = e.sprite_bounds();
    let b = game.player.sprite_bounds();
    let dx = (b.x - a.x - a.w).max(a.x - b.x - b.w).max(0.0);
    let dy = (b.y - a.y - a.h).max(a.y - b.y - b.h).max(0.0);
    dx.hypot(dy)
}

pub(super) fn animation_end(game: &mut Game, e: &mut Entity, ended_sprite: &str) {
    match (e.object.as_str(), ended_sprite) {
        ("oYeti", "sYetiThrowL") => {
            e.state = 0;
            e.set_sprite("sYetiLeft");
        }
        ("oYetiKing", "sYetiKingTurnR") => {
            e.facing = 1;
            e.state = 1;
        }
        ("oYetiKing", "sYetiKingTurnL") => {
            e.facing = -1;
            e.state = 1;
        }
        ("oYetiKing", "sYetiKingYellL" | "sYetiKingYellR") => {
            e.state = 0;
            e.data.insert("counter".into(), 30.0);
            e.set_image_speed(0.25);
        }
        ("oAlienBoss", "sAlienBossDie") => {
            e.set_sprite("sAlienBossDead");
            let emitters: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|emitter| emitter.alive && emitter.object == "oBarrierEmitter")
                .map(|emitter| emitter.id)
                .collect();
            for id in emitters {
                super::destroy_barrier_emitter(game, id);
            }
        }
        ("oAlienBoss", "sAlienBossHurt") => e.set_sprite("sAlienBoss"),
        ("oUFO", "sUFOBlast") => {
            e.state = 1;
            e.set_sprite("sUFO");
        }
        ("oAlienEject", "sAlienDeploy") => e.set_sprite("sAlienParachute"),
        ("oPsychicWave" | "oPsychicCreate", _) => e.alive = false,
        _ => {}
    }
}

pub fn tick(game: &mut Game, e: &mut Entity) {
    if e.value("ice_context_created") == 0.0 {
        create(game, e);
    }
    match e.object.as_str() {
        "oYeti" => yeti(game, e),
        "oYetiKing" => king(game, e),
        "oAlien" => alien(game, e),
        "oUFO" => ufo(game, e),
        "oAlienBoss" => boss(game, e),
        "oAlienEject" => eject(game, e),
        "oUFOCrash" => crash(game, e),
        "oLaser" => laser(game, e),
        "oPsychicWave" => psychic(game, e),
        "oPsychicCreate" => {
            e.pos.x += e.vel.x;
            e.pos.y += e.vel.y;
        }
        _ => {}
    }
}

fn yeti(game: &mut Game, e: &mut Entity) {
    let impact_y = (e.vel.y + super::inherited::gravity(e, 0.6)).min(10.0);
    let (wall, floor) = move_body(&game.world, e, 0.6);
    if e.state != 99 && e.state != 98 && e.health < 1 {
        e.state = 99;
    }
    let crush_y = if e.state >= 98 { 12.0 } else { 8.0 };
    if (e.state >= 98 || !e.held)
        && game
            .world
            .solid_point_except(Vec2::new(e.pos.x + 8.0, e.pos.y + crush_y), e.id)
    {
        // This source destruction bypasses DEAD and its kill-credit block.
        e.alive = false;
        game.sound("sndCavemanDie", e.pos);
        return;
    }
    if e.state == 99 {
        if e.value("dead") == 0.0 {
            super::record_death(game, e);
            game.sound("sndCavemanDie", e.pos);
            e.data.insert("dead".into(), 1.0);
        }
        e.set_sprite("sYetiDeadL");
        if e.vel.x > 0.0 || e.vel.y > 0.0 {
            e.state = 98;
        }
        settle_yeti(e, wall, floor, impact_y);
        if e.vel.x == 0.0 && e.vel.y == 0.0 && e.health < 1 {
            e.state = 99;
        }
        // Source's final held-sprite selection also follows the DEAD branch.
        if e.held {
            e.set_sprite(if e.health > 0 {
                "sYetiHeldL"
            } else {
                "sYetiDHeldL"
            });
        }
        return;
    }
    if e.state == 98 {
        if e.vel.x == 0.0 && e.health > 0 {
            e.set_sprite("sYetiStunL");
        } else if e.value("bounced") != 0.0 {
            e.set_sprite(if e.vel.y < 0.0 {
                "sYetiBounceL"
            } else {
                "sYetiFallL"
            });
        } else {
            e.set_sprite(if e.vel.x.abs() > 0.0 {
                "sYetiDieLL"
            } else {
                "sYetiDieLR"
            });
        }
        // Source recovery runs while carried or supported, including a
        // carried Yeti in midair; horizontal settling is independent below.
        if e.held || floor {
            if e.timer > 0 {
                e.timer -= 1;
            } else if e.health > 0 {
                enter(e, 0);
                if e.held {
                    e.held = false;
                    game.player.held = None;
                    game.player.pickup_item_type.clear();
                }
            }
        }
        settle_yeti(e, wall, floor, impact_y);
        if e.vel.x == 0.0 && e.vel.y == 0.0 && e.health < 1 {
            e.state = 99;
        }
        if e.held {
            e.set_sprite(if e.health > 0 {
                "sYetiHeldL"
            } else {
                "sYetiDHeldL"
            });
        }
        return;
    }
    if e.held {
        e.set_sprite("sYetiHeldL");
        return;
    }
    if e.state == 3 {
        e.vel.x = 0.0;
        e.set_sprite("sYetiThrowL");
        return;
    }
    match e.state {
        0 => {
            if floor
                && (game
                    .world
                    .solid_point_except(Vec2::new(e.pos.x - 1.0, e.pos.y), e.id)
                    || game
                        .world
                        .solid_point_except(Vec2::new(e.pos.x + 16.0, e.pos.y), e.id))
            {
                e.vel.y = -6.0;
                e.vel.x = f64::from(e.facing);
                e.data.insert("counter".into(), e.value("counter") - 10.0);
            }
            if floor && e.value("counter") > 0.0 {
                counter(e, "counter");
            }
            if e.value("counter") < 1.0 {
                e.facing = if game.rng.range(2) == 0 { -1 } else { 1 };
                enter(e, 1);
            }
        }
        1 => {
            if wall {
                e.facing = -e.facing;
            }
            if floor {
                if game.rng.range(100) == 0
                    || !game.world.solid_at(
                        e.pos.x + if e.facing < 0 { -1.0 } else { 16.0 },
                        e.pos.y + 16.0,
                    )
                {
                    enter(e, 0);
                    e.vel.x = 0.0;
                    e.data
                        .insert("counter".into(), f64::from(20 + game.rng.range(31)));
                } else {
                    e.vel.x = f64::from(e.facing) * 1.5;
                }
            }
        }
        2 => {
            e.set_image_speed(1.0);
            if wall {
                e.facing = -e.facing;
            }
            e.vel.x = f64::from(e.facing) * 3.0;
        }
        _ => {}
    }
    // Source Yeti sight rays alert cavemen and hawkmen via oEnemySight. The
    // source sight collision does not place the Yeti itself in ATTACK.
    if e.value("sight") == 0.0 && matches!(e.state, 0 | 1) {
        let id = game.spawn("oEnemySight", e.pos);
        if let Some(sight) = game.world.entity_mut(id) {
            sight.vel.x = f64::from(e.facing) * 10.0;
            sight.data.insert("owner".into(), e.id as f64);
        }
        e.data.insert("sight".into(), 5.0);
    } else if matches!(e.state, 0 | 1) {
        counter(e, "sight");
    }
    if e.vel.x > 0.0 {
        e.vel.x -= 0.1;
    } else if e.vel.x < 0.0 {
        e.vel.x += 0.1;
    }
    if e.vel.x.abs() < 0.5 {
        e.vel.x = 0.0;
    }
    if e.state != 3 {
        let sprite = if e.vel.x == 0.0 {
            "sYetiLeft"
        } else {
            "sYetiRunLeft"
        };
        e.set_sprite(sprite);
    }
}

fn settle_yeti(e: &mut Entity, wall: bool, floor: bool, impact_y: f64) {
    // scrCheckCollisions narrows the body after a knockdown and applies ground
    // friction and a half-height bounce. Capture impact speed before move_body
    // zeroes its resolved vertical collision velocity.
    for (key, value) in [
        ("collision_left", 2.0),
        ("collision_top", 6.0),
        ("collision_right", 14.0),
        ("collision_bottom", 16.0),
    ] {
        e.data.insert(key.into(), value);
    }
    if wall {
        e.vel.x *= -0.5;
    }
    if floor {
        e.vel.y = if impact_y > 1.0 {
            -impact_y * 0.5
        } else if impact_y.abs() < 1.0 {
            0.0
        } else {
            impact_y
        };
        e.vel.x = if e.vel.x.abs() < 0.1 {
            0.0
        } else {
            e.vel.x * 0.3
        };
        e.data.insert("bounced".into(), 1.0);
    }
    e.vel.x = (e.vel.x.abs() - 0.1).max(0.0) * e.vel.x.signum();
    if e.vel.x.abs() < 0.5 {
        e.vel.x = 0.0;
    }
}

fn alien(game: &mut Game, e: &mut Entity) {
    let next_y = e.vel.y + 0.6;
    let (wall, floor) = move_body(&game.world, e, 0.6);
    // Alien Step does not apply the inherited yVelLimit and moveTo itself
    // does not clear velocity when it resolves a ceiling collision.
    e.vel.y = if floor && e.state != 98 { 0.0 } else { next_y };
    if game
        .world
        .solid_point_except(Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), e.id)
    {
        e.health = 0;
    }
    if e.health < 1 {
        super::record_death(game, e);
        e.alive = false;
        return;
    }
    if e.state == 98 {
        if floor {
            e.timer = e.timer.saturating_sub(1);
            if e.timer == 0 {
                enter(e, 0);
            }
        }
        return;
    }
    if e.state == 0 {
        if counter(e, "counter") == 0.0 {
            e.facing = if game.rng.range(2) == 0 { -1 } else { 1 };
            enter(e, 1);
        }
    } else if e.state == 1 {
        if wall {
            e.facing = -e.facing;
        }
        let left = game.world.solid_at(e.pos.x - 1.0, e.pos.y + 16.0)
            && !game.world.solid_at(e.pos.x - 1.0, e.pos.y);
        let right = game.world.solid_at(e.pos.x + 16.0, e.pos.y + 16.0)
            && !game.world.solid_at(e.pos.x + 16.0, e.pos.y);
        if e.facing < 0 && !left {
            e.facing = 1;
        } else if e.facing > 0 && !right {
            e.facing = -1;
        }
        e.vel.x = if !left && !right {
            0.0
        } else {
            f64::from(e.facing)
        };
        if game.rng.range(100) == 0 {
            enter(e, 0);
            e.vel.x = 0.0;
            e.data
                .insert("counter".into(), f64::from(20 + game.rng.range(31)));
        }
    }
}

fn king(game: &mut Game, e: &mut Entity) {
    let next_y = (e.vel.y + super::inherited::gravity(e, 0.6)).min(10.0);
    let (wall, floor) = move_body(&game.world, e, 0.6);
    e.vel.y = if floor && e.state != 98 { 0.0 } else { next_y };
    if game
        .world
        .solid_point_except(Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0), e.id)
    {
        e.health = 0;
    }
    if e.health < 1 {
        for _ in 0..4 {
            let item = if game.rng.range(12) == 0 {
                "oSpikeShoes"
            } else if game.rng.range(2) == 0 {
                "oSapphireBig"
            } else {
                "oRopePile"
            };
            let id = game.spawn(item, Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
            let vx = f64::from(game.rng.range(4)) - f64::from(game.rng.range(4));
            let vy = -f64::from(1 + game.rng.range(2));
            if let Some(loot) = game.world.entity_mut(id) {
                loot.vel = Vec2::new(vx, vy);
            }
        }
        super::record_death(game, e);
        e.alive = false;
        return;
    }
    counter(e, "hit_cooldown");
    counter(e, "attack_timer");
    if matches!(e.state, 0 | 1) {
        counter(e, "counter");
    }
    match e.state {
        0 => {
            e.vel.x = 0.0;
            if e.value("counter") == 0.0 {
                enter(e, 1);
            }
        }
        1 => {
            let dx = game.player.pos.x - (e.pos.x + 16.0);
            let dy = (game.player.pos.y - (e.pos.y + 16.0)).abs();
            let turn_dy = (game.player.pos.y - (e.pos.y + 32.0)).abs();
            if wall
                || (dx * f64::from(e.facing) < 0.0 && turn_dy < 16.0 && e.value("counter") == 0.0)
            {
                enter(e, 2);
                e.vel.x = 0.0;
                e.data.insert("counter".into(), 30.0);
            } else if dx * f64::from(e.facing) > 0.0 && dy < 32.0 && e.value("attack_timer") == 0.0
            {
                enter(e, 3);
                e.animation = 0.0;
                e.vel.x = 0.0;
            } else {
                e.vel.x = f64::from(e.facing);
            }
        }
        2 => {
            e.vel.x = 0.0;
        }
        3 => {
            e.vel.x = 0.0;
            e.data.insert("attack_timer".into(), 100.0);
            e.set_image_speed(0.5);
            let frame = e.animation;
            if (7.0..=12.0).contains(&frame) {
                if frame == 7.0 {
                    game.sound("sndYetiYell", e.pos);
                }
                king_yell_ice(game, e);
            }
        }
        _ => {}
    }
    let sprite = match (e.state, e.facing < 0) {
        (2, true) => "sYetiKingTurnR",
        (2, false) => "sYetiKingTurnL",
        (3, true) => "sYetiKingYellL",
        (3, false) => "sYetiKingYellR",
        (1, true) => "sYetiKingWalkL",
        (1, false) => "sYetiKingWalkR",
        (_, true) => "sYetiKingLeft",
        _ => "sYetiKingRight",
    };
    e.set_sprite(sprite);
    if super::solid_mask(&game.world, e.bounds(), e.id) {
        e.pos.y -= 2.0;
    }
}

fn king_yell_ice(game: &mut Game, receiver: &Entity) {
    use crate::dispatcher::Instance;

    // Object-qualified oYetiKing.x/y select the first active family member.
    // with_actor detaches this event's receiver, so retain its position at its
    // original registration slot instead of treating the only King as absent.
    let target = game
        .dispatcher
        .registration_order
        .iter()
        .copied()
        .find_map(|instance| {
            if instance == Instance::Entity(receiver.id) {
                return Some(receiver.pos);
            }
            (crate::dispatcher::is_active(game, instance)
                && crate::catalog::is_a(instance.object(game), "oYetiKing")
                && matches!(instance, Instance::Entity(id) if game.world.entity(id).is_some()))
            .then(|| instance.position(game))
        })
        // Focused event callers may supply a receiver before registration.
        .unwrap_or(receiver.pos);

    // Source with(oIce) captures its initial count and skips dead slots before
    // the body. Its only family deletion is the current receiver; IceBlock,
    // Drip and released Caveman births are outside oIce. Snapshot identities
    // preserve that traversal even when nested Rust Create compacts the lists.
    let ice = crate::dispatcher::descendant_instances(game, "oIce");
    for instance in ice {
        if !crate::dispatcher::is_active(game, instance)
            || !crate::catalog::is_a(instance.object(game), "oIce")
            || matches!(instance, Instance::Entity(id) if game.world.entity(id).is_none())
        {
            continue;
        }
        let roll = game.rng.range(60);
        let origin = instance.position(game);
        if roll == 0
            && ((target.x + 16.0) - (origin.x + 8.0)).abs() > 16.0
            && origin.distance(target) < 96.0
        {
            // Source creates the falling block before dispatching Ice Destroy.
            // Own destruction bypasses attack invincibility and completes its
            // Destroy callbacks before the next receiver's roll.
            game.spawn("oIceBlock", origin);
            match instance {
                Instance::Terrain(index) => {
                    let (x, y) = (index % game.world.width, index / game.world.width);
                    crate::items::explosives::destroy_terrain_self(game, x, y);
                }
                Instance::Entity(id) => {
                    crate::items::explosives::destroy_solid_self(game, id);
                }
                Instance::Player => unreachable!("oPlayer1 is outside oIce"),
            }
        }
    }

    // ThinIce is a sibling of Ice, so its distinct pass consumes no ice roll.
    // Moving-trap thickness is stored on entity instances, initialized to 60.
    for instance in crate::dispatcher::descendant_instances(game, "oThinIce") {
        if !crate::dispatcher::is_active(game, instance) {
            continue;
        }
        if let Instance::Entity(id) = instance
            && let Some(ice) = game.world.entity_mut(id)
        {
            *ice.data.entry("thickness".into()).or_insert(60.0) -= 2.0;
        }
    }
}

fn ufo(game: &mut Game, e: &mut Entity) {
    let previous_y = e.vel.y;
    let (wall, _) = move_body(&game.world, e, 0.0);
    e.vel.y = previous_y;
    if e.health < 1 {
        game.spawn("oUFOCrash", e.pos);
        super::record_death(game, e);
        e.alive = false;
        return;
    }

    if e.pos.distance(game.player.pos) < 160.0 && e.value("alerted") == 0.0 {
        e.data.insert("alerted".into(), 1.0);
        game.sound("sndAlien", e.pos);
    }
    if e.state == 2 {
        e.vel = Vec2::default();
        e.set_sprite("sUFOBlast");
        let frame = e.animation;
        if frame == 16.0 {
            game.spawn("oLaser", Vec2::new(e.pos.x + 8.0, e.pos.y + 16.0));
            game.sound("sndLaser", e.pos);
        }
        return;
    }
    // Only SEARCH checks the character's swimming/dead flags. Its moveTo
    // above still uses the previous velocity while this branch is skipped.
    if e.state != 1
        && (!super::player_alive(game)
            || game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0)
    {
        return;
    }
    let limit = if e.state == 1 { 64.0 } else { 32.0 };
    let mut shift = e.value("shift");
    if e.value("shift_toggle") == 0.0 {
        e.vel.x = 1.0;
        shift += 1.0;
        if shift >= limit {
            e.data.insert("shift_toggle".into(), 1.0);
        }
    } else {
        e.vel.x = -1.0;
        shift -= 1.0;
        if shift <= -limit {
            e.data.insert("shift_toggle".into(), 0.0);
        }
    }
    if wall {
        e.data
            .insert("shift_toggle".into(), if e.vel.x > 0.0 { 1.0 } else { 0.0 });
    }
    e.data.insert("shift".into(), shift);
    if e.state == 1 {
        e.vel.y = 0.0;
        if (game.player.pos.x - (e.pos.x + 8.0)).abs() < 8.0 {
            enter(e, 2);
            game.sound("sndLaserCharge", e.pos);
        }
        if game.player.pos.y < e.pos.y || e.pos.y < game.player.pos.y - 96.0 {
            enter(e, 0);
        }
    } else {
        let next = if e.pos.y > game.player.pos.y - 48.0 {
            -2.0
        } else {
            2.0
        };
        if e.vel.y == -next {
            enter(e, 1);
        }
        e.vel.y = next;
    }
}

fn boss(game: &mut Game, e: &mut Entity) {
    // Boss Step checks crushing before its moveTo, unlike the walkers.
    if game
        .world
        .solid_point_except(Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), e.id)
    {
        e.health = 0;
    }
    if e.health < 1 && e.state != 99 {
        enter(e, 99);
        e.set_sprite("sAlienBossDie");
        for _ in 0..4 {
            let name = match game.rng.range(3) {
                0 => "oEmeraldBig",
                1 => "oSapphireBig",
                _ => "oRubyBig",
            };
            let id = game.spawn(name, Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
            let vx = f64::from(game.rng.range(4)) - f64::from(game.rng.range(4));
            if let Some(gem) = game.world.entity_mut(id) {
                gem.vel = Vec2::new(vx, -2.0);
            }
        }
        super::record_death(game, e);
    }
    // oAlienBoss Step emits from the death sprite before gravity/moveTo.
    // scrCreateBlood has its own bloodless/cap-16 guard; the following direct
    // Create is independent and retains inherited Detritus cap-32 behavior.
    if e.sprite == "sAlienBossDie" && game.rng.range(2) == 0 {
        let pos = Vec2::new(e.pos.x + 8.0, e.pos.y + 14.0 + f64::from(game.rng.range(5)));
        super::create_blood(game, e, pos, 1);
        let pos = Vec2::new(e.pos.x + 8.0, e.pos.y + 14.0 + f64::from(game.rng.range(5)));
        game.spawn("oBlood", pos);
    }
    e.vel.x = (e.vel.x.abs() - 0.1).max(0.0) * e.vel.x.signum();
    if e.vel.x.abs() < 0.5 || e.state == 0 {
        e.vel.x = 0.0;
    }
    // Boss Step applies gravity before moveTo (the walkers do it after).
    e.vel.y = (e.vel.y + 0.6).min(10.0);
    let b = e.bounds();
    let feet = Rect::new(b.x, b.y + b.h, b.w, 1.0);
    if super::solid_mask(&game.world, feet, e.id) {
        e.vel.y = 0.0;
    }
    move_body(&game.world, e, 0.0);
    if super::solid_mask(&game.world, e.bounds(), e.id) {
        e.pos.y -= 2.0;
    }
    if e.sprite != "sAlienBossHurt" {
        e.set_image_speed(0.25);
    }
    if e.state == 99 {
        return;
    }
    let recovered = e.value("psychic_recover") == 0.0;
    counter(e, "psychic_recover");
    if recovered
        && distance_to_player(game, e) < 96.0
        && super::player_alive(game)
        && game.player.stun == 0
        && game.player.state != PlayerState::Stunned
        && game.player.invincible == 0
    {
        for _ in 0..6 {
            let pos = Vec2::new(
                e.pos.x + 16.0 + f64::from(game.rng.range(33)) - f64::from(game.rng.range(33)),
                e.pos.y + 16.0 + f64::from(game.rng.range(33)) - f64::from(game.rng.range(33)),
            );
            spawn_psychic_create(game, pos, e);
        }
        game.spawn("oPsychicWave", Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
        e.data.insert("psychic_recover".into(), 100.0);
        game.sound("sndPsychic", e.pos);
    }
}

fn eject(game: &mut Game, e: &mut Entity) {
    // Eject Step uses raw x/y motion rather than the platform moveTo solver.
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    if e.state == 0 {
        e.vel.x = 0.0;
        e.vel.y += 0.5;
        let top = Rect::new(e.pos.x - 4.0, e.pos.y - 5.0, 8.0, 1.0);
        if e.vel.y >= 0.0 || super::solid_mask(&game.world, top, e.id) {
            enter(e, 2);
            e.vel.y = 0.0;
            e.set_sprite("sAlienDeploy");
        }
        // Still executes on the EJECT-to-FLOAT transition frame.
        if game.rng.chance(1, 5) {
            let pos = Vec2::new(
                e.pos.x + f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
                e.pos.y + f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
            );
            game.spawn("oBurn", pos);
        }
    } else {
        e.vel.x = 0.0;
        e.vel.y = 2.0;
        if game
            .world
            .solid_point_except(Vec2::new(e.pos.x, e.pos.y + 6.0), e.id)
        {
            game.spawn("oAlien", Vec2::new(e.pos.x - 8.0, e.pos.y - 12.0));
            e.alive = false;
            return;
        }
        e.vel.x = f64::from(e.facing);
        if game
            .world
            .solid_point_except(Vec2::new(e.pos.x + e.vel.x * 8.0, e.pos.y), e.id)
        {
            e.facing = 0;
        }
    }
}

fn crash(_game: &mut Game, e: &mut Entity) {
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    if e.vel.y < 6.0 {
        e.vel.y += 0.6;
    }
}

/// These two hazards explicitly subtract life in oPlayer1 Step. They bypass
/// contact invincibility and do not grant a new invincibility interval.
fn hurt_player(game: &mut Game, amount: i32) {
    if game.player.health > 0 {
        game.player.health -= amount;
        game.events.push(GameEvent::Damage { amount });
    }
    game.sound("sndHurt", game.player.pos);
}

fn laser(_game: &mut Game, e: &mut Entity) {
    e.pos.y += e.vel.y;
    e.vel.y += 0.6;
    if e.vel.y > 4.0 {
        e.vel.y = 0.0;
    }
}

fn psychic(game: &mut Game, e: &mut Entity) {
    let dx = game.player.pos.x - e.pos.x;
    let dy = game.player.pos.y - e.pos.y;
    let distance = dx.hypot(dy);
    if distance > 0.0 {
        e.pos.x += dx / distance * 2.0;
        e.pos.y += dy / distance * 2.0;
    }
}

/// Literal late oPlayer1 Step queries: first test an imprecise rectangle,
/// then affect the nearest active instance, which need not be that overlap.
pub(super) fn player_hurt(game: &mut Game) {
    // The dispatcher latches the enclosing source hurt guard before Crush;
    // writes inside that block do not reevaluate it for Laser/PsychicWave.
    for (object, damage, stun, cause) in [
        ("oLaser", 3, 20, crate::statistics::EnemyDeath::Ufo),
        (
            "oPsychicWave",
            1,
            40,
            crate::statistics::EnemyDeath::AlienBoss,
        ),
    ] {
        if object == "oPsychicWave"
            && (game.player.stun > 0
                || game.player.state == PlayerState::Stunned
                || !super::player_alive(game))
        {
            continue;
        }
        let first = Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 8.0);
        let second = Vec2::new(game.player.pos.x + 8.0, game.player.pos.y + 8.0);
        if !game.world.entities.iter().any(|entity| {
            entity.alive
                && entity.engine_active
                && entity.object == object
                && entity
                    .collision_mask()
                    .intersects_rectangle_bbox(first, second)
        }) {
            continue;
        }
        let Some(target) = game
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && entity.engine_active && entity.object == object)
            .min_by(|a, b| {
                a.pos
                    .distance(game.player.pos)
                    .total_cmp(&b.pos.distance(game.player.pos))
            })
        else {
            continue;
        };
        let (id, pos) = (target.id, target.pos);
        if object == "oLaser" {
            let had_life = game.player.health > 0;
            if had_life {
                game.player.health -= damage;
                game.events.push(GameEvent::Damage { amount: damage });
                if game.player.health <= 0 && crate::statistics::is_real_level(game) {
                    game.lifetime_stats.enemy_death(cause);
                }
            }
            game.player.vel = Vec2::new(if pos.x < game.player.pos.x { 2.0 } else { -2.0 }, -4.0);
            crate::player::script_player_blood(game, 3);
            crate::world_events::spawn_effect(game, "oLaserExplode", pos);
            if let Some(target) = game
                .world
                .entities
                .iter_mut()
                .find(|entity| entity.id == id)
            {
                target.alive = false;
            }
            game.sound("sndHurt", game.player.pos);
            game.player.stun = stun;
            game.player.state = PlayerState::Stunned;
        } else {
            // PsychicWave has no source blood call; retain its independent path.
            let had_life = game.player.health > 0;
            if had_life {
                game.player.health -= damage;
                game.events.push(GameEvent::Damage { amount: damage });
                if game.player.health <= 0 && crate::statistics::is_real_level(game) {
                    game.lifetime_stats.enemy_death(cause);
                }
            }
            game.player.vel = Vec2::new(if pos.x < game.player.pos.x { 2.0 } else { -2.0 }, -4.0);
            game.sound("sndHurt", game.player.pos);
            game.player.stun = stun;
            game.player.state = PlayerState::Stunned;
        }
    }
}

/// Direct source Collision handlers. Pair precision and event ordering are
/// supplied by the dispatcher; this callback never scans for another pair.
pub(super) fn collision(
    game: &mut Game,
    e: &mut Entity,
    other: crate::dispatcher::Instance,
) -> bool {
    use crate::dispatcher::Instance;
    match other {
        Instance::Player => match e.object.as_str() {
            "oYeti" => {
                yeti_player_collision(game, e);
                true
            }
            "oYetiKing" | "oAlienBoss" => {
                giant_player_collision(game, e);
                true
            }
            "oAlienEject" => {
                eject_player_collision(game, e);
                true
            }
            _ => false,
        },
        Instance::Terrain(index) => {
            let solid = game
                .world
                .tiles
                .get(index)
                .is_some_and(|tile| crate::catalog::is_a(&tile.object, "oSolid"));
            if !solid {
                return false;
            }
            match e.object.as_str() {
                "oUFOCrash" => {
                    crash_impact(game, e);
                    true
                }
                "oLaser" => {
                    let source = laser_terrain_source(game, index);
                    if source.value("invincible") == 0.0 {
                        remove_embedded_gold(game, &source);
                        crate::items::explosives::destroy_terrain_self(
                            game,
                            index % game.world.width,
                            index / game.world.width,
                        );
                    }
                    laser_solid_decoration(game, source.pos);
                    laser_impact(game, e);
                    true
                }
                _ => false,
            }
        }
        Instance::Entity(id) => {
            let Some(target) = game.world.entities.iter().find(|target| target.id == id) else {
                return false;
            };
            let object = target.object.clone();
            let solid = crate::catalog::is_a(&object, "oSolid");
            let enemy = crate::catalog::is_a(&object, "oEnemy");
            match e.object.as_str() {
                "oAlienEject" if object == "oWeb" => {
                    game.spawn("oAlien", Vec2::new(e.pos.x - 8.0, e.pos.y - 12.0));
                    e.alive = false;
                    true
                }
                "oUFOCrash" if solid || enemy => {
                    crash_impact(game, e);
                    true
                }
                "oLaser" if solid => {
                    let target = target.clone();
                    if target.value("invincible") == 0.0 {
                        remove_embedded_gold(game, &target);
                        crate::items::explosives::destroy_solid_self(game, id);
                    }
                    // The receiver continues here even after instance_destroy,
                    // and invincible targets also lose one layer-3 tile.
                    laser_solid_decoration(game, target.pos);
                    laser_impact(game, e);
                    true
                }
                "oLaser" | "oPsychicWave" if enemy || object == "oDamsel" => {
                    let laser = e.object == "oLaser";
                    let damsel = object == "oDamsel";
                    let excluded = if laser {
                        object == "oUFO"
                    } else {
                        object == "oAlienBoss"
                    };
                    let vulnerable = !excluded && target.value("invincible") == 0.0;
                    if vulnerable {
                        // Both original handlers consume these two draws even
                        // though the following xVel=-1 overwrites their result.
                        let _ = game.rng.range(3);
                        let _ = game.rng.range(2);
                        if let Some(target) = game
                            .world
                            .entities
                            .iter_mut()
                            .find(|target| target.id == id)
                        {
                            target.health -= 3;
                            target.vel = Vec2::new(-1.0, -6.0);
                        }
                        if damsel {
                            e.state = 2;
                        }
                        if laser {
                            laser_impact(game, e);
                        }
                    }
                    if !laser && damsel {
                        e.alive = false;
                    }
                    true
                }
                _ => false,
            }
        }
    }
}

fn laser_terrain_source(game: &Game, index: usize) -> Entity {
    let origin = Vec2::new(
        (index % game.world.width) as f64 * 16.0,
        (index / game.world.width) as f64 * 16.0,
    );
    let object = &game.world.tiles[index].object;
    let mut source = Entity::new(0, object, origin);
    // Generated protected terrain records its instance flag in metadata;
    // otherwise retain the literal invincible defaults of these own Creates.
    let invincible = matches!(
        object.as_str(),
        "oHardBlock" | "oLavaSolid" | "oMoai" | "oMoai2" | "oMoai3" | "oMoaiInside"
    );
    source
        .data
        .insert("invincible".into(), f64::from(invincible));
    if let Some(metadata) = game.world.entities.iter().find(|metadata| {
        metadata.alive
            && metadata.object == "oTerrainMetadata"
            && metadata.pos == Vec2::new(origin.x + 8.0, origin.y + 8.0)
    }) {
        source.data.extend(metadata.data.clone());
        if let Some(sprite) = metadata.text.get("terrain_sprite") {
            source.set_sprite(sprite);
        }
    }
    source
}

fn remove_embedded_gold(game: &mut Game, source: &Entity) {
    for object in ["oGold", "oGoldBig"] {
        // Source first gates with collision_point(...,precise=0), then selects
        // one instance_place using the solid receiver's full sprite mask.
        if !game.world.entities.iter().any(|gold| {
            gold.alive
                && crate::catalog::is_a(&gold.object, object)
                && gold.mask_contains(source.pos, crate::CollisionPrecision::BoundingBox)
        }) {
            continue;
        }
        if let Some(id) = game
            .world
            .entities
            .iter()
            .find(|gold| {
                gold.alive
                    && crate::catalog::is_a(&gold.object, object)
                    && source.event_intersects(gold)
            })
            .map(|gold| gold.id)
        {
            crate::items::collectibles::destroy_gold(game, id);
        }
    }
}

fn laser_solid_decoration(game: &mut Game, origin: Vec2) {
    let point = Vec2::new(origin.x, origin.y - 16.0);
    if let Some(tile) = game.world.entities.iter_mut().find(|tile| {
        tile.alive
            && tile.object == "oGenerationDecoration"
            && tile.value("depth") == 3.0
            && Rect::new(
                tile.pos.x,
                tile.pos.y,
                tile.data.get("source_w").copied().unwrap_or(16.0),
                tile.data.get("source_h").copied().unwrap_or(16.0),
            )
            .contains(point)
    }) {
        tile.alive = false;
    }
}

fn laser_impact(game: &mut Game, e: &mut Entity) {
    crate::world_events::spawn_effect(game, "oLaserExplode", e.pos);
    e.alive = false;
}

fn crash_impact(game: &mut Game, e: &mut Entity) {
    game.spawn("oExplosion", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
    game.sound("sndExplosion", e.pos);
    e.alive = false;
}

fn hurt_player_cause(game: &mut Game, amount: i32, cause: crate::statistics::EnemyDeath) {
    let had_life = game.player.health > 0;
    hurt_player(game, amount);
    if had_life && game.player.health <= 0 && crate::statistics::is_real_level(game) {
        game.lifetime_stats.enemy_death(cause);
    }
}

fn yeti_player_collision(game: &mut Game, e: &mut Entity) {
    if (game.player.pos.x - (e.pos.x + 8.0)).abs() > 8.0 {
        return;
    }
    if super::player_alive(game)
        && game.player.stun == 0
        && game.player.state != PlayerState::Stunned
        && matches!(
            game.player.state,
            PlayerState::Jumping | PlayerState::Falling
        )
        && game.player.pos.y < e.pos.y + 5.0
        && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
    {
        if e.state < 98 {
            let damage =
                (game.player.data.get("fall_timer").copied().unwrap_or(0.0) / 16.0).ceil() as i32;
            e.health -= damage * if game.player.has("spike_shoes") { 3 } else { 1 };
            game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
            game.player.data.insert("fall_timer".into(), 0.0);
            e.state = 98;
            e.timer = e.value("stun_time") as u32;
            e.vel.y = -6.0;
            e.vel.x += if game.player.pos.x < e.pos.x + 8.0 {
                1.0
            } else {
                -1.0
            };
            e.set_image_speed(0.5);
            // Unlike inherited stomps, Yeti's override does not set countsAsKill.
            game.sound("sndHit", e.pos);
        }
        return;
    }
    if game.player.invincible == 0 && e.state < 98 {
        if game.world.solid_at(e.pos.x + 8.0, e.pos.y - 4.0) {
            hurt_player_cause(game, 1, crate::statistics::EnemyDeath::Yeti);
            game.player.invincible = 30;
            game.player.data.insert("blink".into(), 30.0);
            game.player.vel.x = if game.player.pos.x < e.pos.x {
                -6.0
            } else {
                6.0
            };
        } else if e.state != 3 {
            e.facing = if game.player.pos.x > e.pos.x + 8.0 {
                1
            } else {
                -1
            };
            game.player.pos = Vec2::new(e.pos.x + if e.facing > 0 { 0.0 } else { 16.0 }, e.pos.y);
            game.player.previous_pos = game.player.pos;
            enter(e, 3);
            e.vel.x = 0.0;
            game.player.vel = Vec2::new(f64::from(e.facing) * 6.0, -6.0);
            game.player.state = PlayerState::Stunned;
            game.player.data.insert("wall_hurt".into(), 1.0);
            game.player.data.insert("bounced".into(), 0.0);
            super::drop_from_throw(game);
            e.set_sprite("sYetiThrowL");
            return;
        }
    }
}

fn giant_player_collision(game: &mut Game, e: &mut Entity) {
    let king = e.object == "oYetiKing";
    if king && (game.player.pos.x - (e.pos.x + 16.0)).abs() > 16.0 {
        return;
    }
    if super::player_alive(game)
        && matches!(
            game.player.state,
            PlayerState::Jumping | PlayerState::Falling
        )
        && game.player.pos.y < e.pos.y + 8.0
        && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
    {
        let fall = game.player.data.get("fall_timer").copied().unwrap_or(0.0) / 16.0;
        let damage = if king {
            fall.ceil()
        } else {
            fall.floor() + 1.0
        } as i32;
        e.health -= damage * if game.player.has("spike_shoes") { 3 } else { 1 };
        super::set_counts_as_kill(e, true);
        game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
        game.player.data.insert("fall_timer".into(), 0.0);
        game.sound("sndHit", e.pos);
    } else if game.player.invincible == 0 && (king || e.state != 99) {
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
        if !king || crate::statistics::is_real_level(game) {
            hurt_player_cause(
                game,
                if king { 2 } else { 1 },
                if king {
                    crate::statistics::EnemyDeath::YetiKing
                } else {
                    crate::statistics::EnemyDeath::AlienBoss
                },
            );
        } else {
            game.sound("sndHurt", game.player.pos);
        }
    }
}

fn eject_player_collision(game: &mut Game, e: &mut Entity) {
    if super::player_alive(game)
        && matches!(
            game.player.state,
            PlayerState::Jumping | PlayerState::Falling
        )
        && game.player.pos.y < e.pos.y - 3.0
        && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
    {
        game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
        game.player.data.insert("fall_timer".into(), 0.0);
        super::record_death(game, e);
        e.alive = false;
        game.sound("sndHit", e.pos);
    } else if game.player.invincible == 0 {
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
        // AlienEject's override subtracts life without an enemyDeaths write.
        hurt_player(game, 1);
    }
}

pub(super) fn alarm(game: &mut Game, e: &mut Entity, index: u8) {
    if index != 0 {
        return;
    }
    match e.object.as_str() {
        "oYeti" => {
            e.data.insert("hit_cooldown".into(), 0.0);
            e.data.insert("whipped".into(), 0.0);
        }
        "oUFOCrash" => {
            let object = if game.rng.range(2) == 0 {
                "oFlameTrail"
            } else {
                "oBurn"
            };
            let pos = Vec2::new(
                e.pos.x + f64::from(game.rng.range(17)),
                e.pos.y + f64::from(game.rng.range(17)),
            );
            crate::world_events::spawn_effect(game, object, pos);
            e.alarms[0] = 3;
        }
        "oLaser" => {
            crate::world_events::spawn_effect(game, "oLaserTrail", e.pos);
            e.alarms[0] = 1;
        }
        _ => {}
    }
}

pub(super) fn outside_room_event(e: &mut Entity) {
    if matches!(e.object.as_str(), "oAlienEject" | "oLaser" | "oPsychicWave") {
        e.alive = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, RulesPreset, World};
    fn game() -> Game {
        let mut g = Game::shell(7, RulesPreset::Classic);
        g.world = World::new(20, 20, Area::Ice, 9);
        g.player.pos = Vec2::new(100.0, 100.0);
        g
    }
    fn first_family_psychic_room(rules: RulesPreset, two_bosses: bool) -> (Game, Vec<u64>) {
        let mut game = Game::shell(42, rules);
        game.world = World::new(20, 16, Area::Ice, 9);
        game.world.name = "rLevel".into();
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBlock");
        }
        // Authored starting conditions precede every actor's synchronous Create.
        game.player.pos = Vec2::new(176.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        let mut bosses = vec![game.spawn("oAlienBoss", Vec2::new(96.0, 128.0))];
        if two_bosses {
            bosses.push(game.spawn("oAlienBoss", Vec2::new(192.0, 128.0)));
        }
        (game, bosses)
    }

    fn assert_psychic_create_heading(effect: &Entity, origin: Vec2, target: Vec2) {
        let angle = (target.y - origin.y).atan2(target.x - origin.x);
        let expected = Vec2::new(2.0 * angle.cos(), 2.0 * angle.sin());
        assert!((effect.vel.x - expected.x).abs() < 1e-12);
        assert!((effect.vel.y - expected.y).abs() < 1e-12);
    }

    fn first_family_psychic_attack(game: &mut Game, bosses: &[u64]) {
        let before_attack_rng = game.rng.clone();
        for tick in 1..=100 {
            game.step(&crate::InputFrame::default());
            assert_eq!(game.player.pos, Vec2::new(176.0, 152.0));
            assert_eq!(game.player.health, 4);
            assert_eq!(game.player.stun, 0);
            assert_eq!(game.player.invincible, 0);
            assert!(!game.world.entities.iter().any(|entity| {
                matches!(entity.object.as_str(), "oPsychicCreate" | "oPsychicWave")
            }));
            for (index, id) in bosses.iter().enumerate() {
                let boss = game.world.entity(*id).unwrap();
                assert!(boss.engine_active);
                assert_eq!(boss.state, 0);
                assert_eq!(boss.pos, Vec2::new(96.0 + index as f64 * 96.0, 128.0));
                assert_eq!(boss.value("psychic_recover"), 100.0 - f64::from(tick));
            }
        }
        assert_eq!(
            game.rng.clone().next_u64(),
            before_attack_rng.clone().next_u64()
        );

        let mut expected_rng = game.rng.clone();
        let mut expected_origins = Vec::new();
        for id in bosses {
            let boss = game.world.entity(*id).unwrap();
            for _ in 0..6 {
                expected_origins.push(Vec2::new(
                    boss.pos.x + 16.0 + f64::from(expected_rng.range(33))
                        - f64::from(expected_rng.range(33)),
                    boss.pos.y + 16.0 + f64::from(expected_rng.range(33))
                        - f64::from(expected_rng.range(33)),
                ));
            }
        }
        game.step(&crate::InputFrame::default());

        let target = Vec2::new(112.0, 144.0);
        let mut effects: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && entity.object == "oPsychicCreate")
            .collect();
        effects.sort_by_key(|entity| entity.creation_serial);
        assert_eq!(effects.len(), bosses.len() * 6);
        for (effect, origin) in effects.iter().zip(expected_origins) {
            assert_eq!(effect.previous_pos, origin);
            assert_eq!(effect.value("create_target_x"), target.x);
            assert_eq!(effect.value("create_target_y"), target.y);
            assert_psychic_create_heading(effect, origin, target);
            // Resource665 follows Boss658, so the newborn gets its real Step.
            assert!((effect.pos.x - origin.x - effect.vel.x).abs() < 1e-12);
            assert!((effect.pos.y - origin.y - effect.vel.y).abs() < 1e-12);
        }
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oPsychicWave")
                .count(),
            bosses.len()
        );
        for id in bosses {
            assert_eq!(
                game.world.entity(*id).unwrap().value("psychic_recover"),
                100.0
            );
        }
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.stun, 0);
        assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
    }

    #[test]
    fn actual_step_two_fresh_bosses_capture_first_family_heading_after_recovery() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, bosses) = first_family_psychic_room(rules, true);
            // Both bosses naturally wait100 own Steps, then attack on101.
            // The second caller's six children must still aim at the first Boss.
            first_family_psychic_attack(&mut game, &bosses);
        }
    }

    #[test]
    fn actual_step_single_boss_and_initial_storage_order_preserve_create_rng() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut single, bosses) = first_family_psychic_room(rules, false);
            first_family_psychic_attack(&mut single, &bosses);

            let (mut reordered, bosses) = first_family_psychic_room(rules, true);
            // Before the first Step, vary only storage. Game::spawn already
            // registered A then B; no actor state or source array is rewritten.
            reordered.world.entities.reverse();
            first_family_psychic_attack(&mut reordered, &bosses);
        }
    }

    #[test]
    fn psychic_create_external_birth_uses_registered_order_and_keeps_supplied_target() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, bosses) = first_family_psychic_room(rules, true);
            game.world.entities.reverse();
            assert_eq!(game.world.entities[0].id, bosses[1]);
            let origin = Vec2::new(144.0, 140.0);
            let unchanged_rng = game.rng.clone();
            let effect_id = game.spawn("oPsychicCreate", origin);
            let effect = game.world.entity(effect_id).unwrap();
            assert_eq!(
                effect.pos, origin,
                "Create captures heading without movement"
            );
            assert_psychic_create_heading(effect, origin, Vec2::new(112.0, 144.0));
            let captured_velocity = effect.vel;
            assert!(
                crate::dispatcher::descendant_instances(&game, "oPsychicCreate")
                    .contains(&crate::dispatcher::Instance::Entity(effect_id))
            );
            assert_eq!(
                game.rng.clone().next_u64(),
                unchanged_rng.clone().next_u64()
            );

            // Preserve the existing producer-supplied target before Create.
            let supplied_id = game.world.spawn("oPsychicCreate", origin);
            let effect = game.world.entity_mut(supplied_id).unwrap();
            effect.data.insert("create_target_x".into(), 200.0);
            effect.data.insert("create_target_y".into(), 140.0);
            crate::dispatcher::create(&mut game, supplied_id);
            assert_eq!(
                game.world.entity(supplied_id).unwrap().vel,
                Vec2::new(2.0, 0.0)
            );
            assert!(
                crate::dispatcher::descendant_instances(&game, "oPsychicCreate")
                    .contains(&crate::dispatcher::Instance::Entity(supplied_id))
            );
            assert_eq!(
                game.rng.clone().next_u64(),
                unchanged_rng.clone().next_u64()
            );

            // The externally created effect participates in the next ordinary
            // world Step immediately; querying Create must not hide its birth.
            game.step(&crate::InputFrame::default());
            let effect = game.world.entity(effect_id).unwrap();
            assert_eq!(effect.previous_pos, origin);
            assert!((effect.pos.x - origin.x - captured_velocity.x).abs() < 1e-12);
            assert!((effect.pos.y - origin.y - captured_velocity.y).abs() < 1e-12);
            assert_eq!(
                game.world.entity(supplied_id).unwrap().pos,
                Vec2::new(146.0, 140.0)
            );
            assert_eq!(
                game.rng.clone().next_u64(),
                unchanged_rng.clone().next_u64()
            );
        }
    }

    fn source_frame(game: &mut Game, entity: &mut Entity) {
        tick(game, entity);
        let sprite = entity.sprite.clone();
        let frames = crate::catalog::sprite_frame_count(&sprite);
        let speed = entity.image_speed();
        if crate::animation::advance(&mut entity.animation, speed, frames) {
            animation_end(game, entity, &sprite);
        }
    }

    fn king_yell_room(seed: u64, position: Vec2) -> (Game, u64) {
        let mut game = Game::shell(seed, RulesPreset::Classic);
        game.world = World::new(60, 30, Area::Ice, 9);
        game.world.name = "Yeti King source yell fixture".into();
        game.player.pos = Vec2::new(40.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.grounded = true;
        game.player.state = PlayerState::Standing;
        game.camera = Vec2::default();
        for x in 0..60 {
            game.world.set_tile(x, 10, "oBlock");
        }
        let id = game.spawn("oYetiKing", position);
        let king = game.world.entity_mut(id).unwrap();
        king.state = 3;
        king.animation = 8.0;
        king.vel = Vec2::default();
        king.set_sprite("sYetiKingYellR");
        (game, id)
    }

    // Literal Step139–154 + Ice Destroy oracle. Inputs are the independently
    // authored active-family sequence, not results from the dispatcher/helper.
    // IceBlock and the Drip inherited Create chain make no random calls.
    fn source_yell_oracle(
        mut rng: crate::Rng,
        king: Vec2,
        ice: &[Vec2],
        clean: bool,
    ) -> (crate::Rng, Vec<Vec2>, Vec<Vec2>) {
        let mut blocks = Vec::new();
        let mut drips = Vec::new();
        for origin in ice {
            let n = rng.range(60) + 1;
            let dx = origin.x - king.x;
            let dy = origin.y - king.y;
            if n == 1
                && ((king.x + 16.0) - (origin.x + 8.0)).abs() > 16.0
                && (dx * dx + dy * dy).sqrt() < 96.0
            {
                blocks.push(*origin);
                if !clean {
                    for _ in 0..3 {
                        let x = origin.x + f64::from(rng.range(17));
                        let y = origin.y + f64::from(rng.range(17));
                        drips.push(Vec2::new(x, y));
                    }
                }
            }
        }
        (rng, blocks, drips)
    }

    #[test]
    fn actual_yell_source_order_rolls_active_ice_before_geometry_and_destroy_callbacks() {
        let king = Vec2::new(96.0, 128.0);
        // Cross-storage birth order deliberately disagrees with the grid scan.
        let origins = [
            Vec2::new(160.0, 128.0),
            Vec2::new(80.0, 64.0),
            Vec2::new(96.0, 48.0),  // aligned: still consumes its roll
            Vec2::new(400.0, 96.0), // distant, outside view but engine-active
            Vec2::new(176.0, 128.0),
        ];
        let mut conversions = 0;
        for seed in 0..128 {
            let (mut game, id) = king_yell_room(seed, king);
            let dynamic = game.world.spawn("oIce", origins[0]);
            game.world.set_tile(5, 4, "oIce");
            game.world.set_tile(6, 3, "oIce");
            let distant = game.world.spawn("oIce", origins[3]);
            game.world.set_tile(11, 8, "oIce");
            let inactive = game.world.spawn("oIce", Vec2::new(704.0, 64.0));
            game.world.set_tile(40, 4, "oIce");
            let dead = game.world.spawn("oIce", Vec2::new(224.0, 64.0));
            game.world.entity_mut(dead).unwrap().alive = false;
            let thin = game.spawn("oThinIce", Vec2::new(240.0, 64.0));
            let inactive_thin = game.spawn("oThinIce", Vec2::new(704.0, 96.0));
            let excluded = game.spawn("oIceBlock", Vec2::new(288.0, 64.0));
            // Source instance_destroy ignores attack invincibility.
            game.world
                .entity_mut(dynamic)
                .unwrap()
                .data
                .insert("invincible".into(), 1.0);
            let (mut expected_rng, blocks, drips) =
                source_yell_oracle(game.rng.clone(), king, &origins, false);
            conversions += blocks.len();

            game.step(&crate::InputFrame::default());

            assert_eq!(game.world.entity(id).unwrap().pos, king, "seed {seed}");
            assert!(game.world.entity(distant).unwrap().engine_active);
            assert!(!game.world.entity(inactive).unwrap().engine_active);
            assert!(!game.world.tile(40, 4).unwrap().engine_active);
            assert!(game.world.entity(dead).is_none());
            assert_eq!(game.world.entity(thin).unwrap().value("thickness"), 58.0);
            assert_eq!(
                game.world.entity(inactive_thin).unwrap().value("thickness"),
                60.0
            );
            let actual_blocks: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|entity| {
                    entity.alive && entity.object == "oIceBlock" && entity.id != excluded
                })
                .map(|entity| entity.pos)
                .collect();
            assert_eq!(actual_blocks, blocks, "seed {seed}");
            let actual_drips: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oDrip")
                .map(|entity| entity.pos)
                .collect();
            assert_eq!(actual_drips, drips, "seed {seed}");
            assert_eq!(game.rng.next_u64(), expected_rng.next_u64(), "seed {seed}");
        }
        assert!(
            conversions > 0,
            "exercise synchronous successful conversions"
        );
    }

    #[test]
    fn actual_yell_source_frame_window_and_camera_gate_preserve_rng() {
        for (x, frame, rolls) in [
            (96.0, 6.5, 0),
            (96.0, 7.0, 1),
            (96.0, 12.0, 1),
            (96.0, 12.5, 0),
            (335.0, 8.0, 0), // active margin, outside King's narrower AI view
        ] {
            let (mut game, king) = king_yell_room(42, Vec2::new(x, 128.0));
            game.world.entity_mut(king).unwrap().animation = frame;
            let ice = game.world.spawn("oIce", Vec2::new(400.0, 64.0));
            let mut expected_rng = game.rng.clone();
            for _ in 0..rolls {
                expected_rng.range(60);
            }
            game.step(&crate::InputFrame::default());
            assert!(game.world.entity(king).unwrap().engine_active);
            assert!(game.world.entity(ice).unwrap().engine_active);
            assert_eq!(
                game.rng.next_u64(),
                expected_rng.next_u64(),
                "x={x}, frame={frame}"
            );
        }
    }

    #[test]
    fn actual_yell_source_first_king_and_strict_origin_boundaries() {
        let first = Vec2::new(96.0, 128.0);
        let origins = [
            Vec2::new(120.0, 128.0), // alignment exactly16: excluded
            Vec2::new(121.0, 128.0), // alignment17: eligible
            Vec2::new(191.0, 128.0), // origin distance95: eligible
            Vec2::new(192.0, 128.0), // origin distance96: excluded
            Vec2::new(368.0, 128.0), // near yelling King, far from first King
        ];
        let mut conversions = 0;
        for seed in 0..128 {
            let (mut game, first_id) = king_yell_room(seed, first);
            let idle = game.world.entity_mut(first_id).unwrap();
            idle.state = 0;
            idle.animation = 0.0;
            idle.data.insert("counter".into(), 999.0);
            let second = game.spawn("oYetiKing", Vec2::new(304.0, 128.0));
            let yelling = game.world.entity_mut(second).unwrap();
            yelling.state = 3;
            yelling.animation = 8.0;
            yelling.vel = Vec2::default();
            yelling.set_sprite("sYetiKingYellR");
            for origin in origins {
                game.world.spawn("oIce", origin);
            }
            // Clean source Ice Destroy suppresses Drips; its60roll remains.
            game.flags.insert("cleanSolids".into());
            let (mut expected_rng, blocks, _) =
                source_yell_oracle(game.rng.clone(), first, &origins, true);
            conversions += blocks.len();
            game.step(&crate::InputFrame::default());
            let actual: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oIceBlock")
                .map(|entity| entity.pos)
                .collect();
            assert_eq!(actual, blocks, "seed {seed}");
            assert_eq!(game.rng.next_u64(), expected_rng.next_u64(), "seed {seed}");
        }
        assert!(conversions > 0);
    }

    #[test]
    fn ufo_local_create_consumes_one_source_draw_and_runtime_does_not_repeat_it() {
        for seed in [1, 7, 42, 129] {
            let mut game = Game::shell(seed, RulesPreset::Classic);
            let mut ufo = Entity::new(1, "oUFO", Vec2::new(100.0, 40.0));
            init(&mut ufo);
            let mut source_rng = game.rng.clone();
            let toggle = f64::from(source_rng.range(2));
            for _ in 0..2 {
                create_local(
                    &mut crate::creation::EntityCreateContext {
                        world: &game.world,
                        rng: &mut game.rng,
                    },
                    &mut ufo,
                );
            }
            assert_eq!(ufo.value("shift_toggle"), toggle);
            assert_eq!(ufo.value("ice_ufo_local_created"), 1.0);
            assert_eq!(ufo.value("ice_context_created"), 0.0);
            assert_eq!(game.rng.clone().next_u64(), source_rng.clone().next_u64());
            // Caller overrides after Create must survive later runtime hooks.
            ufo.data.insert("shift_toggle".into(), 7.0);
            create(&mut game, &mut ufo);
            create(&mut game, &mut ufo);
            assert_eq!(ufo.value("shift_toggle"), 7.0);
            assert_eq!(game.rng.next_u64(), source_rng.next_u64());
        }
    }

    #[test]
    fn local_ufo_create_leaves_eject_randomness_for_runtime_create() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let mut eject = Entity::new(1, "oAlienEject", Vec2::new(100.0, 40.0));
        init(&mut eject);
        let mut source_rng = game.rng.clone();
        create_local(
            &mut crate::creation::EntityCreateContext {
                world: &game.world,
                rng: &mut game.rng,
            },
            &mut eject,
        );
        assert_eq!(eject.value("drift_unset"), 1.0);
        assert_eq!(eject.value("ice_context_created"), 0.0);
        assert_eq!(game.rng.clone().next_u64(), source_rng.clone().next_u64());
        let facing = if source_rng.range(2) == 0 { -1 } else { 1 };
        create(&mut game, &mut eject);
        create(&mut game, &mut eject);
        assert_eq!(eject.facing, facing);
        assert_eq!(eject.value("drift_unset"), 0.0);
        assert_eq!(game.rng.next_u64(), source_rng.next_u64());
    }

    #[test]
    fn source_stomp_restores_prior_false_credit_only_for_king_and_boss() {
        for (object, center_x, restores) in [
            ("oYeti", 8.0, false),
            ("oYetiKing", 16.0, true),
            ("oAlienBoss", 16.0, true),
        ] {
            let mut g = game();
            let mut enemy = Entity::new(1, object, Vec2::new(96.0, 96.0));
            init(&mut enemy);
            super::super::set_counts_as_kill(&mut enemy, false);
            // Imported saves may retain this legacy alias of the source flag.
            enemy.data.insert("environmental_death".into(), 1.0);
            g.player.pos = Vec2::new(enemy.pos.x + center_x, enemy.pos.y);
            g.player.state = PlayerState::Falling;
            g.player.data.insert("fall_timer".into(), 16.0);
            assert!(collision(
                &mut g,
                &mut enemy,
                crate::dispatcher::Instance::Player
            ));
            assert_eq!(super::super::counts_as_kill(&enemy), restores, "{object}");
            assert_eq!(enemy.value("environmental_death") == 0.0, restores);
        }
    }

    #[test]
    fn source_health_values() {
        for (name, hp) in [
            ("oYeti", 5),
            ("oYetiKing", 30),
            ("oAlienBoss", 10),
            ("oUFO", 1),
        ] {
            let mut e = Entity::new(1, name, Vec2::new(50.0, 50.0));
            init(&mut e);
            assert_eq!(e.health, hp);
        }
    }
    #[test]
    fn ufo_fires_once_at_frame_sixteen() {
        let mut g = game();
        let mut e = Entity::new(1, "oUFO", Vec2::new(100.0, 40.0));
        init(&mut e);
        e.state = 2;
        for _ in 0..40 {
            source_frame(&mut g, &mut e);
        }
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oLaser")
                .count(),
            1
        );
        assert_eq!(e.state, 1);
    }
    #[test]
    fn psychic_attack_requires_recovery_and_range() {
        let mut g = game();
        let mut e = Entity::new(1, "oAlienBoss", Vec2::new(100.0, 100.0));
        init(&mut e);
        e.data.insert("psychic_recover".into(), 2.0);
        tick(&mut g, &mut e);
        assert!(g.world.entities.is_empty());
        tick(&mut g, &mut e);
        assert!(g.world.entities.is_empty());
        tick(&mut g, &mut e);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oPsychicWave")
                .count(),
            1
        );
    }
    #[test]
    fn dead_king_drops_four_items() {
        let mut g = game();
        let mut e = Entity::new(1, "oYetiKing", Vec2::new(100.0, 100.0));
        init(&mut e);
        e.health = 0;
        tick(&mut g, &mut e);
        assert!(!e.alive);
        assert_eq!(g.world.entities.len(), 4);
    }
    #[test]
    fn laser_removes_destructible_tile_only() {
        let mut g = game();
        g.world.set_tile(4, 4, "oIce");
        let mut e = Entity::new(1, "oLaser", Vec2::new(72.0, 72.0));
        init(&mut e);
        tick(&mut g, &mut e);
        assert!(collision(
            &mut g,
            &mut e,
            crate::dispatcher::Instance::Terrain(4 + 4 * 20)
        ));
        assert!(!e.alive);
        assert!(!g.world.tile(4, 4).unwrap().is_solid());
    }

    #[test]
    fn psychic_wave_has_source_damage_and_lifetime() {
        let mut g = game();
        let mut e = Entity::new(1, "oPsychicWave", g.player.pos);
        init(&mut e);
        g.world.entities.push(e.clone());
        player_hurt(&mut g);
        g.world.entities.clear();
        source_frame(&mut g, &mut e);
        assert_eq!(g.player.health, 3);
        assert_eq!(g.player.stun, 40);
        assert!(e.alive);
        for _ in 1..68 {
            source_frame(&mut g, &mut e);
        }
        assert!(!e.alive);
    }

    fn dying_boss_blood_room(detritus: usize, bloodless: bool) -> (Game, u64) {
        let mut g = boss_item_room();
        let id = g.spawn("oAlienBoss", Vec2::new(128.0, 96.0));
        let boss = g.world.entity_mut(id).unwrap();
        boss.health = 0;
        boss.state = 99;
        boss.set_sprite("sAlienBossDie");
        boss.vel = Vec2::new(1.0, 1.0);
        boss.data.insert("bloodless".into(), f64::from(bloodless));
        boss.data.insert("blood_left".into(), 0.0);
        for _ in 0..detritus {
            g.spawn("oBone", Vec2::new(280.0, 48.0));
        }
        // Reset only the fixture's RNG after synchronous Create callbacks.
        // Independent source evaluation gives success on seed 6's first roll.
        g.rng = crate::Rng::new(6);
        (g, id)
    }

    fn live_boss_blood(g: &Game) -> Vec<&Entity> {
        g.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oBlood")
            .collect()
    }

    #[test]
    fn actual_step_dying_boss_blood_preserves_script_direct_guards_and_create_order() {
        for (detritus, bloodless, expected_count, next_rng) in [
            (0, false, 2, 9_187_981_958_480_619_105),
            (15, false, 2, 9_187_981_958_480_619_105),
            (16, false, 1, 3_738_407_306_160_466_512),
            (31, false, 1, 3_738_407_306_160_466_512),
            (32, false, 0, 3_738_407_306_160_466_512),
            (0, true, 1, 3_738_407_306_160_466_512),
            (15, true, 1, 3_738_407_306_160_466_512),
        ] {
            let (mut g, boss_id) = dying_boss_blood_room(detritus, bloodless);
            g.step(&crate::InputFrame::default());
            let boss = g.world.entity(boss_id).unwrap();
            assert_eq!(
                boss.pos,
                Vec2::new(129.0, 98.0),
                "the dying body actually moves"
            );
            assert_eq!(
                boss.value("blood_left"),
                0.0,
                "this source producer has no budget"
            );
            let blood = live_boss_blood(&g);
            assert_eq!(
                blood.len(),
                expected_count,
                "detritus={detritus}, bloodless={bloodless}"
            );
            let positions: Vec<_> = blood.iter().map(|e| e.pos).collect();
            assert_eq!(
                positions,
                match expected_count {
                    2 => vec![Vec2::new(136.0, 113.0), Vec2::new(136.0, 110.0)],
                    1 => vec![Vec2::new(136.0, 111.0)],
                    _ => Vec::new(),
                },
                "separate source Y draws and synchronous first Create precede movement"
            );
            for drop in &blood {
                assert_eq!(
                    drop.age, 0,
                    "earlier Blood resource does not run a newborn Step"
                );
                assert_eq!(drop.sprite, "sBlood");
                assert_eq!(drop.value("life"), 60.0);
                assert_eq!(drop.value("world_effect_initialized"), 1.0);
                assert_eq!(&drop.alarms[..3], &[1, 1, 5]);
                assert_eq!(drop.value("collectible"), 0.0);
                assert_eq!(drop.value("bounce"), 0.0);
            }
            if expected_count == 2 {
                assert!((blood[0].vel.x - -0.19678379546949554).abs() < 1e-12);
                assert!((blood[1].vel.x - 0.3391272229582798).abs() < 1e-12);
            } else if expected_count == 1 {
                assert!((blood[0].vel.x - -1.7808133884482826).abs() < 1e-12);
            }
            assert_eq!(
                g.rng.clone().next_u64(),
                next_rng,
                "callback RNG even at direct cap32"
            );
        }
    }

    #[test]
    fn actual_step_boss_blood_uses_dying_sprite_and_source_half_probability() {
        for (state, sprite, seed, expected_count, next_rng) in [
            (99, "sAlienBossDie", 0, 0, 7_960_286_522_194_355_700),
            (99, "sAlienBossDead", 6, 0, 13_647_215_125_184_110_592),
            (99, "sAlienBossHurt", 6, 0, 13_647_215_125_184_110_592),
            (0, "sAlienBossDie", 6, 2, 9_187_981_958_480_619_105),
        ] {
            let (mut g, id) = dying_boss_blood_room(0, false);
            let boss = g.world.entity_mut(id).unwrap();
            if state == 0 {
                boss.health = 10;
            }
            boss.state = state;
            boss.set_sprite(sprite);
            g.rng = crate::Rng::new(seed);
            g.step(&crate::InputFrame::default());
            assert_eq!(
                live_boss_blood(&g).len(),
                expected_count,
                "{sprite}, state={state}, seed={seed}"
            );
            assert_eq!(g.rng.clone().next_u64(), next_rng);
        }
    }

    #[test]
    fn actual_step_crushed_live_boss_creates_gems_then_first_dying_blood() {
        let mut g = boss_item_room();
        let id = g.spawn("oAlienBoss", Vec2::new(128.0, 96.0));
        assert_eq!(g.world.entity(id).unwrap().health, 10);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienBoss");
        // Source Step's center-in-solid check kills a live boss naturally;
        // no health, dying sprite, or state is assigned by this fixture.
        g.world.set_tile(8, 6, "oBlock");
        g.rng = crate::Rng::new(6);
        g.step(&crate::InputFrame::default());
        let boss = g.world.entity(id).unwrap();
        assert_eq!(boss.health, 0);
        assert_eq!(boss.state, 99);
        assert_eq!(boss.sprite, "sAlienBossDie");
        let gems: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| {
                matches!(
                    e.object.as_str(),
                    "oEmeraldBig" | "oSapphireBig" | "oRubyBig"
                )
            })
            .collect();
        assert_eq!(gems.len(), 4);
        let blood = live_boss_blood(&g);
        assert_eq!(
            blood.len(),
            2,
            "first death-entry Step also emits the source pair"
        );
        assert!(gems.iter().all(|gem| gem.id < blood[0].id));
        assert_eq!(blood[0].pos, Vec2::new(136.0, 112.0));
        assert_eq!(blood[1].pos, Vec2::new(136.0, 113.0));
        assert_eq!(g.rng.clone().next_u64(), 213_719_968_000_392_126);
        assert_eq!(g.player.health, 4);
    }

    #[test]
    fn actual_step_naturally_rock_killed_boss_bleeds_repeatedly_until_death_art_finishes() {
        let mut g = boss_item_room();
        let id = g.spawn("oAlienBoss", Vec2::new(128.0, 128.0));
        for expected_health in (0..10).rev() {
            throw_boss_rock(&mut g, 128.0);
            assert_eq!(g.world.entity(id).unwrap().health, expected_health);
            if expected_health > 0 {
                for _ in 0..6 {
                    g.step(&crate::InputFrame::default());
                }
                assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienBoss");
            }
        }
        let mut emission_steps = 0;
        let mut dead_art_steps = 0;
        for _ in 0..40 {
            let previous_sprite = g.world.entity(id).unwrap().sprite.clone();
            let previous_ids: Vec<_> = live_boss_blood(&g).iter().map(|e| e.id).collect();
            g.step(&crate::InputFrame::default());
            let new_count = live_boss_blood(&g)
                .iter()
                .filter(|e| !previous_ids.contains(&e.id))
                .count();
            if previous_sprite == "sAlienBossDead" {
                dead_art_steps += 1;
                assert_eq!(new_count, 0, "the completed death art stops this producer");
            } else if new_count > 0 {
                emission_steps += 1;
            }
        }
        assert!(
            emission_steps > 2,
            "dying blood repeats across actual animation Steps"
        );
        assert!(
            dead_art_steps > 0,
            "natural animation-end callback must complete"
        );
        assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienBossDead");
        assert_eq!(g.player.health, 4);
    }

    #[test]
    fn boss_death_releases_barrier_after_animation() {
        let mut g = game();
        let id = g.world.spawn("oBarrierEmitter", Vec2::new(200.0, 100.0));
        let mut e = Entity::new(9, "oAlienBoss", Vec2::new(50.0, 50.0));
        init(&mut e);
        e.health = 0;
        for _ in 0..36 {
            source_frame(&mut g, &mut e);
        }
        assert_eq!(e.sprite, "sAlienBossDead");
        assert!(
            !g.world
                .entities
                .iter()
                .find(|entity| entity.id == id)
                .unwrap()
                .alive
        );
    }

    fn boss_item_room() -> Game {
        let mut g = Game::shell(5, RulesPreset::Classic);
        g.world = World::new(60, 30, Area::Ice, 9);
        g.world.name = "boss item animation fixture".into();
        g.player.pos = Vec2::new(40.0, 152.0);
        g.player.state = PlayerState::Standing;
        g.player.grounded = true;
        for x in 0..60 {
            g.world.set_tile(x, 10, "oBlock");
        }
        g
    }

    fn throw_boss_rock(g: &mut Game, x: f64) {
        let id = g.world.spawn("oRock", Vec2::new(x - 1.0, 140.0));
        g.world.entity_mut(id).unwrap().vel.x = 5.0;
        g.step(&crate::InputFrame::default());
        // The first rock need not remain overlapping during recovery.
        if let Some(rock) = g.world.entity_mut(id) {
            rock.alive = false;
        }
    }

    #[test]
    fn actual_step_king_whip_cooldown_expires_and_accepts_a_second_attack() {
        let mut g = boss_item_room();
        g.world.set_tile(5, 9, "oBlock");
        g.world.set_tile(8, 8, "oBlock");
        g.world.set_tile(8, 9, "oBlock");
        g.player.pos = Vec2::new(88.0, 136.0);
        let king = g.spawn("oYetiKing", Vec2::new(96.0, 128.0));
        for _ in 0..40 {
            g.step(&crate::InputFrame::default());
        }
        assert_eq!(g.world.entity(king).unwrap().facing, -1);
        let attack = crate::InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        };
        for attack_number in 0..2 {
            g.step(&attack);
            let mut hit = false;
            for _ in 0..20 {
                g.step(&crate::InputFrame::default());
                if g.world.entity(king).unwrap().value("hit_cooldown") > 0.0 {
                    hit = true;
                    break;
                }
            }
            assert!(
                hit,
                "natural whip attack {attack_number} must contact the King's head"
            );
            assert_eq!(
                g.world.entity(king).unwrap().health,
                30,
                "a normal whip sets the cooldown without puncture damage"
            );
            for _ in 0..30 {
                g.step(&crate::InputFrame::default());
                if g.world.entity(king).unwrap().value("hit_cooldown") == 0.0
                    && g.player.attack_timer == 0
                {
                    break;
                }
            }
            assert_eq!(g.world.entity(king).unwrap().value("hit_cooldown"), 0.0);
            assert_eq!(g.player.attack_timer, 0);
        }
    }
    #[test]
    fn actual_step_ejected_alien_finishes_deployment_after_natural_scroll_outside_ai_guard() {
        let mut g = boss_item_room();
        g.player.pos.x = 208.0;
        g.update_camera();
        let left = crate::InputFrame {
            left: true,
            run: true,
            ..Default::default()
        };
        while g.player.pos.x > 152.0 {
            g.step(&left);
        }
        assert_eq!(g.camera.x, 16.0);
        let id = g.spawn("oAlienEject", Vec2::new(335.0, 70.0));
        for _ in 0..16 {
            g.step(&crate::InputFrame::default());
            if g.world.entity(id).unwrap().sprite == "sAlienDeploy" {
                break;
            }
        }
        assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienDeploy");
        let mut parachute = false;
        for _ in 0..20 {
            g.step(&left);
            let alien = g.world.entity(id).unwrap();
            if alien.sprite == "sAlienParachute" {
                assert_eq!(alien.state, 2);
                assert!(
                    alien.pos.x > g.camera.x + crate::VIEW_WIDTH + 4.0,
                    "natural scroll must cross the narrower Step guard first"
                );
                parachute = true;
                break;
            }
        }
        assert!(parachute);
    }

    #[test]
    fn actual_step_psychic_create_keeps_its_create_heading_through_the_target() {
        let mut g = boss_item_room();
        g.spawn("oAlienBoss", Vec2::new(128.0, 128.0));
        let effect = g.spawn("oPsychicCreate", Vec2::new(144.0, 140.0));
        assert!(
            (g.world.entity(effect).unwrap().vel.y - 2.0).abs() < 1e-9,
            "context Create must capture heading synchronously"
        );
        for _ in 0..3 {
            g.step(&crate::InputFrame::default());
        }
        let effect = g.world.entity(effect).unwrap();
        assert!((effect.pos.x - 144.0).abs() < 1e-9);
        assert!(
            (effect.pos.y - 146.0).abs() < 1e-9,
            "source motion continues past the captured target"
        );
        assert!((effect.animation - 1.2).abs() < 1e-9);
    }

    #[test]
    fn actual_step_player_laser_query_hurts_once_and_records_ufo_death() {
        let mut g = boss_item_room();
        crate::characters::apply(&mut g, crate::characters::Character::TunnelMan);
        let pos = g.player.pos;
        g.world.spawn("oLaser", pos);
        g.world.spawn("oLaser", Vec2::new(pos.x + 1.0, pos.y));
        g.step(&crate::InputFrame::default());
        assert_eq!(g.player.health, 0);
        assert_eq!(
            g.events
                .iter()
                .filter(|event| **event == GameEvent::Damage { amount: 3 })
                .count(),
            1
        );
        assert_eq!(
            g.lifetime_stats.enemy_deaths[crate::statistics::EnemyDeath::Ufo as usize],
            1
        );
        assert_eq!(g.lifetime_stats.enemy_deaths.iter().sum::<u64>(), 1);
    }

    #[test]
    fn actual_step_lethally_shot_yeti_is_credited_only_after_stunned_corpse_settles() {
        let mut g = boss_item_room();
        let yeti = g.world.spawn("oYeti", Vec2::new(96.0, 144.0));
        for _ in 0..2 {
            g.world.spawn("oBullet", Vec2::new(104.0, 150.0));
        }
        g.step(&crate::InputFrame::default());
        let body = g.world.entity(yeti).unwrap();
        assert!(body.health < 1, "both source bullet pairs must hit");
        assert_eq!(body.state, 98);
        assert_eq!(body.value("death_recorded"), 0.0);
        assert_eq!(g.data.get("kills"), None);
        let mut credited = false;
        for _ in 0..180 {
            let previous_state = g.world.entity(yeti).unwrap().state;
            g.step(&crate::InputFrame::default());
            if g.data.get("kills").copied().unwrap_or(0.0) != 0.0 {
                assert_eq!(previous_state, 99, "credit belongs to the next DEAD Step");
                assert_eq!(g.data["kills"], 1.0);
                assert_eq!(g.data["yetis"], 1.0);
                assert_eq!(
                    g.lifetime_stats.enemy_kills[crate::statistics::EnemyKill::Yeti as usize],
                    1
                );
                credited = true;
                break;
            }
        }
        assert!(credited, "the natural corpse trajectory must settle");
        for _ in 0..20 {
            g.step(&crate::InputFrame::default());
        }
        assert_eq!(g.data["kills"], 1.0);
    }

    #[test]
    fn actual_step_yeti_source_crush_destroys_without_death_credit() {
        let mut g = boss_item_room();
        g.world.set_tile(6, 9, "oBlock");
        let yeti = g.world.spawn("oYeti", Vec2::new(96.0, 144.0));
        g.step(&crate::InputFrame::default());
        assert!(g.world.entity(yeti).is_none());
        assert_eq!(g.data.get("kills"), None);
        assert_eq!(g.data.get("yetis"), None);
        assert_eq!(
            g.lifetime_stats.enemy_kills[crate::statistics::EnemyKill::Yeti as usize],
            0
        );
    }

    #[test]
    fn actual_step_laser_and_psychic_hit_damsel_without_changing_her_state() {
        for object in ["oLaser", "oPsychicWave"] {
            let mut g = boss_item_room();
            let damsel = g.world.spawn("oDamsel", Vec2::new(136.0, 136.0));
            let projectile = g.world.spawn(object, Vec2::new(138.0, 136.0));
            g.step(&crate::InputFrame::default());
            let target = g.world.entity(damsel).unwrap();
            assert_eq!(target.health, 1, "{object}");
            assert_eq!(target.state, 0, "{object}");
            assert_eq!(target.vel, Vec2::new(-1.0, -6.0), "{object}");
            assert!(g.world.entity(projectile).is_none(), "{object}");
        }
    }

    #[test]
    fn actual_step_rescued_damsel_blocks_damage_and_consumes_only_psychic_wave() {
        for (object, consumed) in [("oLaser", false), ("oPsychicWave", true)] {
            let mut g = boss_item_room();
            let damsel = g.world.spawn("oDamsel", Vec2::new(136.0, 136.0));
            g.world.spawn("oExit", Vec2::new(128.0, 128.0));
            // Let the normal rescue enter its visible exit frames before the
            // incoming projectile arrives. No NPC flags or state are injected.
            for _ in 0..7 {
                g.step(&crate::InputFrame::default());
            }
            let projectile = g.world.spawn(object, Vec2::new(138.0, 136.0));
            assert!(
                g.world
                    .entity(projectile)
                    .unwrap()
                    .sprite_bounds()
                    .intersects(g.world.entity(damsel).unwrap().sprite_bounds()),
                "{object} must overlap the exit sprite"
            );
            g.step(&crate::InputFrame::default());
            let target = g.world.entity(damsel).unwrap();
            assert_eq!(target.state, 4, "{object}");
            assert_eq!(target.health, 4, "{object}");
            assert_eq!(g.world.entity(projectile).is_none(), consumed, "{object}");
            for _ in 0..40 {
                g.step(&crate::InputFrame::default());
            }
            assert!(g.world.entity(damsel).is_none(), "{object}");
            assert_eq!(g.data["pending_damsel_kisses"], 1.0);
        }
    }

    #[test]
    fn actual_step_ufo_search_does_not_start_chasing_a_swimming_player() {
        let mut g = boss_item_room();
        for x in 1..5 {
            for y in 7..10 {
                g.world
                    .spawn("oWaterSwim", Vec2::new((x * 16) as f64, (y * 16) as f64));
            }
        }
        let id = g.world.spawn("oUFO", Vec2::new(200.0, 70.0));
        for _ in 0..3 {
            g.step(&crate::InputFrame::default());
            assert_eq!(g.player.data["swimming"], 1.0);
            let ufo = g.world.entity(id).unwrap();
            assert_eq!(ufo.state, 0);
            assert_eq!(ufo.vel, Vec2::default());
            assert_eq!(ufo.pos, Vec2::new(200.0, 70.0));
        }
    }

    #[test]
    fn actual_step_ufo_search_does_not_start_chasing_after_natural_player_death() {
        let mut g = boss_item_room();
        g.world.spawn("oGhost", Vec2::new(32.0, 144.0));
        g.step(&crate::InputFrame::default());
        assert!(g.player.health < 1);
        // Ghost's automatic Collision happens after Player Step; the source
        // dead flag is established by the following character event.
        g.step(&crate::InputFrame::default());
        assert_eq!(g.player.state, PlayerState::Dead);
        let id = g.world.spawn("oUFO", Vec2::new(200.0, 70.0));
        for _ in 0..3 {
            g.step(&crate::InputFrame::default());
            let ufo = g.world.entity(id).unwrap();
            assert_eq!(ufo.state, 0);
            assert_eq!(ufo.vel, Vec2::default());
            assert_eq!(ufo.pos, Vec2::new(200.0, 70.0));
        }
    }

    #[test]
    fn actual_door_exit_excludes_laser_and_psychic_hurt_for_all_characters() {
        use crate::characters::{self, Character};
        use crate::{GameEvent, InputFrame, RunState};
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            let mut level = crate::level::Level::default();
            level.set_tile(6, 7, '@').unwrap();
            level.set_tile(7, 7, 'X').unwrap();
            for x in 0..40 {
                level.set_tile(x, 8, '1').unwrap();
            }
            let mut game = Game::shell(11, RulesPreset::Classic);
            game.load_custom(&level, "ice-projectile-exit", "");
            characters::apply(&mut game, character);
            for _ in 0..8 {
                game.step(&InputFrame::default());
            }
            let ordinary = game.clone();
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
            assert_eq!(game.state, RunState::Ending, "{character:?}");
            assert!(game.player.is_exiting());
            for (object, damage, stun) in [("oLaser", 3, 20), ("oPsychicWave", 1, 40)] {
                for exiting in [false, true] {
                    let mut target = if exiting {
                        game.clone()
                    } else {
                        ordinary.clone()
                    };
                    let position = target.player.pos;
                    let velocity = target.player.vel;
                    let health = target.player.health;
                    let offset = if object == "oPsychicWave" { 2.0 } else { 0.0 };
                    let id = target
                        .world
                        .spawn(object, Vec2::new(position.x + offset, position.y));
                    target.step(&InputFrame::default());
                    if exiting {
                        assert_eq!(target.player.health, health, "{character:?}/{object}");
                        assert_eq!(target.player.pos, position);
                        assert_eq!(target.player.vel, velocity);
                        assert_eq!(target.player.stun, 0);
                        assert!(target.player.is_exiting());
                        assert_eq!(target.state, RunState::Ending);
                        assert!(
                            !target
                                .events
                                .contains(&GameEvent::Damage { amount: damage })
                        );
                        let projectile = target
                            .world
                            .entity(id)
                            .expect("exit contact must not consume projectile");
                        assert_eq!(projectile.age, 1);
                        if object == "oLaser" {
                            assert_eq!(projectile.vel.y, 0.6);
                        } else {
                            assert_eq!(projectile.animation, 0.25);
                        }
                    } else {
                        assert_eq!(
                            target.player.health,
                            (health - damage).max(0),
                            "{character:?}/{object}"
                        );
                        assert_eq!(
                            target.player.state,
                            if health <= damage {
                                PlayerState::Dead
                            } else {
                                PlayerState::Stunned
                            }
                        );
                        assert_eq!(target.player.stun, stun);
                        assert_eq!(target.player.vel.y, -4.0);
                        assert!(
                            target
                                .events
                                .contains(&GameEvent::Damage { amount: damage })
                        );
                        assert_eq!(target.world.entity(id).is_none(), object == "oLaser");
                    }
                }
            }
        }
    }

    #[test]
    fn actual_step_boss_item_hurt_finishes_and_accepts_another_hit() {
        let mut g = boss_item_room();
        let id = g.world.spawn("oAlienBoss", Vec2::new(128.0, 128.0));
        throw_boss_rock(&mut g, 128.0);
        assert_eq!(g.world.entity(id).unwrap().health, 9);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienBossHurt");
        for _ in 0..5 {
            g.step(&crate::InputFrame::default());
        }
        let boss = g.world.entity(id).unwrap();
        assert_eq!(boss.sprite, "sAlienBossHurt");
        assert!((boss.animation - 4.8).abs() < 1e-9);
        assert_eq!(boss.value("animation_speed"), 0.8);
        g.step(&crate::InputFrame::default());
        let boss = g.world.entity(id).unwrap();
        assert_eq!(boss.sprite, "sAlienBoss");
        assert!((boss.animation - 0.6).abs() < 1e-9);
        assert_eq!(boss.value("animation_speed"), 0.8);
        throw_boss_rock(&mut g, 128.0);
        assert_eq!(g.world.entity(id).unwrap().health, 8);
    }

    #[test]
    fn actual_step_boss_late_item_hurt_retains_current_source_frame() {
        let mut g = boss_item_room();
        let id = g.world.spawn("oAlienBoss", Vec2::new(128.0, 128.0));
        for _ in 0..40 {
            g.step(&crate::InputFrame::default());
        }
        throw_boss_rock(&mut g, 128.0);
        assert!((g.world.entity(id).unwrap().animation - 2.8).abs() < 1e-9);
        for _ in 0..2 {
            g.step(&crate::InputFrame::default());
        }
        assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienBossHurt");
        g.step(&crate::InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienBoss");
    }

    #[test]
    fn actual_step_boss_hurt_animation_end_runs_outside_ai_viewport() {
        let mut g = boss_item_room();
        g.player.pos.x = 208.0;
        g.update_camera();
        assert_eq!(g.camera.x, 16.0);
        let left = crate::InputFrame {
            left: true,
            run: true,
            ..Default::default()
        };
        while g.player.pos.x > 152.0 {
            g.step(&left);
        }
        assert_eq!(g.camera.x, 16.0);
        // The rock's source Step gate is still active on the hit frame. Running
        // farther left then scrolls the boss beyond its narrower AI guard.
        let id = g.world.spawn("oAlienBoss", Vec2::new(335.0, 128.0));
        throw_boss_rock(&mut g, 335.0);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienBossHurt");
        for _ in 0..7 {
            g.step(&left);
        }
        let boss = g.world.entity(id).unwrap();
        assert_eq!(boss.sprite, "sAlienBoss");
        assert_eq!(boss.pos, Vec2::new(335.0, 128.0));
        assert!(g.camera.x < 11.0);
        // Animation End changes the sprite; the skipped Step has not restored
        // its normal image speed yet.
        assert_eq!(boss.value("animation_speed"), 0.8);
        let frozen_counter = boss.value("psychic_recover");
        g.step(&left);
        assert_eq!(
            g.world.entity(id).unwrap().value("psychic_recover"),
            frozen_counter
        );
    }

    #[test]
    fn actual_step_king_turn_finishes_while_falling_player_scrolls_away() {
        let mut g = boss_item_room();
        for x in 0..8 {
            g.world.set_tile(x, 10, "");
            g.world.set_tile(x, 21, "oBlock");
        }
        g.player.state = PlayerState::Falling;
        g.player.grounded = false;
        g.player.vel.y = 10.0;
        let id = g.world.spawn("oYetiKing", Vec2::new(128.0, 128.0));
        g.step(&crate::InputFrame::default());
        g.step(&crate::InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 2);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sYetiKingTurnL");
        for _ in 0..48 {
            g.step(&crate::InputFrame::default());
        }
        let king = g.world.entity(id).unwrap();
        assert_eq!(g.camera.y, 184.0);
        assert_eq!(king.pos, Vec2::new(131.0, 128.0));
        assert_eq!(king.state, 1);
        assert_eq!(king.facing, -1);
        // Animation End changes facing/state even when Step is skipped. Its
        // counter and attack timer remain frozen by the source viewport guard.
        assert_eq!(king.value("counter"), 30.0);
        assert_eq!(king.value("attack_timer"), 0.0);
    }

    #[test]
    fn actual_step_king_yell_finishes_offscreen_without_running_ai_timers() {
        let mut g = boss_item_room();
        for x in 0..8 {
            g.world.set_tile(x, 10, "");
            g.world.set_tile(x, 21, "oBlock");
        }
        // The king remains supported while the player's narrower body falls
        // alongside this tile and scrolls the camera down.
        g.world.set_tile(1, 10, "oBlock");
        g.player.state = PlayerState::Falling;
        g.player.grounded = false;
        g.player.vel.y = 10.0;
        let id = g.world.spawn("oYetiKing", Vec2::new(16.0, 128.0));
        g.step(&crate::InputFrame::default());
        g.step(&crate::InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 3);
        for _ in 0..48 {
            g.step(&crate::InputFrame::default());
        }
        let king = g.world.entity(id).unwrap();
        assert_eq!(g.camera.y, 184.0);
        assert_eq!(king.pos, Vec2::new(19.0, 128.0));
        assert_eq!(king.state, 0);
        assert_eq!(king.value("counter"), 30.0);
        assert_eq!(king.value("attack_timer"), 100.0);
        assert_eq!(king.value("animation_speed"), 0.25);
    }

    fn natural_yeti_grab_room(player_x: f64) -> (Game, u64) {
        let mut g = boss_item_room();
        g.player.pos = Vec2::new(player_x, 152.0);
        g.player.previous_pos = g.player.pos;
        // Preserve source Yeti Create's IDLE/counter0/xVel2.5. Its first Step
        // moves normally and enters WALK before the real Character collision.
        let id = g.spawn("oYeti", Vec2::new(128.0, 144.0));
        assert_eq!(g.world.entity(id).unwrap().vel.x, 2.5);
        assert_eq!(g.world.entity(id).unwrap().state, 0);
        (g, id)
    }

    #[test]
    fn actual_step_natural_yeti_grab_stops_thrower_before_the_following_move() {
        for (player_x, facing) in [(136.0, -1), (144.0, 1)] {
            let (mut g, id) = natural_yeti_grab_room(player_x);
            g.step(&crate::InputFrame::default());
            let yeti = g.world.entity(id).unwrap();
            assert!(
                yeti.pos.x > 128.0,
                "the source Create velocity actually moved"
            );
            assert!(
                !yeti.held,
                "the parent held branch must not mask the omission"
            );
            assert_eq!(yeti.state, 3, "actual Character collision entered THROW");
            assert_eq!(yeti.sprite, "sYetiThrowL");
            assert_eq!(yeti.facing, facing);
            assert_eq!(
                yeti.vel.x, 0.0,
                "Character THROW entry explicitly stops xVel"
            );
            assert_eq!(
                yeti.value("counter"),
                0.0,
                "THROW does not reset the counter"
            );
            assert!(g.player.is_stunned());
            assert_eq!(
                g.player.stun, 0,
                "source throw uses an independent stunned flag"
            );
            assert_eq!(g.player.vel, Vec2::new(f64::from(facing) * 6.0, -6.0));
            let stopped_x = yeti.pos.x;
            let thrown_x = g.player.pos.x;
            for _ in 0..3 {
                g.step(&crate::InputFrame::default());
                let yeti = g.world.entity(id).unwrap();
                assert_eq!(yeti.state, 3);
                assert_eq!(yeti.pos.x, stopped_x, "moveTo runs before the THROW branch");
                assert_eq!(yeti.vel.x, 0.0);
            }
            assert!((g.player.pos.x - thrown_x) * f64::from(facing) > 5.0);
            assert_eq!(g.player.health, 4);
        }
    }

    #[test]
    fn actual_step_yeti_stops_itself_but_preserves_player_and_dropped_item_momentum() {
        for (player_x, facing) in [(136.0, -1), (144.0, 1)] {
            let (mut g, yeti_id) = natural_yeti_grab_room(player_x);
            // A valid initial carried item; the fixture performs no state or
            // position edits after simulation starts. The real grab drops it.
            let rock = g.spawn("oRock", g.player.pos);
            g.player.held = Some(rock);
            g.world.entity_mut(rock).unwrap().held = true;
            g.step(&crate::InputFrame::default());
            let yeti = g.world.entity(yeti_id).unwrap();
            assert_eq!(yeti.state, 3);
            assert_eq!(yeti.vel.x, 0.0);
            assert_eq!(g.player.held, None);
            let dropped = g.world.entity(rock).unwrap();
            assert!(!dropped.held);
            let velocity = Vec2::new(f64::from(facing) * 6.0, -6.0);
            assert_eq!(g.player.vel, velocity);
            assert_eq!(
                dropped.vel, velocity,
                "the zero belongs to Yeti, not the thrown pair"
            );
            assert_eq!(g.player.health, 4);
        }
    }

    #[test]
    fn integrated_yeti_stomps_before_grabbing_even_while_rising() {
        let mut g = game();
        let id = g.world.spawn("oYeti", Vec2::new(64.0, 64.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::default();
        g.player.pos = Vec2::new(72.0, 65.0);
        g.player.state = PlayerState::Jumping;
        g.player.vel.y = -1.0;
        g.step(&crate::InputFrame::default());
        let yeti = g.world.entity(id).unwrap();
        assert_eq!(yeti.health, 5);
        assert_eq!(yeti.state, 98);
        assert_eq!(g.player.stun, 0);
        assert_eq!(g.player.pos, Vec2::new(72.0, 65.0));
    }

    #[test]
    fn integrated_yeti_ignores_outer_collision_without_center_overlap() {
        let mut g = game();
        let id = g.world.spawn("oYeti", Vec2::new(64.0, 64.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::default();
        g.player.pos = Vec2::new(85.0, 72.0);
        g.player.state = PlayerState::Standing;
        g.step(&crate::InputFrame::default());
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.stun, 0);
        assert_ne!(g.world.entity(id).unwrap().state, 3);
    }

    #[test]
    fn integrated_laser_ignores_contact_invincibility_and_uses_hazard_rectangle() {
        let mut g = game();
        g.player.invincible = 20;
        let id = g.world.spawn("oLaser", Vec2::new(110.0, 100.0));
        g.step(&crate::InputFrame::default());
        assert_eq!(g.player.health, 1);
        assert_eq!(g.player.invincible, 19);
        assert_eq!(g.player.stun, 20);
        assert_eq!(g.player.vel, Vec2::new(-2.0, -4.0));
        assert!(g.world.entity(id).is_none());
    }

    #[test]
    fn integrated_psychic_wave_ignores_invincibility_but_respects_stun() {
        let mut g = game();
        g.player.invincible = 20;
        g.world.spawn("oPsychicWave", g.player.pos);
        g.step(&crate::InputFrame::default());
        assert_eq!(g.player.health, 3);
        assert_eq!(g.player.stun, 40);
        g.step(&crate::InputFrame::default());
        assert_eq!(g.player.health, 3);
    }

    #[test]
    fn integrated_laser_collides_at_tile_mask_edge() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        g.world.set_tile(4, 4, "oIce");
        let id = g.world.spawn("oLaser", Vec2::new(62.0, 72.0));
        g.step(&crate::InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert!(!g.world.tile(4, 4).unwrap().is_solid());
    }

    #[test]
    fn integrated_laser_hits_moving_source_solids() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        let block = g.world.spawn("oPushBlock", Vec2::new(64.0, 64.0));
        let laser = g.world.spawn("oLaser", Vec2::new(62.0, 72.0));
        g.step(&crate::InputFrame::default());
        assert!(g.world.entity(block).is_none());
        assert!(g.world.entity(laser).is_none());
    }

    #[test]
    fn integrated_yeti_low_ceiling_hit_grants_thirty_ticks_without_stun() {
        let mut g = game();
        g.world.set_tile(4, 3, "oBlock");
        let id = g.world.spawn("oYeti", Vec2::new(64.0, 64.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::default();
        g.player.pos = Vec2::new(72.0, 72.0);
        g.player.state = PlayerState::Standing;
        g.step(&crate::InputFrame::default());
        assert_eq!(g.player.health, 3);
        assert_eq!(g.player.invincible, 30);
        assert_eq!(g.player.stun, 0);
        assert_eq!(g.player.vel.x, 6.0);
    }

    #[test]
    fn integrated_yeti_sight_emits_source_rays_without_synthetic_charge() {
        let mut g = game();
        g.player.pos = Vec2::new(140.0, 72.0);
        let id = g.world.spawn("oYeti", Vec2::new(64.0, 64.0));
        let yeti = g.world.entity_mut(id).unwrap();
        yeti.state = 1;
        yeti.vel = Vec2::default();
        g.step(&crate::InputFrame::default());
        assert_ne!(g.world.entity(id).unwrap().state, 2);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oEnemySight")
                .count(),
            1
        );
    }

    #[test]
    fn integrated_yeti_corpse_bounces_and_loses_horizontal_speed() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        for x in 0..20 {
            g.world.set_tile(x, 5, "oBlock");
        }
        let id = g.world.spawn("oYeti", Vec2::new(64.0, 64.0));
        let yeti = g.world.entity_mut(id).unwrap();
        yeti.health = 0;
        yeti.vel = Vec2::new(4.0, 3.0);
        g.step(&crate::InputFrame::default());
        let yeti = g.world.entity(id).unwrap();
        assert!((yeti.vel.x - 1.1).abs() < 1e-9);
        assert!((yeti.vel.y + 1.8).abs() < 1e-9);
        assert_eq!(yeti.bounds(), Rect::new(70.0, 70.0, 12.0, 10.0));
    }

    #[test]
    fn integrated_ejected_alien_contact_uses_source_invincibility_without_stun() {
        let mut g = game();
        let id = g.world.spawn("oAlienEject", g.player.pos);
        let alien = g.world.entity_mut(id).unwrap();
        alien.state = 2;
        alien.vel = Vec2::default();
        g.step(&crate::InputFrame::default());
        assert_eq!(g.player.health, 3);
        assert_eq!(g.player.invincible, 30);
        assert_eq!(g.player.stun, 0);
    }

    #[test]
    fn integrated_ejected_alien_converts_on_source_web_collision() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        let id = g.world.spawn("oAlienEject", Vec2::new(100.0, 100.0));
        let alien = g.world.entity_mut(id).unwrap();
        alien.state = 2;
        alien.vel = Vec2::default();
        g.world.spawn("oWeb", Vec2::new(96.0, 96.0));
        g.step(&crate::InputFrame::default());
        assert!(g.world.entity(id).is_none());
        let alien = g
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oAlien")
            .unwrap();
        assert_eq!(alien.pos, Vec2::new(92.0, 88.0));
        assert_eq!(g.data.get("kills"), None);
    }

    #[test]
    fn integrated_boss_crush_uses_source_point_instead_of_body_center() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        g.world.set_tile(4, 4, "oBlock");
        let id = g.world.spawn("oAlienBoss", Vec2::new(52.0, 52.0));
        g.step(&crate::InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().health, 10);
    }

    #[test]
    fn integrated_alien_fall_velocity_has_no_source_terminal_clamp() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        let id = g.world.spawn("oAlien", Vec2::new(50.0, 50.0));
        g.world.entity_mut(id).unwrap().vel.y = 12.0;
        g.step(&crate::InputFrame::default());
        assert!((g.world.entity(id).unwrap().vel.y - 12.6).abs() < 1e-9);
    }

    #[test]
    fn integrated_boss_applies_gravity_before_source_move_to() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        let id = g.world.spawn("oAlienBoss", Vec2::new(50.0, 50.0));
        g.step(&crate::InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().pos.y, 51.0);
    }

    #[test]
    fn integrated_ufo_preserves_search_velocity_when_blocked_by_ceiling() {
        let mut g = game();
        g.world.set_tile(3, 2, "oBlock");
        let id = g.world.spawn("oUFO", Vec2::new(50.0, 46.0));
        g.world.entity_mut(id).unwrap().vel.y = -2.0;
        g.step(&crate::InputFrame::default());
        let ufo = g.world.entity(id).unwrap();
        assert_eq!(ufo.health, 1);
        assert_eq!(ufo.pos.y, 46.0);
        assert_eq!(ufo.state, 1);
    }
}

#[cfg(test)]
#[path = "player_laser_blood_contract_tests.rs"]
mod player_laser_blood_contract_tests;
