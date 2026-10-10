//! Original GameMaker object defaults and sprite collision rectangles.
//!
//! Positions use the original instance origin. Sprite bounds are inclusive in
//! the source; [`bounds`] converts them to the core's half-open rectangles.

use crate::{Rect, Vec2};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
pub struct ObjectDef {
    pub name: String,
    pub id: i32,
    pub sprite: String,
    pub parent: String,
    /// Original GameMaker property; this is not the gameplay collision group.
    pub solid: bool,
    pub visible: bool,
    pub depth: i32,
    pub persistent: bool,
    pub mask: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpriteDef {
    pub id: i32,
    pub origin: [i32; 2],
    /// Inclusive source coordinates: left, top, right, bottom.
    pub bounds: [i32; 4],
}

#[derive(Debug, Deserialize)]
struct Catalog {
    objects: BTreeMap<String, ObjectDef>,
    sprites: BTreeMap<String, SpriteDef>,
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../../../assets/catalog.json"))
            .expect("bundled Classic catalog must be valid")
    })
}

pub fn object(name: &str) -> Option<&'static ObjectDef> {
    catalog().objects.get(name)
}

/// True for the named object itself and for all descendants of `parent`.
pub fn is_a(name: &str, parent: &str) -> bool {
    let mut current = name;
    // The bound also keeps malformed metadata from hanging a game loop.
    for _ in 0..catalog().objects.len() {
        let Some(def) = object(current) else {
            return false;
        };
        if def.name == parent {
            return true;
        }
        current = &def.parent;
    }
    false
}

/// Classic tests collisions against the oSolid family, including moving blocks.
pub fn is_solid(name: &str) -> bool {
    is_a(name, "oSolid")
}

/// Default object sprite (an empty sprite is not inherited in GameMaker).
pub fn sprite(name: &str) -> Option<&'static str> {
    let def = object(name)?;
    (!def.sprite.is_empty()).then_some(def.sprite.as_str())
}

pub fn sprite_def(name: &str) -> Option<&'static SpriteDef> {
    catalog().sprites.get(name)
}

/// Unscaled first-frame sprite mask at its original source origin.
pub fn sprite_bounds(name: &str, pos: Vec2) -> Rect {
    transformed_sprite_bounds(name, pos, 0.0, Vec2::new(1.0, 1.0), 0.0)
}

#[derive(Deserialize)]
struct SpriteFrames {
    frames: Vec<FrameBounds>,
    mask: SpriteMask,
}

#[derive(Deserialize)]
struct FrameBounds {
    bounds: Option<[i32; 4]>,
}

#[derive(Deserialize)]
struct SpriteMask {
    separate: bool,
}

fn sprite_frames() -> &'static BTreeMap<String, SpriteFrames> {
    #[derive(Deserialize)]
    struct Atlas {
        sprites: BTreeMap<String, SpriteFrames>,
    }
    static FRAMES: OnceLock<BTreeMap<String, SpriteFrames>> = OnceLock::new();
    FRAMES.get_or_init(|| {
        serde_json::from_str::<Atlas>(include_str!("../../../assets/atlas.json"))
            .expect("bundled Classic frame metadata must be valid")
            .sprites
    })
}

/// Current collision-event mask, independent of movement-engine overrides.
///
/// GameMaker uses the explicit object mask when present, otherwise the current
/// sprite, and selects per-frame bounds for separate masks. `angle` uses screen
/// radians (positive clockwise); adapters convert GameMaker's positive
/// counterclockwise `image_angle`. Precise masks still use their AABB here.
pub fn instance_sprite_bounds(
    object: &str,
    current_sprite: &str,
    pos: Vec2,
    animation: f64,
    scale: Vec2,
    angle: f64,
) -> Rect {
    let mask = object_mask(object);
    transformed_sprite_bounds(
        mask_sprite(mask, current_sprite),
        pos,
        animation,
        scale,
        angle,
    )
}

fn mask_sprite<'a>(mask: &'a str, current_sprite: &'a str) -> &'a str {
    if mask.is_empty() {
        current_sprite
    } else {
        mask
    }
}

fn object_mask(name: &str) -> &'static str {
    object(name).map_or("", |def| def.mask.as_str())
}

fn transformed_sprite_bounds(
    name: &str,
    pos: Vec2,
    animation: f64,
    scale: Vec2,
    angle: f64,
) -> Rect {
    let empty = Rect::new(pos.x, pos.y, 0.0, 0.0);
    if !pos.x.is_finite()
        || !pos.y.is_finite()
        || !animation.is_finite()
        || !scale.x.is_finite()
        || !scale.y.is_finite()
        || !angle.is_finite()
        || scale.x == 0.0
        || scale.y == 0.0
    {
        return empty;
    }
    let Some(sprite) = sprite_def(name) else {
        return empty;
    };
    let mut bounds = Some(sprite.bounds);
    if let Some(frames) = sprite_frames().get(name)
        && frames.mask.separate
        && !frames.frames.is_empty()
    {
        let frame = animation.floor().rem_euclid(frames.frames.len() as f64) as usize;
        bounds = frames.frames[frame].bounds;
    }
    let Some([left, top, right, bottom]) = bounds else {
        return empty;
    };
    if right < left || bottom < top {
        return empty;
    }
    let [ox, oy] = sprite.origin;
    let (sin, cos) = angle.sin_cos();
    let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
    let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for x in [left - ox, right - ox] {
        for y in [top - oy, bottom - oy] {
            let x = f64::from(x) * scale.x;
            let y = f64::from(y) * scale.y;
            let tx = x * cos - y * sin;
            let ty = x * sin + y * cos;
            min.x = min.x.min(tx);
            min.y = min.y.min(ty);
            max.x = max.x.max(tx);
            max.y = max.y.max(ty);
        }
    }
    // Inclusive mask pixel extrema retain the final pixel after mirroring.
    Rect::new(
        pos.x + min.x,
        pos.y + min.y,
        max.x - min.x + (cos * scale.x).abs() + (sin * scale.y).abs(),
        max.y - min.y + (sin * scale.x).abs() + (cos * scale.y).abs(),
    )
}

/// The default mask sprite takes precedence over the visible sprite.
pub fn bounds(name: &str, pos: Vec2) -> Rect {
    let Some(def) = object(name) else {
        return Rect::new(pos.x - 6.0, pos.y - 7.0, 12.0, 14.0);
    };
    let sprite_name = if def.mask.is_empty() {
        &def.sprite
    } else {
        &def.mask
    };
    let Some(sprite) = sprite_def(sprite_name) else {
        return Rect::new(pos.x, pos.y, 0.0, 0.0);
    };
    let [left, top, right, bottom] = sprite.bounds;
    Rect::new(
        pos.x + f64::from(left - sprite.origin[0]),
        pos.y + f64::from(top - sprite.origin[1]),
        f64::from((right - left + 1).max(0)),
        f64::from((bottom - top + 1).max(0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn gameplay_solidity_follows_source_hierarchy() {
        assert!(is_solid("oHardBlock"));
        assert!(!object("oHardBlock").unwrap().solid);
        assert!(is_solid("oBlock"));
        assert!(is_solid("oPushBlock"));
        assert!(is_a("oPushBlock", "oMoveableSolid"));
        assert!(is_a("oBlock", "oBlock"));
        assert!(!is_solid("oGold"));
        assert!(object("oGold").unwrap().solid);
        assert!(!is_a("missing", "missing"));
    }

    #[test]
    fn original_positions_use_sprite_origins_and_inclusive_bounds() {
        assert_eq!(sprite("oBlock"), Some("sBlock"));
        assert_eq!(sprite("oHardBlock"), Some("sBrick"));
        assert_eq!(
            bounds("oBlock", Vec2::new(32.0, 48.0)),
            Rect::new(32.0, 48.0, 16.0, 16.0)
        );
        let sprite = sprite_def("sBlock").unwrap();
        assert_eq!(sprite.origin, [0, 0]);
        assert_eq!(sprite.bounds, [0, 0, 15, 15]);
        assert_eq!(
            bounds("oDamsel", Vec2::new(32.0, 48.0)),
            Rect::new(26.0, 40.0, 12.0, 16.0)
        );
        assert_eq!(
            bounds("oSnake", Vec2::new(32.0, 48.0)),
            Rect::new(33.0, 52.0, 14.0, 12.0)
        );
    }

    #[test]
    fn catalog_has_valid_references_and_acyclic_inheritance() {
        assert_eq!(catalog().objects.len(), 453);
        for def in catalog().objects.values() {
            for sprite in [&def.sprite, &def.mask] {
                assert!(
                    sprite.is_empty() || sprite_def(sprite).is_some(),
                    "{}: {sprite}",
                    def.name
                );
            }
            let mut seen = BTreeSet::new();
            let mut current = def.name.as_str();
            while !current.is_empty() {
                assert!(seen.insert(current), "cycle in {}", def.name);
                current = &object(current).expect("parent must exist").parent;
            }
        }
    }

    #[test]
    fn current_frame_and_mask_precedence_use_source_metadata() {
        let pos = Vec2::new(32.0, 48.0);
        let unit = Vec2::new(1.0, 1.0);
        let run = instance_sprite_bounds("oSnake", "sRunLeft", pos, 2.8, unit, 0.0);
        assert_eq!(run, Rect::new(26.0, 42.0, 12.0, 14.0));
        assert_eq!(
            instance_sprite_bounds("oSnake", "sRunLeft", pos, 8.8, unit, 0.0),
            run,
            "image index wraps within the selected sprite"
        );
        // Classic 1.1 has no nonempty object mask defaults. Exercise the same
        // resolution function with a synthetic explicit mask to prevent
        // silently regressing mask precedence when runtime masks are added.
        assert_eq!(mask_sprite("sBlock", "sRunLeft"), "sBlock");
        assert_eq!(
            transformed_sprite_bounds(mask_sprite("sBlock", "sRunLeft"), pos, 2.8, unit, 0.0),
            Rect::new(32.0, 48.0, 16.0, 16.0)
        );
        assert_eq!(mask_sprite("", "sRunLeft"), "sRunLeft");
    }

    #[test]
    fn sprite_transforms_preserve_inclusive_mask_pixels() {
        let pos = Vec2::new(32.0, 48.0);
        assert_eq!(
            transformed_sprite_bounds("sStandLeft", pos, 0.0, Vec2::new(-1.0, 1.0), 0.0),
            Rect::new(27.0, 40.0, 12.0, 16.0)
        );
        assert_eq!(
            transformed_sprite_bounds("sBlock", pos, 0.0, Vec2::new(2.0, 3.0), 0.0),
            Rect::new(32.0, 48.0, 32.0, 48.0)
        );
        let rotated = transformed_sprite_bounds(
            "sStandLeft",
            pos,
            0.0,
            Vec2::new(1.0, 1.0),
            std::f64::consts::FRAC_PI_2,
        );
        assert!((rotated.x - 25.0).abs() < 1e-10);
        assert!((rotated.y - 42.0).abs() < 1e-10);
        assert!((rotated.w - 16.0).abs() < 1e-10);
        assert!((rotated.h - 12.0).abs() < 1e-10);
        for rect in [
            sprite_bounds("missing", pos),
            transformed_sprite_bounds("sBlock", pos, 0.0, Vec2::new(0.0, 1.0), 0.0),
            transformed_sprite_bounds("sBlock", pos, f64::NAN, Vec2::new(1.0, 1.0), 0.0),
        ] {
            assert!(!rect.is_valid());
            assert!(!rect.intersects(Rect::new(0.0, 0.0, 100.0, 100.0)));
        }
    }
}
