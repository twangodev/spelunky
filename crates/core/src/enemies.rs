//! Source-derived enemy state machines. Explicit precise solid queries use source
//! sprite masks; collision events and unvalidated transforms retain their limits.
use crate::model::*;

mod caves;
mod ice;
mod inherited;
mod jungle;
mod npcs;
pub(crate) mod shop_dialogue;
mod temple;
mod undead;

use crate::dispatcher::Instance;

pub fn is_enemy(object: &str) -> bool {
    crate::catalog::is_a(object, "oEnemy")
}

/// Create events execute at instance creation, including outside AI viewports.
pub(crate) fn on_spawn(world: &mut World, id: u64) {
    caves::on_spawn(world, id);
    let Some(emitter) = world.entity(id).filter(|e| e.object == "oBarrierEmitter") else {
        return;
    };
    let pos = Vec2::new(emitter.pos.x, emitter.pos.y + 16.0);
    if !world
        .entities
        .iter()
        .any(|e| e.alive && e.object == "oBarrier")
    {
        world.spawn("oBarrier", pos);
    }
}

fn is_actor(object: &str) -> bool {
    caves::matches(object)
        || ice::matches(object)
        || jungle::matches(object)
        || temple::matches(object)
        || undead::matches(object)
        || matches!(
            object,
            "oGhost"
                | "oShopkeeper"
                | "oShopkeeper2"
                | "oDamsel"
                | "oScarab"
                | "oDeadFish"
                | "oBarrier"
                | "oBarrierEmitter"
        )
}

/// Initialize once, including enemies spawned during other systems' updates.
pub fn init_entity(e: &mut Entity) {
    if e.value("enemy_initialized") != 0.0 || !is_actor(&e.object) {
        return;
    }
    e.data.insert("enemy_initialized".into(), 1.0);
    e.data
        .insert("enemy".into(), f64::from(is_enemy(&e.object)));
    if is_enemy(&e.object) || e.object == "oDamsel" {
        e.data.insert("heavy".into(), 1.0);
    }
    e.data.insert("stun_time".into(), 200.0);
    e.data.entry("counts_as_kill".into()).or_insert(1.0);
    e.data.entry("blood_left".into()).or_insert(4.0);
    if caves::matches(&e.object) {
        caves::init(e);
    } else if jungle::matches(&e.object) {
        jungle::init(e);
    } else if ice::matches(&e.object) {
        ice::init(e);
    } else if temple::matches(&e.object) {
        temple::init(e);
    } else if undead::matches(&e.object) {
        undead::init(e);
    } else {
        npcs::init(e);
    }
    let collision = match e.object.as_str() {
        "oSnake" | "oCaveman" | "oManTrap" | "oShopkeeper" | "oShopkeeper2" | "oYeti"
        | "oHawkman" | "oHawkmanWorship" | "oSkeleton" | "oVampire" | "oMagmaMan" => {
            Some([2.0, 0.0, 14.0, 16.0])
        }
        "oBat" => Some([2.0, 2.0, 14.0, 14.0]),
        "oSpider" => Some([1.0, 5.0, 15.0, 16.0]),
        "oGiantSpider" => Some([2.0, 16.0, 30.0, 32.0]),
        "oFrog" | "oFireFrog" | "oZombie" => Some([4.0, 4.0, 12.0, 16.0]),
        "oMonkey" => Some([4.0, 6.0, 12.0, 16.0]),
        "oPiranha" | "oDeadFish" => Some([0.0, 0.0, 8.0, 8.0]),
        "oJaws" => Some([0.0, 0.0, 48.0, 32.0]),
        "oDamsel" => Some([-4.0, -4.0, 4.0, 8.0]),
        "oGhost" => Some([4.0, 0.0, 12.0, 16.0]),
        "oScarab" => Some([4.0, 4.0, 12.0, 12.0]),
        "oYetiKing" | "oTombLord" => Some([6.0, 0.0, 26.0, 32.0]),
        "oAlien" => Some([2.0, 6.0, 14.0, 16.0]),
        "oUFO" => Some([4.0, 2.0, 12.0, 14.0]),
        "oAlienBoss" => Some([0.0, 0.0, 32.0, 32.0]),
        "oOlmec" => Some([2.0, 0.0, 62.0, 64.0]),
        "oMagma" => Some([-8.0, -8.0, 8.0, 8.0]),
        _ => None,
    };
    if let Some(values) = collision {
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
}

/// Source engine dispatches one instance per event phase.
pub(crate) fn handles(object: &str) -> bool {
    is_actor(object)
}

fn with_actor(game: &mut Game, id: u64, action: impl FnOnce(&mut Game, &mut Entity)) {
    with_actor_inner(game, id, false, action);
}
fn with_actor_inner(
    game: &mut Game,
    id: u64,
    tombstone: bool,
    action: impl FnOnce(&mut Game, &mut Entity),
) {
    let Some(index) = game
        .world
        .entities
        .iter()
        .position(|e| e.id == id && (tombstone || e.alive))
    else {
        return;
    };
    let mut actor = game.world.entities.remove(index);
    init_entity(&mut actor);
    action(game, &mut actor);
    destroyed(game, &mut actor);
    game.world
        .entities
        .insert(index.min(game.world.entities.len()), actor);
}

pub(crate) fn step_instance(game: &mut Game, id: u64) {
    with_actor(game, id, |game, e| {
        e.data
            .insert("simulation_tick".into(), game.engine_time() as f64);
        inherited::tick(game, e);
        if !e.alive {
            return;
        }
        if e.object == "oDamsel" && e.state == 4 {
            npcs::tick(game, e);
            return;
        }
        if source_active(game, e) && e.value("inactive") == 0. {
            if caves::matches(&e.object) {
                caves::tick(game, e);
            } else if jungle::matches(&e.object) {
                jungle::tick(game, e);
            } else if ice::matches(&e.object) {
                ice::tick(game, e);
            } else if temple::matches(&e.object) {
                temple::tick(game, e);
            } else if undead::matches(&e.object) {
                undead::tick(game, e);
            } else {
                npcs::tick(game, e);
            }
        }
        if e.alive {
            caves::tick_independent_events(game, e);
        }
    });
}

/// Context-dependent Create runs synchronously from Game::spawn and once on
/// authored/generated room bootstrap; fixed World Create already ran on insertion.
pub(crate) fn create_instance(game: &mut Game, id: u64) {
    if !game
        .world
        .entity(id)
        .is_some_and(|e| handles(&e.object) && e.value("enemy_context_created") == 0.)
    {
        return;
    }
    with_actor(game, id, |game, e| {
        e.data.insert("enemy_context_created".into(), 1.);
        caves::create(game, e);
        jungle::create(game, e);
        ice::create(game, e);
        npcs::create(game, e);
        undead::create(game, e);
    });
}

pub(crate) fn draw_instance(game: &mut Game, id: u64) {
    with_actor(game, id, |game, e| {
        npcs::draw(game, e);
        if matches!(
            e.object.as_str(),
            "oPDummy" | "oPDummy2" | "oPDummy3" | "oPDummy6"
        ) {
            e.data.insert(
                "image_xscale".into(),
                if e.object == "oPDummy6" || e.facing > 0 {
                    -1.
                } else {
                    1.
                },
            );
        }
    });
}

pub(crate) fn builtin_motion(_game: &mut Game, _id: u64) {}
pub(crate) fn outside_room(game: &mut Game, id: u64) {
    with_actor(game, id, |_game, e| {
        if is_enemy(&e.object) || e.object == "oYellowBall" {
            e.alive = false;
        }
        ice::outside_room_event(e);
    });
}
pub(crate) fn animation_end(game: &mut Game, id: u64) {
    with_actor_inner(game, id, true, |game, e| {
        let sprite = e.sprite.clone();
        caves::animation_end(game, e);
        jungle::animation_end(game, e, &sprite);
        ice::animation_end(game, e, &sprite);
        temple::animation_end(game, e, &sprite);
        npcs::animation_end(e, &sprite);
    });
}
pub(crate) fn alarm(game: &mut Game, id: u64, index: u8) {
    with_actor(game, id, |game, e| {
        caves::alarm(game, e, index);
        jungle::alarm(game, e, index);
        ice::alarm(game, e, index);
        temple::alarm(game, e, index);
        undead::alarm(game, e, index);
        npcs::alarm(game, e, index);
    });
}
pub(crate) fn collision(game: &mut Game, id: u64, other: Instance) {
    with_actor_inner(game, id, true, |game, e| {
        let handled = caves::collision(game, e, other)
            || jungle::collision(game, e, other)
            || ice::collision(game, e, other)
            || temple::collision(game, e, other)
            || undead::collision(game, e, other)
            || npcs::collision(game, e, other);
        if handled {
            return;
        }
        match other {
            Instance::Player => base_character_collision(game, e),
            Instance::Entity(other_id) => {
                let Some(projectile) = game.world.entity(other_id).cloned() else {
                    return;
                };
                if matches!(projectile.object.as_str(), "oWhip" | "oWhipPre")
                    && crate::dispatcher::collision_owner(&e.object, &projectile.object)
                        == Some("oEnemy")
                {
                    e.health -= projectile.data.get("damage").copied().unwrap_or(1.) as i32;
                    set_counts_as_kill(e, true);
                    if e.value("blood_left") > 0. {
                        let b = e.sprite_bounds();
                        create_blood(
                            game,
                            e,
                            Vec2::new(e.pos.x + b.w / 2., e.pos.y + b.h / 2.),
                            1,
                        );
                        if e.health < 0 {
                            e.data
                                .insert("blood_left".into(), e.value("blood_left") - 1.);
                        }
                    }
                    game.sound("sndHit", e.pos);
                    return;
                }
                if !matches!(
                    projectile.object.as_str(),
                    "oWhip" | "oWhipPre" | "oSlash" | "oMachetePre" | "oMattockPre" | "oMattockHit"
                ) || e.value("invincible") > 0.
                {
                    return;
                }
                let pre = matches!(
                    projectile.object.as_str(),
                    "oWhipPre" | "oMachetePre" | "oMattockPre"
                );
                let machete = matches!(projectile.object.as_str(), "oSlash" | "oMachetePre");
                let damage = projectile.value("damage").max(1.) as i32;
                apply_melee(
                    game,
                    e,
                    damage,
                    projectile.pos,
                    pre,
                    machete,
                    projectile.value("puncture") > 0.,
                );
            }
            _ => {}
        }
    });
}

/// Compatibility for direct family tests; production uses the event dispatcher.
#[cfg(test)]
pub fn tick(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && is_actor(&e.object))
        .map(|e| e.id)
        .collect();
    for id in ids {
        step_instance(game, id);
    }
}

/// Source Destroy runs even when a different system destroys an instance.
/// Keep this idempotent because a callback may already run at its producer.
fn destroyed(game: &mut Game, e: &mut Entity) {
    if e.alive {
        return;
    }
    npcs::destroyed(game, e);
    if is_enemy(&e.object) && e.held {
        game.player.held = None;
        e.held = false;
    }
}

pub(crate) fn finish_destroy_events(game: &mut Game) {
    npcs::finish_destroy_events(game);
    if game
        .world
        .entities
        .iter()
        .any(|e| !e.alive && is_enemy(&e.object) && e.held)
    {
        game.player.held = None;
        for e in &mut game.world.entities {
            if !e.alive && is_enemy(&e.object) {
                e.held = false;
            }
        }
    }
}

/// oPsychicWaveP owns separate Collision events; its Enemy event preserves
/// the target state and the wave, while its Damsel event always consumes it.
pub(crate) fn projectile_collision(game: &mut Game, projectile_id: u64, target_id: u64) {
    let Some(projectile) = game.world.entity(projectile_id).cloned() else {
        return;
    };
    with_actor(game, target_id, |game, target| {
        if !is_enemy(&target.object) && target.object != "oDamsel" {
            return;
        }
        let damsel = target.object == "oDamsel";
        if projectile.object == "oPsychicWaveP" {
            if damsel {
                if let Some(wave) = game.world.entity_mut(projectile_id) {
                    wave.alive = false;
                }
            }
            if target.object == "oAlienBoss" || target.value("invincible") != 0. {
                return;
            }
            target.health -= 3;
            let _discarded = game.rng.range(3) as i32 - (1 + game.rng.range(2)) as i32;
            target.vel = Vec2::new(-1., -6.);
            if damsel {
                if let Some(wave) = game
                    .world
                    .entities
                    .iter_mut()
                    .find(|w| w.id == projectile_id)
                {
                    wave.state = 2;
                }
            }
        } else if projectile.object == "oBullet"
            && (if damsel {
                target.value("invincible") == 0.
            } else {
                projectile.value("safe") == 0.
            })
        {
            if damsel && target.value("blood_left") > 0. {
                let b = target.sprite_bounds();
                create_blood(
                    game,
                    target,
                    Vec2::new(target.pos.x + b.w / 2., target.pos.y + b.h / 2.),
                    1,
                );
                if target.health < 0 {
                    target
                        .data
                        .insert("blood_left".into(), target.value("blood_left") - 1.);
                }
            }
            target.health -= 4;
            target.data.insert("assaulted".into(), 1.);
            target.vel = if matches!(target.object.as_str(), "oYetiKing" | "oTombLord") {
                Vec2::new(projectile.vel.x * 0.5, -2.)
            } else {
                Vec2::new(projectile.vel.x, -4.)
            };
            if matches!(
                target.object.as_str(),
                "oCaveman" | "oYeti" | "oHawkman" | "oShopkeeper" | "oShopkeeper2"
            ) && target.state != 99
            {
                target.state = 98;
                target.timer = 20;
            }
            if damsel {
                target.state = 2;
                target.timer = 120;
                target.vel = Vec2::new(projectile.vel.x * 0.3, -6.);
                target.held = false;
                game.player.held = game.player.held.filter(|id| *id != target.id);
            }
            if !damsel && target.value("blood_left") > 0. {
                let b = target.sprite_bounds();
                create_blood(
                    game,
                    target,
                    Vec2::new(target.pos.x + b.w / 2., target.pos.y + b.h / 2.),
                    1,
                );
                if target.health < 0 {
                    target
                        .data
                        .insert("blood_left".into(), target.value("blood_left") - 1.);
                }
            }
            if let Some(bullet) = game.world.entity_mut(projectile_id) {
                bullet.alive = false;
            }
            game.sound(if damsel { "sndDamsel" } else { "sndHit" }, target.pos);
        }
    });
}

pub(crate) fn player_hurt(game: &mut Game) {
    ice::player_hurt(game);
}

/// Explicit imprecise rectangle queries in the inherited item Step.
pub(crate) fn item_step_impacts(game: &mut Game, item_id: u64) {
    let Some(item) = game.world.entity(item_id).cloned() else {
        return;
    };
    let fragile = matches!(item.object.as_str(), "oJar" | "oSkull");
    let speed = if item.object == "oDice" { 3. } else { 2. };
    if item.held
        || (item.vel.x.abs() <= speed && item.vel.y.abs() <= speed)
        || (item.object == "oBomb" && item.value("sticky") > 0.)
    {
        return;
    }
    let radius = if fragile { 3. } else { 2. };
    let area = Rect::new(
        item.pos.x - radius,
        item.pos.y - radius,
        2. * radius + 1.,
        2. * radius + 1.,
    );
    let overlaps_enemy = game.world.entities.iter().any(|e| {
        e.alive && e.id != item_id && is_enemy(&e.object) && e.sprite_bounds().intersects(area)
    });
    let enemy = if overlaps_enemy {
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.id != item_id && is_enemy(&e.object))
            .min_by(|a, b| {
                a.pos
                    .distance(item.pos)
                    .total_cmp(&b.pos.distance(item.pos))
            })
            .map(|e| e.id)
    } else {
        None
    };
    let damsel = game
        .world
        .entities
        .iter()
        .find(|e| {
            e.alive
                && e.id != item_id
                && e.object == "oDamsel"
                && e.sprite_bounds().intersects(area)
        })
        .map(|e| e.id);
    let mut broke = false;
    for target_id in enemy.into_iter().chain(damsel) {
        with_actor(game, target_id, |game, e| {
            if e.value("invincible") != 0. || (!fragile && e.object == "oMagmaMan") {
                return;
            }
            if e.object == "oDamsel"
                && (e.state == 99
                    || e.value("hit_cooldown") > 0.
                    || (e.held && item.value("safe") > 0.))
            {
                return;
            }
            apply_item_hit(game, e, &item.object, item.vel, fragile);
            if fragile {
                broke = true;
            }
            if matches!(item.object.as_str(), "oArrow" | "oFishBone") {
                if let Some(projectile) = game.world.entities.iter_mut().find(|e| e.id == item_id) {
                    projectile.alive = false;
                }
            }
            if item.object == "oArrow" && item.value("bomb_arrow") > 0. {
                game.spawn("oExplosion", item.pos);
            }
        });
    }
    if broke {
        crate::items::break_item(game, item_id);
    }
}

pub(super) fn create_blood(game: &mut Game, e: &Entity, pos: Vec2, count: u32) {
    if e.value("bloodless") != 0. {
        return;
    }
    for _ in 0..count {
        if game
            .world
            .entities
            .iter()
            .filter(|other| other.alive && crate::catalog::is_a(&other.object, "oDetritus"))
            .count()
            < 16
        {
            crate::world_events::spawn_detritus(game, "oBlood", pos);
        }
    }
}

pub(super) fn bounds(e: &Entity) -> Rect {
    e.bounds()
}

fn source_active(game: &Game, e: &Entity) -> bool {
    let (before, after_x, after_y) = match e.object.as_str() {
        "oGhost" | "oOlmec" | "oEnemySight" | "oUFOCrash" | "oLaser" | "oPsychicWave"
        | "oPsychicCreate" | "oFly" | "oOlmecDebris" | "oOlmecSlam" | "oYellowBall"
        | "oMagmaMan" | "oMagma" | "oMagmaTrail" | "oBarrier" | "oCavemanWorship"
        | "oHawkmanWorship" | "oPiranha" | "oDamsel" => return true,
        "oGiantSpider" | "oGiantSpiderHang" => (32.0, 0.0, 0.0),
        "oYetiKing" | "oTombLord" | "oAlienBoss" => (36.0, 0.0, 0.0),
        "oJaws" => (48.0, 48.0, 48.0),
        "oDeadFish" => (16.0, 0.0, 0.0),
        "oBarrierEmitter" => (8.0, 8.0, 8.0),
        "oAlienEject" => (20.0, 4.0, 16.0),
        _ => (20.0, 4.0, 4.0),
    };
    e.pos.x > game.camera.x - before
        && e.pos.x < game.camera.x + VIEW_WIDTH + after_x
        && e.pos.y > game.camera.y - before
        && e.pos.y < game.camera.y + VIEW_HEIGHT + after_y
}

pub(super) fn center(e: &Entity) -> Vec2 {
    let b = bounds(e);
    Vec2::new(b.x + b.w / 2.0, b.y + b.h / 2.0)
}

pub(super) fn event_bounds(e: &Entity) -> Rect {
    e.sprite_bounds()
}
pub(super) fn touches_player(game: &Game, e: &Entity) -> bool {
    event_bounds(e).intersects(game.player.sprite_bounds())
}

pub(super) fn player_alive(game: &Game) -> bool {
    game.player.state != PlayerState::Dead && game.state != RunState::Dead
}

fn contact_gate(e: &Entity) -> (f64, f64, f64) {
    match e.object.as_str() {
        "oCaveman" | "oVampire" | "oYeti" | "oHawkman" | "oShopkeeper" | "oShopkeeper2"
        | "oManTrap" => (8.0, 8.0, 5.0),
        "oMonkey" => (8.0, 4.0, 2.0),
        "oGiantSpider" => (16.0, 16.0, 16.0),
        "oTombLord" | "oYetiKing" => (16.0, 16.0, 8.0),
        "oAlienBoss" => (16.0, f64::INFINITY, 8.0),
        _ => (8.0, 12.0, 8.0),
    }
}

pub(super) fn can_stomp(game: &Game, e: &Entity) -> bool {
    let (x, max_x, y) = contact_gate(e);
    let spike = game.player.has("SpikeShoes");
    player_alive(game)
        && matches!(
            game.player.state,
            PlayerState::Jumping | PlayerState::Falling
        )
        && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
        && (game.player.pos.x - (e.pos.x + x)).abs() <= max_x
        && game.player.pos.y < e.pos.y + y
        && e.value("no_stomp") == 0.0
        && !(body_enemy(&e.object) && e.state >= 98)
        && !(e.object == "oMonkey" && e.state == 7)
        && !(matches!(
            e.object.as_str(),
            "oYeti" | "oHawkman" | "oShopkeeper" | "oShopkeeper2" | "oManTrap"
        ) && game.player.stun > 0)
        && (e.object != "oManTrap" || spike || e.state == 97)
}

/// Source isCollision*/getIdCollision* helpers explicitly request precise masks.
/// Direct collision_point(..., 0, ...) calls intentionally keep bbox queries.
pub(super) fn solid_mask(world: &World, r: Rect, excluded: u64) -> bool {
    world.tiles_collide_precision(r, "oSolid", crate::CollisionPrecision::Precise)
        || world.entities.iter().any(|other| {
            other.id != excluded
                && other.alive
                && other.solid
                && !other.held
                && other.mask_intersects(r, crate::CollisionPrecision::Precise)
        })
}

/// Classic moveTo quantizes fractional speeds by global time, and resolves X first.
/// Movable-solid pushing and one-way platforms are not represented here yet.
pub(super) fn move_body(world: &World, e: &mut Entity, gravity: f64) -> (bool, bool) {
    fn collides(world: &World, e: &Entity, r: Rect) -> bool {
        solid_mask(world, r, e.id)
    }
    fn displacement(v: f64, time: u64) -> i32 {
        let f = v.abs().fract();
        let extra = f > 0.0 && time.is_multiple_of((1.0 / f).round().max(1.0) as u64);
        ((v.abs().floor() as i32) + i32::from(extra)) * if v < 0.0 { -1 } else { 1 }
    }
    let time = e.value("simulation_tick") as u64;
    let dx = displacement(e.vel.x, time);
    let dy = displacement(e.vel.y, time);
    let mut wall = false;
    for _ in 0..dx.unsigned_abs() {
        let b = bounds(e);
        let x = if dx < 0 { b.x - 1.0 } else { b.x + b.w };
        if collides(world, e, Rect::new(x, b.y + 5.0, 1.0, (b.h - 5.0).max(1.0))) {
            wall = true;
            break;
        }
        e.pos.x += f64::from(dx.signum());
    }
    for _ in 0..dy.unsigned_abs() {
        let b = bounds(e);
        let y = if dy < 0 { b.y - 1.0 } else { b.y + b.h };
        if collides(world, e, Rect::new(b.x, y, b.w, 1.0)) {
            e.vel.y = 0.0;
            break;
        }
        e.pos.y += f64::from(dy.signum());
    }
    let b = bounds(e);
    let feet = Rect::new(b.x, b.y + b.h, b.w, 1.0);
    let grounded = collides(world, e, feet);
    let gravity = if gravity > 0.0 {
        inherited::gravity(e, gravity)
    } else {
        gravity
    };
    e.vel.y = (e.vel.y + gravity).min(10.0);
    if grounded && dy >= 0 {
        e.vel.y = 0.0;
    }
    (wall, grounded)
}

pub(super) fn steer_toward(e: &mut Entity, target: Vec2, speed: f64) {
    let origin = center(e);
    let distance = origin.distance(target);
    if distance > 0.0 {
        e.vel = Vec2::new(
            (target.x - origin.x) / distance * speed,
            (target.y - origin.y) / distance * speed,
        );
    } else {
        e.vel = Vec2::default();
    }
    e.facing = if target.x < origin.x { -1 } else { 1 };
}

/// scrCheckCollisions uses the collision flags captured before narrowing a
/// stunned body, and retains its impact speed until applying the bounce.
pub(super) fn settle_body(world: &World, e: &mut Entity, impact_y: f64) {
    let b = e.bounds();
    let left = solid_mask(world, Rect::new(b.x - 1.0, b.y, 1.0, b.h), e.id);
    let right = solid_mask(world, Rect::new(b.x + b.w, b.y, 1.0, b.h), e.id);
    let top = solid_mask(world, Rect::new(b.x, b.y - 1.0, b.w, 1.0), e.id);
    let bottom = solid_mask(world, Rect::new(b.x, b.y + b.h, b.w, 1.0), e.id);
    for (key, value) in [
        ("collision_left", 2.0),
        ("collision_top", 6.0),
        ("collision_right", 14.0),
        ("collision_bottom", 16.0),
    ] {
        e.data.insert(key.into(), value);
    }
    if left && !right {
        e.pos.x += 1.0;
    } else if right {
        e.pos.x -= 1.0;
    }
    if left || right {
        e.vel.x *= -0.5;
    }
    if top && !bottom {
        e.pos.y += 1.0;
    } else if bottom {
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
}

pub(super) fn drop_from_throw(game: &mut Game) {
    if let Some(item) = game.player.held.and_then(|id| game.world.entity_mut(id)) {
        if crate::catalog::is_a(&item.object, "oGoldIdol") {
            item.pos.y -= 8.0;
        }
    }
    if let Some(id) = crate::items::drop_held_with_velocity(game, game.player.vel) {
        if let Some(item) = game.world.entity_mut(id) {
            item.data
                .insert("released_by_enemy_tick".into(), game.tick as f64);
        }
    }
}

fn base_character_collision(game: &mut Game, e: &mut Entity) {
    if e.object == "oDamsel" {
        return;
    }
    if !touches_player(game, e) {
        return;
    }
    let spiked = game.player.has("SpikeShoes")
        || game.player.has("spike_shoes")
        || game.player.has("Spike Shoes")
        || game.player.has("oSpikeShoes");
    let (offset, limit, _) = contact_gate(e);
    if (game.player.pos.x - (e.pos.x + offset)).abs() > limit {
        return;
    }
    if can_stomp(game, e) {
        let fall = game.player.data.get("fall_timer").copied().unwrap_or(0.0);
        let multiplier = if matches!(
            e.object.as_str(),
            "oYeti" | "oYetiKing" | "oShopkeeper" | "oShopkeeper2"
        ) {
            (fall / 16.0).ceil() as i32
        } else {
            (fall / 16.0).floor() as i32 + 1
        };
        let damage = if e.object == "oMonkey" {
            1
        } else {
            multiplier * if spiked { 3 } else { 1 }
        };
        e.health -= damage;
        if e.health <= 0 {
            e.data.insert("killed_by_player".into(), 1.0);
        }
        game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
        game.player.data.insert("fall_timer".into(), 0.0);
        if body_enemy(&e.object) {
            e.state = 98;
            e.timer = e.value("stun_time") as u32;
            e.vel.y = -6.0;
            if game.player.pos.x < e.pos.x + 8.0 {
                e.vel.x += 1.0;
            } else {
                e.vel.x -= 1.0;
            }
        }
        game.sound("sndHit", e.pos);
    } else if !matches!(
        e.object.as_str(),
        "oYeti" | "oManTrap" | "oMonkey" | "oHawkman" | "oShopkeeper" | "oShopkeeper2"
    ) && game.player.invincible == 0
    {
        let damage = 1;
        let prior_health = game.player.health;
        if prior_health > 0 {
            game.player.health -= damage;
        }
        if prior_health > 0 && game.player.health <= 0 && crate::statistics::is_real_level(game) {
            use crate::statistics::EnemyDeath;
            let cause = match e.object.as_str() {
                "oBat" => Some(EnemyDeath::Bat),
                "oSnake" => Some(EnemyDeath::Snake),
                "oSpider" | "oSpiderHang" => Some(EnemyDeath::Spider),
                "oGiantSpider" | "oGiantSpiderHang" => Some(EnemyDeath::GiantSpider),
                "oCaveman" => Some(EnemyDeath::Caveman),
                "oSkeleton" => Some(EnemyDeath::Skeleton),
                "oZombie" => Some(EnemyDeath::Zombie),
                "oVampire" => Some(EnemyDeath::Vampire),
                "oFrog" => Some(EnemyDeath::Frog),
                "oFireFrog" => Some(EnemyDeath::FireFrog),
                "oPiranha" => Some(EnemyDeath::Piranha),
                // Jaws Create uses "Mega Mouth", while the inherited event
                // compares "Megamouth". Preserve the original unmatched row.
                "oAlien" => Some(EnemyDeath::Alien),
                "oUFO" => Some(EnemyDeath::Ufo),
                _ => None,
            };
            if let Some(cause) = cause {
                game.lifetime_stats.enemy_death(cause);
            }
        }
        game.events.push(GameEvent::Damage { amount: damage });
        game.sound("sndHurt", game.player.pos);
        game.player.invincible = 30;
        game.player.data.insert("blink".into(), 30.0);
        game.player.vel.x = if game.player.pos.x < e.pos.x {
            -6.0
        } else {
            6.0
        };
        if e.object == "oMagmaMan" || e.object == "oMagma" {
            game.player.vel.y = -4.0;
            game.player.stun = 20;
            game.player.data.insert("burning".into(), 100.0);
            game.sound("sndFlame", game.player.pos);
        }
    }
}

fn apply_item_hit(game: &mut Game, e: &mut Entity, object: &str, velocity: Vec2, fragile: bool) {
    e.vel.x = velocity.x * 0.3;
    let mut hit = false;
    if e.object == "oDamsel" {
        if object != "oDamsel" {
            if e.held {
                e.held = false;
                game.player.held = game.player.held.filter(|id| *id != e.id);
            }
            e.health -= 1;
            e.state = 2;
            e.timer = 120;
            e.data.insert("hit_cooldown".into(), 10.0);
            e.vel.y = -6.0;
            hit = true;
        }
    } else if fragile {
        if matches!(
            e.object.as_str(),
            "oCaveman" | "oManTrap" | "oYeti" | "oHawkman" | "oShopkeeper" | "oShopkeeper2"
        ) {
            if e.state != 98 {
                e.state = 98;
                e.timer = e.value("stun_time") as u32;
                e.vel.y = -6.0;
                hit = true;
            }
        } else {
            e.health -= 1;
            hit = true;
        }
    } else {
        if object == "oArrow" && e.object == "oVampire" && e.state != 98 {
            e.health -= 3;
        }
        match e.object.as_str() {
            "oCaveman" | "oManTrap" | "oVampire" | "oYeti" | "oHawkman" if e.state != 98 => {
                e.health -= 1;
                e.state = 98;
                e.timer = e.value("stun_time") as u32;
                e.vel.y = -6.0;
                hit = true;
            }
            "oShopkeeper" | "oShopkeeper2" if e.state < 98 => {
                e.health -= 1;
                e.state = 2;
                e.vel.y = -6.0;
                hit = true;
            }
            "oGiantSpider" | "oTombLord" if e.value("hit_cooldown") == 0.0 => {
                e.health -= 1;
                e.data.insert(
                    "hit_cooldown".into(),
                    if e.object == "oTombLord" { 20.0 } else { 10.0 },
                );
                hit = true;
            }
            "oAlienBoss" if e.state != 99 && e.sprite != "sAlienBossHurt" => {
                e.health -= 1;
                e.set_sprite("sAlienBossHurt");
                e.data.insert("animation_speed".into(), 0.8);
                hit = true;
            }
            "oUFO" => {
                let pos = Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0);
                game.spawn("oExplosion", pos);
                game.sound("sndExplosion", pos);
                if game.rng.range(3) == 0 {
                    game.spawn("oAlienEject", pos);
                }
                e.health = 0;
                record_death(game, e);
                e.alive = false;
                // This source branch plays its own explosion sounds, without
                // the ordinary item-hit sound below.
                return;
            }
            "oCaveman" | "oManTrap" | "oVampire" | "oYeti" | "oHawkman" | "oShopkeeper"
            | "oShopkeeper2" | "oGiantSpider" | "oTombLord" | "oAlienBoss" => {}
            _ => {
                e.health -= 1;
                hit = true;
            }
        }
    }
    if hit {
        e.data.insert("assaulted".into(), 1.0);
        game.sound("sndHit", e.pos);
    }
}

fn apply_melee(
    game: &mut Game,
    e: &mut Entity,
    damage: i32,
    pos: Vec2,
    pre: bool,
    machete: bool,
    puncture: bool,
) -> bool {
    let mut actual = damage;
    let mut stun = body_enemy(&e.object);
    let mut bump = false;
    match e.object.as_str() {
        "oCaveman" | "oHawkman" | "oVampire" => {
            if e.state >= 98 && !machete {
                return false;
            }
        }
        "oManTrap" => {
            if e.state == 98 {
                return false;
            }
            if machete {
                actual *= 2;
            }
        }
        "oYeti" => {
            if e.value("hit_cooldown") > 0.0 {
                return false;
            }
            e.data.insert("hit_cooldown".into(), 10.0);
            e.alarms[0] = 10;
            if !(if pre { machete } else { puncture }) {
                actual = 0;
                stun = false;
                bump = true;
            }
        }
        "oShopkeeper" | "oShopkeeper2" => {
            if e.value("hit_cooldown") > 0.0 {
                return false;
            }
            e.data.insert("hit_cooldown".into(), 10.0);
            e.alarms[0] = 10;
            actual = if machete { damage } else { 0 };
            stun = false;
            bump = true;
            e.state = 2;
        }
        "oDamsel" => {
            if e.value("hit_cooldown") > 0.0 {
                return false;
            }
            if !machete && (e.state == 2 || !(e.state < 4 || e.state == 5 || e.state == 6)) {
                return false;
            }
            e.data.insert("hit_cooldown".into(), 10.0);
            actual = if machete { damage } else { 0 };
            stun = false;
            if !machete {
                e.vel.y = -2.0;
            }
        }
        "oGiantSpider" if !pre => {
            if e.value("hit_cooldown") > 0.0 {
                return false;
            }
            actual = 1;
            e.data.insert("hit_cooldown".into(), 10.0);
        }
        "oTombLord" | "oYetiKing" if !pre => {
            if e.value("hit_cooldown") > 0.0 || pos.y >= e.pos.y + 12.0 {
                return false;
            }
            e.data.insert("hit_cooldown".into(), 10.0);
            if e.object == "oYetiKing" && !puncture {
                actual = 0;
            }
        }
        "oMagmaMan" if !pre => {
            if e.value("whipped") == 0. {
                game.sound("sndHit", e.pos);
                e.data.insert("whipped".into(), 1.);
                e.alarms[0] = 10;
            }
            return false;
        }
        _ => {
            if e.value("no_whip") > 0.0 {
                return false;
            }
        }
    }
    e.health -= actual;
    e.data.insert("assaulted".into(), 1.0);
    if actual > 0 {
        set_counts_as_kill(e, true);
    }
    if e.health <= 0 {
        e.data.insert("killed_by_player".into(), 1.0);
    }
    if stun {
        e.state = 98;
        e.timer = e.value("stun_time") as u32;
        e.vel.y = -3.0;
        e.vel.x = if pos.x < e.pos.x + 8.0 { 2.0 } else { -2.0 };
    }
    if bump {
        e.vel.y = -2.0;
        let x = if e.object == "oYeti" && !pre {
            game.player.pos.x - 8.0
        } else {
            pos.x
        };
        e.vel.x = if x < e.pos.x { 1.0 } else { -1.0 };
    }
    if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") && !in_stars(game) {
        game.flags.insert("shopkeepers_angry".into());
    }
    game.sound("sndHit", e.pos);
    true
}

fn body_enemy(object: &str) -> bool {
    matches!(
        object,
        "oCaveman"
            | "oManTrap"
            | "oVampire"
            | "oYeti"
            | "oHawkman"
            | "oShopkeeper"
            | "oShopkeeper2"
    )
}

/// Child Step handlers call this at their literal source death-credit branch.
/// It never changes status, destroys a corpse, or creates death loot.
pub(super) fn record_death(game: &mut Game, e: &mut Entity) {
    use crate::statistics::{EnemyKill, LifetimeCounter};
    if e.value("death_recorded") != 0.0 {
        return;
    }
    e.data.insert("death_recorded".into(), 1.0);
    e.data.insert("dead".into(), 1.0);
    if matches!(e.object.as_str(), "oScarab" | "oMagmaMan" | "oMagma") {
        return;
    }
    let keeper = matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2");
    if keeper && in_stars(game) {
        let kills = game.data.entry("stars_kills".into()).or_default();
        *kills = (*kills + 1.0).min(99.0);
        return;
    }
    if !keeper && e.object != "oSpiderHang" && e.object != "oOlmec" && !counts_as_kill(e) {
        return;
    }
    *game.data.entry("kills".into()).or_default() += 1.0;
    if let Some(category) = kill_category(&e.object) {
        *game.data.entry(category.into()).or_default() += 1.0;
    }
    let real = crate::statistics::is_real_level(game);
    if e.object == "oMonkey" {
        if real {
            game.lifetime_stats
                .increment(LifetimeCounter::TotalMonkeyKills);
        }
    } else if let Some(kind) = match e.object.as_str() {
        "oBat" => Some(EnemyKill::Bat),
        "oSnake" => Some(EnemyKill::Snake),
        "oSpider" | "oSpiderHang" => Some(EnemyKill::Spider),
        "oGiantSpider" | "oGiantSpiderHang" => Some(EnemyKill::GiantSpider),
        "oCaveman" => Some(EnemyKill::Caveman),
        "oSkeleton" => Some(EnemyKill::Skeleton),
        "oZombie" => Some(EnemyKill::Zombie),
        "oVampire" => Some(EnemyKill::Vampire),
        "oFrog" => Some(EnemyKill::Frog),
        "oFireFrog" => Some(EnemyKill::FireFrog),
        "oManTrap" => Some(EnemyKill::ManTrap),
        "oPiranha" | "oDeadFish" => Some(EnemyKill::Piranha),
        "oJaws" => Some(EnemyKill::Jaws),
        "oYeti" => Some(EnemyKill::Yeti),
        "oYetiKing" => Some(EnemyKill::YetiKing),
        "oAlien" | "oAlienEject" => Some(EnemyKill::Alien),
        "oUFO" => Some(EnemyKill::Ufo),
        "oAlienBoss" => Some(EnemyKill::AlienBoss),
        "oHawkman" => Some(EnemyKill::Hawkman),
        "oShopkeeper" | "oShopkeeper2" => Some(EnemyKill::Shopkeeper),
        "oTombLord" => Some(EnemyKill::TombLord),
        "oOlmec" => Some(EnemyKill::Olmec),
        _ => None,
    } {
        if real || matches!(kind, EnemyKill::YetiKing | EnemyKill::Olmec) {
            game.lifetime_stats.enemy_kill(kind);
        }
    }
    if keeper {
        game.flags.insert("murderer".into());
        game.flags.insert("shopkeepers_angry".into());
    }
}

pub(super) fn counts_as_kill(e: &Entity) -> bool {
    e.data.get("counts_as_kill").copied().unwrap_or(1.0) >= 0.5
        && e.value("environmental_death") == 0.0
}

pub(super) fn set_counts_as_kill(e: &mut Entity, value: bool) {
    e.data.insert("counts_as_kill".into(), f64::from(value));
    if value {
        e.data.remove("environmental_death");
    } else {
        e.data.insert("environmental_death".into(), 1.0);
    }
}

pub(super) fn record_keeper_crush(game: &mut Game, e: &mut Entity) {
    if e.value("death_recorded") == 0.0 && e.health > 0 {
        *game.data.entry("kills".into()).or_default() += 1.0;
        *game.data.entry("shopkeepers".into()).or_default() += 1.0;
        if crate::statistics::is_real_level(game) {
            game.lifetime_stats
                .enemy_kill(crate::statistics::EnemyKill::Shopkeeper);
        }
    }
    e.data.insert("death_recorded".into(), 1.0);
    e.data.insert("dead".into(), 1.0);
    game.flags.insert("murderer".into());
}

pub(super) fn record_damsel_death(game: &mut Game, e: &mut Entity, run_credit: bool) {
    if e.value("death_recorded") != 0.0 || e.value("dead") != 0.0 {
        return;
    }
    e.data.insert("death_recorded".into(), 1.0);
    e.data.insert("dead".into(), 1.0);
    game.lifetime_stats
        .increment(crate::statistics::LifetimeCounter::DamselsKilledTotal);
    if run_credit {
        *game.data.entry("kills".into()).or_default() += 1.0;
        *game.data.entry("damselsKilled".into()).or_default() += 1.0;
    }
}

// Source child death handlers maintain the counters consumed by Transition.
// Keep them in the same once-only credit branch as the campaign kill total.
fn kill_category(object: &str) -> Option<&'static str> {
    Some(match object {
        "oBat" => "bats",
        "oSnake" => "snakes",
        "oSpider" | "oSpiderHang" => "spiders",
        "oGiantSpider" | "oGiantSpiderHang" => "giantspiders",
        "oCaveman" => "cavemen",
        "oSkeleton" => "skeletons",
        "oZombie" => "zombies",
        "oVampire" => "vampires",
        "oMonkey" => "monkeys",
        "oFrog" => "frogs",
        "oFireFrog" => "firefrogs",
        "oManTrap" => "mantraps",
        "oPiranha" => "piranhas",
        "oDeadFish" => "deadfish",
        "oJaws" => "megamouths",
        "oYeti" => "yetis",
        "oYetiKing" => "yetikings",
        "oAlien" | "oAlienEject" => "aliens",
        "oUFO" => "ufos",
        "oAlienBoss" => "alienbosses",
        "oHawkman" => "hawkmen",
        "oShopkeeper" | "oShopkeeper2" => "shopkeepers",
        "oTombLord" => "tomblords",
        "oDamsel" => "damselsKilled",
        _ => return None,
    })
}

pub(super) fn in_stars(game: &Game) -> bool {
    game.flags.contains("hub_stars") || game.world.name == "rStars"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_step_item_ufo_death_runs_explosion_and_eject_create_immediately() {
        for seed in 0..64 {
            let mut game = Game::shell(seed, RulesPreset::Classic);
            game.world = World::new(30, 20, Area::Ice, 9);
            game.player.pos = Vec2::new(240., 184.);
            game.player.previous_pos = game.player.pos;
            for x in 0..30 {
                game.world.set_tile(x, 12, "oBrick");
            }
            let ufo = game.spawn("oUFO", Vec2::new(80., 80.));
            let arrow = game.spawn("oArrow", Vec2::new(83., 88.));
            let item = game.world.entity_mut(arrow).unwrap();
            item.vel = Vec2::new(3., 0.);
            item.data.insert("gravity".into(), 0.);
            let mut expected_rng = game.rng.clone();
            if expected_rng.range(3) != 0 {
                continue;
            }
            let expected_facing = if expected_rng.range(2) == 0 { -1 } else { 1 };

            game.step(&InputFrame::default());

            assert!(game.world.entity(ufo).is_none());
            let explosion = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oExplosion")
                .unwrap();
            assert_eq!(explosion.value("announced"), 1.);
            assert_eq!(explosion.age, 0, "earlier resource has no newborn Step");
            let eject = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oAlienEject")
                .unwrap();
            assert_eq!(eject.value("ice_context_created"), 1.);
            assert_eq!(eject.facing, expected_facing);
            assert_eq!(
                eject.pos,
                Vec2::new(88., 82.),
                "later resource runs its newborn Step"
            );
            assert_eq!(game.rng.range(10000), expected_rng.range(10000));
            let sounds: Vec<_> = game
                .events
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Sound { name, .. } => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(
                sounds
                    .iter()
                    .filter(|name| **name == "sndExplosion")
                    .count(),
                2
            );
            assert!(!sounds.contains(&"sndHit"));
            assert_eq!(
                game.lifetime_stats.enemy_kills[crate::statistics::EnemyKill::Ufo as usize],
                1
            );
            return;
        }
        panic!("fixture seeds must include an ejecting UFO");
    }
    #[test]
    fn classic_fraction_uses_global_tick() {
        let world = World::new(20, 20, Area::Mines, 1);
        let mut e = Entity::new(1, "oSnake", Vec2::new(50.0, 50.0));
        e.vel.x = 1.5;
        e.data.insert("simulation_tick".into(), 1.0);
        move_body(&world, &mut e, 0.0);
        assert_eq!(e.pos.x, 51.0);
        e.data.insert("simulation_tick".into(), 2.0);
        move_body(&world, &mut e, 0.0);
        assert_eq!(e.pos.x, 53.0);
    }
    #[test]
    fn held_enemy_tracks_source_player_offset_and_zeroes_velocity() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(100.0, 100.0);
        game.player.facing = 1;
        let id = game.world.spawn("oCaveman", Vec2::new(50.0, 50.0));
        game.player.held = Some(id);
        let e = game.world.entity_mut(id).unwrap();
        e.held = true;
        e.state = 98;
        e.timer = 20;
        e.vel = Vec2::new(4.0, 6.0);
        tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().pos, Vec2::new(96.0, 88.0));
        assert_eq!(game.world.entity(id).unwrap().vel, Vec2::default());
    }
    #[test]
    fn actual_step_later_collision_uses_dead_latch_instead_of_current_hp() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(20, 16, Area::Mines, 1);
        game.player.pos = Vec2::new(136., 144.);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Falling;
        game.player.health = 1;
        game.spawn("oCaveman", Vec2::new(128., 136.));
        let snake = game.spawn("oSnake", Vec2::new(128., 144.));
        game.step(&InputFrame::default());
        // Caveman325 kills first; Snake552 still sees not-other.dead until
        // Player319 runs on the next frame, and accepts the falling stomp.
        assert_eq!(game.player.health, 0);
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.world.entity(snake).unwrap().health, 0);
        assert_eq!(
            game.lifetime_stats.enemy_deaths[crate::statistics::EnemyDeath::Caveman as usize],
            1
        );
        game.step(&InputFrame::default());
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(
            game.lifetime_stats.enemy_kills[crate::statistics::EnemyKill::Snake as usize],
            1
        );
    }

    #[test]
    fn actual_step_inherited_ufo_whip_waits_for_own_next_step_death() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(20, 16, Area::Ice, 9);
        game.player.pos = Vec2::new(66., 81.);
        game.player.previous_pos = game.player.pos;
        game.player.facing = 1;
        game.player.attack_timer = 10;
        game.player.data.insert("whip_elapsed".into(), 0.);
        let ufo = game.spawn("oUFO", Vec2::new(80., 80.));
        let whip = game.spawn("oWhip", Vec2::new(82., 82.));
        let weapon = game.world.entity_mut(whip).unwrap();
        weapon.set_sprite("sWhipRight");
        weapon.facing = 1;
        weapon.timer = 7;
        weapon.data.insert("damage".into(), 1.);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(ufo).unwrap().health, 0);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oExplosion" | "oUFOCrash"))
        );
        assert_eq!(game.data.get("kills"), None);
        game.step(&InputFrame::default());
        assert!(game.world.entity(ufo).is_none());
        assert!(game.world.entities.iter().any(|e| e.object == "oUFOCrash"));
        assert_eq!(
            game.lifetime_stats.enemy_kills[crate::statistics::EnemyKill::Ufo as usize],
            1
        );
    }

    #[test]
    fn actual_step_whip_restores_generic_credit_but_nonpuncture_yeti_keeps_it_false() {
        for (object, restores) in [("oFrog", true), ("oYeti", false)] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.world = World::new(20, 16, Area::Jungle, 5);
            game.player.pos = Vec2::new(66., 81.);
            game.player.previous_pos = game.player.pos;
            game.player.facing = 1;
            game.player.attack_timer = 10;
            game.player.data.insert("whip_elapsed".into(), 0.);
            let target = game.spawn(object, Vec2::new(80., 80.));
            set_counts_as_kill(game.world.entity_mut(target).unwrap(), false);
            let whip = game.spawn("oWhip", Vec2::new(82., 82.));
            let weapon = game.world.entity_mut(whip).unwrap();
            weapon.set_sprite("sWhipRight");
            weapon.facing = 1;
            weapon.timer = 7;
            weapon.data.insert("damage".into(), 1.);
            game.step(&InputFrame::default());
            let victim = game.world.entity(target).unwrap();
            assert_eq!(counts_as_kill(victim), restores, "{object}");
            if object == "oFrog" {
                assert_eq!(victim.health, 0);
            } else {
                assert_eq!(victim.health, 5);
                assert_eq!(victim.value("hit_cooldown"), 10.);
            }
        }
    }

    #[test]
    fn actual_step_magma_man_whip_uses_own_sound_alarm_without_damage() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(20, 16, Area::Temple, 13);
        game.player.pos = Vec2::new(66., 81.);
        game.player.previous_pos = game.player.pos;
        game.player.facing = 1;
        game.player.attack_timer = 10;
        game.player.data.insert("whip_elapsed".into(), 0.);
        game.world.set_tile(5, 6, "oBlock");
        let man = game.spawn("oMagmaMan", Vec2::new(80., 80.));
        game.world.entity_mut(man).unwrap().timer = 20;
        let whip = game.spawn("oWhip", Vec2::new(82., 82.));
        let weapon = game.world.entity_mut(whip).unwrap();
        weapon.set_sprite("sWhipRight");
        weapon.facing = 1;
        weapon.timer = 7;
        game.step(&InputFrame::default());
        let target = game.world.entity(man).unwrap();
        assert_eq!(target.health, 199);
        assert_eq!(target.value("whipped"), 1.);
        assert_eq!(target.alarms[0], 10);
        assert!(
            game.events
                .iter()
                .any(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndHit"))
        );
    }

    #[test]
    fn actual_step_bullet_damsel_uses_own_safe_and_velocity_contract() {
        let mut game = psychic_room();
        let damsel = game.spawn("oDamsel", Vec2::new(136., 152.));
        let bullet = game.spawn("oBullet", Vec2::new(128., 152.));
        let projectile = game.world.entity_mut(bullet).unwrap();
        projectile.vel.x = 8.;
        projectile.data.insert("safe".into(), 1.);
        game.step(&InputFrame::default());
        let victim = game.world.entity(damsel).unwrap();
        assert_eq!(victim.health, 0);
        assert_eq!(victim.state, 2);
        assert_eq!(victim.timer, 120);
        assert_eq!(victim.vel, Vec2::new(2.4, -6.));
        assert!(game.world.entity(bullet).is_none());
        assert_eq!(game.data.get("kills"), None);
        assert_eq!(game.lifetime_stats.damsels_killed_total, 0);
    }

    #[test]
    fn actual_step_generic_contact_counts_only_literal_lethal_real_room_hit() {
        for custom in [false, true] {
            let mut game = psychic_room();
            if custom {
                game.flags.insert("custom_level".into());
            }
            game.player.pos = Vec2::new(136., 152.);
            game.player.previous_pos = game.player.pos;
            game.player.health = 1;
            game.spawn("oSnake", Vec2::new(128., 144.));
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, 0);
            assert_eq!(
                game.lifetime_stats.enemy_deaths[crate::statistics::EnemyDeath::Snake as usize],
                u64::from(!custom)
            );
            // Automatic Collision follows the source Player's own Step.
            assert_eq!(game.state, RunState::Playing);
            game.step(&InputFrame::default());
            assert_eq!(game.state, RunState::Dead);
            assert_eq!(
                game.lifetime_stats.enemy_deaths[crate::statistics::EnemyDeath::Snake as usize],
                u64::from(!custom)
            );
        }
    }

    #[test]
    fn movement_collides_with_source_dynamic_solids() {
        let mut world = World::new(20, 20, Area::Mines, 1);
        world.spawn("oPushBlock", Vec2::new(64.0, 48.0));
        let mut e = Entity::new(99, "oSnake", Vec2::new(50.0, 48.0));
        init_entity(&mut e);
        e.vel.x = 2.0;
        assert!(move_body(&world, &mut e, 0.0).0);
        assert_eq!(e.pos.x, 50.0);
    }
    #[test]
    fn stars_shopkeeper_inherits_combat_without_campaign_statistics() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags.insert("hub_stars".into());
        game.data.insert("stars_kills".into(), 98.0);
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.set_tile(4, 5, "oBlock");
        let id = game.world.spawn("oShopkeeper2", Vec2::new(64.0, 64.0));
        let keeper = game.world.entity_mut(id).unwrap();
        assert_eq!(keeper.object, "oShopkeeper2");
        assert_eq!(keeper.health, 20);
        assert_eq!(keeper.state, 2);
        assert_eq!(keeper.value("has_gun"), 0.0);
        assert_eq!(keeper.value("death_timer"), 200.0);
        keeper.health = 0;
        tick(&mut game);
        assert_eq!(game.data["stars_kills"], 99.0);
        assert_eq!(game.data.get("kills"), None);
        assert!(!game.flags.contains("murderer"));
        assert!(!game.flags.contains("shopkeepers_angry"));
        tick(&mut game);
        assert_eq!(game.data["stars_kills"], 99.0);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oShotgun" | "oGoldNugget"))
        );
    }

    #[test]
    fn actual_step_held_enemy_destroy_releases_player_on_inherited_lava_path() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(30, 20, Area::Temple, 13);
        game.player.pos = Vec2::new(136.0, 152.0);
        game.world.set_tile(8, 10, "oBlock");
        game.world.set_tile(8, 8, "oLava");
        let id = game.world.spawn("oHawkman", Vec2::new(132.0, 140.0));
        let hawk = game.world.entity_mut(id).unwrap();
        hawk.held = true;
        hawk.state = 98;
        hawk.timer = 200;
        game.player.held = Some(id);

        game.step(&InputFrame::default());

        assert!(game.world.entity(id).is_none());
        assert_eq!(game.player.held, None);
    }

    #[test]
    fn actual_step_held_enemy_corpse_keeps_hold_until_destroyed() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(30, 20, Area::Temple, 13);
        game.player.pos = Vec2::new(136.0, 152.0);
        game.world.set_tile(8, 10, "oBlock");
        let id = game.world.spawn("oHawkman", Vec2::new(132.0, 140.0));
        let hawk = game.world.entity_mut(id).unwrap();
        hawk.held = true;
        hawk.health = 0;
        hawk.state = 99;
        game.player.held = Some(id);

        game.step(&InputFrame::default());

        assert!(game.world.entity(id).is_some());
        assert_eq!(game.player.held, Some(id));
    }

    #[test]
    fn actual_step_monkey_whip_event_requires_shared_opaque_pixels() {
        for (player_pos, expected_health) in
            [(Vec2::new(64.0, 79.0), 1), (Vec2::new(66.0, 81.0), 0)]
        {
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.world = World::new(16, 12, Area::Jungle, 5);
            game.player.pos = player_pos;
            game.player.previous_pos = player_pos;
            game.player.facing = 1;
            game.player.attack_timer = 10;
            game.player.data.insert("whip_elapsed".into(), 0.0);
            let monkey = game.world.spawn("oMonkey", Vec2::new(80.0, 80.0));
            game.world.entity_mut(monkey).unwrap().timer = 60;
            let whip = game
                .world
                .spawn("oWhip", Vec2::new(player_pos.x + 16.0, player_pos.y));
            let entity = game.world.entity_mut(whip).unwrap();
            entity.facing = 1;
            entity.sprite = "sWhipRight".into();
            entity.timer = 7;
            entity.data.insert("damage".into(), 1.0);

            game.step(&InputFrame::default());

            let target = game.world.entity(monkey).unwrap();
            let weapon = game.world.entity(whip).unwrap();
            assert!(target.sprite_bounds().intersects(weapon.sprite_bounds()));
            assert_eq!(weapon.event_intersects(target), expected_health == 0);
            assert_eq!(target.health, expected_health);
        }
    }

    #[test]
    fn actual_step_frog_inherited_whip_event_uses_the_precise_weapon_mask() {
        for (player_y, expected_health) in [(79.0, 1), (81.0, 0)] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.world = World::new(30, 20, Area::Jungle, 5);
            game.player.pos = Vec2::new(61.0, player_y);
            game.player.previous_pos = game.player.pos;
            game.player.facing = 1;
            game.player.attack_timer = 10;
            game.player.data.insert("whip_elapsed".into(), 0.0);
            let frog = game.world.spawn("oFrog", Vec2::new(80.0, 80.0));
            let whip = game.world.spawn("oWhip", Vec2::new(77.0, player_y + 1.0));
            let weapon = game.world.entity_mut(whip).unwrap();
            weapon.sprite = "sWhipRight".into();
            weapon.facing = 1;
            weapon.timer = 7;
            weapon.data.insert("damage".into(), 1.0);

            game.step(&InputFrame::default());

            let target = game.world.entity(frog).unwrap();
            let weapon = game.world.entity(whip).unwrap();
            assert!(target.sprite_bounds().intersects(weapon.sprite_bounds()));
            assert_eq!(weapon.event_intersects(target), expected_health == 0);
            assert_eq!(target.health, expected_health);
        }
    }

    #[test]
    fn actual_step_bullet_enemy_event_uses_shared_opaque_pixels() {
        for (bullet_y, expected_health) in [(77.0, 1), (78.0, -3)] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.world = World::new(30, 20, Area::Jungle, 5);
            game.player.pos = Vec2::new(220.0, 152.0);
            let frog = game.world.spawn("oFrog", Vec2::new(80.0, 80.0));
            let bullet = game.world.spawn("oBullet", Vec2::new(80.0, bullet_y));
            let mask = game.world.entity(bullet).unwrap().clone();

            game.step(&InputFrame::default());

            let target = game.world.entity(frog).unwrap();
            assert!(mask.sprite_bounds().intersects(target.sprite_bounds()));
            assert_eq!(mask.event_intersects(target), expected_health < 0);
            assert_eq!(target.health, expected_health);
            assert_eq!(game.world.entity(bullet).is_some(), expected_health == 1);
        }
    }

    fn psychic_room() -> Game {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(40, 30, Area::Ice, 9);
        game.player.pos = Vec2::new(40.0, 152.0);
        for x in 0..40 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game
    }

    #[test]
    fn actual_step_shotgun_kill_populates_transition_family_tally_once() {
        let mut game = psychic_room();
        game.player.pos = Vec2::new(80.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.facing = 1;
        let snake = game.world.spawn("oSnake", Vec2::new(128.0, 144.0));
        let shotgun = game.world.spawn("oShotgun", game.player.pos);
        game.player.held = Some(shotgun);
        game.world.entity_mut(shotgun).unwrap().held = true;
        for tick in 0..30 {
            game.step(&InputFrame {
                attack: tick == 0,
                attack_pressed: tick == 0,
                ..Default::default()
            });
        }
        assert!(game.world.entity(snake).is_none());
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.data.get("snakes"), Some(&1.0));
    }

    #[test]
    fn actual_step_retained_enemy_corpse_credits_family_only_once() {
        let mut game = psychic_room();
        let yeti = game.world.spawn("oYeti", Vec2::new(128.0, 144.0));
        // A real fired projectile is lethal, while the source keeps the corpse.
        for _ in 0..2 {
            game.world.spawn("oBullet", Vec2::new(136.0, 150.0));
        }
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(yeti).unwrap().state, 98);
        assert_eq!(game.data.get("kills"), None);
        for _ in 0..180 {
            game.step(&InputFrame::default());
            if game.world.entity(yeti).unwrap().state == 99 {
                break;
            }
        }
        assert_eq!(game.world.entity(yeti).unwrap().state, 99);
        game.step(&InputFrame::default());
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.data.get("yetis"), Some(&1.0));
    }

    #[test]
    fn actual_step_inherited_lava_death_has_no_family_tally() {
        let mut game = psychic_room();
        game.world.set_tile(8, 9, "oLava");
        let snake = game.world.spawn("oSnake", Vec2::new(128.0, 130.0));
        game.step(&InputFrame::default());
        game.step(&InputFrame::default());
        assert!(game.world.entity(snake).is_none());
        assert_eq!(game.data.get("kills"), None);
        assert_eq!(game.data.get("snakes"), None);
    }

    #[test]
    fn actual_step_picked_up_sceptre_hits_yeti_without_inventing_stun() {
        let mut game = psychic_room();
        game.player.pos.x = 100.0;
        game.player.facing = 1;
        let sceptre = game.world.spawn("oSceptre", Vec2::new(102.0, 152.0));
        let yeti = game.world.spawn("oYeti", Vec2::new(144.0, 144.0));
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(sceptre));
        game.step(&InputFrame::default());
        let mut hit = false;
        for tick in 3..30 {
            game.step(&InputFrame {
                attack: tick == 3,
                attack_pressed: tick == 3,
                ..Default::default()
            });
            let target = game.world.entity(yeti).unwrap();
            if target.health < 5 {
                assert_eq!(target.health, 2);
                assert_eq!(target.state, 1);
                assert_eq!(target.timer, 0);
                assert_eq!(target.vel, Vec2::new(-1.0, -6.0));
                assert!(
                    game.world
                        .entities
                        .iter()
                        .any(|e| e.object == "oPsychicWaveP")
                );
                hit = true;
                break;
            }
        }
        assert!(hit, "the naturally fired wave must reach the walking Yeti");
    }

    #[test]
    fn actual_step_psychic_wave_repeats_enemy_contacts_and_discarded_rng() {
        let mut game = psychic_room();
        let lord = game.world.spawn("oTombLord", Vec2::new(128.0, 128.0));
        game.world.entity_mut(lord).unwrap().timer = 100;
        let wave = game.world.spawn("oPsychicWaveP", Vec2::new(140.0, 144.0));
        let mut expected = game.rng.clone();
        for hp in [17, 14] {
            expected.range(3);
            expected.range(2);
            game.step(&InputFrame::default());
            let target = game.world.entity(lord).unwrap();
            assert_eq!(target.health, hp);
            assert_eq!(target.state, 0);
            assert_eq!(target.vel, Vec2::new(-1.0, -6.0));
            assert!(game.world.entity(wave).unwrap().event_intersects(target));
            assert_eq!(game.rng.clone().unit(), expected.clone().unit());
        }
    }

    #[test]
    fn actual_step_psychic_wave_damsel_collision_preserves_state_and_always_consumes() {
        for invincible in [0.0, 1.0] {
            let mut game = psychic_room();
            let damsel = game.world.spawn("oDamsel", Vec2::new(136.0, 136.0));
            game.world
                .entity_mut(damsel)
                .unwrap()
                .data
                .insert("invincible".into(), invincible);
            let wave = game.world.spawn("oPsychicWaveP", Vec2::new(138.0, 136.0));

            game.step(&InputFrame::default());

            let target = game.world.entity(damsel).unwrap();
            assert_eq!(target.health, if invincible == 0.0 { 1 } else { 4 });
            assert_eq!(target.state, 0);
            assert!(game.world.entity(wave).is_none());
            if invincible == 0.0 {
                assert_eq!(target.vel, Vec2::new(-1.0, -6.0));
            }
        }
        let mut game = psychic_room();
        let damsel = game.world.spawn("oDamsel", Vec2::new(136.0, 136.0));
        game.world.spawn("oExit", Vec2::new(128.0, 128.0));
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(damsel).unwrap().state, 4);
        // Exit frame zero has an empty source collision mask. Let the real
        // exit animation reach its first opaque frame before testing contact.
        game.step(&InputFrame::default());
        game.step(&InputFrame::default());
        let wave = game.world.spawn("oPsychicWaveP", Vec2::new(138.0, 136.0));

        game.step(&InputFrame::default());

        assert!(game.world.entity(wave).is_none());
        assert_eq!(game.world.entity(damsel).unwrap().health, 4);
        assert_eq!(game.world.entity(damsel).unwrap().state, 4);
    }

    #[test]
    fn actual_step_psychic_wave_preserves_alien_boss_and_invincible_enemy() {
        for object in ["oAlienBoss", "oTombLord"] {
            let mut game = psychic_room();
            let id = game.world.spawn(object, Vec2::new(128.0, 128.0));
            let target = game.world.entity_mut(id).unwrap();
            let hp = target.health;
            target.timer = 100;
            if object == "oTombLord" {
                target.data.insert("invincible".into(), 1.0);
            }
            let wave = game.world.spawn("oPsychicWaveP", Vec2::new(140.0, 144.0));

            game.step(&InputFrame::default());

            let target = game.world.entity(id).unwrap();
            assert!(game.world.entity(wave).unwrap().event_intersects(target));
            assert_eq!(target.health, hp);
        }
    }
}
