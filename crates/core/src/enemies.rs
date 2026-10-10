//! Source-derived enemy state machines. Explicit precise solid queries use source
//! sprite masks; collision events and unvalidated transforms retain their limits.
use crate::model::*;

mod caves;
mod ice;
mod inherited;
mod jungle;
mod npcs;
mod temple;
mod undead;

pub fn is_enemy(object: &str) -> bool {
    crate::catalog::is_a(object, "oEnemy")
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

pub fn tick(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && is_actor(&e.object))
        .map(|e| e.id)
        .collect();
    for id in ids {
        let Some(index) = game
            .world
            .entities
            .iter()
            .position(|e| e.id == id && e.alive)
        else {
            continue;
        };
        // Removing the current actor makes independent spawning deterministic and avoids aliasing.
        let mut e = game.world.entities.remove(index);
        init_entity(&mut e);
        let room = Rect::new(
            0.0,
            0.0,
            game.world.width as f64 * 16.0,
            game.world.height as f64 * 16.0,
        );
        if is_enemy(&e.object) && !e.sprite_bounds().intersects(room) {
            e.alive = false;
            game.world
                .entities
                .insert(index.min(game.world.entities.len()), e);
            continue;
        }
        e.data
            .insert("simulation_tick".into(), game.engine_time() as f64);
        let began_dead = e.health <= 0;
        if e.held
            && e.object != "oVampire"
            && e.object != "oManTrap"
            && e.state == 98
            && e.health > 0
        {
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.held = false;
                game.player.held = game.player.held.filter(|held| *held != e.id);
                e.state = if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") {
                    2
                } else {
                    0
                };
            }
        }
        let active = source_active(game, &e);
        inherited::tick(game, &mut e);
        if !e.alive {
            game.world
                .entities
                .insert(index.min(game.world.entities.len()), e);
            continue;
        }
        if e.object == "oDamsel" && e.state == 4 {
            e.data.insert("depth".into(), 1000.0);
            e.animation += e.value("animation_speed");
            if e.animation >= if e.sprite == "sPExit" { 16.0 } else { 17.0 } {
                e.alive = false;
            }
            game.world
                .entities
                .insert(index.min(game.world.entities.len()), e);
            continue;
        }
        if !active {
            caves::tick_inactive_alarm(game, &mut e);
        }
        if active
            && (!e.held || matches!(e.object.as_str(), "oManTrap" | "oDamsel"))
            && e.value("inactive") == 0.0
        {
            if e.value("hit_cooldown") > 0.0 {
                e.data
                    .insert("hit_cooldown".into(), e.value("hit_cooldown") - 1.0);
            }
            if caves::matches(&e.object)
                || jungle::matches(&e.object)
                || undead::matches(&e.object)
                || matches!(
                    e.object.as_str(),
                    "oDamsel" | "oShopkeeper" | "oShopkeeper2" | "oGhost" | "oScarab"
                )
            {
                e.animation += e.value("animation_speed");
            }
            if caves::matches(&e.object) {
                caves::tick(game, &mut e);
            } else if jungle::matches(&e.object) {
                jungle::tick(game, &mut e);
            } else if ice::matches(&e.object) {
                ice::tick(game, &mut e);
            } else if temple::matches(&e.object) {
                temple::tick(game, &mut e);
            } else if undead::matches(&e.object) {
                undead::tick(game, &mut e);
            } else {
                npcs::tick(game, &mut e);
            }
        }
        // Collision events use engine activation, independently of a child's
        // narrower viewport guard around its AI Step action.
        if e.alive && e.health > 0 {
            combat(game, &mut e);
        }
        if e.held && e.object == "oVampire" {
            undead::tick_held(game, &mut e);
        }
        if e.alive && e.object == "oVampire" {
            undead::absorb_blood(game, &mut e);
        }
        if e.health <= 0 && (began_dead || !e.alive) {
            finish_death(game, &mut e);
        }
        game.world
            .entities
            .insert(index.min(game.world.entities.len()), e);
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
        | "oHawkmanWorship" | "oPiranha" => return true,
        "oGiantSpider" | "oGiantSpiderHang" => (32.0, 0.0, 0.0),
        "oYetiKing" | "oTombLord" => (36.0, 0.0, 0.0),
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
    game.player.health > 0 && game.player.state != PlayerState::Dead && game.state != RunState::Dead
}

fn contact_gate(e: &Entity) -> (f64, f64, f64) {
    match e.object.as_str() {
        "oCaveman" | "oVampire" | "oYeti" | "oHawkman" | "oShopkeeper" | "oShopkeeper2"
        | "oManTrap" => (8.0, 8.0, 5.0),
        "oMonkey" => (8.0, 4.0, 2.0),
        "oGiantSpider" | "oGiantSpiderHang" => (16.0, 16.0, 16.0),
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

fn combat(game: &mut Game, e: &mut Entity) {
    if matches!(
        e.object.as_str(),
        "oGhost"
            | "oScarab"
            | "oOlmec"
            | "oOlmecSlam"
            | "oMagmaTrail"
            | "oYellowBall"
            | "oFly"
            | "oLaser"
            | "oPsychicWave"
            | "oPsychicCreate"
            | "oAlienEject"
            | "oUFOCrash"
            | "oOlmecDebris"
            | "oCavemanWorship"
            | "oHawkmanWorship"
            | "oEnemySight"
            | "oBarrier"
            | "oBarrierEmitter"
    ) {
        return;
    }
    let body = event_bounds(e);
    let mut item_targets: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|other| other.alive && (is_enemy(&other.object) || other.object == "oDamsel"))
        .map(|other| (other.id, other.pos, other.object == "oDamsel"))
        .collect();
    item_targets.push((e.id, e.pos, e.object == "oDamsel"));
    let mut hits = Vec::new();
    let mut item_hits = Vec::new();
    let mut impact_explosions = Vec::new();
    for projectile in &mut game.world.entities {
        if !projectile.alive
            || projectile.held
            || projectile.id == e.id
            || projectile
                .data
                .get("released_by_enemy_tick")
                .is_some_and(|tick| *tick == game.tick as f64)
        {
            continue;
        }
        let melee = matches!(
            projectile.object.as_str(),
            "oWhip" | "oWhipPre" | "oSlash" | "oMachetePre" | "oMattockPre" | "oMattockHit"
        );
        let fired = projectile.object == "oPsychicWaveP"
            || (projectile.object == "oBullet" && projectile.value("safe") == 0.0);
        let thrown = crate::catalog::is_a(&projectile.object, "oItem")
            && (projectile.vel.x.abs() > 2.0 || projectile.vel.y.abs() > 2.0)
            && !matches!(projectile.object.as_str(), "oBall" | "oBall2")
            && !(projectile.object == "oBomb" && projectile.value("sticky") > 0.0);
        let fragile = matches!(projectile.object.as_str(), "oJar" | "oSkull");
        let radius = if fragile { 3.0 } else { 2.0 };
        let impact = if thrown {
            Rect::new(
                projectile.pos.x - radius,
                projectile.pos.y - radius,
                radius * 2.0 + 1.0,
                radius * 2.0 + 1.0,
            )
        } else {
            event_bounds(projectile)
        };
        if !body.intersects(impact) {
            continue;
        }
        if !(melee || fired || thrown)
            || e.value("invincible") > 0.0
            || (projectile.object == "oPsychicWaveP" && e.object == "oAlienBoss")
            || (!melee && !fired && e.value("hit_cooldown") > 0.0)
        {
            continue;
        }
        if thrown && !melee && !fired {
            if !is_enemy(&e.object) && e.object != "oDamsel" {
                continue;
            }
            let nearest = item_targets
                .iter()
                .filter(|(_, _, damsel)| *damsel == (e.object == "oDamsel"))
                .min_by(|(_, a, _), (_, b, _)| {
                    a.distance(projectile.pos)
                        .total_cmp(&b.distance(projectile.pos))
                });
            if nearest.is_some_and(|(id, _, _)| *id != e.id) {
                continue;
            }
            if !fragile && e.object == "oMagmaMan" {
                continue;
            }
            if e.object == "oDamsel"
                && (e.state == 99
                    || e.value("hit_cooldown") > 0.0
                    || (e.held && projectile.value("safe") > 0.0))
            {
                continue;
            }
            item_hits.push((projectile.object.clone(), projectile.vel, fragile));
            if projectile.object == "oJar" {
                projectile.data.insert("broken".into(), 1.0);
            }
            if projectile.object == "oSkull" {
                projectile.alive = false;
            }
            if projectile.object == "oArrow" && projectile.value("bomb_arrow") > 0.0 {
                impact_explosions.push(projectile.pos);
            }
            if matches!(projectile.object.as_str(), "oArrow" | "oFishBone") {
                projectile.alive = false;
            }
            continue;
        }
        let key = format!("hit_{}", e.id);
        if projectile.value(&key) > 0.0 {
            continue;
        }
        let damage = if projectile.object == "oBullet" {
            4
        } else if projectile.value("damage") > 0.0 {
            projectile.value("damage") as i32
        } else {
            1
        };
        hits.push((
            damage,
            projectile.pos,
            melee,
            projectile.object == "oBullet",
            projectile.vel,
            projectile.id,
            matches!(
                projectile.object.as_str(),
                "oWhipPre" | "oMachetePre" | "oMattockPre"
            ),
            matches!(projectile.object.as_str(), "oSlash" | "oMachetePre"),
            projectile.value("puncture") > 0.0,
        ));
        if projectile.object == "oArrow" && projectile.value("bomb_arrow") > 0.0 {
            impact_explosions.push(projectile.pos);
        }
        if projectile.object == "oJar" {
            projectile.data.insert("broken".into(), 1.0);
        } else if matches!(
            projectile.object.as_str(),
            "oBullet" | "oArrow" | "oSkull" | "oFishBone"
        ) {
            projectile.alive = false;
        }
    }
    for pos in impact_explosions {
        game.world.spawn("oExplosion", pos);
        game.sound("sndExplosion", pos);
    }
    for (object, velocity, fragile) in item_hits {
        apply_item_hit(game, e, &object, velocity, fragile);
    }
    for (damage, pos, melee, bullet, velocity, projectile_id, pre, machete, puncture) in hits {
        if melee {
            if apply_melee(game, e, damage, pos, pre, machete, puncture) {
                if let Some(projectile) = game
                    .world
                    .entities
                    .iter_mut()
                    .find(|p| p.id == projectile_id)
                {
                    projectile.data.insert(format!("hit_{}", e.id), 1.0);
                }
            }
            continue;
        }
        let x = pos.x;
        e.health -= damage;
        e.data.insert("assaulted".into(), 1.0);
        if e.health <= 0 {
            e.data.insert("killed_by_player".into(), 1.0);
        }
        game.sound("sndHit", e.pos);
        if body_enemy(&e.object) && (e.state < 98 || !melee) {
            e.state = 98;
            e.timer = e.value("stun_time") as u32;
            e.vel.y = -3.0;
            e.vel.x = if x < e.pos.x { 2.0 } else { -2.0 };
        }
        if bullet {
            e.vel = if matches!(e.object.as_str(), "oYetiKing" | "oTombLord") {
                Vec2::new(velocity.x * 0.5, -2.0)
            } else {
                Vec2::new(velocity.x, -4.0)
            };
            if matches!(
                e.object.as_str(),
                "oCaveman" | "oYeti" | "oHawkman" | "oShopkeeper" | "oShopkeeper2"
            ) {
                e.state = 98;
                e.timer = 20;
            }
        }
        if e.object == "oGiantSpider" {
            e.data.insert("hit_cooldown".into(), 10.0);
        }
        if e.object == "oTombLord" {
            e.data.insert("hit_cooldown".into(), 20.0);
        }
        if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") && !in_stars(game) {
            game.flags.insert("shopkeepers_angry".into());
        }
        if e.object == "oDamsel" {
            e.state = 2;
            e.timer = 120;
            e.vel.y = -3.0;
        }
    }
    if e.health <= 0 || e.state >= 98 || e.object == "oDamsel" {
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
        e.data.insert("counts_as_kill".into(), 1.0);
        e.data.remove("environmental_death");
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
        let damage = if e.value("contact_damage") > 0.0 {
            e.value("contact_damage") as i32
        } else {
            1
        };
        game.player.health -= damage;
        game.events.push(GameEvent::Damage { amount: damage });
        game.sound("sndHurt", game.player.pos);
        game.player.invincible = 30;
        game.player.data.insert("blink".into(), 30.0);
        game.player.vel.x = if game.player.pos.x < e.pos.x {
            -6.0
        } else {
            6.0
        };
        if matches!(
            e.object.as_str(),
            "oCaveman"
                | "oVampire"
                | "oGiantSpider"
                | "oGiantSpiderHang"
                | "oYetiKing"
                | "oTombLord"
                | "oAlienBoss"
        ) && game.player.pos.y < e.pos.y
        {
            game.player.vel.y = -6.0;
        }
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
                e.sprite = "sAlienBossHurt".into();
                e.data.insert("animation_speed".into(), 0.8);
                hit = true;
            }
            "oUFO" => {
                let pos = Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0);
                game.world.spawn("oExplosion", pos);
                if game.rng.range(3) == 0 {
                    game.world.spawn("oAlienEject", pos);
                }
                game.sound("sndExplosion", pos);
                e.health = 0;
                e.alive = false;
                hit = true;
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
        "oMagmaMan" if !pre => return false,
        _ => {
            if e.value("no_whip") > 0.0 {
                return false;
            }
        }
    }
    e.health -= actual;
    e.data.insert("assaulted".into(), 1.0);
    if actual > 0 {
        e.data.insert("counts_as_kill".into(), 1.0);
        e.data.remove("environmental_death");
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
    if e.object == "oUFO" {
        game.world
            .spawn("oExplosion", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        if game.rng.range(3) == 0 {
            game.world
                .spawn("oAlienEject", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        }
        game.sound("sndExplosion", e.pos);
        e.health = 0;
        e.alive = false;
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

fn finish_death(game: &mut Game, e: &mut Entity) {
    if e.object == "oVampire" && e.alive {
        return;
    }
    if e.object == "oMagmaMan" && e.alive {
        return;
    }
    if matches!(
        e.object.as_str(),
        "oFly"
            | "oLaser"
            | "oPsychicWave"
            | "oPsychicCreate"
            | "oOlmecSlam"
            | "oOlmecDebris"
            | "oYellowBall"
            | "oMagmaTrail"
            | "oUFOCrash"
    ) {
        e.alive = false;
        return;
    }
    if e.value("death_recorded") == 0.0 {
        e.data.insert("death_recorded".into(), 1.0);
        if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") && in_stars(game) {
            let kills = game.data.entry("stars_kills".into()).or_default();
            *kills = (*kills + 1.0).min(99.0);
        } else if !matches!(e.object.as_str(), "oScarab" | "oMagmaMan")
            && e.value("environmental_death") == 0.0
        {
            *game.data.entry("kills".into()).or_default() += 1.0;
        }
        if e.object == "oAlienEject" {
            *game.data.entry("aliens".into()).or_default() += 1.0;
        }
        if matches!(e.object.as_str(), "oGiantSpider" | "oGiantSpiderHang") {
            let drop = Vec2::new(e.pos.x + 16.0, e.pos.y + 24.0);
            game.world.spawn("oPaste", drop);
            let count = 1 + game.rng.range(3);
            for _ in 0..count {
                let gem = match game.rng.range(3) {
                    0 => "oEmeraldBig",
                    1 => "oSapphireBig",
                    _ => "oRubyBig",
                };
                game.world.spawn(gem, drop);
            }
        }
        if e.object == "oVampire" {
            game.world
                .spawn("oCapePickup", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        }
        if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") && !in_stars(game) {
            game.flags.insert("murderer".into());
            game.flags.insert("shopkeepers_angry".into());
        }
    }
    if e.object == "oAlienBoss" && e.alive {
        return;
    }
    if e.object == "oShopkeeper2" {
        return;
    }
    if body_enemy(&e.object) && e.object != "oVampire" || e.object == "oDamsel" {
        e.state = 99;
    } else {
        e.alive = false;
    }
}

pub(super) fn in_stars(game: &Game) -> bool {
    game.flags.contains("hub_stars") || game.world.name == "rStars"
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn whip_hits_each_actor_once() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(100.0, 100.0);
        let mut e = Entity::new(99, "oCaveman", Vec2::new(50.0, 50.0));
        init_entity(&mut e);
        game.world.spawn("oWhip", e.pos);
        combat(&mut game, &mut e);
        assert_eq!(e.health, 2);
        combat(&mut game, &mut e);
        assert_eq!(e.health, 2);
    }
    #[test]
    fn shotgun_pellets_damage_independently_in_one_tick() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        let mut target = Entity::new(99, "oShopkeeper", Vec2::new(50.0, 50.0));
        init_entity(&mut target);
        for _ in 0..6 {
            let id = game.world.spawn("oBullet", Vec2::new(58.0, 58.0));
            let bullet = game.world.entity_mut(id).unwrap();
            bullet.data.insert("owner_player".into(), 1.0);
            bullet.data.insert("damage".into(), 4.0);
            bullet.vel.x = 8.0;
        }
        combat(&mut game, &mut target);
        assert_eq!(target.health, -4);
        assert_eq!(target.timer, 20);
        assert!(game.world.entities.iter().all(|e| !e.alive));
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
}
