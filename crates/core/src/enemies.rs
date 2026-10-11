//! Source-derived enemy state machines. Explicit precise solid queries use source
//! sprite masks; collision events and unvalidated transforms retain their limits.
use crate::model::*;

mod caves;
#[cfg(test)]
mod destroy_lifecycle_tests;
mod fragile_item;
pub(crate) use fragile_item::step_impacts as fragile_item_step_impacts;
mod ice;
mod inherited;
mod jungle;
mod npcs;
#[cfg(test)]
mod ordinary_item_blood_tests;
pub(crate) mod shop_dialogue;
#[cfg(test)]
mod source_facing_tests;
mod temple;
mod undead;

use crate::dispatcher::Instance;

pub fn is_enemy(object: &str) -> bool {
    crate::catalog::is_a(object, "oEnemy")
}

/// Source facing is distinct from the dir/aim fields reused by these AIs.
/// Families with actual facing writes continue using the existing property.
pub(crate) fn source_facing(entity: &Entity) -> i8 {
    entity.data.get("source_facing").map_or(
        entity.facing,
        |facing| {
            if *facing == 0.0 { -1 } else { 1 }
        },
    )
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
        e.data.entry("bomb_id".into()).or_insert(0.0);
    }
    if is_enemy(&e.object) {
        // These Creates finish facing LEFT, while the port uses `facing`
        // for their separate AI direction. Only Spider's inherited held
        // Step can subsequently write source facing; preserve it separately.
        if matches!(
            e.object.as_str(),
            "oSpider"
                | "oSpiderHang"
                | "oGiantSpider"
                | "oGiantSpiderHang"
                | "oPiranha"
                | "oDeadFish"
                | "oScarab"
                | "oJaws"
                | "oAlienBoss"
        ) {
            e.data.insert("source_facing".into(), 0.0);
        }
        // Inherited oEnemy Create sets false; only Bat/UFO override it.
        e.data.insert(
            "flying".into(),
            f64::from(matches!(e.object.as_str(), "oBat" | "oUFO")),
        );
        // oEnemy Create enables carrying; these own Create events override it.
        let can_pick_up = !matches!(
            e.object.as_str(),
            "oYetiKing" | "oTombLord" | "oJaws" | "oAlienBoss"
        );
        e.data
            .entry("canPickUp".into())
            .or_insert(f64::from(can_pick_up));
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
        // Inherited Destroy retires the receiver without aborting the current
        // event's following child action. with_actor already checks that the
        // receiver was alive on entry; do not revive it or dispatch a new Step.
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

/// The shared world/RNG-only Create bodies also run at generator birth sites.
pub(crate) fn create_local(
    context: &mut crate::creation::EntityCreateContext<'_>,
    entity: &mut Entity,
) {
    jungle::create_local(context, entity);
    ice::create_local(context, entity);
    undead::create_local(context, entity);
    npcs::create_local(context, entity);
}

/// Complete runtime Create without repeating generated local initialization.
/// Game-dependent effects still run on authored/generated room bootstrap.
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
        create_local(
            &mut crate::creation::EntityCreateContext {
                world: &game.world,
                rng: &mut game.rng,
            },
            e,
        );
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
                        // Source sprite_width/height refer to the complete
                        // current image, independently of mask, origin, or scale.
                        let (width, height) =
                            crate::catalog::sprite_dimensions(&e.sprite).unwrap_or((0, 0));
                        create_blood(
                            game,
                            e,
                            Vec2::new(
                                e.pos.x + f64::from(width) / 2.0,
                                e.pos.y + f64::from(height) / 2.0,
                            ),
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
    crate::items::explosives::sticky_receiver_destroyed(game, e);
    npcs::destroyed(game, e);
    // Scarab replaces Enemy Destroy without action_inherited. Every other
    // Enemy inherits this callback, even when another family retires it.
    if is_enemy(&e.object) && e.object != "oScarab" && e.value("enemy_destroyed") == 0.0 {
        e.data.insert("enemy_destroyed".into(), 1.0);
        if e.held {
            game.player.held = None;
            // Source writes pickupItem, not pickupItemType. Its instance-local
            // pickupItem has no readers; preserve the remembered inventory type.
        }
        // Destroy does not change the retired receiver's held flag. Remaining
        // statements in its current event still observe that original value.
    }
}

/// Finish a cross-system actor instance_destroy before the caller resumes.
/// Raw lookup retains the receiver whose alive flag the producer just cleared.
pub(crate) fn finish_actor_destroy(game: &mut Game, id: u64) {
    let Some(mut actor) = game
        .world
        .entities
        .iter()
        .find(|e| e.id == id && !e.alive && (is_actor(&e.object) || is_enemy(&e.object)))
        .cloned()
    else {
        return;
    };
    destroyed(game, &mut actor);
    if let Some(stored) = game.world.entities.iter_mut().find(|e| e.id == id) {
        *stored = actor;
    }
}

/// Source cross-system instance_destroy of an emitter finishes its own Destroy
/// synchronously, before later collision resource pairs can query the barrier.
pub(crate) fn destroy_barrier_emitter(game: &mut Game, id: u64) {
    let Some(mut emitter) = game
        .world
        .entities
        .iter()
        .find(|e| e.id == id && e.object == "oBarrierEmitter" && e.value("npc_destroyed") == 0.0)
        .cloned()
    else {
        return;
    };
    emitter.alive = false;
    npcs::destroyed(game, &mut emitter);
    if let Some(stored) = game.world.entities.iter_mut().find(|e| e.id == id) {
        *stored = emitter;
    }
}

pub(crate) fn finish_destroy_events(game: &mut Game) {
    // Most actor callbacks finish Destroy through with_actor. Preserve the
    // backlink callback for deaths written by another family before cleanup.
    let dead: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            !e.alive
                && ((is_enemy(&e.object)
                    && e.object != "oScarab"
                    && e.value("enemy_destroyed") == 0.0)
                    || (e.object == "oDamsel" && e.value("sticky_destroyed") == 0.0))
        })
        .map(|e| e.id)
        .collect();
    for id in dead {
        if let Some(mut receiver) = game.world.entities.iter().find(|e| e.id == id).cloned() {
            destroyed(game, &mut receiver);
            if let Some(stored) = game.world.entities.iter_mut().find(|e| e.id == id) {
                *stored = receiver;
            }
        }
    }
    crate::items::explosives::finish_sticky_destroy_events(game);
    npcs::finish_destroy_events(game);
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
    if game
        .world
        .entity(item_id)
        .is_some_and(|item| matches!(item.object.as_str(), "oJar" | "oSkull"))
    {
        if fragile_item_step_impacts(game, item_id) {
            crate::items::break_item(game, item_id);
        }
        return;
    }
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
    let first = Vec2::new(item.pos.x - radius, item.pos.y - radius);
    let second = Vec2::new(item.pos.x + radius, item.pos.y + radius);
    let overlaps_enemy = game.world.entities.iter().any(|e| {
        e.alive
            && e.id != item_id
            && is_enemy(&e.object)
            && e.collision_mask().intersects_rectangle_bbox(first, second)
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
            item.object != "oDice"
                && e.alive
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
            apply_item_hit(game, e, &item, fragile);
            if fragile {
                broke = true;
            }
            // oItem's enemy branch emits before destroying the arrow. The
            // separate Damsel branch has no bomb-arrow explosion callback.
            if e.object != "oDamsel" && item.object == "oArrow" && item.sprite == "sBombArrowRight"
            {
                game.spawn("oExplosion", item.pos);
                if !game.flags.contains("low_graphics") {
                    crate::world_events::create_flames(game, item.pos, 3);
                }
            }
            if matches!(item.object.as_str(), "oArrow" | "oFishBone") {
                if let Some(projectile) = game.world.entities.iter_mut().find(|e| e.id == item_id) {
                    projectile.alive = false;
                }
            }
        });
    }
    if item.object == "oDice" {
        // Dice's separate Damsel statement follows the entire Enemy branch.
        dice_damsel_impact(game, &item);
    }
    if broke {
        crate::items::break_item(game, item_id);
    }
}

/// oDice Step200-220: bbox rectangle only gates the subsequent whole-family
/// nearest query. Target selection and eligibility run after the Enemy body.
fn dice_damsel_impact(game: &mut Game, item: &Entity) {
    crate::dispatcher::register_instances(game);
    let members = crate::dispatcher::descendant_instances(game, "oDamsel");
    let first = Vec2::new(item.pos.x - 2.0, item.pos.y - 2.0);
    let second = Vec2::new(item.pos.x + 2.0, item.pos.y + 2.0);
    let overlaps = members.iter().any(|instance| {
        let Instance::Entity(id) = *instance else {
            return false;
        };
        game.world.entity(id).is_some_and(|target| {
            target.engine_active
                && target
                    .collision_mask()
                    .intersects_rectangle_bbox(first, second)
        })
    });
    if !overlaps {
        return;
    }
    let mut nearest = None;
    let mut distance = f64::INFINITY;
    for instance in members {
        let Instance::Entity(id) = instance else {
            continue;
        };
        let Some(target) = game.world.entity(id) else {
            continue;
        };
        let candidate = target.pos.distance(item.pos);
        if target.engine_active && candidate < distance {
            nearest = Some(id);
            distance = candidate;
        }
    }
    let Some(id) = nearest else {
        return;
    };
    if !game
        .world
        .entity(id)
        .is_some_and(|target| target.value("invincible") == 0.0 && !matches!(target.state, 2 | 99))
    {
        return;
    }
    // Direct instance_create, not scrCreateBlood's bloodless/16 precheck.
    // Blood's synchronous inherited Detritus Create still owns its 32 cap.
    game.spawn("oBlood", item.pos);
    let target = game.world.entity_mut(id).unwrap();
    let release = target.held;
    target.held = false;
    target.health -= 1;
    target.vel.y = -6.0;
    target.state = 2;
    target.timer = 120;
    target.vel.x = item.vel.x * 0.3;
    if release {
        // The source clears the Player pointer unconditionally when held.
        game.player.held = None;
        game.player.pickup_item_type.clear();
    }
    // No hit_cooldown, safe, assaulted or source Blood-script side effects.
    game.sound("sndHit", item.pos);
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
            .filter(|other| {
                other.alive
                    && other.engine_active
                    && crate::catalog::is_a(&other.object, "oDetritus")
            })
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
        let extra = f > 0.0 && time.is_multiple_of((1.0 / f).round_ties_even().max(1.0) as u64);
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

/// Spider Steps use moveTo without assigning velocity at solid contact.
/// Keep their post-move gravity separate from families that settle support.
pub(super) fn move_body_preserving_velocity(
    world: &World,
    e: &mut Entity,
    gravity: f64,
) -> (bool, bool) {
    let velocity = e.vel;
    let contacts = move_body(world, e, 0.0);
    e.vel = Vec2::new(
        velocity.x,
        (velocity.y + inherited::gravity(e, gravity)).min(10.0),
    );
    contacts
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
        if spiked {
            // Parent Character collision emits this extra drop before the
            // animal's own next-Step death burst, with scrCreateBlood guards.
            create_blood(
                game,
                e,
                Vec2::new(game.player.pos.x, game.player.pos.y + 8.0),
                1,
            );
        }
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
        let source_type = crate::items::explosives::source_type(e);
        let prior_health = game.player.health;
        if prior_health > 0 {
            game.player.health -= damage;
        }
        if prior_health > 0 && game.player.health <= 0 && crate::statistics::is_real_level(game) {
            use crate::statistics::EnemyDeath;
            // The source compares type, not object_index. Bat, Zombie and
            // hanging spiders retain inherited "None" until a caller writes it.
            let cause = match source_type.as_str() {
                "Bat" => Some(EnemyDeath::Bat),
                "Snake" => Some(EnemyDeath::Snake),
                "Spider" => Some(EnemyDeath::Spider),
                "Giant Spider" => Some(EnemyDeath::GiantSpider),
                "Caveman" => Some(EnemyDeath::Caveman),
                "Skeleton" => Some(EnemyDeath::Skeleton),
                "Zombie" => Some(EnemyDeath::Zombie),
                "Vampire" => Some(EnemyDeath::Vampire),
                "Frog" => Some(EnemyDeath::Frog),
                "Fire Frog" => Some(EnemyDeath::FireFrog),
                "Mantrap" => Some(EnemyDeath::ManTrap),
                "Piranha" => Some(EnemyDeath::Piranha),
                // Jaws Create uses "Mega Mouth", not this source comparison.
                "Megamouth" => Some(EnemyDeath::Jaws),
                "Yeti" => Some(EnemyDeath::Yeti),
                "Yeti King" => Some(EnemyDeath::YetiKing),
                "Alien" => Some(EnemyDeath::Alien),
                "UFO" => Some(EnemyDeath::Ufo),
                "Alien Boss" => Some(EnemyDeath::AlienBoss),
                "Hawkman" => Some(EnemyDeath::Hawkman),
                "Shopkeeper" => Some(EnemyDeath::Shopkeeper),
                "Tomb Lord" => Some(EnemyDeath::TombLord),
                "Magma Man" => Some(EnemyDeath::MagmaMan),
                "Olmec" => Some(EnemyDeath::Olmec),
                _ => None,
            };
            if let Some(cause) = cause {
                game.lifetime_stats.enemy_death(cause);
            }
        }
        game.events.push(GameEvent::Damage { amount: damage });
        // Literal oEnemy Character event: these inherited contacts bleed at
        // the enemy origin. Vampire's own callback returns before this path.
        if matches!(source_type.as_str(), "Bat" | "Piranha" | "Vampire") {
            create_blood(game, e, Vec2::new(e.pos.x + 4.0, e.pos.y + 4.0), 1);
        }
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

fn apply_item_hit(game: &mut Game, e: &mut Entity, item: &Entity, fragile: bool) {
    let object = item.object.as_str();
    let velocity = item.vel;
    // Dice and fragile bodies replace oItem Step and create Blood directly.
    // Their literal event branches are handled in a separate parity change.
    let ordinary = !fragile && object != "oDice";
    e.vel.x = velocity.x * 0.3;
    let mut hit = false;
    if e.object == "oDamsel" {
        if object != "oDamsel" {
            if ordinary {
                // This call is outside `with obj`: bloodless belongs to the item.
                create_blood(game, item, e.pos, 1);
            }
            if e.held {
                e.held = false;
                game.player.held = game.player.held.filter(|id| *id != e.id);
                // Ordinary Item and Dice clear the remembered type; the
                // Jar/Skull replacement Steps intentionally retain it.
                if !fragile {
                    game.player.pickup_item_type.clear();
                }
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
                if ordinary && e.object != "oManTrap" {
                    create_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 1);
                }
                e.health -= 1;
                e.state = 98;
                e.timer = e.value("stun_time") as u32;
                e.vel.y = -6.0;
                hit = true;
            }
            "oShopkeeper" | "oShopkeeper2" if e.state < 98 => {
                if ordinary {
                    create_blood(game, e, e.pos, 1);
                }
                e.health -= 1;
                e.state = 2;
                e.vel.y = -6.0;
                hit = true;
            }
            "oGiantSpider" | "oTombLord" if e.value("hit_cooldown") == 0.0 => {
                if ordinary {
                    let offset_y = if e.object == "oTombLord" { 16.0 } else { 24.0 };
                    create_blood(game, e, Vec2::new(e.pos.x + 16.0, e.pos.y + offset_y), 1);
                }
                e.health -= 1;
                e.data.insert(
                    "hit_cooldown".into(),
                    if e.object == "oTombLord" { 20.0 } else { 10.0 },
                );
                hit = true;
            }
            "oAlienBoss" if e.state != 99 && e.sprite != "sAlienBossHurt" => {
                if ordinary {
                    create_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 1);
                }
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
                if ordinary {
                    create_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 1);
                }
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
        "oGiantSpider" | "oTombLord" if !pre => {
            if e.value("hit_cooldown") > 0.0 || (e.object == "oTombLord" && pos.y >= e.pos.y + 12.0)
            {
                return false;
            }
            // These front-whip overrides create Blood directly, bypassing
            // scrCreateBlood's bloodless flag and 16-detritus gate. Their
            // inherited rear-whip events remain in the oEnemy handler.
            e.health -= if e.object == "oGiantSpider" {
                1
            } else {
                damage
            };
            if e.object == "oTombLord" {
                set_counts_as_kill(e, true);
            }
            game.spawn("oBlood", Vec2::new(e.pos.x + 16.0, e.pos.y + 24.0));
            game.sound("sndHit", e.pos);
            e.data.insert("hit_cooldown".into(), 10.0);
            e.data.insert("assaulted".into(), 1.0);
            if e.health <= 0 {
                e.data.insert("killed_by_player".into(), 1.0);
            }
            return true;
        }
        "oYetiKing" if !pre => {
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
    if matches!(e.object.as_str(), "oCaveman" | "oHawkman") && e.value("blood_left") > 0.0 {
        // Source uses the current full canvas size, never mask dimensions
        // or an origin-adjusted center (including Hawk's 24px resting poses).
        let (width, height) = crate::catalog::sprite_dimensions(&e.sprite).unwrap_or((0, 0));
        create_blood(
            game,
            e,
            Vec2::new(
                e.pos.x + f64::from(width) / 2.0,
                e.pos.y + f64::from(height) / 2.0,
            ),
            1,
        );
        // Exhaustion depends on post-hit HP, even when bloodless or capped.
        if e.health < 0 {
            e.data
                .insert("blood_left".into(), e.value("blood_left") - 1.0);
        }
    }
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

    fn flying_spring_room(object: &str, rules: RulesPreset) -> (Game, u64, u64) {
        let mut game = Game::shell(42, rules);
        game.world = World::new(20, 16, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..20 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(40.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        let spring = game.spawn("oSpringTrap", Vec2::new(96.0, 144.0));
        // The source RECTANGLE/AUTO masks are Spring y12..15, BatHang
        // y0..11, UFO y1..12, and SnakeWalkL y4..15. Bat's two-pixel
        // offset produces real event overlap while its movement bottom
        // touches the floor at160. No actor is changed after birth.
        let enemy = game.spawn(
            object,
            Vec2::new(96.0, if object == "oBat" { 146.0 } else { 144.0 }),
        );
        assert!(
            game.world
                .entity(enemy)
                .unwrap()
                .event_intersects(game.world.entity(spring).unwrap())
        );
        (game, enemy, spring)
    }

    #[test]
    fn natural_bat_spring_contact_preserves_flight_in_both_presets() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, enemy, spring) = flying_spring_room("oBat", rules);
            game.step(&crate::InputFrame::default());
            let spring = game.world.entity(spring).unwrap();
            // Check the observable response before the declaration: missing
            // flying would spring this genuinely overlapping source mask.
            assert_eq!((spring.state, spring.timer), (0, 0));
            assert!(
                !game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndBoing")
                )
            );
            let bat = game.world.entity(enemy).unwrap();
            assert_eq!(bat.pos, Vec2::new(96.0, 146.0));
            assert_eq!(bat.vel, Vec2::default());
            assert_eq!(bat.state, 1); // The ordinary Hang Step woke it.
            assert!(bat.event_intersects(spring));
            assert_eq!(bat.value("flying"), 1.0);
        }
    }

    #[test]
    fn natural_ufo_spring_contact_preserves_search_in_both_presets() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, enemy, spring) = flying_spring_room("oUFO", rules);
            game.step(&crate::InputFrame::default());
            let spring = game.world.entity(spring).unwrap();
            assert_eq!((spring.state, spring.timer), (0, 0));
            assert!(
                !game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndBoing")
                )
            );
            let ufo = game.world.entity(enemy).unwrap();
            assert_eq!(ufo.pos, Vec2::new(96.0, 144.0));
            assert_eq!(ufo.vel.y, -2.0); // Source SEARCH, rather than -8 launch.
            assert_eq!(ufo.vel.x.abs(), 1.0);
            assert_eq!(ufo.state, 0);
            assert!(ufo.event_intersects(spring));
            assert_eq!(ufo.value("flying"), 1.0);
        }
    }

    #[test]
    fn natural_grounded_snake_spring_contact_launches_in_both_presets() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, enemy, spring) = flying_spring_room("oSnake", rules);
            let snake = game.world.entity(enemy).unwrap();
            assert_eq!(snake.bounds().y + snake.bounds().h, 160.0);
            game.step(&crate::InputFrame::default());
            let spring = game.world.entity(spring).unwrap();
            assert_eq!((spring.state, spring.timer), (1, 10));
            assert_eq!(
                game.events
                    .iter()
                    .filter(
                        |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndBoing")
                    )
                    .count(),
                1
            );
            let snake = game.world.entity(enemy).unwrap();
            // Initial source xVel2.5 moves three pixels at this clock; the
            // remaining three-pixel alignment still accepts the collision.
            assert_eq!(snake.pos, Vec2::new(99.0, 128.0));
            assert_eq!(snake.vel.y, -8.0);
            assert_eq!(snake.value("flying"), 0.0);
            assert_eq!(snake.health, 1);
        }
    }

    #[test]
    fn enemy_create_flying_defaults_match_all_source_descendants_and_run_once() {
        let mut checked = 0;
        for object in
            crate::catalog::object_names().filter(|object| *object != "oEnemy" && is_enemy(object))
        {
            let mut enemy = Entity::new(1, object, Vec2::new(96.0, 144.0));
            init_entity(&mut enemy);
            assert_eq!(
                enemy.data.get("flying"),
                Some(&f64::from(matches!(object, "oBat" | "oUFO"))),
                "{object}"
            );
            // A repeated bootstrap must preserve a caller's post-Create write.
            enemy.data.insert("flying".into(), 7.0);
            init_entity(&mut enemy);
            assert_eq!(enemy.value("flying"), 7.0, "{object}");
            checked += 1;
        }
        assert_eq!(checked, 28);
    }

    // These fixtures use only initial scene setup followed by ordinary input.
    // No hit boxes are spawned manually, and no family/collision callback is
    // invoked directly. Stop at the first hit, before later bounce/death blood.
    fn custom_melee_blood_room(object: &str, rear: bool, rules: RulesPreset) -> (Game, u64) {
        let mut game = Game::shell(42, rules);
        game.world = World::new(20, 16, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..20 {
            game.world.set_tile(x, 10, "oBlock");
        }
        let large = matches!(object, "oGiantSpider" | "oTombLord");
        let upper_ledge = object == "oTombLord" && !rear;
        if upper_ledge {
            // Source front-whip gate requires other.y < TombLord.y + 12.
            // The ledge supports the player without touching the boss center.
            game.world.set_tile(4, 9, "oBlock");
            game.world.set_tile(5, 9, "oBlock");
        }
        game.player.pos = Vec2::new(
            if upper_ledge { 86.0 } else { 80.0 },
            if upper_ledge { 136.0 } else { 152.0 },
        );
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.player.facing = 1;
        let target = game.spawn(
            object,
            Vec2::new(
                if rear {
                    if large { 40.0 } else { 56.0 }
                } else if upper_ledge {
                    92.0
                } else {
                    96.0
                },
                if large { 128.0 } else { 144.0 },
            ),
        );
        let actor = game.world.entity_mut(target).unwrap();
        // Valid idle/waiting states keep the scene still during one attack.
        actor.vel = Vec2::default();
        actor.timer = 200;
        if object == "oYeti" {
            // Yeti Step uses its source counter variable, rather than timer.
            actor.data.insert("counter".into(), 50.0);
        }
        actor.data.insert("sight_timer".into(), 200.0);
        actor.data.insert("hit_cooldown".into(), 0.0);
        set_counts_as_kill(actor, false);
        if object == "oGiantSpider" {
            actor.state = 2;
            actor.alarms[0] = 100;
        }
        (game, target)
    }

    fn custom_melee_blood_fill(game: &mut Game, count: usize) {
        for _ in 0..count {
            // Real detritus, distant from the fight and safely above the floor.
            game.spawn("oBone", Vec2::new(240.0, 64.0));
        }
    }

    fn custom_melee_blood_pick_up_machete(game: &mut Game) {
        let id = game.spawn("oMachete", game.player.pos);
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(id));
        assert!(game.world.entity(id).unwrap().held);
        game.step(&InputFrame::default());
    }

    fn custom_melee_blood_first_hit(game: &mut Game, target: u64, hitbox: &str) -> Rng {
        let hp = game.world.entity(target).unwrap().health;
        for frame in 0..18 {
            let before = game.rng.clone();
            game.step(&InputFrame {
                attack: frame == 0,
                attack_pressed: frame == 0,
                ..Default::default()
            });
            let actor = game
                .world
                .entity(target)
                .expect("victim survives the hit frame");
            if actor.health != hp {
                assert!(
                    game.world
                        .entities
                        .iter()
                        .any(|e| e.alive && e.object == hitbox),
                    "the input must create the expected source attack: {hitbox}"
                );
                assert_eq!(game.player.health, 4, "fixture must avoid body contact");
                return before;
            }
            assert!(
                source_contact_live_blood(game).is_empty(),
                "no earlier blood producer"
            );
        }
        panic!("ordinary input did not reach the target with {hitbox}");
    }

    // Independent oracle from oBlood/Create's literal random(4)-random(4),
    // -1-random(2), rand(1,6)*0.1. Do not call the production blood helper.
    fn custom_melee_blood_create_oracle(rng: &mut Rng) -> (Vec2, f64) {
        let velocity = Vec2::new(rng.unit() * 4.0 - rng.unit() * 4.0, -1.0 - rng.unit() * 2.0);
        let gravity = f64::from(1 + rng.range(6)) * 0.1;
        (velocity, gravity)
    }

    #[test]
    fn actual_step_custom_melee_blood_caveman_hawkman_front_and_rear_guards() {
        // Both custom XML events: subtract HP; if bloodLeft>0 call script;
        // decrement only for post-hit HP<0; then countsAsKill/stun/sound.
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oCaveman", "oHawkman"] {
                for rear in [false, true] {
                    for (detritus, bloodless, left) in [
                        (0, false, 1.0),
                        (15, false, 1.0),
                        (16, false, 1.0),
                        (0, true, 1.0),
                        (16, true, 1.0),
                        (0, false, 0.0),
                    ] {
                        let (mut game, target) = custom_melee_blood_room(object, rear, rules);
                        custom_melee_blood_fill(&mut game, detritus);
                        let actor = game.world.entity_mut(target).unwrap();
                        let hp = actor.health;
                        actor.data.insert("bloodless".into(), f64::from(bloodless));
                        actor.data.insert("blood_left".into(), left);
                        let mut expected_rng = custom_melee_blood_first_hit(
                            &mut game,
                            target,
                            if rear { "oWhipPre" } else { "oWhip" },
                        );
                        let emits = left > 0.0 && !bloodless && detritus < 16;
                        let expected_create =
                            emits.then(|| custom_melee_blood_create_oracle(&mut expected_rng));
                        let actor = game.world.entity(target).unwrap();
                        assert_eq!(actor.health, hp - 1, "{object} rear={rear}");
                        assert_eq!(actor.state, 98);
                        assert!(counts_as_kill(actor));
                        assert_eq!(actor.value("blood_left"), left);
                        let blood = source_contact_live_blood(&game);
                        assert_eq!(
                            blood.len(),
                            usize::from(emits),
                            "{object} rear={rear} detritus={detritus} bloodless={bloodless} left={left}"
                        );
                        if let Some((velocity, gravity)) = expected_create {
                            assert_eq!(
                                blood[0].pos,
                                Vec2::new(actor.pos.x + 8.0, actor.pos.y + 8.0)
                            );
                            assert_eq!(blood[0].vel, velocity);
                            assert_eq!(blood[0].value("gravity"), gravity);
                            assert_source_contact_blood_create(blood[0]);
                        }
                        assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
                    }
                }
            }
        }
    }

    #[test]
    fn actual_step_custom_melee_blood_natural_machete_post_hit_exhaustion_and_hawk_pose() {
        // Initial stunned bodies are legal Machete targets. Their source Step
        // selects the resting pose before the natural rear-blade collision.
        // Hawk Stun is 24x16 with origin(4,0): source center is x+12,y+8,
        // irrespective of the current opaque mask width or sprite origin.
        for object in ["oCaveman", "oHawkman"] {
            for hp in [1, 2] {
                for (detritus, bloodless, left) in [
                    (0, false, 1.0),
                    (16, false, 1.0),
                    (0, true, 1.0),
                    (0, false, 0.0),
                ] {
                    let (mut game, target) =
                        custom_melee_blood_room(object, true, RulesPreset::Classic);
                    let actor = game.world.entity_mut(target).unwrap();
                    actor.health = hp;
                    actor.state = 98;
                    actor.data.insert("bounced".into(), 1.0);
                    actor.data.insert("bloodless".into(), f64::from(bloodless));
                    actor.data.insert("blood_left".into(), left);
                    custom_melee_blood_fill(&mut game, detritus);
                    custom_melee_blood_pick_up_machete(&mut game);
                    assert_eq!(game.world.entity(target).unwrap().health, hp);
                    assert!(source_contact_live_blood(&game).is_empty());
                    let mut expected_rng =
                        custom_melee_blood_first_hit(&mut game, target, "oMachetePre");
                    let emits = left > 0.0 && !bloodless && detritus < 16;
                    if emits {
                        custom_melee_blood_create_oracle(&mut expected_rng);
                    }
                    let actor = game.world.entity(target).unwrap();
                    assert_eq!(actor.health, hp - 2);
                    assert_eq!(
                        actor.value("blood_left"),
                        left - f64::from(left > 0.0 && hp - 2 < 0)
                    );
                    assert!(counts_as_kill(actor));
                    let blood = source_contact_live_blood(&game);
                    assert_eq!(blood.len(), usize::from(emits));
                    if emits {
                        assert_eq!(
                            blood[0].pos,
                            Vec2::new(
                                actor.pos.x + if object == "oHawkman" { 12.0 } else { 8.0 },
                                actor.pos.y + 8.0
                            )
                        );
                        assert_source_contact_blood_create(blood[0]);
                    }
                    assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
                }
            }
        }
    }

    #[test]
    fn actual_step_custom_melee_blood_giant_spider_tomb_lord_direct_caps_and_credit() {
        // Front overrides: Spider subtracts literal 1 and preserves credit;
        // TombLord subtracts other.damage and restores credit. Direct Create
        // ignores caller bloodless, bloodLeft, and script cap16, but inherits
        // oDetritus' >32 destruction. Blood Create RNG still runs at cap32.
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oGiantSpider", "oTombLord"] {
                for detritus in [0, 16, 31, 32] {
                    for bloodless in [false, true] {
                        let (mut game, target) = custom_melee_blood_room(object, false, rules);
                        custom_melee_blood_fill(&mut game, detritus);
                        let actor = game.world.entity_mut(target).unwrap();
                        let hp = actor.health;
                        actor.data.insert("bloodless".into(), f64::from(bloodless));
                        actor.data.insert("blood_left".into(), 0.0);
                        let mut expected_rng =
                            custom_melee_blood_first_hit(&mut game, target, "oWhip");
                        let (velocity, gravity) =
                            custom_melee_blood_create_oracle(&mut expected_rng);
                        let actor = game.world.entity(target).unwrap();
                        assert_eq!(actor.health, hp - 1);
                        assert_eq!(counts_as_kill(actor), object == "oTombLord");
                        assert_eq!(actor.value("blood_left"), 0.0);
                        assert_eq!(actor.value("hit_cooldown"), 10.0);
                        let blood = source_contact_live_blood(&game);
                        assert_eq!(
                            blood.len(),
                            usize::from(detritus < 32),
                            "{object} detritus={detritus} bloodless={bloodless}"
                        );
                        if let Some(drop) = blood.first() {
                            assert_eq!(drop.pos, Vec2::new(actor.pos.x + 16.0, actor.pos.y + 24.0));
                            assert_eq!(drop.vel, velocity);
                            assert_eq!(drop.value("gravity"), gravity);
                            assert_source_contact_blood_create(drop);
                        }
                        assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
                    }
                }
            }
        }
    }

    #[test]
    fn actual_step_custom_melee_blood_boss_rear_is_inherited_script_not_direct() {
        for object in ["oGiantSpider", "oTombLord"] {
            for (detritus, bloodless) in [(0, false), (16, false), (0, true)] {
                let (mut game, target) =
                    custom_melee_blood_room(object, true, RulesPreset::Classic);
                custom_melee_blood_fill(&mut game, detritus);
                game.world
                    .entity_mut(target)
                    .unwrap()
                    .data
                    .insert("bloodless".into(), f64::from(bloodless));
                let hp = game.world.entity(target).unwrap().health;
                custom_melee_blood_first_hit(&mut game, target, "oWhipPre");
                let actor = game.world.entity(target).unwrap();
                assert_eq!(actor.health, hp - 1);
                assert!(counts_as_kill(actor));
                assert_eq!(actor.value("hit_cooldown"), 0.0);
                assert_eq!(
                    source_contact_live_blood(&game).len(),
                    usize::from(detritus < 16 && !bloodless)
                );
            }
        }
    }

    #[test]
    fn actual_step_custom_melee_blood_no_hit_no_blood_and_plain_yeti_is_bloodless_bump() {
        let (mut game, target) = custom_melee_blood_room("oTombLord", false, RulesPreset::Classic);
        // Remove the upper ledge: forward whip is below the source y+12 gate.
        game.world.set_tile(4, 9, "");
        game.world.set_tile(5, 9, "");
        game.player.pos.y = 152.0;
        game.player.previous_pos = game.player.pos;
        for frame in 0..18 {
            game.step(&InputFrame {
                attack: frame == 0,
                attack_pressed: frame == 0,
                ..Default::default()
            });
            assert_eq!(game.world.entity(target).unwrap().health, 20);
            assert_eq!(
                game.world.entity(target).unwrap().value("hit_cooldown"),
                0.0
            );
            assert!(source_contact_live_blood(&game).is_empty());
        }
        for rear in [false, true] {
            let (mut game, target) = custom_melee_blood_room("oYeti", rear, RulesPreset::Classic);
            let hp = game.world.entity(target).unwrap().health;
            let mut accepted = false;
            for frame in 0..18 {
                game.step(&InputFrame {
                    attack: frame == 0,
                    attack_pressed: frame == 0,
                    ..Default::default()
                });
                assert_eq!(game.world.entity(target).unwrap().health, hp);
                assert!(source_contact_live_blood(&game).is_empty());
                if game.world.entity(target).unwrap().value("hit_cooldown") > 0.0 {
                    accepted = true;
                    break;
                }
            }
            assert!(accepted, "plain Yeti whip must still reach its bump branch");
        }
        let (mut game, target) = custom_melee_blood_room("oManTrap", true, RulesPreset::Classic);
        custom_melee_blood_first_hit(&mut game, target, "oWhipPre");
        assert!(
            source_contact_live_blood(&game).is_empty(),
            "ManTrap has a Leaf producer, never Blood"
        );
    }
    #[test]
    fn actual_step_inherited_rear_whip_blood_uses_full_current_image_center() {
        // Basis/oEnemy/WhipPre: subtract HP; guarded scrCreateBlood at
        // x+sprite_width/2,y+sprite_height/2; hp<0 exhaustion; then sound.
        // Original GiantSpider PNG is32x32; its separate frame0 mask occupies
        // only x1..30,y18..31. The source center remains x+16,y+16.
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (detritus, bloodless, left) in [
                (0, false, 1.0),
                (15, false, 1.0),
                (16, false, 1.0),
                (0, true, 1.0),
                (0, false, 0.0),
            ] {
                let mut game = Game::shell(42, rules);
                game.world = World::new(20, 16, Area::Mines, 1);
                game.world.name = "rLevel".into();
                for x in 0..20 {
                    game.world.set_tile(x, 10, "oBlock");
                }
                game.player.pos = Vec2::new(80.0, 152.0);
                game.player.previous_pos = game.player.pos;
                game.player.state = PlayerState::Standing;
                game.player.grounded = true;
                game.player.facing = 1;
                let target = game.spawn("oGiantSpider", Vec2::new(40.0, 128.0));
                let actor = game.world.entity_mut(target).unwrap();
                actor.state = 2;
                actor.vel = Vec2::default();
                actor.alarms[0] = 100;
                actor.data.insert("hit_cooldown".into(), 0.0);
                actor.data.insert("bloodless".into(), f64::from(bloodless));
                actor.data.insert("blood_left".into(), left);
                set_counts_as_kill(actor, false);
                for _ in 0..detritus {
                    game.spawn("oBone", Vec2::new(240.0, 64.0));
                }
                let mut hit = false;
                for frame in 0..6 {
                    let mut expected_rng = game.rng.clone();
                    game.step(&InputFrame {
                        attack: frame == 0,
                        attack_pressed: frame == 0,
                        ..Default::default()
                    });
                    let actor = game.world.entity(target).unwrap();
                    if actor.health == 1 {
                        continue;
                    }
                    assert_eq!(actor.health, 0);
                    assert!(counts_as_kill(actor));
                    assert_eq!(
                        actor.value("blood_left"),
                        left,
                        "zero HP does not exhaust bloodLeft"
                    );
                    assert_eq!(
                        actor.value("hit_cooldown"),
                        0.0,
                        "inherited rear has no front cooldown"
                    );
                    assert!(
                        game.world
                            .entities
                            .iter()
                            .any(|e| e.alive && e.object == "oWhipPre"),
                        "real input must create rear whip"
                    );
                    assert_eq!(game.player.health, 4, "no body collision masks the fixture");
                    let emits = left > 0.0 && !bloodless && detritus < 16;
                    let blood = source_contact_live_blood(&game);
                    assert_eq!(blood.len(), usize::from(emits));
                    if emits {
                        assert_eq!(
                            blood[0].pos,
                            Vec2::new(actor.pos.x + 16.0, actor.pos.y + 16.0)
                        );
                        // Independently transcribed oBlood/Create random draws.
                        let velocity = Vec2::new(
                            expected_rng.unit() * 4.0 - expected_rng.unit() * 4.0,
                            -1.0 - expected_rng.unit() * 2.0,
                        );
                        let gravity = f64::from(1 + expected_rng.range(6)) * 0.1;
                        assert_eq!(blood[0].vel, velocity);
                        assert_eq!(blood[0].value("gravity"), gravity);
                        assert_source_contact_blood_create(blood[0]);
                    }
                    assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
                    hit = true;
                    break;
                }
                assert!(
                    hit,
                    "fresh rear-whip input must hit the waiting GiantSpider"
                );
            }
        }
    }
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
            // The later-resource newborn Eject moves to (88,82), then always
            // rolls rand(1,5); a successful roll draws four coordinate offsets.
            let expected_burn = if expected_rng.range(5) == 0 {
                let x = 88.0 + f64::from(expected_rng.range(4)) - f64::from(expected_rng.range(4));
                let y = 82.0 + f64::from(expected_rng.range(4)) - f64::from(expected_rng.range(4));
                Some(Vec2::new(x, y))
            } else {
                None
            };

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
            let burns: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oBurn")
                .collect();
            assert_eq!(burns.len(), usize::from(expected_burn.is_some()));
            if let Some(pos) = expected_burn {
                assert_eq!(burns[0].pos, pos);
                assert_eq!(burns[0].age, 0, "earlier Burn resource has no newborn Step");
            }
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

    fn source_contact_blood_room(rules: RulesPreset, object: &str) -> (Game, u64) {
        let mut game = Game::shell(42, rules);
        game.world = World::new(20, 16, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..20 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(136.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.grounded = true;
        if object == "oPiranha" {
            // Preserve the real fish Step instead of its dry-land FishBone exit.
            for x in 7..=10 {
                game.world.set_tile(x, 9, "oWater");
            }
            game.player.data.insert("bubble_timer".into(), 20.0);
        }
        let id = game.spawn(object, Vec2::new(128.0, 144.0));
        let actor = game.world.entity_mut(id).unwrap();
        actor.vel = Vec2::default();
        actor.timer = 37;
        actor.data.insert("bubble_timer".into(), 20.0);
        (game, id)
    }

    fn source_contact_live_blood(game: &Game) -> Vec<&Entity> {
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oBlood")
            .collect()
    }

    fn assert_source_contact_blood_create(blood: &Entity) {
        assert!(blood.engine_active, "the next Draw must see the newborn");
        assert_eq!(blood.sprite, "sBlood");
        assert_eq!(blood.value("world_effect_initialized"), 1.0);
        assert_eq!(blood.alarms, [1, 1, 5, -1, -1, -1, -1, -1, -1, -1, -1, -1]);
        assert_eq!(blood.value("collectible"), 0.0);
        assert!((-4.0..4.0).contains(&blood.vel.x));
        assert!((-3.0..=-1.0).contains(&blood.vel.y));
        assert!((0.1..=0.6 + f64::EPSILON).contains(&blood.value("gravity")));
    }

    #[test]
    fn actual_step_effective_type_gates_source_contact_blood() {
        // oEnemy/Character compares type, not object_index. Piranha's own
        // Create sets its type; Bat inherits "None", and Snake is not in the
        // contact-blood type list. The script still applies bloodless/cap16.
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oBat", "oPiranha", "oSnake"] {
                for (detritus, bloodless, invincible) in [
                    (0, false, false),
                    (15, false, false),
                    (16, false, false),
                    (0, true, false),
                    (0, false, true),
                ] {
                    let (mut game, id) = source_contact_blood_room(rules, object);
                    game.world
                        .entity_mut(id)
                        .unwrap()
                        .data
                        .insert("bloodless".into(), f64::from(bloodless));
                    if invincible {
                        game.player.invincible = 20;
                    }
                    for _ in 0..detritus {
                        game.spawn("oBone", Vec2::new(280.0, 48.0));
                    }
                    game.step(&InputFrame::default());
                    let actor = game
                        .world
                        .entity(id)
                        .expect("contact is not an animal kill");
                    assert_eq!(actor.health, 1, "{object}");
                    assert_eq!(
                        game.player.health,
                        if invincible { 4 } else { 3 },
                        "{object}"
                    );
                    let expected = usize::from(
                        object == "oPiranha" && !invincible && !bloodless && detritus < 16,
                    );
                    let blood = source_contact_live_blood(&game);
                    assert_eq!(
                        blood.len(),
                        expected,
                        "{object} detritus={detritus} bloodless={bloodless} invincible={invincible}"
                    );
                    if expected == 1 {
                        assert_eq!(
                            blood[0].pos,
                            Vec2::new(actor.pos.x + 4.0, actor.pos.y + 4.0)
                        );
                        assert_source_contact_blood_create(blood[0]);
                    }
                }
            }
        }
    }

    fn fresh_type_contact_room(rules: RulesPreset, player_x: f64, health: i32) -> Game {
        let mut game = Game::shell(42, rules);
        game.world = World::new(20, 16, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBlock");
        }
        // Authored starting conditions, before any actor is created or stepped.
        game.player.pos = Vec2::new(player_x, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.player.health = health;
        game
    }

    #[test]
    fn actual_step_fresh_bat_none_contact_preserves_hurt_without_blood_rng() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = fresh_type_contact_room(rules, 104.0, 4);
            let id = game.spawn("oBat", Vec2::new(96.0, 144.0));
            let expected_rng = game.rng.clone();
            game.step(&InputFrame::default());

            let bat = game.world.entity(id).unwrap();
            assert_eq!(crate::items::explosives::source_type(bat), "None");
            assert_eq!(bat.health, 1);
            assert_eq!(bat.state, 1, "no ceiling naturally wakes the fresh Bat");
            assert_eq!(
                bat.sprite, "sBatHang",
                "wake does not change this image yet"
            );
            assert_eq!(game.player.health, 3);
            assert_eq!(game.player.invincible, 30);
            assert_eq!(game.player.data.get("blink"), Some(&30.0));
            assert_eq!(game.player.stun, 0);
            assert_eq!(game.player.vel.x, 6.0);
            assert!(game.events.contains(&GameEvent::Damage { amount: 1 }));
            assert!(game.events.iter().any(|event| matches!(
                event, GameEvent::Sound { name, .. } if name == "sndHurt"
            )));
            assert!(source_contact_live_blood(&game).is_empty());
            assert_eq!(game.rng.clone().next_u64(), expected_rng.clone().next_u64());
        }
    }

    #[test]
    fn actual_step_fresh_lethal_contacts_use_create_type_and_late_death_timing() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (object, player_x, typed_row) in [
                ("oBat", 104.0, None),
                ("oZombie", 104.0, None),
                ("oSpiderHang", 112.0, None),
                ("oGiantSpiderHang", 104.0, None),
                ("oSnake", 104.0, Some(crate::statistics::EnemyDeath::Snake)),
            ] {
                let mut game = fresh_type_contact_room(rules, player_x, 1);
                if matches!(object, "oSpiderHang" | "oGiantSpiderHang") {
                    // Exact8px from the drop center fails the source's strict
                    // abs<8 condition; the real ceiling keeps this form hanging.
                    game.world.set_tile(6, 8, "oBlock");
                }
                let id = game.spawn(object, Vec2::new(96.0, 144.0));
                game.step(&InputFrame::default());

                let actor = game.world.entity(id).unwrap();
                assert_eq!(actor.object, object, "{object} must not transform");
                assert_eq!(
                    crate::items::explosives::source_type(actor),
                    if typed_row.is_some() { "Snake" } else { "None" },
                    "{object}"
                );
                assert_eq!(game.player.health, 0, "{object}");
                assert_eq!(game.player.invincible, 30, "{object}");
                assert_eq!(game.player.data.get("blink"), Some(&30.0), "{object}");
                assert!(game.events.contains(&GameEvent::Damage { amount: 1 }));
                assert!(source_contact_live_blood(&game).is_empty(), "{object}");
                let mut expected_rows = [0; 24];
                if let Some(row) = typed_row {
                    expected_rows[row as usize] = 1;
                }
                assert_eq!(game.lifetime_stats.enemy_deaths, expected_rows, "{object}");
                // Automatic Character collision follows Player Step. Lethal
                // contact is observed by the next real Player Step, not here.
                assert_eq!(game.state, RunState::Playing, "{object}");
                assert!(!game.events.contains(&GameEvent::Died), "{object}");
                game.step(&InputFrame::default());
                assert_eq!(game.state, RunState::Dead, "{object}");
                assert!(game.events.contains(&GameEvent::Died), "{object}");
                assert_eq!(game.lifetime_stats.enemy_deaths, expected_rows, "{object}");
            }
        }
    }

    #[test]
    fn actual_step_fresh_piranha_typed_contact_keeps_blood_create_rng() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = fresh_type_contact_room(rules, 104.0, 4);
            for x in 5..=8 {
                game.world.set_tile(x, 9, "oWaterSwim");
            }
            let id = game.spawn("oPiranha", Vec2::new(96.0, 144.0));
            let mut expected_rng = game.rng.clone();
            // Fresh submerged Player and fish each create a Bubble. Fish Step
            // then resets bubbleTimer to rand(30,50), before contact Blood.
            expected_rng.range(3);
            expected_rng.range(3);
            expected_rng.range(21);
            let blood_velocity = Vec2::new(
                expected_rng.unit() * 4.0 - expected_rng.unit() * 4.0,
                -1.0 - expected_rng.unit() * 2.0,
            );
            let blood_gravity = f64::from(1 + expected_rng.range(6)) * 0.1;
            game.step(&InputFrame::default());

            let fish = game.world.entity(id).unwrap();
            assert_eq!(crate::items::explosives::source_type(fish), "Piranha");
            assert_eq!(fish.health, 1);
            assert_eq!(game.player.health, 3);
            assert_eq!(game.player.invincible, 30);
            assert_eq!(game.player.data.get("blink"), Some(&30.0));
            let blood = source_contact_live_blood(&game);
            assert_eq!(blood.len(), 1);
            assert_eq!(blood[0].pos, Vec2::new(fish.pos.x + 4.0, fish.pos.y + 4.0));
            assert_eq!(blood[0].vel, blood_velocity);
            assert_eq!(blood[0].value("gravity"), blood_gravity);
            assert_source_contact_blood_create(blood[0]);
            assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
        }
    }

    #[test]
    fn actual_step_spike_stomp_source_contact_blood() {
        // Literal parent Character line9 calls scrCreateBlood(player.x,
        // player.y+8,1) only with SpikeShoes. The animal's next Step still
        // produces its separate three-drop death burst in both cases.
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oSnake", "oFrog"] {
                for spiked in [false, true] {
                    for bloodless in [false, true] {
                        let (mut game, id) = source_contact_blood_room(rules, object);
                        game.player.pos.y = 144.0;
                        game.player.previous_pos = game.player.pos;
                        game.player.grounded = false;
                        game.player.state = PlayerState::Falling;
                        if spiked {
                            game.player.equipment.insert("SpikeShoes".into());
                        }
                        game.world
                            .entity_mut(id)
                            .unwrap()
                            .data
                            .insert("bloodless".into(), f64::from(bloodless));
                        game.step(&InputFrame::default());
                        let actor = game
                            .world
                            .entity(id)
                            .expect("death waits for the animal's next Step");
                        assert_eq!(actor.health, if spiked { -2 } else { 0 }, "{object}");
                        assert_eq!(game.player.health, 4);
                        assert!(game.player.vel.y < 0.0, "actual stomp bounces the player");
                        let blood = source_contact_live_blood(&game);
                        assert_eq!(blood.len(), usize::from(spiked && !bloodless), "{object}");
                        if let Some(drop) = blood.first() {
                            assert_eq!(
                                drop.pos,
                                Vec2::new(game.player.pos.x, game.player.pos.y + 8.0)
                            );
                            assert_source_contact_blood_create(drop);
                        }
                        let contact_ids: Vec<_> = blood.iter().map(|e| e.id).collect();
                        game.step(&InputFrame::default());
                        assert!(game.world.entity(id).is_none(), "{object}");
                        let death: Vec<_> = source_contact_live_blood(&game)
                            .into_iter()
                            .filter(|e| !contact_ids.contains(&e.id))
                            .collect();
                        assert_eq!(death.len(), if bloodless { 0 } else { 3 }, "{object}");
                        for drop in death {
                            assert_source_contact_blood_create(drop);
                        }
                    }
                }
            }
        }
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

    #[test]
    fn shared_move_body_reciprocal_rounding_uses_gml_nearest_even_vectors() {
        // Independent binary64 vectors from moveTo.gml and the named GM8 round
        // handler (FISTP, startup CW 0x1332). These are arithmetic probes, not
        // claims that every velocity occurs in an authored enemy trajectory.
        let world = World::new(40, 40, Area::Mines, 1);
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
            let mut actor = Entity::new(1, "oFrog", origin);
            actor.vel = Vec2::new(velocity, velocity);
            actor.data.insert("simulation_tick".into(), tick as f64);
            move_body(&world, &mut actor, 0.0);
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
    fn naturally_falling_frog_uses_even_reciprocal_period_in_both_presets() {
        // Source Frog Create starts at yVel=0; each falling Step calls moveTo
        // before adding myGrav=.2. No actor state/velocity is injected here.
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = Game::shell(7, rules);
            game.world = World::new(20, 20, Area::Jungle, 5);
            for x in 0..20 {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.player.pos = Vec2::new(40.0, 152.0);
            game.player.previous_pos = game.player.pos;
            let id = game.spawn("oFrog", Vec2::new(160.0, 64.0));
            let frog = game.world.entity(id).unwrap();
            assert_eq!((frog.state, frog.timer, frog.vel), (0, 0, Vec2::default()));
            for expected_velocity in [0.2, 0.4] {
                game.step(&InputFrame::default());
                let frog = game.world.entity(id).unwrap();
                assert_eq!(frog.pos, Vec2::new(160.0, 64.0));
                assert_eq!(frog.vel.y, expected_velocity);
            }
            assert_eq!(game.engine_time(), 3);
            assert_eq!(1.0 / game.world.entity(id).unwrap().vel.y, 2.5);
            game.step(&InputFrame::default());
            assert_eq!(game.engine_time(), 4);
            assert_eq!(game.world.entity(id).unwrap().pos, Vec2::new(160.0, 65.0));
            assert_eq!(game.player.health, 4);
        }
    }
}

// Scenes use Classic 1.1 birth data and ordinary input.
#[cfg(test)]
mod source_motion_regressions {
    use super::*;

    fn room(seed: u64, rules: RulesPreset, player_x: f64) -> Game {
        let mut game = Game::shell(seed, rules);
        game.world = World::new(20, 16, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..20 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(player_x, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game
    }

    fn water(game: &mut Game, first_row: i32) {
        for y in first_row..=9 {
            for x in 4..=10 {
                game.world.set_tile(x, y as usize, "oWaterSwim");
            }
        }
    }

    fn boings(game: &Game) -> usize {
        game.events
            .iter()
            .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndBoing"))
            .count()
    }

    #[test]
    fn initial_spider_flip_bounds_miss_spring_while_supported_gravity_survives() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = room(42, rules, 240.0);
            let spring = game.spawn("oSpringTrap", Vec2::new(96.0, 144.0));
            let spider = game.spawn("oSpider", Vec2::new(96.0, 144.0));
            let initial = game.world.entity(spider).unwrap();
            assert_eq!(initial.sprite, "sSpiderFlip");
            assert_eq!(initial.sprite_bounds(), Rect::new(96.0, 144.0, 16.0, 10.0));
            assert_eq!(initial.bounds().y + initial.bounds().h, 160.0);
            assert!(!initial.event_intersects(game.world.entity(spring).unwrap()));
            game.step(&crate::InputFrame::default());
            let actor = game.world.entity(spider).unwrap();
            assert_eq!(actor.pos, Vec2::new(96.0, 144.0));
            assert_eq!(actor.sprite, "sSpiderFlip");
            assert_eq!(actor.vel, Vec2::new(0.0, 0.2));
            assert_eq!(actor.state, 2);
            let spring = game.world.entity(spring).unwrap();
            assert_eq!((spring.state, spring.timer), (0, 0));
            assert_eq!(boings(&game), 0);
        }
    }

    #[test]
    fn initial_spider_flip_finishes_into_its_normal_sprite_through_full_steps() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = room(42, rules, 240.0);
            let spider = game.spawn("oSpider", Vec2::new(96.0, 144.0));
            assert_eq!(game.world.entity(spider).unwrap().sprite, "sSpiderFlip");
            // Seed42 selects alarm10, then launch speed -5. Alarm0 launches
            // even while the player is beyond the Step bounce-distance gate.
            for step in 1..=24 {
                game.step(&crate::InputFrame::default());
                let actor = game.world.entity(spider).unwrap();
                if step == 1 {
                    assert_eq!(actor.alarms[0], 10);
                } else if step == 10 {
                    assert_eq!(actor.alarms[0], 1);
                    assert_eq!(actor.pos, Vec2::new(96.0, 144.0));
                } else if step == 11 {
                    // oGame.time starts at1 and advances before movement:
                    // time12 schedules3 horizontal pixels and5 upward pixels.
                    assert_eq!(game.engine_time(), 12);
                    assert_eq!(actor.pos, Vec2::new(99.0, 139.0));
                    assert_eq!(actor.vel.x, 2.5);
                    assert!((actor.vel.y + 4.8).abs() < 1e-12);
                } else if step == 22 {
                    assert_eq!(actor.sprite, "sSpiderFlip");
                } else if step == 23 {
                    assert_eq!(actor.sprite, "sSpider");
                }
            }
            let actor = game.world.entity(spider).unwrap();
            assert_eq!(actor.sprite, "sSpider");
            assert!(actor.alive);
            // Fourteen natural launch Steps, using moveTo's source-time
            // fractional schedule and post-move gravity .2, end here.
            assert_eq!(actor.pos, Vec2::new(131.0, 92.0));
            assert_eq!(actor.vel.x, 2.5);
            assert!((actor.vel.y + 2.2).abs() < 1e-12);
        }
    }

    #[test]
    fn supported_spider_accumulates_gravity_through_zero_and_blocked_displacements() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = room(42, rules, 240.0);
            let spider = game.spawn("oSpider", Vec2::new(96.0, 144.0));
            // These four Steps precede even the shortest Create-selected alarm.
            // At vy=.4, a scheduled pixel at an even source clock is blocked.
            for step in 1..=4 {
                game.step(&crate::InputFrame::default());
                let actor = game.world.entity(spider).unwrap();
                assert_eq!(actor.pos, Vec2::new(96.0, 144.0));
                assert!((actor.vel.y - f64::from(step) * 0.2).abs() < 1e-12);
                assert_eq!(actor.vel.x, 0.0);
                assert_eq!(actor.state, 2);
            }
        }
    }

    #[test]
    fn initial_giant_spider_flip_bounds_miss_spring_and_keep_source_health_one() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = room(42, rules, 240.0);
            let spring = game.spawn("oSpringTrap", Vec2::new(96.0, 144.0));
            let spider = game.spawn("oGiantSpider", Vec2::new(96.0, 128.0));
            let initial = game.world.entity(spider).unwrap();
            assert_eq!(initial.sprite, "sGiantSpiderFlip");
            assert_eq!(initial.sprite_bounds(), Rect::new(97.0, 128.0, 30.0, 15.0));
            assert_eq!(initial.bounds().y + initial.bounds().h, 160.0);
            assert_eq!(initial.health, 1);
            assert!(!initial.event_intersects(game.world.entity(spring).unwrap()));
            game.step(&crate::InputFrame::default());
            let actor = game.world.entity(spider).unwrap();
            assert_eq!(actor.sprite, "sGiantSpiderFlip");
            assert_eq!(actor.pos, Vec2::new(96.0, 128.0));
            assert_eq!(actor.vel.y, 0.3);
            assert_eq!(actor.health, 1);
            assert_eq!(game.world.entity(spring).unwrap().state, 0);
            assert_eq!(boings(&game), 0);
        }
    }

    #[test]
    fn giant_spider_flip_finishes_before_its_natural_alarm_and_slows_animation() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            // Seed1's first draw creates squirtTimer; its next draw selects
            // alarm12. Nine Flip images at speed.8 finish in twelve Steps,
            // before that alarm fires on the thirteenth ordinary Step.
            let mut game = room(1, rules, 240.0);
            let spider = game.spawn("oGiantSpider", Vec2::new(96.0, 128.0));
            assert_eq!(
                game.world.entity(spider).unwrap().sprite,
                "sGiantSpiderFlip"
            );
            for _ in 0..12 {
                game.step(&crate::InputFrame::default());
            }
            let actor = game.world.entity(spider).unwrap();
            assert_eq!(actor.sprite, "sGiantSpider");
            assert_eq!(actor.image_speed(), 0.4);
            assert_eq!(actor.health, 1);
            assert_eq!(actor.pos.y, 128.0);
        }
    }

    #[test]
    fn giant_spider_also_retains_post_move_gravity_on_support() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = room(42, rules, 240.0);
            let spider = game.spawn("oGiantSpider", Vec2::new(96.0, 128.0));
            for step in 1..=3 {
                game.step(&crate::InputFrame::default());
                let actor = game.world.entity(spider).unwrap();
                assert_eq!(actor.pos, Vec2::new(96.0, 128.0));
                assert!((actor.vel.y - f64::from(step) * 0.3).abs() < 1e-12);
                assert!(actor.alive);
            }
        }
    }

    #[test]
    fn native_spider_flip_overlap_springs_after_its_own_step_velocity_writes() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for with_spring in [false, true] {
                let mut game = room(42, rules, 240.0);
                let spring = with_spring.then(|| game.spawn("oSpringTrap", Vec2::new(96.0, 144.0)));
                let spider = game.spawn("oSpider", Vec2::new(96.0, 147.0));
                let initial = game.world.entity(spider).unwrap();
                assert_eq!(initial.sprite_bounds(), Rect::new(96.0, 147.0, 16.0, 10.0));
                if let Some(spring) = spring {
                    assert!(initial.event_intersects(game.world.entity(spring).unwrap()));
                }
                game.step(&crate::InputFrame::default());
                let actor = game.world.entity(spider).unwrap();
                assert_eq!(actor.health, 1);
                if let Some(spring) = spring {
                    assert_eq!(actor.pos, Vec2::new(96.0, 131.0));
                    assert_eq!(actor.vel, Vec2::new(-2.0, -8.0));
                    let spring = game.world.entity(spring).unwrap();
                    assert_eq!((spring.state, spring.timer), (1, 10));
                    assert_eq!(boings(&game), 1);
                } else {
                    assert_eq!(actor.pos, Vec2::new(96.0, 147.0));
                    assert_eq!(actor.vel, Vec2::new(-1.0, 0.2));
                    assert_eq!(boings(&game), 0);
                }
            }
        }
    }

    #[test]
    fn supported_nonstunned_families_keep_their_explicit_source_resets() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oSnake", "oCaveman", "oYeti", "oManTrap", "oHawkman"] {
                let mut game = room(42, rules, 240.0);
                let id = game.spawn(object, Vec2::new(96.0, 144.0));
                game.step(&crate::InputFrame::default());
                let actor = game.world.entity(id).unwrap();
                assert_eq!(actor.pos.y, 144.0, "{object}");
                assert_eq!(actor.vel.y, 0.0, "{object}");
                assert!(actor.alive, "{object}");
            }
        }
    }

    #[test]
    fn piranha_idle_displacement_keeps_zero_velocity_and_natural_spring_boost() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (seed, delta) in [(2, -1.0), (42, 1.0)] {
                for with_spring in [false, true] {
                    let mut game = room(seed, rules, 40.0);
                    water(&mut game, 8);
                    let spring =
                        with_spring.then(|| game.spawn("oSpringTrap", Vec2::new(96.0, 144.0)));
                    let fish = game.spawn("oPiranha", Vec2::new(96.0, 149.0));
                    assert_eq!(game.world.entity(fish).unwrap().vel, Vec2::default());
                    if let Some(spring) = spring {
                        assert!(
                            game.world
                                .entity(fish)
                                .unwrap()
                                .event_intersects(game.world.entity(spring).unwrap())
                        );
                    }
                    game.step(&crate::InputFrame::default());
                    let actor = game.world.entity(fish).unwrap();
                    assert_eq!(actor.state, 0);
                    let expected_y = if with_spring { 133.0 } else { 149.0 };
                    let expected_vel = if with_spring {
                        Vec2::new(-1.0, -8.0)
                    } else {
                        Vec2::default()
                    };
                    assert_eq!(actor.pos, Vec2::new(96.0 + delta, expected_y));
                    assert_eq!(actor.vel, expected_vel);
                    assert_eq!(boings(&game), usize::from(with_spring));
                    if let Some(spring) = spring {
                        let spring = game.world.entity(spring).unwrap();
                        assert_eq!((spring.state, spring.timer), (1, 10));
                    }
                    // The next literal moveTo still moves horizontally; it must
                    // preserve the collision's stored (-1,-8), not consume it.
                    game.step(&crate::InputFrame::default());
                    let actor = game.world.entity(fish).unwrap();
                    assert_eq!(actor.pos, Vec2::new(96.0 + 2.0 * delta, expected_y));
                    assert_eq!(actor.vel, expected_vel);
                }
            }
        }
    }

    #[test]
    fn piranha_natural_player_attack_and_pause_keep_stored_velocity() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = room(42, rules, 64.0);
            water(&mut game, 4);
            game.player.pos = Vec2::new(64.0, 120.0);
            game.player.previous_pos = game.player.pos;
            let fish = game.spawn("oPiranha", Vec2::new(96.0, 96.0));
            let mut saw_attack = false;
            for _ in 0..4 {
                game.step(&crate::InputFrame::default());
                let actor = game.world.entity(fish).unwrap();
                saw_attack |= actor.state == 1;
                assert_eq!(actor.vel, Vec2::default());
            }
            assert!(
                saw_attack,
                "ordinary player water entry must trigger ATTACK"
            );
            let mut saw_pause = false;
            let input = crate::InputFrame {
                left: true,
                ..Default::default()
            };
            for _ in 0..24 {
                game.step(&input);
                let actor = game.world.entity(fish).unwrap();
                assert_eq!(actor.vel, Vec2::default());
                if actor.state == 2 {
                    saw_pause = true;
                    break;
                }
            }
            assert!(
                saw_pause,
                "ordinary left input must exit water and trigger PAUSE"
            );
        }
    }
}
