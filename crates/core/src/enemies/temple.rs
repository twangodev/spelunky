//! Temple enemies translated from Classic 1.1's individual object events.
//! Instances retain the original GameMaker origins, including top-left boss origins.
use super::{center, move_body};
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
    match e.object.as_str() {
        "oHawkman" | "oHawkmanWorship" => {
            e.health = 4;
            e.vel.x = 2.5;
            e.sprite = "sHawkLeft".into();
            e.data.insert("contact_damage".into(), 0.0);
        }
        "oTombLord" => {
            e.health = 20;
            e.vel.x = 2.5;
            e.sprite = "sTombLordRight".into();
            e.data.insert("contact_damage".into(), 2.0);
            e.data.insert("animation_speed".into(), 0.25);
        }
        "oOlmec" => {
            e.health = 1;
            e.solid = true;
            e.state = -1;
            e.timer = 0;
            e.sprite = "sOlmecStart1".into();
            e.data.insert("intro_stage".into(), 0.0);
            e.data.insert("invincible".into(), 1.0);
            e.data.insert("no_stomp".into(), 1.0);
            e.data.insert("no_whip".into(), 1.0);
        }
        "oOlmecSlam" => {
            e.timer = 1;
            e.sprite = String::new();
        }
        "oMagmaMan" | "oMagma" => {
            e.health = 200;
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
        "oMagmaTrail" => {
            e.timer = 13;
        }
        "oFly" => {
            e.sprite = "sFlyRight".into();
        }
        "oYellowBall" => {
            e.data.insert("launch".into(), 1.0);
            e.data.insert("trail_alarm".into(), 1.0);
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
        "oMagmaTrail" => {
            e.animation += 0.4;
            if e.timer == 0 {
                e.alive = false;
            } else {
                e.timer -= 1;
            }
        }
        _ => {}
    }
}

pub(super) fn animation_end(e: &mut Entity, sprite: &str) {
    if e.object != "oTombLord" {
        return;
    }
    match sprite {
        "sTombLordTurnL" => {
            e.facing = -1;
            e.state = 1;
        }
        "sTombLordTurnR" => {
            e.facing = 1;
            e.state = 1;
        }
        "sTombLordAttackL" | "sTombLordAttackR" => {
            e.state = 0;
            e.timer = 30;
            e.data.insert("animation_speed".into(), 0.25);
        }
        _ => {}
    }
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
    if e.facing < 0 {
        "L"
    } else {
        "R"
    }
}
fn spawn(game: &mut Game, object: &str, pos: Vec2) -> u64 {
    let id = game.world.spawn(object, pos);
    if let Some(e) = game.world.entity_mut(id) {
        super::init_entity(e);
        if object == "oOlmecDebris" {
            // Source instance_create finishes Create before the caller writes
            // the directional velocities used by Olmec's intro alarms.
            initialize_debris(&mut game.rng, e);
        }
    }
    id
}

fn hawk(game: &mut Game, e: &mut Entity) {
    e.animation += if e.state == 2 { 1.0 } else { 0.5 };
    let impact_y = (e.vel.y + super::inherited::gravity(e, 0.6)).min(10.0);
    let (wall, landed) = move_body(&game.world, e, 0.6);
    let grounded = landed || ground(&game.world, e);
    // oHawkman Step checks the lower corpse origin after moveTo, and the
    // center for a live, unheld actor. This must not run before movement.
    let crush_y = if e.state >= 98 { 12.0 } else { 8.0 };
    if (e.state >= 98 || !e.held) && game.world.solid_at(e.pos.x + 8.0, e.pos.y + crush_y) {
        e.health = 0;
        e.alive = false;
        game.sound("sndCavemanDie", e.pos);
        return;
    }
    if e.state >= 98 {
        if grounded {
            if e.timer > 0 {
                e.timer -= 1;
            } else if e.health > 0 {
                e.state = 0;
            }
        }
        super::settle_body(&game.world, e, impact_y);
        e.vel.x = (e.vel.x.abs() - 0.1).max(0.0) * e.vel.x.signum();
        if e.vel.x.abs() < 0.5 {
            e.vel.x = 0.0;
        }
        e.sprite = if e.health <= 0 {
            "sHawkDeadL"
        } else {
            "sHawkStunL"
        }
        .into();
        return;
    }
    if matches!(e.state, 0 | 1) {
        // oHawkman Step creates the same moving oEnemySight instances as
        // Caveman. Their collision event alerts nearby enemies as a group.
        let timer = e.value("sight_timer");
        if timer <= 0.0 {
            let id = spawn(game, "oEnemySight", e.pos);
            if let Some(sight) = game.world.entity_mut(id) {
                sight.vel.x = f64::from(e.facing) * 10.0;
                sight.data.insert("owner".into(), e.id as f64);
            }
            e.data.insert("sight_timer".into(), 5.0);
        } else {
            e.data.insert("sight_timer".into(), timer - 1.0);
        }
    }
    match e.state {
        0 => {
            if grounded && wall {
                e.vel.y = -6.0;
                e.vel.x = e.facing as f64;
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
            if grounded
                && (!game.world.solid_at(
                    e.pos.x + if e.facing < 0 { -1.0 } else { 16.0 },
                    e.pos.y + 16.0,
                ) || game.rng.chance(1, 100))
            {
                e.state = 0;
                e.timer = 20 + game.rng.range(31);
                e.vel.x = 0.0;
            }
        }
        2 => {
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
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 0;
            }
        }
        _ => {}
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
        e.sprite = if e.vel.x.abs() > 0.0 {
            "sHawkRunLeft"
        } else {
            "sHawkLeft"
        }
        .into();
    }
    // Hawkman's source grab/throw is an explicit attack, rather than contact damage.
    if e.state < 3
        && super::touches_player(game, e)
        && game.player.invincible == 0
        && !(game.player.vel.y > 0.0 && game.player.pos.y < e.pos.y + 5.0)
    {
        if game.world.solid_at(e.pos.x + 8.0, e.pos.y - 4.0) {
            game.player.health -= 1;
            game.player.invincible = 30;
            game.player.data.insert("blink".into(), 30.0);
            game.player.vel.x = if game.player.pos.x < e.pos.x {
                -6.0
            } else {
                6.0
            };
            game.events.push(GameEvent::Damage { amount: 1 });
            game.sound("sndHurt", game.player.pos);
        } else {
            e.facing = if game.player.pos.x > e.pos.x + 8.0 {
                1
            } else {
                -1
            };
            e.state = 3;
            e.timer = 14;
            e.vel.x = 0.0;
            e.sprite = "sHawkThrowL".into();
            // Collision with oCharacter: the grabbed character moves to the
            // opposite side of the Hawkman before being thrown.
            game.player.pos = Vec2::new(e.pos.x + if e.facing > 0 { 0.0 } else { 16.0 }, e.pos.y);
            game.player.previous_pos = game.player.pos;
            game.player.vel = Vec2::new(6.0 * e.facing as f64, -6.0);
            game.player.state = crate::PlayerState::Stunned;
            game.player.data.insert("bounced".into(), 0.0);
            super::drop_from_throw(game);
            game.player.data.insert("wall_hurt".into(), 1.0);
        }
    }
}

fn tomb_lord(game: &mut Game, e: &mut Entity) {
    countdown(e, "attack_cooldown");
    let (wall, _) = move_body(&game.world, e, 0.6);
    // oTombLord Step: the source center-in-solid death check follows moveTo.
    if game.world.solid_at(e.pos.x + 16.0, e.pos.y + 16.0) {
        e.health = 0;
    }
    if e.health <= 0 {
        if game.world.level == 13 {
            spawn(game, "oSceptre", Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
        }
        for _ in 0..4 {
            spawn(game, "oBone", e.pos);
        }
        e.alive = false;
        return;
    }
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
                e.animation = 0.0;
                e.sprite = if e.facing < 0 {
                    "sTombLordTurnR"
                } else {
                    "sTombLordTurnL"
                }
                .into();
            } else if d * e.facing as f64 > 0.0
                && (game.player.pos.y - center(e).y).abs() < 32.0
                && e.value("attack_cooldown") == 0.0
            {
                e.state = 3;
                e.animation = 0.0;
                e.vel.x = 0.0;
                e.sprite = format!("sTombLordAttack{}", side(e));
                e.data.insert("attack_cooldown".into(), 100.0);
            } else {
                e.vel.x = e.facing as f64;
                e.sprite = format!("sTombLordWalk{}", side(e));
            }
        }
        2 => {
            e.vel.x = 0.0;
        }
        3 => {
            e.vel.x = 0.0;
            e.data.insert("animation_speed".into(), 0.5);
            // Step refreshes attackTimer throughout the attack animation.
            e.data.insert("attack_cooldown".into(), 100.0);
            if (7.0..=12.0).contains(&e.animation) {
                let pos = Vec2::new(
                    e.pos.x + if e.facing < 0 { 8.0 } else { 24.0 },
                    e.pos.y + 12.0 + game.rng.range(5) as f64,
                );
                let vx = (3 + game.rng.range(3)) as f64 * e.facing as f64;
                let vy = 0.5 - 3.0 * game.rng.unit();
                let id = spawn(game, "oFly", pos);
                if let Some(f) = game.world.entity_mut(id) {
                    f.vel = Vec2::new(vx, vy);
                }
            }
        }
        _ => {}
    }
    if e.state == 1 && e.timer > 0 {
        e.timer -= 1;
    }
}

fn fly(game: &mut Game, e: &mut Entity) {
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    e.animation += 0.5;
    e.sprite = if e.vel.x < 0.0 {
        "sFlyLeft"
    } else {
        "sFlyRight"
    }
    .into();
    if game.world.collides(e.bounds()) {
        e.alive = false;
        game.sound("sndHit", e.pos);
        return;
    }
    if super::touches_player(game, e)
        && !matches!(
            game.player.sprite.as_str(),
            "sPExit" | "sDamselExit" | "sTunnelExit"
        )
    {
        // oFly Collision with oCharacter has no invincibility guard and does
        // not grant new invincibility. Several flies may hit during one volley.
        let invincible = game.player.invincible;
        game.player.invincible = 0;
        game.damage_player(2);
        game.player.invincible = invincible;
        game.player.vel = Vec2::new(e.vel.x, -4.0);
        e.alive = false;
        return;
    }
    if let Some(index) = game.world.entities.iter().position(|other| {
        other.alive
            && other.object == "oDamsel"
            && other.value("invincible") == 0.0
            && other.sprite_bounds().intersects(e.sprite_bounds())
    }) {
        // oFly has a dedicated Damsel collision event, including held bodies.
        // Its throw counter and knockback differ from the oEnemy event below.
        let other = &mut game.world.entities[index];
        let bloodless = other.value("bloodless") != 0.0;
        let blood_left = other.data.entry("blood_left".into()).or_insert(4.0);
        let blood = if *blood_left > 0.0 {
            if other.health < 0 {
                *blood_left -= 1.0;
            }
            (!bloodless).then_some(Vec2::new(other.pos.x + 8.0, other.pos.y + 8.0))
        } else {
            None
        };
        other.held = false;
        game.player.held = game.player.held.filter(|held| *held != other.id);
        other.health -= 2;
        other.vel = Vec2::new(e.vel.x * 0.3, -6.0);
        other.state = 2;
        other.timer = 120;
        let pos = other.pos;
        if let Some(pos) = blood.filter(|_| {
            game.world
                .entities
                .iter()
                .filter(|particle| {
                    particle.alive && crate::catalog::is_a(&particle.object, "oDetritus")
                })
                .count()
                < 16
        }) {
            game.world.spawn("oBlood", pos);
        }
        game.sound("sndDamsel", pos);
        e.alive = false;
        return;
    }
    if let Some(other) = game.world.entities.iter_mut().find(|o| {
        o.alive
            && o.id != e.id
            && o.object != "oTombLord"
            && o.object != "oFly"
            && super::is_enemy(&o.object)
            && o.sprite_bounds().intersects(e.sprite_bounds())
    }) {
        other.health -= 2;
        other.state = 98;
        other.timer = 20;
        other.vel = Vec2::new(e.vel.x, -4.0);
        e.alive = false;
    }
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
        let extra = f > 0.0 && time.is_multiple_of((1.0 / f).round().max(1.0) as u64);
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
pub(crate) fn intro_alarms(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oOlmec")
        .map(|e| e.id)
        .collect();
    for id in ids {
        let index = game.world.entities.iter().position(|e| e.id == id).unwrap();
        let mut boss = game.world.entities.remove(index);
        let mut ready = Vec::new();
        for key in ["intro_alarm", "worship_alarm", "release_alarm"] {
            let remaining = boss.value(key);
            if remaining > 0.0 {
                boss.data.insert(key.into(), remaining - 1.0);
                if remaining <= 1.0 {
                    ready.push(key);
                }
            }
        }
        for alarm in ready {
            match alarm {
                "intro_alarm" => {
                    let stage = boss.value("intro_stage") as u32;
                    boss.data.insert("intro_stage".into(), f64::from(stage + 1));
                    if stage < 3 {
                        boss.sprite = match stage {
                            0 => "sOlmecStart2",
                            1 => "sOlmecStart3",
                            _ => "sOlmec",
                        }
                        .into();
                        for _ in 0..if stage < 2 { 6 } else { 12 } {
                            let (pos, velocity) = match stage {
                                0 => (
                                    Vec2::new(
                                        boss.pos.x + 32.0 + f64::from(game.rng.range(33)),
                                        boss.pos.y + f64::from(game.rng.range(33)),
                                    ),
                                    Vec2::new(
                                        1.0 + f64::from(game.rng.range(4)),
                                        -(1.0 + f64::from(game.rng.range(3))),
                                    ),
                                ),
                                1 => (
                                    Vec2::new(
                                        boss.pos.x + f64::from(game.rng.range(33)),
                                        boss.pos.y + f64::from(game.rng.range(33)),
                                    ),
                                    Vec2::new(
                                        -(1.0 + f64::from(game.rng.range(4))),
                                        -(1.0 + f64::from(game.rng.range(3))),
                                    ),
                                ),
                                _ => (
                                    Vec2::new(
                                        boss.pos.x + f64::from(game.rng.range(65)),
                                        boss.pos.y + 32.0 + f64::from(game.rng.range(33)),
                                    ),
                                    Vec2::new(
                                        f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
                                        -(1.0 + f64::from(game.rng.range(3))),
                                    ),
                                ),
                            };
                            let debris = spawn(game, "oOlmecDebris", pos);
                            game.world.entity_mut(debris).unwrap().vel = velocity;
                        }
                        game.sound("sndThump", boss.pos);
                        boss.data.insert("intro_alarm".into(), 50.0);
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
                        boss.data.insert("worship_alarm".into(), 20.0);
                        game.sound("sndBigJump", boss.pos);
                        game.sound("sndAlert", boss.pos);
                    }
                }
                "worship_alarm" => {
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
                "release_alarm" => {
                    game.player.data.insert("active".into(), 1.0);
                    game.camera.x = 0.0;
                    boss.state = 0;
                    boss.timer = 100;
                }
                _ => unreachable!(),
            }
        }
        game.world
            .entities
            .insert(index.min(game.world.entities.len()), boss);
    }
}

fn olmec(game: &mut Game, e: &mut Entity) {
    let old = e.pos;
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
    let player_on_top = (game.player.pos.y + 8.0 - old.y).abs() < 3.0
        && (game.player.pos.x - (old.x + 32.0)).abs() < 34.0;
    e.data.insert(
        "carryPlayer".into(),
        f64::from(
            Rect::new(e.pos.x, e.pos.y - 2.0, 65.0, 67.0).intersects(game.player.sprite_bounds()),
        ),
    );
    // oPlayer1 Step's crushed check is origin-in-oSolid in every state;
    // touching the boss with a limb is not by itself lethal.
    if olmec_rect(e).contains(game.player.pos) && !player_on_top && game.player.health > 0 {
        game.player.invincible = 0;
        game.damage_player(99);
    }
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
                e.data.insert("intro_alarm".into(), 100.0);
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
            game.data.insert("shake".into(), 10.0);
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
                    game.data.insert("shake".into(), 5.0);
                } else {
                    let idle = game.rng.chance(1, 2);
                    let inactive = game.player.data.get("active") == Some(&0.0);
                    e.state = if idle || inactive { 0 } else { 7 };
                    e.vel = Vec2::default();
                    e.timer = 60;
                    if inactive {
                        e.data.insert("release_alarm".into(), 50.0);
                    }
                }
            }
        }
        7 => {
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
    e.animation += 0.4;
}

fn slam(game: &mut Game, e: &mut Entity) {
    // oOlmecSlam Collision with oSolid unconditionally destroys the touched
    // instance. Its source sprite mask is x+2..=x+60, y..=y+7, not a full row.
    let rect = e.bounds();
    let x0 = (rect.x / 16.0).floor() as i32;
    let x1 = ((rect.x + rect.w - 0.001) / 16.0).floor() as i32;
    let y0 = (rect.y / 16.0).floor() as i32;
    let y1 = ((rect.y + rect.h - 0.001) / 16.0).floor() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if x >= 0 && y >= 0 && game.world.tile(x, y).is_some_and(|t| t.is_solid()) {
                game.world.set_tile(x as usize, y as usize, "");
            }
        }
    }
    for other in &mut game.world.entities {
        if other.id != e.id
            && other.object != "oOlmec"
            && other.alive
            && crate::catalog::is_solid(&other.object)
            && other.sprite_bounds().intersects(rect)
        {
            other.alive = false;
        }
    }
    game.sound("sndSlam", e.pos);
    e.alive = false;
}
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
        e.sprite = match rng.range(3) {
            0 => "sOlmecDebris2",
            1 => "sOlmecDebris3",
            _ => "sOlmecDebris",
        }
        .into();
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
    e.animation += 0.3;
}
fn yellow_ball(game: &mut Game, e: &mut Entity) {
    let alarm = e.value("trail_alarm") - 1.0;
    if alarm <= 0.0 {
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
        e.data.insert("trail_alarm".into(), 4.0);
    } else {
        e.data.insert("trail_alarm".into(), alarm);
    }
    if e.value("launch") != 0.0 {
        let direction = if game.rng.chance(1, 2) { -1.0 } else { 1.0 };
        e.vel = Vec2::new(
            (2 + game.rng.range(4)) as f64 * direction,
            -4.0 - game.rng.unit() * 3.0,
        );
        e.data.insert("launch".into(), 0.0);
    }
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
    e.animation += 0.5;
    // Outside Room is separate from collision/hatching and fires once the
    // whole sprite leaves any room edge, including above the room.
    let room = Rect::new(
        0.0,
        0.0,
        game.world.width as f64 * 16.0,
        game.world.height as f64 * 16.0,
    );
    if !e.sprite_bounds().intersects(room) {
        e.alive = false;
    }
}

fn magma_man(game: &mut Game, e: &mut Entity) {
    if e.health > 0 {
        e.health -= 1;
    }
    if e.health <= 0 || game.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0) {
        if e.state != 99 {
            e.animation = 0.0;
        }
        e.state = 99;
        e.vel = Vec2::default();
        e.sprite = "sMagmaManDie".into();
        e.animation += 0.5;
        if e.animation >= 13.0 {
            e.alive = false;
        }
        return;
    }
    if !ground(&game.world, e) {
        // Unlike the voluntary IDLE transformation below, the source falling
        // branch creates a fresh 200-hp magma projectile without copying hp.
        spawn(game, "oMagma", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        e.alive = false;
        return;
    }
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
        if e.timer > 0 {
            e.timer -= 1;
        } else {
            e.facing = if game.rng.chance(1, 2) { -1 } else { 1 };
            e.state = 1;
            if game.rng.chance(1, 6) {
                let id = spawn(game, "oMagma", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
                if let Some(m) = game.world.entity_mut(id) {
                    m.health = e.health;
                }
                e.alive = false;
                return;
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
        ) || game.rng.chance(1, 100)
        {
            e.state = 0;
            e.timer = 20 + game.rng.range(31);
            e.vel.x = 0.0;
        }
    }
    move_body(&game.world, e, 0.0);
    e.sprite = if e.vel.x == 0.0 {
        "sMagmaManLeft"
    } else {
        "sMagmaManWalkL"
    }
    .into();
    e.animation += 0.5;
    ignite(game, e);
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
    if e.age > 0 && e.age.is_multiple_of(2) {
        spawn(game, "oMagmaTrail", e.pos);
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
        if e.value("dying") == 0.0 {
            e.sprite = "sMagmaManCreate".into();
            e.animation = 0.0;
            e.data.insert("dying".into(), 1.0);
        }
    }
    e.animation += 0.3;
    if e.value("dying") != 0.0 && e.animation + 1e-9 >= 7.0 {
        let id = spawn(game, "oMagmaMan", Vec2::new(e.pos.x - 8.0, e.pos.y - 8.0));
        if let Some(m) = game.world.entity_mut(id) {
            m.health = e.health;
        }
        e.alive = false;
        return;
    }
    if game.world.tiles_collide(e.sprite_bounds(), "oWater")
        || game.world.entities.iter().any(|water| {
            water.alive
                && crate::catalog::is_a(&water.object, "oWater")
                && water.sprite_bounds().intersects(e.sprite_bounds())
        })
    {
        e.alive = false;
        spawn(game, "oSmokePuff", e.pos);
        return;
    }
    ignite(game, e);
}
fn ignite(game: &mut Game, e: &Entity) {
    // Parent handles ordinary contact; burning and bombs are specific magma mechanics.
    if super::touches_player(game, e) && game.player.invincible == 0 {
        game.player.data.insert("burning".into(), 100.0);
        game.player.vel = Vec2::new(
            if game.player.pos.x < center(e).x {
                -6.0
            } else {
                6.0
            },
            -4.0,
        );
    }
    for bomb in game.world.entities.iter_mut().filter(|b| {
        e.object == "oMagmaMan"
            && b.object == "oBomb"
            && b.sprite_bounds().intersects(e.sprite_bounds())
    }) {
        if bomb.value("armed") == 0.0 {
            bomb.data.insert("armed".into(), 1.0);
            bomb.timer = 8 + game.rng.range(5);
            bomb.sprite = "sBombArmed".into();
        }
        bomb.vel.x = if bomb.pos.x < e.pos.x { -3.0 } else { 3.0 };
        if bomb.pos.y < e.pos.y {
            bomb.vel.y = -3.0;
        }
        bomb.held = false;
        if game.player.held == Some(bomb.id) {
            game.player.held = None;
        }
    }
    let mut flames = Vec::new();
    for other in game.world.entities.iter_mut().filter(|o| {
        o.alive
            && super::is_enemy(&o.object)
            && o.object != "oMagmaMan"
            && o.sprite_bounds().intersects(e.sprite_bounds())
    }) {
        if other.state != 98 {
            flames.push(other.pos);
        }
        other.health -= 2;
        other.vel = Vec2::new(if e.pos.x < other.pos.x { -3.0 } else { 3.0 }, -4.0);
        other.data.insert("burning".into(), 100.0);
        if e.object == "oMagma" || !matches!(other.object.as_str(), "oTombLord" | "oYetiKing") {
            other.state = 98;
            other.timer = 50;
        }
    }
    for pos in flames {
        game.sound("sndFlame", pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Area, RulesPreset};
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
                    assert_eq!(boss.value("intro_alarm"), 100.0);
                }
                189 => {
                    assert_eq!(boss.sprite, "sOlmecStart2");
                    assert!(!game
                        .world
                        .entities
                        .iter()
                        .any(|e| e.alive && e.object == "oHawkmanWorship"));
                    let cavemen: Vec<_> = game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oCavemanWorship")
                        .collect();
                    assert_eq!(cavemen.len(), 3);
                    assert!(cavemen
                        .iter()
                        .all(|e| e.animation == 0.0 && e.value("animation_speed") == 0.0));
                }
                239 => assert_eq!(boss.sprite, "sOlmecStart3"),
                289 => assert_eq!(boss.sprite, "sOlmec"),
                339 => {
                    assert_eq!(boss.state, 1);
                    assert!(boss.vel.y < 0.0);
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
            if boss.value("release_alarm") == 50.0 {
                scheduled_at = Some(tick);
            }
            if tick == 359 {
                assert!(!game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oCavemanWorship"));
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
                assert_eq!(game.camera, Vec2::new(0.0, 400.0));
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
            let mut game = Game::shell(42, RulesPreset::Classic);
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
        g.player.pos = Vec2::new(88.0, 148.0);
        g.player.vel.y = 1.0;
        let id = g.world.spawn("oHawkman", Vec2::new(80.0, 144.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::default();
        super::super::tick(&mut g);
        assert_eq!(g.player.health, 4);
        assert_eq!(g.world.entity(id).unwrap().health, 3);
        assert_eq!(g.world.entity(id).unwrap().state, 98);
    }
    #[test]
    fn hawk_throw_preserves_sprite_and_drops_held_item() {
        // oHawkman collision + animation-end events, seven frames at 0.5.
        let mut g = room();
        g.player.pos = Vec2::new(90.0, 152.0);
        let held = g.world.spawn("oRopePile", g.player.pos);
        g.world.entity_mut(held).unwrap().held = true;
        g.player.held = Some(held);
        let id = g.world.spawn("oHawkman", Vec2::new(80.0, 144.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::default();
        super::super::tick(&mut g);
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.pos, Vec2::new(80.0, 144.0));
        assert_eq!(g.player.held, None);
        assert!(!g.world.entity(held).unwrap().held);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sHawkThrowL");
        // Keep the animation check independent of the source-correct thrown
        // item collision: a dropped, fast RopePile can itself stun Hawkman.
        g.world.entity_mut(held).unwrap().pos = Vec2::new(250.0, 64.0);
        super::super::tick(&mut g);
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
        tick(&mut g, &mut e);
        for x in 6..=10 {
            assert!(g.world.tile(x, 10).unwrap().object.is_empty());
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
        tick(&mut g, &mut e);
        assert_eq!(g.player.health, 2);
        assert_eq!(g.player.invincible, 27);
        assert!(!e.alive);
    }
    #[test]
    fn magma_extinguishes_in_authored_water_entities() {
        // oMagma Collision with oWater; authored water is an entity rather than a tile.
        let mut g = room();
        g.world.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
        let mut e = Entity::new(100, "oMagma", Vec2::new(88.0, 88.0));
        init(&mut e);
        e.data.insert("started".into(), 1.0);
        tick(&mut g, &mut e);
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
        assert_eq!(rays[0].vel.x, 10.0);
        assert_eq!(rays[0].value("owner"), 100.0);
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
        tick(&mut g, &mut e);
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
        ignite(&mut g, &magma);
        let bomb = g.world.entity_mut(id).unwrap();
        assert_eq!(bomb.value("armed"), 1.0);
        assert!((8..=12).contains(&bomb.timer));
        bomb.timer = 3;
        ignite(&mut g, &magma);
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
}
