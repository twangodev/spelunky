//! NPC/ghost behavior from oGhost, oDamsel, oShopkeeper and oScarab events.
//! Shop purchase UI/dice/kissing-parlor economics belong to the economy system.
use super::{center, in_stars, move_body};
use crate::statistics::LifetimeCounter;
use crate::*;

pub fn init(e: &mut Entity) {
    // Barrier has no Create event or parent; retain the runner default of 1.
    // Its emitter and the NPCs below explicitly select 0.5 in their Creates.
    if e.object != "oBarrier" {
        e.set_image_speed(0.5);
    }
    match e.object.as_str() {
        "oGhost" => {
            e.health = 1;
            e.state = 1;
            e.set_sprite("sGhostRight");
            e.data.insert("invincible".into(), 1.0);
        }
        "oDamsel" => {
            e.health = 4;
            e.timer = 200;
            e.set_sprite("sDamselLeft");
            e.facing = -1;
            e.data.entry("canPickUp".into()).or_insert(1.0);
            e.data.entry("heavy".into()).or_insert(1.0);
            e.data.entry("gravity".into()).or_insert(0.6);
            e.data.entry("bounceFactor".into()).or_insert(0.5);
            e.data.entry("frictionFactor".into()).or_insert(0.3);
        }
        "oShopkeeper" | "oShopkeeper2" => {
            e.health = 20;
            e.set_sprite("sShopLeft");
            e.facing = -1;
            e.data.insert("has_gun".into(), 1.0);
            e.data.insert("stun_time".into(), 5.0);
            e.data
                .entry("shop_room_x".into())
                .or_insert((((e.pos.x - 16.0).max(0.0) / 160.0).floor()).min(3.0));
            e.data
                .entry("shop_room_y".into())
                .or_insert((((e.pos.y - 16.0).max(0.0) / 128.0).floor()).min(4.0));
            if e.object == "oShopkeeper2" {
                e.state = 2;
                e.data.insert("has_gun".into(), 0.0);
                e.data.insert("death_timer".into(), 200.0);
            }
        }
        "oScarab" => {
            e.set_sprite("sScarab");
            e.data.insert("bloodless".into(), 1.0);
            e.timer = 20;
        }
        "oDeadFish" => {
            // Create leaves the catalog's sDeadFishLeft until its real Step.
            e.facing = 1;
            e.data.insert("dir".into(), 0.0);
            e.data.insert("bloodless".into(), 1.0);
        }
        "oBarrier" | "oBarrierEmitter" => {
            e.data.insert("invincible".into(), 1.0);
        }
        _ => {}
    }
}

/// Source Create state that can run during generation, before producer writes.
pub(super) fn create_local(ctx: &mut crate::creation::EntityCreateContext<'_>, e: &mut Entity) {
    if !matches!(e.object.as_str(), "oDamsel" | "oScarab") || e.value("npc_local_created") != 0.0 {
        return;
    }
    e.data.insert("npc_local_created".into(), 1.0);
    match e.object.as_str() {
        "oDamsel" => {
            // Preserve post-Create overrides in authored-room bootstraps.
            e.data
                .entry("cost".into())
                .or_insert(15000.0 * f64::from(ctx.world.level));
        }
        "oScarab" => {
            e.timer = 10 + ctx.rng.range(21);
            if let Some(value) = match ctx.world.area {
                Area::Mines => Some(4000.0),
                Area::Jungle => Some(8000.0),
                Area::Temple => Some(12000.0),
                _ => None,
            } {
                e.data.insert("value".into(), value);
            }
        }
        _ => {}
    }
}

/// Remaining Game-context effects must run even after generated local Create.
pub(super) fn create(game: &mut Game, e: &mut Entity) {
    create_local(
        &mut crate::creation::EntityCreateContext {
            world: &game.world,
            rng: &mut game.rng,
        },
        e,
    );
    if e.value("npc_created") != 0.0 {
        return;
    }
    e.data.insert("npc_created".into(), 1.0);
    if e.object == "oGhost" {
        game.sound("sndGhost", e.pos);
    }
}

pub fn tick(game: &mut Game, e: &mut Entity) {
    match e.object.as_str() {
        "oGhost" => ghost(game, e),
        "oDamsel" => damsel(game, e),
        "oShopkeeper" => shopkeeper(game, e),
        "oShopkeeper2" => {
            shopkeeper(game, e);
            if e.state == 99 {
                let timer = e.value("death_timer");
                if timer > 0.0 {
                    e.data.insert("death_timer".into(), timer - 1.0);
                } else {
                    create_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 3);
                    e.alive = false;
                }
            }
        }
        "oScarab" => scarab(game, e),
        "oDeadFish" => dead_fish(game, e),
        "oBarrier" | "oBarrierEmitter" => barrier(game, e),
        _ => {}
    }
}

pub(super) fn alarm(_game: &mut Game, e: &mut Entity, index: u8) {
    if index == 0 && matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") {
        e.data.insert("hit_cooldown".into(), 0.0);
        e.data.insert("whipped".into(), 0.0);
    }
}

/// Source oDamsel Draw mutates its collision scale and price-icon frame.
/// The dispatcher owns visibility and when this event is invoked.
pub(super) fn draw(_game: &mut Game, e: &mut Entity) {
    if e.object != "oDamsel" {
        return;
    }
    e.data.insert(
        "image_xscale".into(),
        if e.facing == 1 { -1.0 } else { 1.0 },
    );
    if e.value("cost") > 0.0 {
        let frame = e.value("cimg");
        e.data.insert("draw_cimg".into(), frame);
        let next = frame + 1.0;
        e.data
            .insert("cimg".into(), if next > 9.0 { 0.0 } else { next });
    }
}

pub(super) fn animation_end(e: &mut Entity, sprite: &str) {
    if e.object == "oGhost" {
        match sprite {
            "sGhostDisappear" => e.alive = false,
            "sGhostTurnLeft" => e.set_sprite("sGhostLeft"),
            "sGhostTurnRight" => e.set_sprite("sGhostRight"),
            _ => {}
        }
    } else if e.object == "oDamsel" && matches!(sprite, "sDamselExit2" | "sPExit") {
        e.alive = false;
    } else if e.object == "oDamsel" && e.state == 3 {
        e.state = 0;
        e.timer = 200;
    } else if e.object == "oDamsel" && e.state == 6 {
        e.state = 5;
    } else if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") && sprite == "sShopThrowL"
    {
        e.state = 2;
        e.set_sprite("sShopLeft");
    }
}

/// Automatic collision callbacks operate on the dispatcher-selected pair.
/// Spatial queries in Step remain separate from these engine events.
pub(super) fn collision(
    game: &mut Game,
    e: &mut Entity,
    other: crate::dispatcher::Instance,
) -> bool {
    use crate::dispatcher::Instance;
    if !e.alive {
        return true;
    }
    match (e.object.as_str(), other) {
        ("oGhost", Instance::Player) => {
            ghost_player_collision(game, e);
            true
        }
        ("oScarab", Instance::Player) => {
            collect_scarab(game, e);
            true
        }
        ("oBarrier", Instance::Player) => {
            barrier_player_collision(game, e);
            true
        }
        ("oShopkeeper" | "oShopkeeper2", Instance::Player) => {
            keeper_player_collision(game, e);
            true
        }
        ("oDamsel", Instance::Entity(id)) => {
            let Some(attack) = game.world.entity(id).cloned() else {
                return false;
            };
            if crate::catalog::is_a(&attack.object, "oWhip")
                || crate::catalog::is_a(&attack.object, "oWhipPre")
            {
                damsel_melee(game, e, &attack);
                true
            } else {
                false
            }
        }
        ("oBarrierEmitter", Instance::Entity(id)) => {
            let hit = game.world.entity(id).is_some_and(|item| {
                item.object == "oBullet"
                    || crate::catalog::is_a(&item.object, "oItem")
                        && (item.vel.x.abs() > 2.0 || item.vel.y.abs() > 2.0)
            });
            if hit {
                destroy_emitter(game, e);
            }
            true
        }
        ("oBarrier", Instance::Entity(id)) => {
            barrier_item_collision(game, id);
            true
        }
        ("oShopkeeper" | "oShopkeeper2", Instance::Entity(id)) => {
            if let Some(weapon) = game
                .world
                .entity(id)
                .filter(|weapon| {
                    crate::catalog::is_a(&weapon.object, "oWhip")
                        || crate::catalog::is_a(&weapon.object, "oWhipPre")
                })
                .cloned()
            {
                keeper_melee(game, e, &weapon);
                return true;
            }
            if game
                .world
                .entity(id)
                .is_some_and(|gun| crate::catalog::is_a(&gun.object, "oShotgun"))
            {
                if e.health > 0 && e.state == 2 && e.value("has_gun") == 0.0 {
                    let gun = game.world.entity_mut(id).unwrap();
                    if gun.held {
                        game.player.held = game.player.held.filter(|held| *held != id);
                        game.player.pickup_item_type.clear();
                        gun.held = false;
                    }
                    gun.alive = false;
                    e.data.insert("has_gun".into(), 1.0);
                }
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn damsel_melee(game: &mut Game, damsel: &mut Entity, attack: &Entity) {
    let machete = attack.text.get("type").map_or_else(
        || {
            crate::catalog::is_a(&attack.object, "oSlash")
                || crate::catalog::is_a(&attack.object, "oMachetePre")
        },
        |kind| kind == "Machete",
    );
    if machete && damsel.value("hit_cooldown") == 0.0 {
        // Ordinary attack producers write damage. The fallback preserves the
        // native Create value for directly authored hitboxes without that field.
        let native_damage = if ["oSlash", "oMachetePre", "oMattockHit", "oMattockPre"]
            .iter()
            .any(|object| crate::catalog::is_a(&attack.object, object))
        {
            2.0
        } else {
            1.0
        };
        let damage = attack.data.get("damage").copied().unwrap_or(native_damage);
        damsel.health -= damage as i32;
        damsel.data.insert("hit_cooldown".into(), 10.0);
        game.sound("sndHit", damsel.pos);
        game.sound("sndDamsel", damsel.pos);
        if damsel.value("blood_left") > 0.0 {
            create_blood(game, damsel, super::sprite_blood_center(damsel), 1);
            if damsel.health < 0 {
                damsel
                    .data
                    .insert("blood_left".into(), damsel.value("blood_left") - 1.0);
            }
        }
    } else if damsel.state != 2
        && (damsel.state < 4 || damsel.state == 5 || damsel.state == 6)
        && damsel.value("hit_cooldown") == 0.0
    {
        damsel.vel.y = -2.0;
        damsel.data.insert("hit_cooldown".into(), 10.0);
        game.sound("sndHit", damsel.pos);
        game.sound("sndDamsel", damsel.pos);
        if damsel.value("forSale") != 0.0 {
            crate::economy::anger(game, damsel.pos, "HEY, ONLY I CAN DO THAT!");
        }
    }
}

fn collect_scarab(game: &mut Game, e: &mut Entity) {
    // Literal source Create values; unsupported-area fallback is unchanged.
    let value = e
        .data
        .get("value")
        .copied()
        .unwrap_or(match game.world.area {
            Area::Mines => 4000.0,
            Area::Jungle => 8000.0,
            _ => 12000.0,
        })
        .max(0.0) as u32;
    *game.data.entry("collect".into()).or_default() += f64::from(value);
    let counter = game.data.entry("collectCounter".into()).or_default();
    *counter = (*counter + 20.0).min(100.0);
    *game.data.entry("scarabs".into()).or_default() += 1.0;
    game.events.push(GameEvent::Collected {
        object: "oScarab".into(),
        value,
    });
    game.sound("sndCoin", e.pos);
    e.alive = false;
    destroyed(game, e);
}

fn destroy_emitter(game: &mut Game, e: &mut Entity) {
    e.alive = false;
    destroyed(game, e);
}

/// Source Destroy events also run after collection, outside-room removal, and
/// destruction by another system. Keep the callback idempotent across phases.
pub(super) fn destroyed(game: &mut Game, e: &mut Entity) {
    if e.alive || e.value("npc_destroyed") != 0.0 {
        return;
    }
    match e.object.as_str() {
        "oBarrierEmitter" => {
            e.data.insert("npc_destroyed".into(), 1.0);
            for _ in 0..6 {
                let pos = Vec2::new(
                    e.pos.x + 2.0 + f64::from(game.rng.range(15)),
                    e.pos.y + 2.0 + f64::from(game.rng.range(15)),
                );
                let id = game.spawn("oFlareSpark", pos);
                let vy = f64::from(1 + game.rng.range(3));
                game.world.entity_mut(id).unwrap().vel.y = vy;
            }
            crate::camera_shake::request(game, 10.0);
            game.sound("sndSmallExplode", e.pos);
            for barrier in &mut game.world.entities {
                if barrier.object == "oBarrier" {
                    barrier.alive = false;
                }
            }
        }
        "oScarab" => {
            e.data.insert("npc_destroyed".into(), 1.0);
            // Destroy.xml spells this origin `other.x/y`; its binding during
            // nested GM8 Collision destruction remains unverified. Preserve
            // the actor origin here rather than assume the collecting player.
            for _ in 0..3 {
                let pos = Vec2::new(
                    e.pos.x + 6.0 + f64::from(game.rng.range(5)),
                    e.pos.y + 6.0 + f64::from(game.rng.range(5)),
                );
                game.spawn("oFlareSpark", pos);
            }
        }
        _ => {}
    }
}

pub(super) fn finish_destroy_events(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            !e.alive
                && matches!(e.object.as_str(), "oBarrierEmitter" | "oScarab")
                && e.value("npc_destroyed") == 0.0
        })
        .map(|e| e.id)
        .collect();
    for id in ids {
        let index = game.world.entities.iter().position(|e| e.id == id).unwrap();
        let mut e = game.world.entities.remove(index);
        destroyed(game, &mut e);
        game.world.entities.insert(index, e);
    }
}

fn create_blood(game: &mut Game, e: &Entity, pos: Vec2, count: u32) {
    if e.value("bloodless") != 0.0 {
        return;
    }
    for _ in 0..count {
        if game
            .world
            .entities
            .iter()
            .filter(|p| p.alive && p.engine_active && crate::catalog::is_a(&p.object, "oDetritus"))
            .count()
            < 16
        {
            crate::world_events::spawn_detritus(game, "oBlood", pos);
        }
    }
}

fn ghost(game: &mut Game, e: &mut Entity) {
    if e.health < 1 {
        *game.data.entry("kills".into()).or_default() += 1.0;
        e.data.insert("death_recorded".into(), 1.0);
        e.alive = false;
        return;
    }
    if e.state == 0 {
        return;
    }
    let origin = Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0);
    let distance = origin.distance(game.player.pos);
    if distance > 0.0 {
        e.pos.x += (game.player.pos.x - origin.x) / distance;
        e.pos.y += (game.player.pos.y - origin.y) / distance;
    }
    if game.player.pos.x < e.pos.x + 8.0 {
        if e.sprite == "sGhostRight" {
            e.set_sprite("sGhostTurnLeft");
        }
    } else if e.sprite == "sGhostLeft" {
        e.set_sprite("sGhostTurnRight");
    }
}

fn ghost_player_collision(game: &mut Game, e: &mut Entity) {
    if game.player.cheat_invulnerable || game.player.invincible != 0 {
        return;
    }
    if crate::statistics::is_real_level(game) {
        game.lifetime_stats
            .enemy_death(crate::statistics::EnemyDeath::Ghost);
    }
    game.player.health = -99;
    game.player.invincible = 9999;
    game.player.data.insert("visible".into(), 0.0);
    game.player.data.insert("bounced".into(), 1.0);
    game.data.insert("draw_hud".into(), 0.0);
    for _ in 0..3 {
        crate::world_events::spawn_detritus(game, "oBone", game.player.pos);
    }
    let skull = game.spawn(
        "oSkull",
        Vec2::new(game.player.pos.x, game.player.pos.y - 2.0),
    );
    let vy = -f64::from(1 + game.rng.range(3));
    let vx = f64::from(game.rng.range(4)) - f64::from(game.rng.range(4));
    game.world.entity_mut(skull).unwrap().vel = Vec2::new(vx, vy);
    if let Some(id) = game.player.held.take() {
        if let Some(held) = game.world.entity_mut(id) {
            held.held = false;
            held.vel = Vec2::new(f64::from(e.facing) * 2.0, -4.0);
        }
    }
    game.sound("sndDie", game.player.pos);
    game.sound("sndGhost", e.pos);
    e.state = 0;
    e.set_sprite("sGhostDisappear");
    e.data.insert("animation_speed".into(), 0.2);
}

fn damsel_death_anger(game: &mut Game, damsel: &Entity) {
    if damsel.value("cost") > 0.0 && damsel.health <= 0 {
        crate::economy::anger(game, damsel.pos, "HEY, ONLY I CAN DO THAT!");
    }
}

fn damsel(game: &mut Game, e: &mut Entity) {
    if e.state == 4 {
        if e.value("hit_cooldown") > 0.0 {
            e.data
                .insert("hit_cooldown".into(), e.value("hit_cooldown") - 1.0);
        }
        e.data.insert("depth".into(), 1000.0);
        e.data.insert("invincible".into(), 1.0);
        damsel_death_anger(game, e);
        return;
    }
    // The inherited oItem Step owns held attachment, movement, bouncing,
    // lava sinking, economics and its explicit impact queries. Only that
    // parent event is viewport-gated; child Damsel events continue outside it.
    let inherited_active = crate::items::item_body_active(game, e);
    crate::items::tick_inherited_ball_body(game, e);
    if inherited_active && e.held && game.player.held.is_none() {
        // Parent clears held after attachment; movement resumes next Step.
        e.held = false;
    }
    let body = e.bounds();
    let grounded = super::solid_mask(
        &game.world,
        Rect::new(body.x, body.y + body.h, body.w, 1.0),
        e.id,
    );
    // Source first gates on any origin-point hit, then instance_place selects
    // the first full native-mask overlap in the current active Exit array.
    crate::dispatcher::register_instances(game);
    let exits: Vec<_> = crate::dispatcher::descendant_instances(game, "oExit")
        .into_iter()
        .filter(|instance| crate::dispatcher::is_active(game, *instance))
        .filter_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => game
                .world
                .entity(id)
                .map(|door| (door.pos, door.collision_mask())),
            crate::dispatcher::Instance::Terrain(index) => {
                let pos = instance.position(game);
                Some((
                    pos,
                    game.world.tile_collision_mask(
                        (index % game.world.width) as i32,
                        (index / game.world.width) as i32,
                    ),
                ))
            }
            crate::dispatcher::Instance::Player => None,
        })
        .collect();
    let exit = if exits
        .iter()
        .any(|(_, mask)| mask.contains(e.pos, CollisionPrecision::BoundingBox))
    {
        let own_mask = e.collision_mask();
        exits
            .into_iter()
            .find(|(_, mask)| own_mask.intersects_mask(mask))
            .map(|(pos, _)| pos)
    } else {
        None
    };
    if let Some(exit) = exit.filter(|_| e.health > 0 && e.state != 2 && !e.held) {
        e.state = 4;
        e.vel = Vec2::default();
        e.pos = Vec2::new(exit.x + 8.0, exit.y + 8.0);
        e.previous_pos = e.pos;
        e.set_sprite(if game.flags.contains("character_damsel") {
            "sPExit"
        } else {
            "sDamselExit2"
        });
        e.data.insert("inactive".into(), 1.0);
        e.data.insert("canPickUp".into(), 0.0);
        e.data.insert("depth".into(), 1000.0);
        e.data.insert("invincible".into(), 1.0);
        game.flags.insert("damsel_rescued".into());
        *game.data.entry("pending_damsel_kisses".into()).or_default() += 1.0;
        *game.data.entry("damsels".into()).or_default() += 1.0;
        *game.data.entry("xdamsels".into()).or_default() += 1.0;
        game.lifetime_stats
            .increment(LifetimeCounter::DamselsSavedTotal);
        game.sound("sndSteps", e.pos);
    }
    if e.value("hit_cooldown") > 0.0 {
        e.data
            .insert("hit_cooldown".into(), e.value("hit_cooldown") - 1.0);
    }
    let swimming = super::inherited::point(&game.world, e.pos, "oWaterSwim");
    if swimming && e.value("swimming") == 0.0 {
        game.spawn("oSplash", e.pos);
        game.sound("sndSplash", e.pos);
    }
    e.data.insert("swimming".into(), f64::from(swimming));
    e.data
        .insert("gravity".into(), if swimming { 0.2 } else { 0.6 });
    damsel_death_anger(game, e);
    if e.value("invincible") == 0.0 && !e.held && game.world.solid_at(e.pos.x, e.pos.y) {
        if e.health > 0 {
            super::record_damsel_death(game, e, true);
        }
        create_blood(game, e, e.pos, 3);
        game.sound("sndDamsel", e.pos);
        e.alive = false;
        // Source instance_destroy preserves receiver HP and does not stop
        // the following burning, lava, and spike statements.
    }
    if e.value("invincible") == 0.0 {
        if e.value("burning") > 0.0 {
            if game.rng.chance(1, 5) {
                let pos = Vec2::new(
                    e.pos.x + f64::from(4 + game.rng.range(9)),
                    e.pos.y + f64::from(4 + game.rng.range(9)),
                );
                game.spawn("oBurn", pos);
            }
            e.data.insert("burning".into(), e.value("burning") - 1.0);
        }
        if super::inherited::lava_point_bbox(&game.world, Vec2::new(e.pos.x, e.pos.y + 6.0)) {
            if e.health > 0 {
                super::record_damsel_death(game, e, false);
                e.health = 0;
            }
            e.state = 99;
            e.data.insert("dead".into(), 1.0);
            e.data.insert("death_recorded".into(), 1.0);
            e.vel = Vec2::new(0.0, 0.1);
            e.data.insert("gravity".into(), 0.0);
            e.data.insert("burning".into(), 1.0);
            e.data.insert("depth".into(), 999.0);
        }
        if e.vel.y > 2.0
            && super::inherited::point(&game.world, Vec2::new(e.pos.x, e.pos.y + 6.0), "oSpikes")
        {
            super::inherited::stain_spike_at(game, e, Vec2::new(e.pos.x, e.pos.y + 6.0));
            if e.health > 0 {
                create_blood(game, e, e.pos, 3);
                super::record_damsel_death(game, e, false);
                e.health = 0;
            }
            e.state = 99;
            e.data.insert("dead".into(), 1.0);
            e.data.insert("death_recorded".into(), 1.0);
            e.set_sprite(if game.flags.contains("character_damsel") {
                "sDieL"
            } else {
                "sDamselDieL"
            });
            e.vel = Vec2::new(0.0, 0.2);
            e.data.insert("gravity".into(), 0.0);
        }
        crate::dispatcher::register_instances(game);
        let spears: Vec<_> = crate::dispatcher::descendant_instances(game, "oSpearsLeft")
            .into_iter()
            .filter_map(|instance| match instance {
                crate::dispatcher::Instance::Entity(id) => game.world.entity(id),
                _ => None,
            })
            .filter(|s| s.engine_active)
            .collect();
        let mut spear = None;
        if game.world.rectangle_family_except(
            Vec2::new(e.pos.x - 3.0, e.pos.y - 3.0),
            Vec2::new(e.pos.x + 3.0, e.pos.y + 3.0),
            "oSpearsLeft",
            None,
            CollisionPrecision::BoundingBox,
        ) {
            // Source rectangle contact gates a separate instance_nearest
            // query over the entire active family. Equal distances retain
            // the first instance in the current array.
            let mut nearest_distance = f64::INFINITY;
            for candidate in spears {
                let distance = candidate.pos.distance(e.pos);
                if distance < nearest_distance {
                    nearest_distance = distance;
                    spear = Some((candidate.pos, candidate.animation));
                }
            }
        }
        if let Some((pos, _)) =
            spear.filter(|(_, frame)| *frame >= 19.0 && *frame < 28.0 && (e.state != 2 || grounded))
        {
            e.health -= 2;
            e.state = 2;
            e.timer = 120;
            e.vel = Vec2::new(if pos.x < e.pos.x { 4.0 } else { -4.0 }, -6.0);
            if e.held {
                e.held = false;
                game.player.held = game.player.held.filter(|id| *id != e.id);
                game.player.pickup_item_type.clear();
            }
            e.set_image_speed(0.5);
            game.sound("sndHit", e.pos);
            game.spawn("oBlood", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        }
    }
    if !e.held && e.vel.y > 2.0 && e.state != 2 {
        e.state = 2;
        game.set_sound_pan("sndDamsel", 0.0);
        game.sound("sndDamsel", e.pos);
    }
    if e.held {
        e.facing = game.player.facing;
        crate::traps::altar::offering_entity(game, e);
        return;
    }
    match e.state {
        0 => {
            e.vel.x = 0.0;
            e.set_sprite(if game.flags.contains("character_damsel") {
                "sStandLeft"
            } else {
                "sDamselLeft"
            });
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 3;
                e.set_sprite(if game.flags.contains("character_damsel") {
                    "sYellLeft"
                } else {
                    "sDamselYellL"
                });
                let pan = if game.player.pos.x < e.pos.x {
                    -1.0
                } else if game.player.pos.x > e.pos.x {
                    1.0
                } else {
                    0.0
                };
                game.set_sound_pan("sndDamsel", pan);
                game.sound("sndDamsel", e.pos);
            }
        }
        1 => {
            e.set_image_speed(0.8);
            e.set_sprite(if game.flags.contains("character_damsel") {
                "sRunLeft"
            } else {
                "sDamselRunL"
            });
            // Source RUN uses two independent precise custom-edge checks.
            // Left omits Right's -1 term; round each complete endpoint.
            let body = e.bounds();
            let top = body.y.round_ties_even();
            let bottom = (body.y + body.h - 1.0).round_ties_even();
            let left = (body.x - 2.0).round_ties_even();
            if e.facing < 0
                && game
                    .world
                    .solid_line_except(Vec2::new(left, top), Vec2::new(left, bottom), e.id)
            {
                e.facing = 1;
            }
            let right = (body.x + body.w + 2.0 - 1.0).round_ties_even();
            if e.facing > 0
                && game.world.solid_line_except(
                    Vec2::new(right, top),
                    Vec2::new(right, bottom),
                    e.id,
                )
            {
                e.facing = -1;
            }
            e.vel.x = f64::from(e.facing) * 1.5;
        }
        2 => {
            if e.value("startled") == 0.0 {
                game.lifetime_stats
                    .increment(LifetimeCounter::DamselsGrabbed);
                e.data.insert("startled".into(), 1.0);
            }
            let male = game.flags.contains("character_damsel");
            e.set_sprite(if e.vel.x == 0.0 {
                if male { "sStunL" } else { "sDamselStunL" }
            } else if e.value("bounced") != 0.0 {
                if e.vel.y < 0.0 {
                    if male {
                        "sDieLBounce"
                    } else {
                        "sDamselBounceL"
                    }
                } else if male {
                    "sDieLFall"
                } else {
                    "sDamselFallL"
                }
            } else if e.vel.x < 0.0 {
                if male { "sDieLL" } else { "sDamselDieLL" }
            } else if male {
                "sDieLR"
            } else {
                "sDamselDieLR"
            });
            if grounded && e.value("bounced") == 0.0 {
                e.data.insert("bounced".into(), 1.0);
            }
            let b = e.bounds();
            let supported =
                super::solid_mask(&game.world, Rect::new(b.x, b.y + b.h + 1.0, b.w, 1.0), e.id)
                    || game.world.rectangle_family_except(
                        Vec2::new(e.pos.x - 4.0, e.pos.y - 6.0),
                        Vec2::new(e.pos.x + 4.0, e.pos.y + 8.0),
                        "oWeb",
                        None,
                        CollisionPrecision::BoundingBox,
                    );
            if grounded || supported {
                if e.value("dead") == 0.0 {
                    if e.timer > 0 {
                        e.timer -= 1;
                    } else {
                        e.state = 1;
                    }
                }
                if e.health <= 0 {
                    e.state = 99;
                    e.set_sprite(if game.flags.contains("character_damsel") {
                        "sDieL"
                    } else {
                        "sDamselDieL"
                    });
                    super::record_damsel_death(game, e, true);
                }
            }
        }
        3 => {
            if e.animation == 4.0 {
                game.spawn("oYellHelp", Vec2::new(e.pos.x, e.pos.y - 16.0));
            }
        }
        5 => {
            e.facing = if game.player.pos.x < e.pos.x { -1 } else { 1 };
            e.vel.x = 0.0;
            e.set_sprite(if game.flags.contains("character_damsel") {
                "sStandLeft"
            } else {
                "sDamselLeft"
            });
        }
        6 => {
            if e.animation == 7.0 {
                game.spawn(
                    "oHeart",
                    Vec2::new(
                        e.pos.x + if e.facing < 0 { -8.0 } else { 8.0 },
                        e.pos.y - 8.0,
                    ),
                );
                game.sound("sndKiss", e.pos);
            }
        }
        _ => {}
    }
    crate::traps::altar::offering_entity(game, e);
}

fn shopkeeper(game: &mut Game, e: &mut Entity) {
    let mut impact_y = (e.vel.y
        + if e.held {
            0.0
        } else {
            super::inherited::gravity(e, 0.6)
        })
    .min(8.0);
    let (wall, grounded) = move_body(&game.world, e, if e.held { 0.0 } else { 0.6 });
    e.vel.y = e.vel.y.min(8.0);
    let contacts = super::body_contacts(&game.world, e);
    // The source moveTo resolves displacement without clearing stored yVel.
    e.vel.y = impact_y;
    if contacts.bottom && e.state != 98 {
        e.vel.y = 0.0;
        impact_y = 0.0;
    }
    if (e.state >= 98 || !e.held)
        && game.world.solid_at(
            e.pos.x + 8.0,
            e.pos.y + if e.state >= 98 { 12.0 } else { 8.0 },
        )
    {
        super::record_keeper_crush(game, e);
        create_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 3);
        game.sound("sndCavemanDie", e.pos);
        // Only the ordinary crush branch drops the gun before destruction.
        if e.state < 98 && e.value("has_gun") > 0.0 {
            keeper_drop_gun(game, e);
        }
        e.alive = false;
        // Own crush preserves hp/dead and continues this Step after Destroy.
        super::destroyed(game, e);
    }
    if e.health <= 0 && e.state < 98 {
        e.state = 99;
    }
    let angry = e.value("angry") > 0.0
        || game.flags.contains("wanted")
        || game.flags.contains("shopkeepers_angry")
        || game.flags.contains("murderer")
        || game.flags.contains("thief")
        || game.data.get("thiefLevel").copied().unwrap_or(0.0) > 0.0
        || game.data.get("thief_level").copied().unwrap_or(0.0) > 0.0;
    let distance = keeper_player_distance(game, e);
    if matches!(e.state, 1 | 4)
        && ((super::player_alive(game)
            && distance < 64.0
            && game.player.pos.y - (e.pos.y + 8.0) < 16.0)
            || (game.player.pos.x - (e.pos.x + 8.0)).abs() < 4.0)
    {
        e.state = 2;
    }
    super::shop_dialogue::tick(game, e, !angry);
    if e.state >= 98 {
        if e.state == 99 {
            let first_death = e.value("dead") == 0.0;
            super::record_death(game, e);
            if first_death {
                if !in_stars(game) {
                    for _ in 0..1 + game.rng.range(4) {
                        let id = game.spawn("oGoldNugget", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
                        let vx =
                            f64::from(1 + game.rng.range(3)) - f64::from(1 + game.rng.range(3));
                        game.world.entity_mut(id).unwrap().vel = Vec2::new(vx, -1.0);
                    }
                }
                game.sound("sndCavemanDie", e.pos);
            }
            e.set_sprite("sShopDieL");
            if e.vel.x != 0.0 || e.vel.y != 0.0 {
                e.state = 98;
            }
        } else {
            e.set_sprite(if grounded {
                "sShopStunL"
            } else if e.value("bounced") != 0.0 {
                if e.vel.y < 0.0 {
                    "sShopBounceL"
                } else {
                    "sShopFallL"
                }
            } else if e.vel.x < 0.0 {
                "sShopDieLL"
            } else {
                "sShopDieLR"
            });
            if grounded && e.value("bounced") == 0.0 {
                e.data.insert("bounced".into(), 1.0);
                create_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 3);
            }
            if e.held || grounded {
                if e.timer > 0 {
                    e.timer -= 1;
                } else if e.health > 0 {
                    e.state = 2;
                    if e.held {
                        e.held = false;
                        game.player.held = game.player.held.filter(|id| *id != e.id);
                        game.player.pickup_item_type.clear();
                    }
                }
            }
        }
        // Source rechecks status after STUNNED recovery before the late tail.
        if e.state >= 98 {
            if e.value("has_gun") > 0.0 {
                keeper_drop_gun(game, e);
            }
            super::settle_body(e, contacts, impact_y);
            if e.vel.x == 0.0 && e.vel.y == 0.0 && e.health <= 0 {
                e.state = 99;
            }
        }
        keeper_friction(e);
        if e.state < 98 && e.state != 3 {
            e.set_sprite(if e.vel.x.abs() > 0.0 {
                "sShopRunLeft"
            } else {
                "sShopLeft"
            });
        }
        if e.held {
            e.set_sprite(if e.health > 0 {
                "sShopHeldL"
            } else {
                "sShopDHeldL"
            });
        }
        return;
    }
    if e.state == 0 {
        e.data.insert("bounced".into(), 0.0);
        if contacts.left {
            e.pos.x += 1.0;
        }
        if contacts.right {
            e.pos.x -= 1.0;
        }
        if contacts.left && contacts.right {
            e.state = 2;
        }
        if contacts.top && e.vel.y < 0.0 {
            e.vel.y = 0.0;
        }
        e.facing = if game.player.pos.x < e.pos.x + 8.0 {
            -1
        } else {
            1
        };
        e.set_sprite("sShopLeft");
        if angry {
            e.state = 4;
        } else {
            crate::economy::keeper_step(game, e);
            if super::shop_dialogue::can_follow(game, e) {
                e.state = 5;
            }
        }
    } else if e.state == 5 {
        e.data.insert("animation_speed".into(), 0.5);
        let d = distance;
        if contacts.left || contacts.right {
            e.facing *= -1;
        }
        if e.value("turn_timer") > 0.0 {
            e.data
                .insert("turn_timer".into(), e.value("turn_timer") - 1.0);
        } else if (game.player.pos.y - (e.pos.y + 8.0)).abs() < 8.0 && grounded && d > 16.0 {
            e.facing = if game.player.pos.x < e.pos.x { -1 } else { 1 };
            e.data.insert("turn_timer".into(), 10.0);
        }
        e.vel.x = if d < 12.0 || game.player.pos.y < e.pos.y {
            0.0
        } else {
            f64::from(e.facing) * (d / 16.0 * 1.5).min(3.0)
        };
        if !game
            .player
            .held
            .and_then(|id| game.world.entity(id))
            .is_some_and(|x| x.value("cost") > 0.0)
        {
            e.state = 0;
        }
    } else if e.state == 4 {
        e.data.insert("bounced".into(), 0.0);
        if contacts.top && e.vel.y < 0.0 {
            e.vel.y = 0.0;
        }
        if grounded && e.timer > 0 {
            e.timer -= 1;
        }
        if e.timer == 0 {
            e.state = 1;
            e.facing = if game.rng.range(2) == 0 { -1 } else { 1 };
        }
    } else if e.state == 1 {
        e.data.insert("animation_speed".into(), 0.5);
        if contacts.left || contacts.right {
            e.facing *= -1;
        }
        let edge = Vec2::new(e.pos.x + if e.facing < 0 { -1.0 } else { 16.0 }, e.pos.y);
        // Native collision_point flags test >=0.5; source -1 means bbox/notme=false.
        if !keeper_solid_point(game, edge, CollisionPrecision::BoundingBox) {
            e.state = 4;
            e.timer = 20 + game.rng.range(31);
            e.vel.x = 0.0;
        }
        // Source overwrites the edge branch's zero before its independent pause roll.
        e.vel.x = f64::from(e.facing) * 1.5;
        if game.rng.range(100) == 0 {
            e.state = 4;
            e.timer = 20 + game.rng.range(31);
            e.vel.x = 0.0;
        }
    } else if e.state == 3 {
    } else if e.state == 2 {
        e.data.insert("animation_speed".into(), 1.0);
        crate::economy::keeper_attack_step(game, e);
        if e.value("turn_timer") > 0.0 {
            e.data
                .insert("turn_timer".into(), e.value("turn_timer") - 1.0);
        } else if grounded && (game.player.pos.y - center(e).y).abs() < 8.0 && distance > 16.0 {
            e.facing = if game.player.pos.x < center(e).x {
                -1
            } else {
                1
            };
            e.data.insert("turn_timer".into(), 20.0);
        }
        if wall {
            e.facing *= -1;
        }
        e.vel.x = f64::from(e.facing) * 3.0;
        let cooldown = e.value("firing");
        if e.value("has_gun") > 0.0 && cooldown > 0.0 {
            e.data.insert("firing".into(), cooldown - 1.0);
        } else if e.value("has_gun") > 0.0
            && (game.player.pos.y - center(e).y).abs() < 32.0
            && distance < 96.0
            && (game.player.pos.x - center(e).x) * f64::from(e.facing) > 0.0
        {
            game.spawn(
                if e.facing < 0 {
                    "oShotgunBlastLeft"
                } else {
                    "oShotgunBlastRight"
                },
                Vec2::new(
                    e.pos.x + if e.facing < 0 { 0.0 } else { 16.0 },
                    e.pos.y + 9.0,
                ),
            );
            for _ in 0..6 {
                let id = game.spawn(
                    "oBullet",
                    Vec2::new(
                        e.pos.x + if e.facing < 0 { 4.0 } else { 12.0 },
                        e.pos.y + 8.0,
                    ),
                );
                let speed = f64::from(6 + game.rng.range(3));
                let spread = game.rng.unit() - game.rng.unit();
                if let Some(b) = game.world.entity_mut(id) {
                    b.vel = Vec2::new(f64::from(e.facing) * (speed + 3.0), spread);
                    b.data.insert("safe".into(), 1.0);
                    b.data.insert("owner_player".into(), 0.0);
                    b.data.insert("damage".into(), 4.0);
                }
                let origin = game.world.entity(id).unwrap().pos;
                if keeper_solid_point(game, origin, CollisionPrecision::BoundingBox) {
                    // Bullet has no Destroy event or parent. This source
                    // instance_destroy is not its later Solid collision callback.
                    game.world.entity_mut(id).unwrap().alive = false;
                }
            }
            e.vel.y -= 1.0;
            e.vel.x -= f64::from(e.facing) * 3.0;
            e.data.insert("firing".into(), 30.0);
            game.sound("sndShotgun", e.pos);
        }
        // Source first suppresses both jump branches for a close lower player.
        if !(game.player.pos.y > e.pos.y && (game.player.pos.x - (e.pos.x + 8.0)).abs() < 64.0) {
            let far_x = e.pos.x + if e.facing < 0 { -16.0 } else { 32.0 };
            let far_wall = keeper_solid_point(
                game,
                Vec2::new(far_x, e.pos.y),
                CollisionPrecision::BoundingBox,
            );
            let missing_ledge = !far_wall
                && game.player.pos.y <= e.pos.y + 16.0
                && !keeper_solid_point(
                    game,
                    Vec2::new(far_x, e.pos.y + 16.0),
                    CollisionPrecision::BoundingBox,
                );
            if contacts.bottom && (far_wall || missing_ledge) {
                let b = e.bounds();
                let top_y = (b.y - 4.0).round_ties_even();
                let blocked_top = game.world.solid_line_except(
                    Vec2::new(b.x.round_ties_even(), top_y),
                    Vec2::new((b.x + b.w - 1.0).round_ties_even(), top_y),
                    e.id,
                );
                if !blocked_top {
                    e.vel.y = -f64::from(7 + game.rng.range(2));
                }
            }
        }
        // Source ATTACK tail uses colBot cached before shooting/jump selection.
        if !contacts.bottom && game.player.pos.y > e.pos.y + 8.0 {
            e.vel.x = f64::from(e.facing) * 1.5;
        }
        e.set_sprite(if e.state == 3 {
            "sShopThrowL"
        } else {
            "sShopRunLeft"
        });
        if !super::player_alive(game) {
            e.state = 1;
        }
    }
    keeper_friction(e);
    // Final own-Step art also supplies the following Collision phase's mask.
    if e.state < 98 && e.state != 3 {
        e.set_sprite(if e.vel.x.abs() > 0.0 {
            "sShopRunLeft"
        } else {
            "sShopLeft"
        });
    }
    if e.held {
        e.set_sprite(if e.health > 0 {
            "sShopHeldL"
        } else {
            "sShopDHeldL"
        });
    }
}

/// The native distance_to_object(oPlayer1) kernel reads cached inclusive
/// integer envelopes, independent of the selected Collision mask frame.
fn keeper_player_distance(game: &Game, e: &Entity) -> f64 {
    if !game.player.engine_active {
        return 1_000_000.0;
    }
    crate::world_events::mask_distance(
        e.collision_mask().point_bounds(),
        game.player.collision_mask().point_bounds(),
    )
    .min(1_000_000.0)
}

/// Literal keeper collision_point queries use active oSolid descendants,
/// including held ones, independently of the engine solid flag.
fn keeper_solid_point(game: &Game, point: Vec2, precision: CollisionPrecision) -> bool {
    game.world.width > 0
        && game.world.tiles.iter().enumerate().any(|(index, tile)| {
            tile.engine_active
                && crate::catalog::is_a(&tile.object, "oSolid")
                && game
                    .world
                    .tile_collision_mask(
                        (index % game.world.width) as i32,
                        (index / game.world.width) as i32,
                    )
                    .contains(point, precision)
        })
        || game.world.entities.iter().any(|solid| {
            solid.alive
                && solid.engine_active
                && crate::catalog::is_a(&solid.object, "oSolid")
                && solid.mask_contains(point, precision)
        })
}

fn keeper_friction(e: &mut Entity) {
    if e.vel.x > 0.0 {
        e.vel.x -= 0.1;
    }
    if e.vel.x < 0.0 {
        e.vel.x += 0.1;
    }
    if e.vel.x.abs() < 0.5 {
        e.vel.x = 0.0;
    }
}

fn keeper_drop_gun(game: &mut Game, e: &mut Entity) {
    let id = game.spawn("oShotgun", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
    let vy = f64::from(4 + game.rng.range(3));
    let vx = f64::from(4 + game.rng.range(3)) * if e.vel.x < 0.0 { -1.0 } else { 1.0 };
    let gun = game.world.entity_mut(id).unwrap();
    gun.vel = Vec2::new(vx, vy);
    gun.data.insert("cost".into(), 0.0);
    gun.data.insert("for_sale".into(), 0.0);
    e.data.insert("has_gun".into(), 0.0);
}

fn scarab(game: &mut Game, e: &mut Entity) {
    let velocity = e.vel;
    move_body(&game.world, e, 0.0);
    // Source moveTo changes position, retaining both velocity components.
    e.vel = velocity;
    if game.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0) {
        e.health = 0;
    }
    if e.health < 1 {
        // The Step death burst is separate from its three-spark Destroy event.
        for _ in 0..3 {
            let pos = Vec2::new(
                e.pos.x + 2.0 + f64::from(game.rng.range(15)),
                e.pos.y + 2.0 + f64::from(game.rng.range(15)),
            );
            let id = game.spawn("oFlareSpark", pos);
            let vy = f64::from(1 + game.rng.range(3));
            game.world.entity_mut(id).unwrap().vel.y = vy;
        }
        e.alive = false;
        destroyed(game, e);
        // instance_destroy executes Destroy synchronously, then resumes Step.
    }
    if e.state == 0 {
        e.vel.x = (e.vel.x.abs() - 0.5).max(0.0) * e.vel.x.signum();
        e.vel.y = (e.vel.y.abs() - 0.5).max(0.0) * e.vel.y.signum();
        if e.vel.x.abs() < 1.0 {
            e.vel.x = 0.0;
        }
        if e.vel.y.abs() < 1.0 {
            e.vel.y = 0.0;
        }
        if e.vel == Vec2::default() && e.timer > 0 {
            e.timer -= 1;
        }
        if e.timer == 0 && e.vel.x < 1.0 && e.vel.y < 1.0 {
            let origin = Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0);
            let angle = if origin.distance(game.player.pos) < 64.0 {
                // GameMaker point_direction is upward-positive; the source
                // adds 180 degrees to flee the character's origin.
                (origin.y - game.player.pos.y).atan2(game.player.pos.x - origin.x)
                    + std::f64::consts::PI
            } else {
                f64::from(game.rng.range(361)).to_radians()
            };
            e.vel = Vec2::new(angle.cos() * 4.0, -angle.sin() * 4.0);
            e.timer = 10 + game.rng.range(21);
        }
        // Source checks each edge after decay and any new launch, in this order.
        let b = e.bounds();
        if super::solid_mask(&game.world, Rect::new(b.x + b.w, b.y, 1.0, b.h), e.id)
            && e.vel.x > 0.0
        {
            e.vel.x = -e.vel.x;
        }
        if super::solid_mask(&game.world, Rect::new(b.x - 1.0, b.y, 1.0, b.h), e.id)
            && e.vel.x < 0.0
        {
            e.vel.x = -e.vel.x;
        }
        if super::solid_mask(&game.world, Rect::new(b.x, b.y - 1.0, b.w, 1.0), e.id)
            && e.vel.y < 0.0
        {
            e.vel.y = -e.vel.y;
        }
        if super::solid_mask(&game.world, Rect::new(b.x, b.y + b.h, b.w, 1.0), e.id)
            && e.vel.y > 0.0
        {
            e.vel.y = -e.vel.y;
        }
    }
}

// oDeadFish uses collision_point(..., oWater, 0, 0), including Lava.
fn dead_fish_water(game: &Game, pos: Vec2) -> bool {
    game.world.tiles_point(pos, "oWater")
        || game.world.entities.iter().any(|water| {
            water.alive
                && water.engine_active
                && crate::catalog::is_a(&water.object, "oWater")
                && water.mask_contains(pos, CollisionPrecision::BoundingBox)
        })
}

fn dead_fish(game: &mut Game, e: &mut Entity) {
    if e.health <= 0 {
        for _ in 0..3 {
            crate::world_events::spawn_detritus(
                game,
                "oBone",
                Vec2::new(e.pos.x + 4.0, e.pos.y + 4.0),
            );
        }
        super::record_death(game, e);
        e.alive = false;
        // Own instance_destroy finishes Enemy Destroy before this Step resumes.
        super::destroyed(game, e);
    }
    let swimming = game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0;
    let dist = e.pos.distance(game.player.pos);
    let mut direction = e.value("dir");
    let mut movement = None;
    if e.state == 0 {
        let right = direction == 0.0;
        let probe = Vec2::new(e.pos.x + if right { 10.0 } else { -2.0 }, e.pos.y);
        if dead_fish_water(game, probe) && !game.world.solid_at(probe.x, probe.y) {
            movement = Some(Vec2::new(if right { 1.0 } else { -1.0 }, 0.0));
        } else {
            direction = if right { 180.0 } else { 0.0 };
        }
        if dist < 90.0 && swimming {
            e.state = 1;
        }
    } else if e.state == 2 {
        if e.timer > 0 {
            e.timer -= 1;
        } else {
            e.state = 0;
            direction = f64::from(game.rng.range(2)) * 180.0;
        }
    } else if crate::dispatcher::is_active(game, crate::dispatcher::Instance::Player) {
        if dist < 90.0 && swimming && super::player_alive(game) {
            // Source point_direction starts at x/y, then consumes both jitter
            // draws in order. The resulting degrees are not normalized again.
            direction = (e.pos.y - game.player.pos.y)
                .atan2(game.player.pos.x - e.pos.x)
                .to_degrees()
                .rem_euclid(360.0)
                + f64::from(game.rng.range(2))
                - f64::from(game.rng.range(2));
            let angle = direction.to_radians();
            let velocity = Vec2::new(angle.cos(), -angle.sin());
            if dead_fish_water(game, Vec2::new(e.pos.x + velocity.x, e.pos.y + velocity.y)) {
                movement = Some(velocity);
            }
        } else {
            e.state = 2;
            e.timer = 20 + game.rng.range(21);
        }
    }
    if let Some(movement) = movement {
        // moveTo takes arguments; it does not overwrite this actor's xVel/yVel.
        let velocity = e.vel;
        e.vel = movement;
        move_body(&game.world, e, 0.0);
        e.vel = velocity;
    }
    e.data.insert("dir".into(), direction);
    let left = direction > 90.0 && direction < 270.0;
    e.facing = if left { -1 } else { 1 };
    e.set_sprite(if left {
        "sDeadFishLeft"
    } else {
        "sDeadFishRight"
    });
    // Source checks for dry land after the independent AI and sprite tail.
    if !dead_fish_water(game, Vec2::new(e.pos.x + 4.0, e.pos.y + 4.0)) {
        game.spawn("oFishBone", e.pos);
        e.alive = false;
        super::destroyed(game, e);
    }
}

fn barrier(game: &mut Game, e: &mut Entity) {
    if e.object == "oBarrierEmitter" && !game.world.solid_at(e.pos.x, e.pos.y - 16.0) {
        destroy_emitter(game, e);
    }
}

fn keeper_melee(game: &mut Game, e: &mut Entity, weapon: &Entity) {
    if e.value("whipped") != 0.0 {
        return;
    }
    let machete = weapon.text.get("type").map_or_else(
        || {
            crate::catalog::is_a(&weapon.object, "oSlash")
                || crate::catalog::is_a(&weapon.object, "oMachetePre")
        },
        |kind| kind == "Machete",
    );
    if machete {
        let damage = weapon.data.get("damage").copied().unwrap_or_else(|| {
            if crate::catalog::is_a(&weapon.object, "oSlash")
                || crate::catalog::is_a(&weapon.object, "oMachetePre")
                || crate::catalog::is_a(&weapon.object, "oMattockHit")
                || crate::catalog::is_a(&weapon.object, "oMattockPre")
            {
                2.0
            } else {
                1.0
            }
        });
        e.health -= damage as i32;
        if e.value("blood_left") > 0.0 {
            super::create_blood(game, e, super::sprite_blood_center(e), 1);
            if e.health < 0 {
                e.data
                    .insert("blood_left".into(), e.value("blood_left") - 1.0);
            }
        }
    }
    e.vel.y = -2.0;
    e.vel.x = if weapon.pos.x < e.pos.x { 1.0 } else { -1.0 };
    game.sound("sndHit", e.pos);
    e.data.insert("whipped".into(), 1.0);
    e.data.insert("hit_cooldown".into(), 10.0);
    e.alarms[0] = 10;
    e.state = 2;
}

fn keeper_player_collision(game: &mut Game, e: &mut Entity) {
    if matches!(e.state, 0 | 5 | 98 | 99)
        || e.health < 1
        || !super::player_alive(game)
        || if game.generation_revision == GenerationRevision::SourceClassicV2 {
            game.player.is_stunned()
        } else {
            game.player.stun > 0 || game.player.state == PlayerState::Stunned
        }
        || (game.player.pos.x - (e.pos.x + 8.0)).abs() > 8.0
    {
        return;
    }
    if crate::player::source_airborne(game)
        && game.player.pos.y < e.pos.y + 5.0
        && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
    {
        if e.state < 98 {
            game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
            let damage =
                (game.player.data.get("fall_timer").copied().unwrap_or(0.0) / 16.0).ceil() as i32;
            if game.player.has("SpikeShoes") {
                e.health -= 3 * damage;
                // This own event bypasses scrCreateBlood's 16-detritus gate.
                game.spawn(
                    "oBlood",
                    Vec2::new(game.player.pos.x, game.player.pos.y + 8.0),
                );
            } else {
                e.health -= damage;
            }
            game.player.data.insert("fall_timer".into(), 0.0);
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
        return;
    }
    if game.player.invincible != 0 || e.state >= 98 {
        return;
    }
    if game.world.solid_at(e.pos.x + 8.0, e.pos.y - 4.0) {
        game.player.invincible = 30;
        game.player.data.insert("blink".into(), 30.0);
        game.player.vel.x = if game.player.pos.x < e.pos.x {
            -6.0
        } else {
            6.0
        };
        crate::world_events::spawn_detritus(game, "oBlood", game.player.pos);
        if game.player.health > 0 {
            game.player.lose_health(1);
            if game.player.health <= 0 && crate::statistics::is_real_level(game) {
                game.lifetime_stats
                    .enemy_death(crate::statistics::EnemyDeath::Shopkeeper);
            }
        }
        game.events.push(GameEvent::Damage { amount: 1 });
        game.sound("sndHurt", game.player.pos);
    } else if e.state != 3 {
        e.state = 3;
        e.set_sprite("sShopThrowL");
        e.facing = if game.player.pos.x > e.pos.x + 8.0 {
            1
        } else {
            -1
        };
        e.vel.x = 0.0;
        game.player.pos = Vec2::new(e.pos.x + if e.facing > 0 { 0.0 } else { 16.0 }, e.pos.y);
        game.player.previous_pos = game.player.pos;
        game.player.vel = Vec2::new(f64::from(e.facing) * 6.0, -6.0);
        crate::player::enter_stunned(game);
        game.player.data.insert("bounced".into(), 0.0);
        game.player.data.insert("wall_hurt".into(), 1.0);
        super::drop_from_throw(game);
    }
}

fn barrier_player_collision(game: &mut Game, e: &Entity) {
    if game.player.invincible != 0 {
        return;
    }
    if game.player.health > 0 {
        game.player.lose_health(1);
    }
    game.events.push(GameEvent::Damage { amount: 1 });
    game.sound("sndHurt", game.player.pos);
    game.player.invincible = 30;
    game.player.data.insert("blink".into(), 30.0);
    game.player.vel = Vec2::new(
        if game.player.pos.x < e.pos.x {
            -6.0
        } else {
            6.0
        },
        -2.0,
    );
}

fn barrier_item_collision(game: &mut Game, id: u64) {
    let Some(item) = game.world.entity(id) else {
        return;
    };
    if item.object == "oBullet" {
        let pos = item.pos;
        game.spawn("oSmokePuff", pos);
        game.sound("sndHit", pos);
        game.world.entity_mut(id).unwrap().alive = false;
        return;
    }
    if item.object == "oWebBall" {
        game.world.entity_mut(id).unwrap().alive = false;
        return;
    }
    if !crate::catalog::is_a(&item.object, "oItem") {
        return;
    }
    // Source arms Bomb before drawing its horizontal reflection speed.
    if matches!(item.object.as_str(), "oBomb" | "oFireFrogBomb") {
        let alarm = 4 + game.rng.range(5);
        let item = game.world.entity_mut(id).unwrap();
        item.set_sprite("sBombArmed");
        item.data.insert("animation_speed".into(), 1.0);
        item.timer = alarm;
        item.alarms[1] = alarm as i32;
        item.data.insert("armed".into(), 1.0);
    }
    let vx = -f64::from(4 + game.rng.range(3));
    let item = game.world.entity_mut(id).unwrap();
    item.vel = Vec2::new(vx, -2.0);
    if item.held {
        // oItem Step releases the held flag after observing holdItem == 0.
        game.player.held = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeper_own_melee_owner_matrix_includes_inherited_keeper_and_all_stock_blades() {
        for keeper in ["oShopkeeper", "oShopkeeper2"] {
            for weapon in [
                "oWhip",
                "oWhipPre",
                "oSlash",
                "oMachetePre",
                "oMattockHit",
                "oMattockPre",
            ] {
                assert_eq!(
                    crate::dispatcher::collision_owner(keeper, weapon),
                    Some("oShopkeeper")
                );
                assert_eq!(crate::dispatcher::collision_owner(weapon, keeper), None);
            }
        }
    }

    #[test]
    fn generated_scarab_create_consumes_its_draw_before_caller_and_not_at_bootstrap() {
        let mut game = actual_room();
        game.world.area = Area::Temple;
        game.world.level = 13;
        let mut expected = game.rng.clone();
        let expected_timer = 10 + expected.range(21);
        let expected_caller_draw = expected.range(97);
        let id = crate::creation::spawn_generated_entity(
            &mut game.world,
            &mut game.rng,
            "oScarab",
            Vec2::new(370.0, 128.0),
        );
        let scarab = game.world.entity(id).unwrap();
        assert_eq!(scarab.timer, expected_timer);
        assert_eq!(scarab.value("value"), 12000.0);
        assert_eq!(scarab.value("npc_local_created"), 1.0);
        assert_eq!(scarab.value("npc_created"), 0.0);
        assert_eq!(game.rng.range(97), expected_caller_draw);

        game.step(&InputFrame::default());

        let scarab = game.world.entity(id).unwrap();
        assert_eq!(scarab.value("npc_created"), 1.0);
        assert_eq!(scarab.timer, expected_timer, "outside its narrow AI view");
        assert_eq!(game.rng.range(10000), expected.range(10000));
    }

    #[test]
    fn generated_damsel_local_price_precedes_free_override_without_skipping_runtime_create() {
        let mut game = actual_room();
        game.world.level = 7;
        let mut expected = game.rng.clone();
        let id = crate::creation::spawn_generated_entity(
            &mut game.world,
            &mut game.rng,
            "oDamsel",
            Vec2::new(370.0, 128.0),
        );
        let damsel = game.world.entity_mut(id).unwrap();
        assert_eq!(damsel.value("cost"), 105000.0);
        assert_eq!(damsel.value("npc_local_created"), 1.0);
        assert_eq!(damsel.value("npc_created"), 0.0);
        damsel.data.insert("cost".into(), 0.0);
        damsel.data.insert("for_sale".into(), 0.0);

        game.step(&InputFrame::default());

        let damsel = game.world.entity(id).unwrap();
        assert_eq!(damsel.value("cost"), 0.0);
        assert_eq!(damsel.value("for_sale"), 0.0);
        assert_eq!(damsel.value("npc_created"), 1.0);
        assert_eq!(game.rng.range(10000), expected.range(10000));
    }

    #[test]
    fn generated_local_create_does_not_mark_or_skip_ghost_runtime_sound() {
        let mut game = actual_room();
        let id = crate::creation::spawn_generated_entity(
            &mut game.world,
            &mut game.rng,
            "oGhost",
            Vec2::new(370.0, 128.0),
        );
        assert_eq!(
            game.world.entity(id).unwrap().value("npc_local_created"),
            0.0
        );
        assert_eq!(game.world.entity(id).unwrap().value("npc_created"), 0.0);
        for expected_sound_count in [1, 0] {
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(id).unwrap().value("npc_created"), 1.0);
            assert_eq!(
                game.events
                    .iter()
                    .filter(|event| {
                        matches!(event, GameEvent::Sound { name, .. } if name == "sndGhost")
                    })
                    .count(),
                expected_sound_count
            );
        }
    }

    #[test]
    fn actual_step_damsel_spear_gate_selects_nearest_from_the_whole_family() {
        for (two_traps, damsel_x, reverse_storage, expected_x_velocity) in [
            (false, 122.0, false, 4.0),
            (true, 122.0, false, -4.0),
            (true, 120.0, false, 4.0),
            (true, 120.0, true, 4.0),
        ] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            for x in 0..game.world.width {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.player.pos = Vec2::new(32.0, 152.0);
            game.player.previous_pos = game.player.pos;
            game.player.grounded = true;
            game.world.set_tile(8, 9, "oSpearTrapTop");
            if two_traps {
                game.world.set_tile(9, 9, "oSpearTrapTop");
            }
            let id = game.spawn("oDamsel", Vec2::new(damsel_x, 152.0));
            for _ in 0..19 {
                game.step(&InputFrame::default());
                assert_eq!(game.world.entity(id).unwrap().health, 4);
            }
            let damsel = game.world.entity(id).unwrap();
            let area = Rect::new(damsel.pos.x - 3.0, damsel.pos.y - 3.0, 7.0, 7.0);
            let spears: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && crate::catalog::is_a(&e.object, "oSpearsLeft"))
                .collect();
            assert_eq!(spears.len(), if two_traps { 2 } else { 1 });
            assert!(spears.iter().all(|s| s.animation == 19.0));
            assert!(spears.iter().any(|s| s.sprite_bounds().intersects(area)));
            if two_traps && damsel_x == 122.0 {
                let nearest = spears
                    .iter()
                    .min_by(|a, b| {
                        a.pos
                            .distance(damsel.pos)
                            .total_cmp(&b.pos.distance(damsel.pos))
                    })
                    .unwrap();
                assert_eq!(nearest.pos, Vec2::new(128.0, 144.0));
                assert!(!nearest.sprite_bounds().intersects(area));
            } else if two_traps {
                assert_eq!(
                    spears[0].pos.distance(damsel.pos),
                    spears[1].pos.distance(damsel.pos)
                );
                let first = crate::dispatcher::descendant_instances(&game, "oSpearsLeft")
                    .into_iter()
                    .find_map(|instance| match instance {
                        crate::dispatcher::Instance::Entity(id) => game.world.entity(id),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(first.pos, Vec2::new(112.0, 144.0));
            }
            if reverse_storage {
                // Storage is independent of the source parent array. The
                // equal-distance result must retain its registered order.
                game.world.entities.reverse();
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .find(|e| e.object == "oSpearsLeft")
                        .unwrap()
                        .pos,
                    Vec2::new(128.0, 144.0)
                );
            }

            game.step(&InputFrame::default());

            let damsel = game.world.entity(id).unwrap();
            assert_eq!(damsel.health, 2);
            assert_eq!(damsel.state, 2);
            assert_eq!(damsel.vel.y, -6.0);
            assert!((damsel.vel.x - expected_x_velocity).abs() < 1e-9);
        }
    }

    #[test]
    fn damsel_draw_mutates_scale_and_only_positive_cost_icon_frame() {
        let mut g = actual_room();
        let mut e = Entity::new(99, "oDamsel", Vec2::new(64.0, 88.0));
        e.facing = 1;
        e.data.insert("cost".into(), 1.0);
        e.data.insert("cimg".into(), 8.0);
        draw(&mut g, &mut e);
        assert_eq!(e.value("image_xscale"), -1.0);
        assert_eq!(e.value("draw_cimg"), 8.0);
        assert_eq!(e.value("cimg"), 9.0);
        draw(&mut g, &mut e);
        assert_eq!(e.value("draw_cimg"), 9.0);
        assert_eq!(e.value("cimg"), 0.0);
        e.facing = -1;
        e.data.insert("cost".into(), 0.0);
        e.data.insert("cimg".into(), 4.0);
        draw(&mut g, &mut e);
        assert_eq!(e.value("image_xscale"), 1.0);
        assert_eq!(e.value("draw_cimg"), 9.0);
        assert_eq!(e.value("cimg"), 4.0);
    }

    #[test]
    fn actual_step_runtime_damsel_create_prices_before_caller_override() {
        let mut g = actual_room();
        g.world.level = 7;
        let id = g.spawn("oDamsel", Vec2::new(104.0, 88.0));
        assert_eq!(g.world.entity(id).unwrap().value("cost"), 105000.0);
        g.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("cost".into(), 0.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().value("cost"), 0.0);
    }

    #[test]
    fn actual_step_generated_damsel_preserves_free_producer_override_at_bootstrap() {
        // Select only the natural pre-Step producer: scrTreasureGen creates
        // an ordinary Damsel and then clears its sale flag and price. Do not
        // select on bootstrap state, cost, or any post-Step behavior.
        let (mut g, id) = (0..64)
            .find_map(|seed| {
                let game = Game::new(seed, RulesPreset::Classic);
                let id = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oDamsel" && e.value("for_sale") == 0.0)
                    .map(|e| e.id)?;
                Some((game, id))
            })
            .expect("canonical seeds 0..64 generate an ordinary free Damsel before Step");
        assert_eq!(g.world.entity(id).unwrap().data.get("cost"), Some(&0.0));
        assert_eq!(g.world.entity(id).unwrap().value("npc_created"), 0.0);
        g.step(&InputFrame::default());
        let damsel = g.world.entity(id).unwrap();
        assert_eq!(damsel.value("npc_created"), 1.0);
        assert_eq!(damsel.value("cost"), 0.0);
        assert_eq!(damsel.value("for_sale"), 0.0);
    }

    fn actual_room() -> Game {
        let mut g = Game::shell(123, RulesPreset::Classic);
        g.world = World::new(30, 20, Area::Mines, 1);
        g.player.pos = Vec2::new(240.0, 128.0);
        g.player.previous_pos = g.player.pos;
        g
    }

    #[test]
    fn actual_step_dead_fish_bullet_damage_preserves_source_bloodless_rng() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = actual_room();
            game.rules = rules;
            for y in 4..8 {
                for x in 4..8 {
                    game.world.set_tile(x, y, "oWaterSwim");
                }
            }
            let fish = game.spawn("oDeadFish", Vec2::new(80.0, 80.0));
            let bullet = game.spawn("oBullet", Vec2::new(84.0, 84.0));
            let mut expected_rng = game.rng.clone();

            game.step(&InputFrame::default());

            let target = game.world.entity(fish).unwrap();
            assert_eq!(target.health, -3, "real bullet collision subtracts four HP");
            assert!(game.world.entity(bullet).is_none());
            // oDeadFish Create sets bloodless=true. Its lethal bullet hit
            // consumes the bullet and bloodLeft, but the source blood script
            // creates no drop and consumes no random draws.
            assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
            assert_eq!(target.value("bloodless"), 1.0);
            assert_eq!(target.value("blood_left"), 3.0);
            assert_eq!(target.vel, Vec2::new(0.0, -4.0));
            assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
        }
    }

    #[test]
    fn actual_step_scarab_source_create_draw_precedes_caller_rng() {
        let mut g = actual_room();
        let mut expected = g.rng.clone();
        let counter = 10 + expected.range(21);
        let id = g.spawn("oScarab", Vec2::new(96.0, 96.0));
        assert_eq!(g.world.entity(id).unwrap().timer, counter);
        assert_eq!(g.world.entity(id).unwrap().value("value"), 4000.0);
        assert_eq!(g.rng.range(1000), expected.range(1000));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().timer, counter - 1);
    }

    #[test]
    fn actual_step_natural_ghost_contact_preserves_valid_animation_phase() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        g.player.pos = Vec2::new(104.0, 136.0);
        let id = g.spawn("oGhost", Vec2::new(64.0, 120.0));
        let mut previous = 0.0;
        for _ in 0..40 {
            g.step(&InputFrame::default());
            let ghost = g.world.entity(id).unwrap();
            if ghost.state == 0 {
                assert_eq!(ghost.sprite, "sGhostDisappear");
                assert!(previous > 0.0, "contact followed natural chase animation");
                assert!((ghost.animation - previous - 0.2).abs() < 1e-9);
                assert_eq!(g.player.health, -99);
                return;
            }
            previous = ghost.animation;
        }
        panic!("natural Ghost chase did not reach the player");
    }

    #[test]
    fn actual_step_damsel_pickup_walk_throw_uses_parent_item_attachment() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 6, "oBlock");
        }
        g.player.pos = Vec2::new(64.0, 88.0);
        let id = g.spawn("oDamsel", g.player.pos);
        assert_eq!(g.world.entity(id).unwrap().value("heavy"), 1.0);
        g.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, Some(id));
        assert_eq!(
            g.world.entity(id).unwrap().pos,
            Vec2::new(g.player.pos.x + 4.0, g.player.pos.y + 2.0)
        );
        assert_eq!(g.lifetime_stats.damsels_grabbed, 0);
        for _ in 0..3 {
            g.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            assert_eq!(
                g.world.entity(id).unwrap().pos,
                Vec2::new(g.player.pos.x + 4.0, g.player.pos.y)
            );
        }
        g.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, None);
        assert_eq!(g.world.entity(id).unwrap().state, 2);
        assert_eq!(g.lifetime_stats.damsels_grabbed, 1);
        g.step(&InputFrame::default());
        assert_eq!(g.lifetime_stats.damsels_grabbed, 1);
    }

    #[test]
    fn actual_step_damsel_natural_fall_counts_first_thrown_once() {
        let mut g = actual_room();
        let id = g.world.spawn("oDamsel", Vec2::new(104.0, 80.0));
        for _ in 0..12 {
            g.step(&InputFrame::default());
            if g.world.entity(id).is_some_and(|e| e.state == 2) {
                break;
            }
        }
        assert_eq!(g.world.entity(id).unwrap().state, 2);
        assert_eq!(g.world.entity(id).unwrap().timer, 197);
        assert_eq!(g.lifetime_stats.damsels_grabbed, 1);
        g.step(&InputFrame::default());
        assert_eq!(g.lifetime_stats.damsels_grabbed, 1);
    }

    #[test]
    fn actual_step_damsel_lava_counts_only_lifetime_death_once() {
        let mut g = actual_room();
        g.world.spawn("oLava", Vec2::new(96.0, 96.0));
        let id = g.world.spawn("oDamsel", Vec2::new(104.0, 90.0));
        for _ in 0..6 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.world.entity(id).unwrap().health, 0);
        assert_eq!(g.world.entity(id).unwrap().value("dead"), 1.0);
        assert_eq!(g.lifetime_stats.damsels_killed_total, 1);
        assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 0.0);
        assert_eq!(g.data.get("damselsKilled").copied().unwrap_or(0.0), 0.0);
    }

    #[test]
    fn actual_step_damsel_inherits_fractional_lava_sinking_before_child_hazards() {
        let mut g = actual_room();
        g.world.spawn("oLava", Vec2::new(96.0, 96.0));
        let id = g.spawn("oDamsel", Vec2::new(104.0, 96.0));
        g.step(&InputFrame::default());
        assert!((g.world.entity(id).unwrap().pos.y - 96.05).abs() < 1e-9);
        assert_eq!(g.world.entity(id).unwrap().health, 0);
        g.step(&InputFrame::default());
        assert!((g.world.entity(id).unwrap().pos.y - 96.1).abs() < 1e-9);
        assert_eq!(g.lifetime_stats.damsels_killed_total, 1);
        assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 0.0);
    }

    #[test]
    fn actual_step_submerged_damsel_runs_parent_destroy_and_child_lava_credit() {
        let mut g = actual_room();
        g.world.spawn("oLava", Vec2::new(96.0, 96.0));
        let id = g.spawn("oDamsel", Vec2::new(104.0, 104.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert_eq!(g.lifetime_stats.damsels_killed_total, 1);
        assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 0.0);
    }

    #[test]
    fn actual_step_damsel_natural_spike_death_latches_without_run_credit() {
        let mut g = actual_room();
        g.world.spawn("oSpikes", Vec2::new(96.0, 128.0));
        let id = g.world.spawn("oDamsel", Vec2::new(104.0, 80.0));
        for _ in 0..30 {
            g.step(&InputFrame::default());
            if g.world.entity(id).is_some_and(|e| e.health <= 0) {
                break;
            }
        }
        assert_eq!(g.world.entity(id).unwrap().health, 0);
        for _ in 0..4 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.lifetime_stats.damsels_killed_total, 1);
        assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 0.0);
        assert_eq!(g.data.get("damselsKilled").copied().unwrap_or(0.0), 0.0);
    }

    #[test]
    fn actual_step_damsel_crush_credits_lifetime_and_run() {
        let mut g = actual_room();
        g.world.set_tile(6, 6, "oBlock");
        let id = g.world.spawn("oDamsel", Vec2::new(104.0, 104.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert_eq!(g.lifetime_stats.damsels_killed_total, 1);
        assert_eq!(g.data["damselsKilled"], 1.0);
        assert_eq!(g.data["kills"], 1.0);
    }

    #[test]
    fn actual_step_keeper_stars_crush_uses_ordinary_crush_counters() {
        let mut g = actual_room();
        g.flags.insert("hub_stars".into());
        g.world.name = "rStars".into();
        g.world.set_tile(6, 6, "oBlock");
        let id = g.world.spawn("oShopkeeper", Vec2::new(96.0, 96.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert_eq!(g.data["shopkeepers"], 1.0);
        assert_eq!(g.data["kills"], 1.0);
        assert_eq!(g.data.get("stars_kills").copied().unwrap_or(0.0), 0.0);
        assert_eq!(g.lifetime_stats.enemy_kills[19], 0);
        assert!(g.flags.contains("murderer"));
    }

    #[test]
    fn actual_step_keeper_lava_death_keeps_source_kill_eligibility() {
        let mut g = actual_room();
        g.world.spawn("oLava", Vec2::new(96.0, 96.0));
        let id = g.world.spawn("oShopkeeper", Vec2::new(96.0, 82.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().health, 0);
        assert_eq!(g.lifetime_stats.enemy_kills[19], 1);
        assert_eq!(g.data["shopkeepers"], 1.0);
        assert_eq!(g.data["kills"], 1.0);
        assert!(g.flags.contains("murderer"));
        for _ in 0..5 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.lifetime_stats.enemy_kills[19], 1);
        assert_eq!(g.data["kills"], 1.0);
    }

    #[test]
    fn actual_step_keeper_ceiling_contact_records_lethal_attribution() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 6, "oBlock");
        }
        g.world.set_tile(6, 4, "oBlock");
        g.player.pos = Vec2::new(104.0, 88.0);
        g.player.health = 1;
        let id = g.world.spawn("oShopkeeper", Vec2::new(96.0, 80.0));
        g.world.entity_mut(id).unwrap().state = 2;
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 0);
        assert_eq!(g.lifetime_stats.enemy_deaths[19], 1);
        assert_eq!(g.player.stun, 0);
    }

    fn shoot_natural_target(object: &str, pos: Vec2) -> (Game, u64) {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 6, "oBlock");
        }
        g.player.pos = Vec2::new(64.0, 88.0);
        g.world.spawn("oShotgun", g.player.pos);
        let id = g.world.spawn(object, pos);
        g.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(g.player.held.is_some(), "natural shotgun pickup");
        g.step(&InputFrame::default());
        g.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        for _ in 0..20 {
            if g.world.entity(id).is_some_and(|e| e.health <= 0) {
                return (g, id);
            }
            g.step(&InputFrame::default());
        }
        panic!("natural shotgun failed to kill {object}");
    }

    #[test]
    fn actual_step_keeper_natural_shotgun_death_waits_for_corpse_settlement() {
        let (mut g, id) = shoot_natural_target("oShopkeeper", Vec2::new(80.0, 80.0));
        assert_eq!(g.world.entity(id).unwrap().state, 98);
        assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 0.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 98);
        assert_eq!(g.lifetime_stats.enemy_kills[19], 0);
        assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 0.0);
        for _ in 0..180 {
            g.step(&InputFrame::default());
            if g.data.get("kills").copied().unwrap_or(0.0) > 0.0 {
                break;
            }
        }
        assert_eq!(g.data["shopkeepers"], 1.0);
        assert_eq!(g.data["kills"], 1.0);
        assert_eq!(g.lifetime_stats.enemy_kills[19], 1);
        for _ in 0..5 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.data["kills"], 1.0);
    }

    #[test]
    fn actual_step_damsel_natural_shotgun_death_waits_for_support() {
        let (mut g, id) = shoot_natural_target("oDamsel", Vec2::new(112.0, 88.0));
        assert_eq!(g.world.entity(id).unwrap().state, 2);
        assert_eq!(g.lifetime_stats.damsels_killed_total, 0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 2);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sDamselDieLR");
        assert_eq!(g.lifetime_stats.damsels_killed_total, 0);
        for _ in 0..180 {
            g.step(&InputFrame::default());
            if g.lifetime_stats.damsels_killed_total > 0 {
                break;
            }
        }
        assert_eq!(g.lifetime_stats.damsels_killed_total, 1);
        assert_eq!(g.data["damselsKilled"], 1.0);
        assert_eq!(g.data["kills"], 1.0);
        for _ in 0..5 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.lifetime_stats.damsels_killed_total, 1);
    }

    #[test]
    fn actual_step_barrier_hit_does_not_stun() {
        let mut g = actual_room();
        g.player.pos = Vec2::new(104.0, 104.0);
        g.world.set_tile(6, 4, "oBlock");
        let emitter = g.world.spawn("oBarrierEmitter", Vec2::new(96.0, 80.0));
        g.world
            .entity_mut(emitter)
            .unwrap()
            .data
            .insert("barrier_created".into(), 1.0);
        g.world.spawn("oBarrier", Vec2::new(96.0, 96.0));
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 3);
        assert_eq!(g.player.stun, 0);
        assert_eq!(g.player.invincible, 30);
        assert_ne!(g.player.state, PlayerState::Stunned);
    }
    #[test]
    fn actual_step_ghost_visible_edge_hits_player() {
        let mut g = actual_room();
        g.player.pos = Vec2::new(101.0, 96.0);
        g.world.spawn("oGhost", Vec2::new(80.0, 80.0));
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, -99);
        assert_eq!(g.state, RunState::Playing);
        assert_eq!(g.lifetime_stats.enemy_deaths[23], 1);
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Dead);
    }
    fn source_bone_create(rng: &mut Rng) -> (Vec2, f64) {
        // oBone/Create.xml: xVel=random(4)-random(4),
        // yVel=-1-random(2), grav=rand(1,6)*.1.
        let velocity = Vec2::new(rng.unit() * 4.0 - rng.unit() * 4.0, -1.0 - rng.unit() * 2.0);
        (velocity, f64::from(1 + rng.range(6)) * 0.1)
    }
    #[test]
    fn actual_step_ghost_bone_creates_precede_skull_launch_draws() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        g.player.pos = Vec2::new(104.0, 136.0);
        g.world.spawn("oGhost", Vec2::new(96.0, 120.0));
        let mut source_rng = g.rng.clone();
        let bones = (0..3)
            .map(|_| source_bone_create(&mut source_rng))
            .collect::<Vec<_>>();
        let skull_vy = -f64::from(1 + source_rng.range(3));
        let skull_vx = f64::from(source_rng.range(4)) - f64::from(source_rng.range(4));
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, -99);
        assert_eq!(g.state, RunState::Playing);
        let actual_bones = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oBone")
            .collect::<Vec<_>>();
        assert_eq!(actual_bones.len(), 3);
        for (bone, (velocity, gravity)) in actual_bones.into_iter().zip(bones) {
            assert_eq!(bone.value("world_effect_initialized"), 1.0);
            assert_eq!(bone.vel.x, velocity.x);
            assert_eq!(bone.age, 0);
            assert_eq!(bone.vel.y, velocity.y);
            assert_eq!(bone.value("gravity"), gravity);
        }
        let skull = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oSkull")
            .unwrap();
        assert_eq!(skull.pos, Vec2::new(104.0, 134.0));
        assert_eq!(skull.vel, Vec2::new(skull_vx, skull_vy));
        assert_eq!(g.rng.next_u64(), source_rng.next_u64());
        let newborns = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oBone")
            .map(|e| (e.id, e.vel.y, e.value("gravity")))
            .collect::<Vec<_>>();
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Dead);
        for (id, vy, gravity) in newborns {
            let bone = g.world.entity(id).unwrap();
            assert_eq!(bone.age, 1);
            assert!((bone.vel.y - vy - gravity).abs() < 1e-9);
        }
    }
    #[test]
    fn actual_step_dead_fish_bone_create_matches_source_rng_order() {
        let mut g = actual_room();
        let id = g.world.spawn("oDeadFish", Vec2::new(96.0, 96.0));
        g.world.entity_mut(id).unwrap().health = 0;
        let mut source_rng = g.rng.clone();
        let bones = (0..3)
            .map(|_| source_bone_create(&mut source_rng))
            .collect::<Vec<_>>();
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        let actual_bones = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oBone")
            .collect::<Vec<_>>();
        assert_eq!(actual_bones.len(), 3);
        for (bone, (velocity, gravity)) in actual_bones.into_iter().zip(bones) {
            assert_eq!(bone.value("world_effect_initialized"), 1.0);
            assert_eq!(bone.vel.x, velocity.x);
            assert_eq!(bone.age, 0);
            assert_eq!(bone.vel.y, velocity.y);
            assert_eq!(bone.value("gravity"), gravity);
        }
        assert_eq!(g.rng.next_u64(), source_rng.next_u64());
        let newborns = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oBone")
            .map(|e| (e.id, e.vel.y, e.value("gravity")))
            .collect::<Vec<_>>();
        g.step(&InputFrame::default());
        for (id, vy, gravity) in newborns {
            let bone = g.world.entity(id).unwrap();
            assert_eq!(bone.age, 1);
            assert!((bone.vel.y - vy - gravity).abs() < 1e-9);
        }
    }
    #[test]
    fn actual_step_ghost_disappears_after_full_fourteen_frame_animation() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        g.player.pos = Vec2::new(104.0, 136.0);
        let id = g.world.spawn("oGhost", Vec2::new(96.0, 120.0));
        g.step(&InputFrame::default());
        let ghost = g.world.entity(id).unwrap();
        assert_eq!(ghost.sprite, "sGhostDisappear");
        assert_eq!(ghost.value("animation_speed"), 0.2);
        assert_eq!(crate::catalog::sprite_frame_count(&ghost.sprite), 14);
        for _ in 0..69 {
            g.step(&InputFrame::default());
            assert!(g.world.entity(id).is_some());
        }
        // Allow the final floating-point image_index addition to cross 14.
        for _ in 0..2 {
            g.step(&InputFrame::default());
        }
        assert!(g.world.entity(id).is_none());
    }
    fn damsel_exit_query_room(rules: RulesPreset, door_xs: &[f64]) -> (Game, u64) {
        let mut game = Game::shell(123, rules);
        game.world = World::new(30, 20, Area::Mines, 1);
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(32.0, 152.0);
        game.player.previous_pos = game.player.pos;
        for &x in door_xs {
            game.spawn("oExit", Vec2::new(x, 144.0));
        }
        let id = game.spawn("oDamsel", Vec2::new(132.0, 152.0));
        let damsel = game.world.entity(id).unwrap();
        // Native sDamselLeft: origin(8,8), rectangle mask[2,0..13,15].
        // Movement bounds[-4,-4..4,8] do not reach the earlier door at112.
        assert_eq!(damsel.sprite_bounds(), Rect::new(126.0, 144.0, 12.0, 16.0));
        for door in game.world.entities.iter().filter(|e| e.object == "oExit") {
            assert_eq!(
                door.sprite_bounds(),
                Rect::new(door.pos.x, 144.0, 16.0, 16.0)
            );
            assert!(damsel.event_intersects(door), "full native masks overlap");
            assert_eq!(
                door.mask_contains(damsel.pos, CollisionPrecision::BoundingBox),
                door.pos.x == 128.0,
                "only door B contains the Damsel origin"
            );
        }
        (game, id)
    }

    #[test]
    fn actual_step_damsel_exit_point_gate_then_first_full_native_mask() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, id) = damsel_exit_query_room(rules, &[112.0, 128.0]);
            game.step(&InputFrame::default());
            let damsel = game.world.entity(id).unwrap();
            assert_eq!(damsel.state, 4);
            assert_eq!(damsel.pos, Vec2::new(120.0, 152.0));
            assert_eq!(damsel.vel, Vec2::default());
            assert_eq!(game.data.get("damsels"), Some(&1.0));
            assert_eq!(game.data.get("xdamsels"), Some(&1.0));
            assert_eq!(game.lifetime_stats.damsels_saved_total, 1);
        }
    }

    #[test]
    fn actual_step_damsel_exit_single_origin_door_control() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, id) = damsel_exit_query_room(rules, &[128.0]);
            game.step(&InputFrame::default());
            let damsel = game.world.entity(id).unwrap();
            assert_eq!(damsel.state, 4);
            assert_eq!(damsel.pos, Vec2::new(136.0, 152.0));
            assert_eq!(game.lifetime_stats.damsels_saved_total, 1);
        }
    }

    #[test]
    fn actual_step_damsel_exit_full_mask_alone_does_not_open_point_gate() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, id) = damsel_exit_query_room(rules, &[112.0]);
            game.step(&InputFrame::default());
            assert_ne!(game.world.entity(id).unwrap().state, 4);
            assert!(!game.flags.contains("damsel_rescued"));
            assert_eq!(game.data.get("damsels"), None);
            assert_eq!(game.data.get("xdamsels"), None);
            assert_eq!(game.lifetime_stats.damsels_saved_total, 0);
        }
    }

    #[test]
    fn actual_step_damsel_exit_full_mask_respects_reverse_birth_order() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, id) = damsel_exit_query_room(rules, &[128.0, 112.0]);
            game.step(&InputFrame::default());
            let damsel = game.world.entity(id).unwrap();
            assert_eq!(damsel.state, 4);
            assert_eq!(damsel.pos, Vec2::new(136.0, 152.0));
            assert_eq!(game.lifetime_stats.damsels_saved_total, 1);
        }
    }

    #[test]
    fn actual_step_rescued_damsel_is_invincible_during_exit_animation() {
        let mut g = actual_room();
        g.world.spawn("oExit", Vec2::new(96.0, 96.0));
        let id = g.world.spawn("oDamsel", Vec2::new(104.0, 104.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 4);
        assert_eq!(g.world.entity(id).unwrap().value("invincible"), 1.0);
        g.world.spawn("oExplosion", Vec2::new(104.0, 104.0));
        g.step(&InputFrame::default());
        let damsel = g.world.entity(id).unwrap();
        assert_eq!(damsel.state, 4);
        assert_eq!(damsel.health, 4);
    }
    #[test]
    fn actual_step_damsel_requires_real_exit_and_finishes_exit_animation_once() {
        let mut g = actual_room();
        g.world.exit = Vec2::new(104.0, 104.0);
        let id = g.world.spawn("oDamsel", g.world.exit);
        g.step(&InputFrame::default());
        assert!(!g.flags.contains("damsel_rescued"));
        assert!(!g.data.contains_key("xdamsels"));
        g.world.spawn("oExit", Vec2::new(96.0, 96.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 4);
        assert_eq!(g.data["damsels"], 1.0);
        assert_eq!(g.data["xdamsels"], 1.0);
        assert_eq!(g.data["pending_damsel_kisses"], 1.0);
        assert_eq!(g.lifetime_stats.damsels_saved_total, 1);
        for _ in 0..40 {
            g.step(&InputFrame::default());
        }
        assert!(g.world.entity(id).is_none());
        assert_eq!(g.data["damsels"], 1.0);
        assert_eq!(g.data["xdamsels"], 1.0);
        assert_eq!(g.lifetime_stats.damsels_saved_total, 1);
    }
    #[test]
    fn actual_step_damsel_water_and_lava_hazards() {
        for (liquid, health, vy) in [("oWaterSwim", 4, 0.6), ("oLava", 0, 0.1)] {
            let mut g = actual_room();
            g.world.spawn(liquid, Vec2::new(96.0, 96.0));
            let y = if liquid == "oLava" { 90.0 } else { 104.0 };
            let id = g.world.spawn("oDamsel", Vec2::new(104.0, y));
            g.step(&InputFrame::default());
            let e = g.world.entity(id).unwrap();
            assert_eq!(e.health, health);
            assert!((e.vel.y - vy).abs() < 1e-9, "{liquid} vy={}", e.vel.y);
            if liquid == "oWaterSwim" {
                assert_eq!(e.value("gravity"), 0.2);
                assert!(g.world.entities.iter().any(|e| e.object == "oSplash"));
                g.step(&InputFrame::default());
                assert!((g.world.entity(id).unwrap().vel.y - 0.8).abs() < 1e-9);
            }
        }
    }
    #[test]
    fn actual_step_kiss_frame_emits_once_and_returns_to_slave() {
        let mut g = actual_room();
        g.world.set_tile(6, 7, "oBlock");
        let id = g.world.spawn("oDamsel", Vec2::new(104.0, 104.0));
        let e = g.world.entity_mut(id).unwrap();
        e.state = 6;
        e.sprite = "sDamselKissL".into();
        e.animation = 6.5;
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().animation, 7.0);
        assert!(!g.world.entities.iter().any(|e| e.object == "oHeart"));
        g.step(&InputFrame::default());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oHeart")
                .count(),
            1
        );
        for _ in 0..6 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.world.entity(id).unwrap().state, 5);
    }
    #[test]
    fn ghost_moves_one_pixel_through_walls() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.player.pos = Vec2::new(100.0, 48.0);
        let mut e = Entity::new(1, "oGhost", Vec2::new(40.0, 40.0));
        init(&mut e);
        g.world.set_tile(2, 2, "oBlock");
        let before = e.pos;
        ghost(&mut g, &mut e);
        assert!((e.pos.distance(before) - 1.0).abs() < 1e-9);
        assert!(e.pos.x > before.x);
    }
    #[test]
    fn ghost_contact_is_lethal() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        let mut e = Entity::new(1, "oGhost", g.player.pos);
        init(&mut e);
        collision(&mut g, &mut e, crate::dispatcher::Instance::Player);
        assert_eq!(g.player.health, -99);
        assert_eq!(e.state, 0);
    }
    #[test]
    fn damsel_exit_sets_rescue_without_immediate_heal() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.world.exit = Vec2::new(40.0, 40.0);
        g.world.spawn("oExit", Vec2::new(32.0, 32.0));
        let mut e = Entity::new(99, "oDamsel", g.world.exit);
        init(&mut e);
        damsel(&mut g, &mut e);
        assert!(g.flags.contains("damsel_rescued"));
        assert_eq!(g.player.health, 4);
        assert_eq!(e.state, 4);
    }
    #[test]
    fn dead_fish_outside_water_becomes_portable_bone() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        let mut e = Entity::new(99, "oDeadFish", Vec2::new(80.0, 80.0));
        init(&mut e);
        dead_fish(&mut g, &mut e);
        assert!(!e.alive);
        assert!(g.world.entities.iter().any(|e| e.object == "oFishBone"));
    }
    #[test]
    fn destroying_emitter_releases_barrier() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.world.set_tile(5, 4, "oBlock");
        let id = g.world.spawn("oBarrierEmitter", Vec2::new(80.0, 80.0));
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBarrier")
        );
        g.world.spawn("oBullet", Vec2::new(88.0, 88.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBarrier")
        );
    }
    #[test]
    fn actual_step_new_murderer_keeper_attacks_without_invented_patrol_wait() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        g.player.pos = Vec2::new(184.0, 136.0);
        g.flags.insert("murderer".into());
        let id = g.world.spawn("oShopkeeper", Vec2::new(128.0, 128.0));
        g.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("has_gun".into(), 0.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 4);
        assert_eq!(g.world.entity(id).unwrap().timer, 0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 2);
    }
    #[test]
    fn actual_step_stars_keeper_stops_attacking_dead_player() {
        let mut g = actual_room();
        g.player.health = 0;
        g.player.state = PlayerState::Dead;
        g.state = RunState::Dead;
        let id = g.world.spawn("oShopkeeper2", Vec2::new(128.0, 128.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 1);
    }
    #[test]
    fn actual_step_damsel_yell_uses_ten_frames_and_frame_four_help() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        let id = g.world.spawn("oDamsel", Vec2::new(120.0, 136.0));
        for _ in 0..201 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.world.entity(id).unwrap().state, 3);
        for frame_tick in 1..=19 {
            g.step(&InputFrame::default());
            if frame_tick == 8 {
                assert!(g.world.entities.iter().any(|e| e.object == "oYellHelp"));
            }
            assert_eq!(
                g.world.entity(id).unwrap().state,
                if frame_tick == 19 { 0 } else { 3 }
            );
        }
        assert_eq!(g.world.entity(id).unwrap().timer, 200);
    }
    #[test]
    fn actual_step_keeper_throw_ends_after_seven_source_frames() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        g.player.pos = Vec2::new(136.0, 136.0);
        let id = g.world.spawn("oShopkeeper2", Vec2::new(128.0, 128.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 3);
        assert_eq!(g.world.entity(id).unwrap().value("animation_speed"), 1.0);
        assert_eq!(g.world.entity(id).unwrap().animation, 1.0);
        for _ in 0..5 {
            g.step(&InputFrame::default());
            assert_eq!(g.world.entity(id).unwrap().state, 3);
        }
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 2);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sShopLeft");
    }
    #[test]
    fn actual_step_damsel_child_events_run_outside_inherited_item_viewport() {
        for lava in [false, true] {
            let mut g = actual_room();
            g.player.pos = Vec2::new(40.0, 136.0);
            let id = g.world.spawn("oDamsel", Vec2::new(370.0, 136.0));
            g.world.entity_mut(id).unwrap().vel.x = 1.0;
            if lava {
                g.world.spawn("oLava", Vec2::new(368.0, 128.0));
            }
            g.step(&InputFrame::default());
            let e = g.world.entity(id).unwrap();
            assert_eq!(e.pos, Vec2::new(370.0, 136.0));
            assert_eq!(e.health, if lava { 0 } else { 4 });
            if !lava {
                assert_eq!(e.timer, 199);
            }
        }
    }
    #[test]
    fn actual_step_offscreen_emitter_creates_once_and_keeps_collision_events() {
        let mut g = actual_room();
        g.player.pos = Vec2::new(40.0, 128.0);
        let id = g.world.spawn("oBarrierEmitter", Vec2::new(352.0, 80.0));
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oBarrier")
                .count(),
            1
        );
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_some()); // narrow Step support check skipped
        let bullet = g.world.spawn("oBullet", Vec2::new(356.0, 84.0));
        g.world.entity_mut(bullet).unwrap().vel = Vec2::default();
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert!(!g.world.entities.iter().any(|e| e.object == "oBarrier"));
    }
    #[test]
    fn actual_step_emitter_destruction_callbacks_run_once_when_causes_overlap() {
        for (supported, bullet) in [(false, false), (true, true), (false, true)] {
            let mut g = actual_room();
            if supported {
                g.world.set_tile(10, 4, "oBlock");
            }
            let emitter = g.world.spawn("oBarrierEmitter", Vec2::new(160.0, 80.0));
            if bullet {
                g.world.spawn("oBullet", Vec2::new(168.0, 88.0));
            }
            g.step(&InputFrame::default());
            assert!(g.world.entity(emitter).is_none());
            assert_eq!(
                g.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oFlareSpark")
                    .count(),
                6
            );
            assert_eq!(g.data["shake"], 10.0);
            assert_eq!(
                g.events
                    .iter()
                    .filter(|event| matches!(event,
                        GameEvent::Sound { name, .. } if name == "sndSmallExplode"
                    ))
                    .count(),
                1,
                "supported={supported}, bullet={bullet}"
            );
        }
    }
    #[test]
    fn actual_step_late_explosion_runs_emitter_destroy_event_once() {
        let mut g = actual_room();
        g.world.set_tile(10, 4, "oBlock");
        let emitter = g.world.spawn("oBarrierEmitter", Vec2::new(160.0, 80.0));
        g.world.spawn("oExplosion", Vec2::new(160.0, 80.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(emitter).is_none());
        assert!(!g.world.entities.iter().any(|e| e.object == "oBarrier"));
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oFlareSpark")
                .count(),
            6
        );
        assert_eq!(g.data["shake"], 10.0);
        g.step(&InputFrame::default());
        assert!(!g.events.iter().any(|event| matches!(event,
            GameEvent::Sound { name, .. } if name == "sndSmallExplode"
        )));
    }
    #[test]
    fn actual_step_scarab_destroy_bursts_distinguish_collection_and_death() {
        for (collected, dead, expected_sparks) in
            [(true, false, 3), (false, true, 6), (false, false, 3)]
        {
            let mut g = actual_room();
            let pos = if collected {
                Vec2::new(96.0, 120.0)
            } else if dead {
                Vec2::new(160.0, 80.0)
            } else {
                Vec2::new(-32.0, 80.0)
            };
            if collected {
                g.player.pos = Vec2::new(104.0, 128.0);
            }
            let id = g.world.spawn("oScarab", pos);
            if dead {
                g.world.entity_mut(id).unwrap().health = 0;
            }
            g.step(&InputFrame::default());
            assert!(
                g.world.entity(id).is_none(),
                "collected={collected}, dead={dead}"
            );
            assert_eq!(
                g.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oFlareSpark")
                    .count(),
                expected_sparks,
                "collected={collected}, dead={dead}"
            );
            assert_eq!(g.player.money, 0);
            assert_eq!(
                g.data.get("collect").copied().unwrap_or(0.0),
                if collected { 4000.0 } else { 0.0 }
            );
            assert_eq!(
                g.data.get("scarabs").copied().unwrap_or(0.0),
                if collected { 1.0 } else { 0.0 }
            );
        }
    }
    #[test]
    fn actual_step_scarab_collection_runs_outside_ai_step_viewport() {
        let mut g = actual_room();
        g.world.area = Area::Temple;
        g.player.pos = Vec2::new(338.0, 88.0);
        let id = g.world.spawn("oScarab", Vec2::new(330.0, 80.0));
        assert_eq!(g.camera, Vec2::default());
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert_eq!(g.player.money, 0);
        assert_eq!(g.data["collect"], 12000.0);
        assert_eq!(g.data["scarabs"], 1.0);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oFlareSpark")
                .count(),
            3
        );
        g.step(&InputFrame::default());
        assert_eq!(g.player.money, 0);
        assert_eq!(g.data["collect"], 12000.0);
        assert_eq!(g.data["scarabs"], 1.0);
        assert!(!g.events.iter().any(|event| matches!(event,
            GameEvent::Collected { object, .. } if object == "oScarab"
        )));
    }
    #[test]
    fn actual_step_scarab_queues_literal_area_value_then_counts_money_and_earnings() {
        for (area, value) in [
            (Area::Mines, 4000),
            (Area::Jungle, 8000),
            (Area::Temple, 12000),
        ] {
            let mut g = actual_room();
            g.world.area = area;
            for x in 0..30 {
                g.world.set_tile(x, 9, "oBlock");
            }
            g.player.pos = Vec2::new(104.0, 128.0);
            g.player.money = 500;
            g.data.insert("xmoney".into(), 25.0);
            let id = g.world.spawn("oScarab", Vec2::new(96.0, 120.0));
            g.step(&InputFrame::default());
            assert!(g.world.entity(id).is_none());
            assert_eq!(g.player.money, 500);
            assert_eq!(g.data["xmoney"], 25.0);
            assert_eq!(g.data["collect"], f64::from(value));
            assert_eq!(g.data["collectCounter"], 20.0);
            assert_eq!(g.data["scarabs"], 1.0);
            for _ in 0..20 {
                g.step(&InputFrame::default());
            }
            assert_eq!(g.data["collectCounter"], 0.0);
            assert_eq!(g.player.money, 500);
            g.step(&InputFrame::default());
            assert_eq!(g.player.money, 600);
            assert_eq!(g.data["xmoney"], 125.0);
            assert_eq!(g.data["collect"], f64::from(value - 100));
            for _ in 1..value / 100 {
                g.step(&InputFrame::default());
            }
            assert_eq!(g.player.money, 500 + value);
            assert_eq!(g.data["xmoney"], 25.0 + f64::from(value));
            assert_eq!(g.data["collect"], 0.0);
            assert_eq!(g.data["scarabs"], 1.0);
        }
    }
    #[test]
    fn actual_step_scarab_queue_adds_to_existing_collection_and_caps_delay() {
        let mut g = actual_room();
        g.player.pos = Vec2::new(104.0, 128.0);
        g.data.insert("collect".into(), 250.0);
        g.data.insert("collectCounter".into(), 95.0);
        g.world.spawn("oScarab", Vec2::new(96.0, 120.0));
        g.step(&InputFrame::default());
        assert_eq!(g.player.money, 0);
        assert_eq!(g.data["collect"], 4250.0);
        assert_eq!(g.data["collectCounter"], 100.0);
        assert_eq!(g.data["scarabs"], 1.0);
    }
    #[test]
    fn actual_step_attacking_unarmed_keeper_consumes_shotgun_and_clears_holder() {
        for held in [false, true] {
            let mut g = actual_room();
            for x in 0..30 {
                g.world.set_tile(x, 6, "oBlock");
            }
            g.player.pos = if held {
                Vec2::new(84.0, 88.0)
            } else {
                Vec2::new(200.0, 88.0)
            };
            g.player.facing = -1;
            g.player.invincible = 30;
            let keeper = g.world.spawn("oShopkeeper2", Vec2::new(72.0, 80.0));
            g.world
                .entity_mut(keeper)
                .unwrap()
                .data
                .insert("firing".into(), 30.0);
            let gun = g.world.spawn("oShotgun", Vec2::new(80.0, 88.0));
            g.world
                .entity_mut(gun)
                .unwrap()
                .data
                .insert("gravity".into(), 0.0);
            g.world.entity_mut(gun).unwrap().held = held;
            if held {
                g.player.held = Some(gun);
            }
            g.step(&InputFrame::default());
            assert_eq!(g.world.entity(keeper).unwrap().value("has_gun"), 1.0);
            assert!(g.world.entity(gun).is_none());
            assert_eq!(g.player.held, None);
        }
    }
    #[test]
    fn stars_corpse_expires_after_source_death_timer() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags.insert("hub_stars".into());
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.set_tile(4, 5, "oBlock");
        let mut e = Entity::new(99, "oShopkeeper2", Vec2::new(64.0, 64.0));
        super::super::init_entity(&mut e);
        e.health = 0;
        for _ in 0..200 {
            tick(&mut game, &mut e);
            assert!(e.alive);
        }
        assert_eq!(e.value("death_timer"), 0.0);
        tick(&mut game, &mut e);
        assert!(!e.alive);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .count(),
            3
        );
    }
}

#[cfg(test)]
mod keeper_own_crush_tail_tests {
    use super::*;
    use crate::RulesPreset;

    fn fixture(rules: RulesPreset, crushed: bool) -> Game {
        let mut game = Game::legacy_shell(42, rules);
        game.world = World::new(30, 20, crate::Area::Mines, 1);
        game.world.name = "rLevel".into();
        game.player.pos = Vec2::new(240.0, 152.0);
        if crushed {
            for y in 5..=6 {
                for x in 6..=7 {
                    game.world.set_tile(x, y, "oBlock");
                }
            }
        } else {
            for x in 0..30 {
                game.world.set_tile(x, 6, "oBlock");
            }
        }
        game
    }
    fn actor() -> Entity {
        let mut actor = Entity::new(999, "oShopkeeper", Vec2::new(96.0, 80.0));
        crate::enemies::init_entity(&mut actor);
        actor
    }
    fn objects<'a>(game: &'a Game, object: &str) -> Vec<&'a Entity> {
        let mut result: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == object)
            .collect();
        result.sort_by_key(|e| e.creation_serial);
        result
    }
    // Raw-port adapter checks the source statement/Create ordering only. It
    // does not claim canonical GameMaker seed or numerical stream equality.
    fn blood_draws(rng: &mut crate::Rng) {
        for _ in 0..3 {
            rng.next_u64();
            rng.next_u64();
            rng.next_u64();
            rng.next_u64();
        }
    }
    fn gun_draws(rng: &mut crate::Rng) -> Vec2 {
        let y = (4 + rng.next_u64() % 3) as f64;
        let x = (4 + rng.next_u64() % 3) as f64;
        Vec2::new(x, y)
    }

    #[test]
    fn controlled_healthy_crush_preserves_hp_and_does_not_prelatch_dead() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = fixture(rules, true);
            let mut actor = actor();
            let mut expected = game.rng.clone();
            blood_draws(&mut expected);
            let gun = gun_draws(&mut expected);
            shopkeeper(&mut game, &mut actor);
            assert!(!actor.alive);
            assert_eq!(actor.health, 20, "CRUSH_SOURCE_DOES_NOT_ASSIGN_HP");
            assert_eq!(
                actor.value("dead"),
                0.0,
                "CRUSH_SOURCE_DOES_NOT_ASSIGN_DEAD"
            );
            assert_eq!(actor.value("death_recorded"), 0.0);
            assert_eq!(actor.value("enemy_destroyed"), 1.0, "DESTROY_AT_PRODUCER");
            assert_eq!(
                actor.state, 4,
                "RETIRED_IDLE_SEES_MURDERER_AND_ENTERS_PATROL"
            );
            assert_eq!(actor.value("has_gun"), 0.0);
            assert_eq!(objects(&game, "oBlood").len(), 3);
            assert!(
                objects(&game, "oBlood")
                    .iter()
                    .all(|e| e.pos == Vec2::new(104.0, 88.0))
            );
            assert_eq!(objects(&game, "oShotgun").len(), 1);
            let dropped = objects(&game, "oShotgun")[0];
            assert_eq!(dropped.pos, Vec2::new(104.0, 88.0));
            assert_eq!(dropped.vel, gun);
            assert_eq!(dropped.value("cost"), 0.0);
            assert!(objects(&game, "oGoldNugget").is_empty());
            assert_eq!(game.data.get("kills"), Some(&1.0));
            assert_eq!(game.lifetime_stats.enemy_kills[19], 1);
            assert_eq!(
                game.rng.next_u64(),
                expected.next_u64(),
                "BLOOD_CREATES_THEN_ORDINARY_CRUSH_GUN"
            );
        }
    }

    #[test]
    fn controlled_dead_crush_runs_dead_loot_then_common_gun_body_tail() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = fixture(rules, true);
            let mut actor = actor();
            actor.health = 0;
            actor.state = 99;
            let mut expected = game.rng.clone();
            blood_draws(&mut expected);
            let count = (1 + expected.next_u64() % 4) as usize;
            let velocities: Vec<_> = (0..count)
                .map(|_| {
                    Vec2::new(
                        (1 + expected.next_u64() % 3) as f64 - (1 + expected.next_u64() % 3) as f64,
                        -1.0,
                    )
                })
                .collect();
            let gun = gun_draws(&mut expected);
            shopkeeper(&mut game, &mut actor);
            assert!(!actor.alive);
            assert_eq!(actor.value("enemy_destroyed"), 1.0);
            assert_eq!(actor.value("dead"), 1.0, "DEAD_BRANCH_OWNS_LATCH");
            assert_eq!(actor.value("death_recorded"), 1.0);
            assert_eq!(
                game.data.get("kills"),
                Some(&1.0),
                "HP_ZERO_CRUSH_HAS_NO_CREDIT_BEFORE_DEAD"
            );
            assert_eq!(game.lifetime_stats.enemy_kills[19], 1);
            let gold = objects(&game, "oGoldNugget");
            assert_eq!(gold.len(), count);
            for (piece, velocity) in gold.into_iter().zip(velocities) {
                assert_eq!(
                    piece.pos,
                    Vec2::new(104.0, 88.0),
                    "DROP_ORIGIN_BEFORE_BODY_TAIL"
                );
                assert_eq!(piece.vel, velocity);
            }
            assert_eq!(
                objects(&game, "oShotgun").len(),
                1,
                "STUNNED_CRUSH_DEFERS_GUN_TO_COMMON_TAIL"
            );
            assert_eq!(objects(&game, "oShotgun")[0].vel, gun);
            assert_eq!(
                actor.value("collision_top"),
                6.0,
                "COMMON_CORPSE_BODY_TAIL_EXECUTED"
            );
            assert_eq!(actor.sprite, "sShopDieL");
            assert_eq!(
                game.rng.next_u64(),
                expected.next_u64(),
                "BLOOD_THEN_DEAD_GOLD_THEN_GUN"
            );
            let count = game.world.entities.len();
            super::super::destroyed(&mut game, &mut actor);
            assert_eq!(
                game.world.entities.len(),
                count,
                "DESTROY_CALLBACK_IDEMPOTENT"
            );
        }
    }

    #[test]
    fn controlled_held_stunned_crush_clears_backlinks_then_keeps_timer_and_live_art() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = fixture(rules, true);
            let mut actor = actor();
            actor.state = 98;
            actor.held = true;
            actor.timer = 1;
            game.player.held = Some(actor.id);
            game.player.pickup_item_type = "Shopkeeper".into();
            // Explicit event-level attachment fixture; not a natural pickup or
            // sticky-bomb route. It observes synchronous callback dependencies.
            let bomb = game.spawn("oBomb", Vec2::new(240.0, 80.0));
            actor.data.insert("bomb_id".into(), bomb as f64);
            game.world
                .entity_mut(bomb)
                .unwrap()
                .data
                .insert("attached_id".into(), actor.id as f64);
            shopkeeper(&mut game, &mut actor);
            assert!(!actor.alive);
            assert_eq!(actor.health, 20);
            assert_eq!(actor.value("dead"), 0.0);
            assert_eq!(actor.value("enemy_destroyed"), 1.0);
            assert_eq!(
                game.player.held, None,
                "ENEMY_DESTROY_BEFORE_COMMON_HELD_TAIL"
            );
            assert_eq!(game.player.pickup_item_type, "Shopkeeper");
            assert_eq!(game.world.entity(bomb).unwrap().value("attached_id"), 0.0);
            assert!(actor.held, "DESTROY_RETAINS_LOCAL_HELD_FIELD");
            assert_eq!(actor.timer, 0, "STUNNED_COUNTER_CONTINUES_AFTER_CRUSH");
            assert_eq!(actor.state, 98);
            assert_eq!(
                actor.sprite, "sShopHeldL",
                "PRESERVED_HP_SELECTS_LIVE_HELD_ART"
            );
            assert_eq!(actor.value("collision_top"), 6.0);
            assert_eq!(objects(&game, "oShotgun").len(), 1);
            assert!(objects(&game, "oGoldNugget").is_empty());
            assert_eq!(game.data.get("kills"), Some(&1.0));
        }
    }

    #[test]
    fn controlled_noncrush_dead_control_keeps_corpse_alive() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = fixture(rules, false);
            let mut actor = actor();
            actor.health = 0;
            actor.state = 99;
            shopkeeper(&mut game, &mut actor);
            assert!(
                actor.alive,
                "DEAD_IS_A_RETAINED_CORPSE_UNLESS_A_DESTROY_BRANCH_RUNS"
            );
            assert_eq!(actor.state, 99);
            assert_eq!(actor.value("enemy_destroyed"), 0.0);
            assert_eq!(actor.value("dead"), 1.0);
            assert_eq!(game.data.get("kills"), Some(&1.0));
            assert!(objects(&game, "oBlood").is_empty());
            let gold = objects(&game, "oGoldNugget").len();
            assert!((1..=4).contains(&gold));
            assert_eq!(objects(&game, "oShotgun").len(), 1);
            // Inspect only this event. A second raw Step without advancing its
            // newborn solid GoldNuggets is not an ordinary lifecycle route.
        }
    }
}
#[cfg(test)]
mod deadfish_isolated_membership_tests {
    use super::*;

    // These direct own-event fixtures deliberately omit Level553, which would
    // reactivate nearby Water and Character before an ordinary DeadFish Step.
    fn inactive_membership_case(canonical: bool) {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = if canonical {
                Game::shell(42, rules)
            } else {
                Game::legacy_shell(42, rules)
            };
            game.world = World::new(30, 20, Area::Jungle, 5);
            game.player.pos = Vec2::new(240.0, 152.0);
            game.player.previous_pos = game.player.pos;
            let water = game.spawn("oWaterSwim", Vec2::new(96.0, 96.0));
            let point = Vec2::new(100.0, 100.0);
            assert!(dead_fish_water(&game, point));
            game.world.entity_mut(water).unwrap().engine_active = false;
            assert!(
                !dead_fish_water(&game, point),
                "ISOLATED_QUERY_OMITS_INACTIVE_WATER"
            );
            game.world.entity_mut(water).unwrap().engine_active = true;
            game.world.entity_mut(water).unwrap().alive = false;
            assert!(
                !dead_fish_water(&game, point),
                "ISOLATED_QUERY_OMITS_RETIRED_WATER"
            );
            game.world
                .entities
                .iter_mut()
                .find(|e| e.id == water)
                .unwrap()
                .alive = true;
            let id = game.spawn("oDeadFish", Vec2::new(96.0, 96.0));
            let mut fish = game.world.entity(id).unwrap().clone();
            fish.state = 1;
            game.player.engine_active = false;
            let rng = game.rng.state_bits();
            dead_fish(&mut game, &mut fish);
            assert!(fish.alive);
            assert_eq!(
                fish.state, 1,
                "ISOLATED_ABSENT_ACTIVE_CHARACTER_SKIPS_ATTACK_BRANCH"
            );
            assert_eq!(game.rng.state_bits(), rng);
        }
    }
    #[test]
    fn legacy_direct_own_event_omits_inactive_water_and_character() {
        inactive_membership_case(false);
    }
    #[test]
    fn canonical_direct_own_event_omits_inactive_water_and_character() {
        inactive_membership_case(true);
    }
}

#[cfg(test)]
#[path = "npc_movement_spike_followup_tests.rs"]
mod npc_movement_spike_followup_tests;

#[cfg(test)]
#[path = "shopkeeper_patrol_burst_source_tests.rs"]
mod shopkeeper_patrol_burst_source_tests;

#[cfg(test)]
#[path = "shopkeeper_cached_distance_tests.rs"]
mod shopkeeper_cached_distance_tests;
