//! Live editor DarkFall collisions and the two effects they create.
//!
//! These callbacks are not disabled by `global.cleanSolids`. In particular,
//! protected oversized previews can hide an existing solid from paint's first
//! point query, allowing a newly painted DarkFall to overlap that solid.

use super::{PreviewInstance, PreviewRoom};
use crate::{CollisionPrecision, Entity, Vec2, catalog};
use std::collections::BTreeMap;

/// Normal Step runs before resource812 Editor/Step. Collision-born effects
/// therefore first run this body on the following source tick.
pub(super) fn step_effect(room: &PreviewRoom, instance: &mut PreviewInstance) {
    let Some(body) = instance.body.as_mut() else {
        return;
    };
    if instance.object == "oSmokePuff" {
        instance.pos.y -= body.vel.y;
    } else if instance.object == "oRubbleDark" {
        // Its own Step ignores xVel; OutsideRoom is a later Other event.
        instance.pos.y += body.vel.y;
        body.vel.y += 0.6;
        if point_family(room, instance.pos, "oBrick") || point_family(room, instance.pos, "oBlock")
        {
            instance.alive = false;
        }
    }
}

fn point_family(room: &PreviewRoom, point: Vec2, family: &str) -> bool {
    room.family_order.iter().copied().any(|id| {
        room.instance(id).is_some_and(|i| {
            catalog::is_a(i.object, family)
                && i.mask().contains(point, CollisionPrecision::BoundingBox)
        })
    })
}

/// Other0 follows Editor Step and precedes Collision. The original action203
/// tests the cached inclusive bbox; touching the room edge still survives.
pub(super) fn outside_room(room: &mut PreviewRoom) {
    let ids: Vec<_> = room
        .family_order
        .iter()
        .copied()
        .filter(|&id| {
            room.instance(id).is_some_and(|i| {
                if i.object != "oRubbleDark" {
                    return false;
                }
                let b = i.mask().point_bounds();
                b.x + b.w - 1.0 < 0.0 || b.x > 672.0 || b.y + b.h - 1.0 < 0.0 || b.y > 544.0
            })
        })
        .collect();
    for id in ids {
        room.destroy(id);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Participant {
    Preview(u64),
    Border(Vec2),
}

fn snapshot(room: &PreviewRoom, participant: Participant) -> Option<PreviewInstance> {
    match participant {
        Participant::Preview(id) => room.instance(id).cloned(),
        Participant::Border(pos) => Some(PreviewInstance {
            id: 0,
            tile: '0',
            object: "oHardBlock",
            sprite: "sBrick",
            pos,
            depth: 100,
            text: String::new(),
            alive: true,
            body: None,
        }),
    }
}

/// Automatic Collision follows Normal Step and builtin motion. The canonical
/// runner enumerates resource pairs ascending (539BB4), then each current
/// resource array forward; 4AD1A0(CL=1) tests precise masks and destroyed guards.
pub(super) fn collisions(room: &mut PreviewRoom) {
    let mut resources: BTreeMap<i32, Vec<Participant>> = BTreeMap::new();
    let border = resources
        .entry(catalog::object("oHardBlock").unwrap().id)
        .or_default();
    for y in 0..34 {
        for x in 0..42 {
            if x == 0 || y == 0 || x == 41 || y == 33 {
                border.push(Participant::Border(Vec2::new(
                    x as f64 * 16.0,
                    y as f64 * 16.0,
                )));
            }
        }
    }
    for &id in &room.family_order {
        if let Some(i) = room
            .instance(id)
            .filter(|i| catalog::is_a(i.object, "oSolid"))
        {
            resources
                .entry(catalog::object(i.object).unwrap().id)
                .or_default()
                .push(Participant::Preview(id));
        }
    }
    let dark_resource = catalog::object("oDarkFall").unwrap().id;
    let keys: Vec<_> = resources.keys().copied().collect();
    for (offset, &a_resource) in keys.iter().enumerate() {
        for &b_resource in &keys[offset..] {
            if a_resource != dark_resource && b_resource != dark_resource {
                continue;
            }
            for (ai, &a) in resources[&a_resource].iter().enumerate() {
                for (bi, &b) in resources[&b_resource].iter().enumerate() {
                    if a == b || (a_resource == b_resource && bi < ai) {
                        continue;
                    }
                    let (Some(first), Some(second)) = (snapshot(room, a), snapshot(room, b)) else {
                        continue;
                    };
                    if !first.mask().intersects_mask(&second.mask()) {
                        continue;
                    }
                    // Both callbacks dispatch for an intersecting pair. Keep
                    // the second receiver even when the first destroys itself.
                    if first.object == "oDarkFall" {
                        break_receiver(room, &first);
                    }
                    if second.object == "oDarkFall" {
                        break_receiver(room, &second);
                    }
                }
            }
        }
    }
}

fn break_receiver(room: &mut PreviewRoom, receiver: &PreviewInstance) {
    if room.instance(receiver.id).is_none() {
        return;
    }
    room.sounds.push("sndBreak");
    let mut smoke = Entity::new(
        0,
        "oSmokePuff",
        Vec2::new(receiver.pos.x + 8.0, receiver.pos.y + 8.0),
    );
    smoke.vel.y = 0.1;
    smoke.set_image_speed(0.4);
    smoke.data.insert("yAcc".into(), 0.1);
    room.append_body(smoke);
    for _ in 0..3 {
        let pos = Vec2::new(
            receiver.pos.x + 2.0 + room.rng.range(13) as f64,
            receiver.pos.y + 2.0 + room.rng.range(13) as f64,
        );
        let mut rubble = Entity::new(0, "oRubbleDark", pos);
        // Create initializes yVel=0,yAcc=.6; the collision caller then writes
        // xVel=rand(1,3)-rand(1,3), yVel=-rand(0,3), exactly five rolls each.
        rubble.vel.x = (1 + room.rng.range(3)) as f64 - (1 + room.rng.range(3)) as f64;
        rubble.vel.y = -(room.rng.range(4) as f64);
        rubble.data.insert("yAcc".into(), 0.6);
        room.append_body(rubble);
    }
    // DarkFall inherits Solid/Destroy, including spike removal/checkWater.
    room.destroy(receiver.id);
}

/// Endframe follows Draw and includes effects born in this frame's collision
/// pass. Smoke's Animation End destroys it; RubbleDark's one frame just wraps.
pub(super) fn advance_animation(room: &mut PreviewRoom) {
    let order = room.global_order.clone();
    for id in order {
        let Some(instance) = room.instance_mut(id) else {
            continue;
        };
        if !matches!(instance.object, "oSmokePuff" | "oRubbleDark") {
            continue;
        }
        let Some(body) = instance.body.as_mut() else {
            continue;
        };
        let speed = if instance.object == "oSmokePuff" {
            0.4
        } else {
            1.0
        };
        let ended = crate::animation::advance(
            &mut body.animation,
            speed,
            catalog::sprite_frame_count(instance.sprite),
        );
        if ended && instance.object == "oSmokePuff" {
            room.destroy(id);
        }
    }
}
