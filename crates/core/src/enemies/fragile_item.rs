//! Jar/Skull replace oItem Step: their hit guards and final break latch differ.
use super::*;

fn selected_target(game: &mut Game, item: &Entity, family: &str) -> Option<u64> {
    crate::dispatcher::register_instances(game);
    let members = crate::dispatcher::descendant_instances(game, family);
    let first = Vec2::new(item.pos.x - 3.0, item.pos.y - 3.0);
    let second = Vec2::new(item.pos.x + 3.0, item.pos.y + 3.0);
    let overlaps = members.iter().any(|instance| {
        let Instance::Entity(id) = *instance else {
            return false;
        };
        game.world.entity(id).is_some_and(|target| {
            target.id != item.id
                && target.engine_active
                && target
                    .collision_mask()
                    .intersects_rectangle_bbox(first, second)
        })
    });
    if !overlaps {
        return None;
    }
    // collision_rectangle only gates the query. instance_nearest searches
    // the entire current parent array and keeps its first equal-distance entry.
    let mut nearest = None;
    let mut distance = f64::INFINITY;
    for instance in members {
        let Instance::Entity(id) = instance else {
            continue;
        };
        let Some(target) = game.world.entity(id) else {
            continue;
        };
        if target.id != item.id && target.engine_active {
            let candidate = target.pos.distance(item.pos);
            if candidate < distance {
                nearest = Some(id);
                distance = candidate;
            }
        }
    }
    nearest
}

/// Return the own Step's destruction latch; the caller runs final Destroy only
/// after both queries. A lava-destroyed receiver still finishes its event.
pub(crate) fn step_impacts(game: &mut Game, item_id: u64) -> bool {
    let Some(item) = game
        .world
        .entities
        .iter()
        .find(|e| e.id == item_id)
        .cloned()
    else {
        return false;
    };
    if !matches!(item.object.as_str(), "oJar" | "oSkull")
        || item.vel.x.abs() <= 2.0 && item.vel.y.abs() <= 2.0
    {
        return false;
    }
    let mut destroy = false;
    if let Some(target_id) = selected_target(game, &item, "oEnemy") {
        with_actor(game, target_id, |game, target| {
            if target.value("invincible") != 0.0 {
                return;
            }
            target.vel.x = item.vel.x;
            let kind = crate::items::explosives::source_type(target);
            if matches!(
                kind.as_str(),
                "Caveman" | "ManTrap" | "Yeti" | "Hawkman" | "Shopkeeper"
            ) {
                if target.state != 98 {
                    if kind != "ManTrap" {
                        // Direct instance_create ignores scrCreateBlood's
                        // bloodless/16 guards; Blood Create still runs normally.
                        game.spawn("oBlood", target.pos);
                    }
                    target.state = 98;
                    target.timer = target.value("stun_time") as u32;
                    target.vel.y = -6.0;
                    game.sound("sndHit", target.pos);
                }
            } else {
                game.spawn("oBlood", Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0));
                target.health -= 1;
                target.data.insert("origX".into(), target.pos.x);
                target.data.insert("origY".into(), target.pos.y);
                target.data.insert("shakeCounter".into(), 10.0);
                game.sound("sndHit", target.pos);
            }
            target.vel.x = item.vel.x * 0.3;
        });
        // This assignment is OUTSIDE the enemy's invincibility guard.
        destroy = true;
    }
    // The source independently queries Damsel AFTER the Enemy branch. It does
    // not gate damage/release/stun on invincibility, corpse state or cooldown.
    if let Some(target_id) = selected_target(game, &item, "oDamsel") {
        with_actor(game, target_id, |game, target| {
            if target.value("invincible") == 0.0 {
                game.spawn("oBlood", item.pos);
            }
            target.held = false;
            game.player.held = None;
            // The replacement Step deliberately retains pickupItemType here.
            target.health -= 1;
            target.vel.y = -6.0;
            target.state = 2;
            target.timer = 120;
            target.vel.x = item.vel.x * 0.3;
        });
        destroy = true;
    }
    destroy
}
