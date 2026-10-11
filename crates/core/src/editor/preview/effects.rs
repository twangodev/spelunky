//! Editor-reachable inherited Destroy callbacks.
//!
//! The owner invokes this selector synchronously before marking the receiver
//! destroyed. It changes preview instances only; the serialized grid keeps its
//! authored spike character, exactly as oSolid's Destroy event does.

use super::{PreviewInstance, PreviewRoom, PreviewSourceError};
use crate::{CollisionPrecision, Vec2, catalog};

/// Palette objects whose resolved Destroy event calls oSolid's Destroy body.
/// Canonical Objects/Basis/oSolid.events/Destroy.xml; the four explicit
/// inherited calls are in Objects/Blocks/{oBrick,oLush,oDark,oTemple}.events.
/// Brick/Lush/Dark/Temple explicitly call inherited; DarkFall and TreeTile
/// inherit the absent override. Block and arrow traps replace Destroy instead.
pub(super) fn inherits_solid_destroy(object: &str) -> bool {
    matches!(
        object,
        "oBrick" | "oLush" | "oDark" | "oTemple" | "oDarkFall" | "oTreeTile"
    )
}

/// oSolid/Destroy's point gate and independent translated instance_place.
/// The gate uses bounding boxes; instance_place uses both precise masks and
/// the current descendant-array first-hit order. These are separate queries.
pub(super) fn inherited_spike_target(
    room: &PreviewRoom,
    receiver: &PreviewInstance,
) -> Option<u64> {
    if !inherits_solid_destroy(receiver.object) {
        return None;
    }
    let probe = Vec2::new(receiver.pos.x + 8.0, receiver.pos.y - 1.0);
    let gated = room.family_order.iter().copied().any(|id| {
        room.instance(id).is_some_and(|candidate| {
            catalog::is_a(candidate.object, "oSpikes")
                && candidate
                    .mask()
                    .contains(probe, CollisionPrecision::BoundingBox)
        })
    });
    if !gated {
        return None;
    }
    let translated = catalog::instance_mask(
        receiver.object,
        receiver.sprite,
        probe,
        0.0,
        Vec2::new(1.0, 1.0),
        0.0,
    );
    room.family_order.iter().copied().find(|&id| {
        id != receiver.id
            && room.instance(id).is_some_and(|candidate| {
                catalog::is_a(candidate.object, "oSpikes")
                    && translated.intersects_mask(&candidate.mask())
            })
    })
}

/// Run the two Block/Destroy child families before resource812 Editor/Step.
/// `treasure_vision` is global.hasSpectacles || global.hasUdjatEye.
pub(super) fn step(room: &mut PreviewRoom, camera: Vec2, treasure_vision: bool) {
    let mut receivers: Vec<_> = room
        .family_order
        .iter()
        .copied()
        .filter(|&id| {
            room.instance(id)
                .is_some_and(|instance| instance.body.is_some())
        })
        .collect();
    // The engine visits object resources ascending, preserving each current
    // descendant-array order. All four child resources precede Editor812.
    receivers.sort_by_key(|&id| {
        catalog::object(room.instance(id).unwrap().object)
            .unwrap()
            .id
    });
    for id in receivers {
        let Some(mut receiver) = room.instance(id).cloned() else {
            continue;
        };
        if catalog::is_a(receiver.object, "oRubblePiece") {
            rubble_step(room, &mut receiver, camera);
        } else if matches!(receiver.object, "oGoldChunk" | "oGoldNugget") {
            if let Some(error) = treasure_step(room, &mut receiver, camera, treasure_vision) {
                room.source_error.get_or_insert(error);
            }
        } else {
            super::dark_fall::step_effect(room, &mut receiver);
        }
        let alive = receiver.alive;
        if let Some(slot) = room.instance_mut(id) {
            *slot = receiver;
        }
        if !alive {
            room.destroyed_room_instance = true;
        }
    }
}

fn point_family(room: &PreviewRoom, point: Vec2, family: &str) -> bool {
    room.family_order.iter().copied().any(|id| {
        room.instance(id).is_some_and(|instance| {
            catalog::is_a(instance.object, family)
                && instance
                    .mask()
                    .contains(point, CollisionPrecision::BoundingBox)
        })
    }) || (family == "oSolid"
        && border_masks().any(|mask| mask.contains(point, CollisionPrecision::BoundingBox)))
}

/// The 148 literal oHardBlock room instances in rLevelEditor are independent
/// of the authored 40x32 grid. Their oSolid ancestry matters even though the
/// resource's engine `solid` property is false.
fn border_masks() -> impl Iterator<Item = catalog::CollisionMask> {
    (0..34).flat_map(|y| {
        (0..42).filter_map(move |x| {
            (x == 0 || x == 41 || y == 0 || y == 33).then(|| {
                catalog::instance_mask(
                    "oHardBlock",
                    "sBrick",
                    Vec2::new(x as f64 * 16.0, y as f64 * 16.0),
                    0.0,
                    Vec2::new(1.0, 1.0),
                    0.0,
                )
            })
        })
    })
}

// Objects/Basis/oRubblePiece.events/Step.xml. The Block descendants have
// type="None", so neither the Drip nor Leaf child-only branches apply.
fn rubble_step(room: &PreviewRoom, receiver: &mut PreviewInstance, camera: Vec2) {
    let body = receiver.body.as_mut().unwrap();
    receiver.pos.x += body.vel.x;
    receiver.pos.y += body.vel.y;
    body.vel.y += body.data.get("gravity").copied().unwrap_or(0.6);
    if point_family(room, receiver.pos, "oWaterSwim") {
        body.vel.y = 0.2;
    } else if point_family(room, receiver.pos, "oLava") {
        receiver.alive = false;
    }
    if point_family(room, receiver.pos, "oSolid")
        || receiver.pos.x < camera.x - 32.0
        || receiver.pos.x > camera.x + 320.0 + 32.0
        || receiver.pos.y < camera.y - 32.0
        || receiver.pos.y > camera.y + 240.0 + 32.0
    {
        receiver.alive = false;
    }
}

fn solid_line(room: &PreviewRoom, excluded: u64, first: Vec2, second: Vec2) -> bool {
    room.family_order.iter().copied().any(|id| {
        id != excluded
            && room.instance(id).is_some_and(|instance| {
                catalog::is_a(instance.object, "oSolid")
                    && instance.mask().intersects_axis_line(
                        first,
                        second,
                        CollisionPrecision::Precise,
                    )
            })
    }) || border_masks()
        .any(|mask| mask.intersects_axis_line(first, second, CollisionPrecision::Precise))
}

// Collision/{horizontal,vertical} scripts call round on both endpoints before
// the engine collision_line; its own transformed-mask rounding remains separate.
fn contact(room: &PreviewRoom, receiver: &PreviewInstance, dx: i8, dy: i8, moving: bool) -> bool {
    let body = receiver.body.as_ref().unwrap();
    let extent = if receiver.object == "oGoldChunk" {
        2.0
    } else {
        4.0
    };
    let lb = receiver.pos.x + body.data.get("bboxLeft").copied().unwrap_or(-extent);
    let rb = receiver.pos.x + body.data.get("bboxRight").copied().unwrap_or(extent);
    let tb = receiver.pos.y + body.data.get("bboxTop").copied().unwrap_or(-extent);
    let bb = receiver.pos.y + body.data.get("bboxBottom").copied().unwrap_or(extent);
    let (first, second) = if dx != 0 {
        let x = if dx < 0 { lb - 1.0 } else { rb };
        (
            Vec2::new(x, tb + if moving { 5.0 } else { 0.0 }),
            Vec2::new(x, bb - 1.0),
        )
    } else {
        let y = if dy < 0 { tb - 1.0 } else { bb };
        (Vec2::new(lb, y), Vec2::new(rb - 1.0, y))
    };
    solid_line(
        room,
        receiver.id,
        Vec2::new(first.x.round_ties_even(), first.y.round_ties_even()),
        Vec2::new(second.x.round_ties_even(), second.y.round_ties_even()),
    )
}

// Objects/Basis/oTreasure.events/Step.xml and Platform Engine/moveTo.gml.
// Gold children cannot push moveable solids or land on platforms: both helper
// predicates select only oCharacter and its immediate children.
fn treasure_step(
    room: &PreviewRoom,
    receiver: &mut PreviewInstance,
    camera: Vec2,
    vision: bool,
) -> Option<PreviewSourceError> {
    if receiver.body.as_ref().unwrap().state != 1
        || receiver.pos.x <= camera.x - 16.0
        || receiver.pos.x >= camera.x + 320.0 + 16.0
        || receiver.pos.y <= camera.y - 16.0
        || receiver.pos.y >= camera.y + 240.0 + 16.0
    {
        return None;
    }
    let left = contact(room, receiver, -1, 0, false);
    let right = contact(room, receiver, 1, 0, false);
    let bottom = contact(room, receiver, 0, 1, false);
    let velocity = receiver.body.as_ref().unwrap().vel;
    {
        let body = receiver.body.as_mut().unwrap();
        body.data.insert("colLeft".into(), f64::from(left));
        body.data.insert("colRight".into(), f64::from(right));
        body.data.insert("colBot".into(), f64::from(bottom));
        body.data.insert("mtXPrev".into(), receiver.pos.x);
        body.data.insert("mtYPrev".into(), receiver.pos.y);
        body.data
            .insert("xVelFrac".into(), velocity.x.abs().fract());
        body.data
            .insert("yVelFrac".into(), velocity.y.abs().fract());
        body.data.insert("xVelInteger".into(), 0.0);
        body.data.insert("yVelInteger".into(), 0.0);
    }
    // moveTo reads oGame.time only for a nonzero fractional component.
    // rLevelEditor contains no oGame, which is nonpersistent. The sealed GM8
    // getter proof establishes a source code-block error, not numeric zero.
    // Preserve all preceding writes and stop before displacement/movement.
    if velocity.x.abs().fract() != 0.0 || velocity.y.abs().fract() != 0.0 {
        return Some(PreviewSourceError::MissingGameClock);
    }
    let dx = velocity.x as i32;
    let dy = velocity.y as i32;
    receiver
        .body
        .as_mut()
        .unwrap()
        .data
        .insert("xVelInteger".into(), dx as f64);
    receiver
        .body
        .as_mut()
        .unwrap()
        .data
        .insert("yVelInteger".into(), dy as f64);
    for _ in 0..dx.unsigned_abs() {
        if contact(room, receiver, dx.signum() as i8, 0, true) {
            break;
        }
        receiver.pos.x += dx.signum() as f64;
    }
    for _ in 0..dy.unsigned_abs() {
        if contact(room, receiver, 0, dy.signum() as i8, false) {
            break;
        }
        receiver.pos.y += dy.signum() as f64;
    }
    {
        let body = receiver.body.as_mut().unwrap();
        if !bottom {
            body.vel.y += body
                .data
                .get("myGrav")
                .or_else(|| body.data.get("gravity"))
                .copied()
                .unwrap_or(0.6);
        }
        body.vel.y = body.vel.y.min(8.0);
    }
    if contact(room, receiver, 0, -1, false) {
        let body = receiver.body.as_mut().unwrap();
        if body.vel.y < 0.0 {
            body.vel.y *= -0.8;
        } else {
            receiver.pos.y += 1.0;
        }
    }
    if left || right {
        receiver.body.as_mut().unwrap().vel.x *= -0.5;
    }
    if bottom {
        let body = receiver.body.as_mut().unwrap();
        body.vel.x = if body.vel.x.abs() < 0.1 {
            0.0
        } else {
            body.vel.x * 0.3
        };
        receiver.pos.y -= 1.0;
        if !contact(room, receiver, 0, 1, false) {
            receiver.pos.y += 1.0;
            // The original writes `status`, not `state`; do not deactivate it.
            receiver
                .body
                .as_mut()
                .unwrap()
                .data
                .insert("status".into(), 0.0);
        }
        receiver.body.as_mut().unwrap().vel.y = 0.0;
    }
    if left {
        if !right {
            receiver.pos.x += 1.0;
        }
    } else if right {
        receiver.pos.x -= 1.0;
    }
    receiver.depth = if vision { 0 } else { 101 };
    // Palette LavaTile is not oLava; retain the literal family selection.
    let lava = room.family_order.iter().copied().any(|id| {
        room.instance(id).is_some_and(|instance| {
            catalog::is_a(instance.object, "oLava")
                && instance.mask().intersects_rectangle_bbox(
                    Vec2::new(receiver.pos.x - 3.0, receiver.pos.y - 3.0),
                    Vec2::new(receiver.pos.x + 3.0, receiver.pos.y + 3.0),
                )
        })
    });
    if lava {
        let body = receiver.body.as_mut().unwrap();
        body.data.insert("myGrav".into(), 0.0);
        body.data.insert("gravity".into(), 0.0);
        body.vel = Vec2::default();
        receiver.pos.y += 0.05;
    }
    if point_family(
        room,
        Vec2::new(receiver.pos.x, receiver.pos.y - 5.0),
        "oLava",
    ) {
        receiver.alive = false;
    }
    None
}
