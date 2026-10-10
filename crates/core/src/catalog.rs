//! Original GameMaker object defaults and sprite collision rectangles.
//!
//! Positions use the original instance origin. Sprite bounds are inclusive in
//! the source; [`bounds`] converts them to the core's half-open rectangles.

use crate::{CollisionPrecision, Rect, Vec2};
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

/// Immutable bundled lookup keys; callers cannot add query-time cache entries.
pub(crate) fn object_names() -> impl Iterator<Item = &'static str> {
    catalog().objects.keys().map(String::as_str)
}

/// True for the named object itself and for all descendants of `parent`.
pub fn is_a(name: &str, parent: &str) -> bool {
    static ANCESTORS: OnceLock<BTreeMap<&'static str, Vec<&'static str>>> = OnceLock::new();
    ANCESTORS
        .get_or_init(|| ancestry(&catalog().objects))
        .get(name)
        .is_some_and(|ancestors| ancestors.contains(&parent))
}

/// Cache only bundled objects, never query strings. Use the original walk's
/// lookup keys, compared names and iteration bound even for malformed metadata.
fn ancestry(objects: &BTreeMap<String, ObjectDef>) -> BTreeMap<&str, Vec<&str>> {
    objects
        .keys()
        .map(|name| {
            let mut ancestors = Vec::new();
            let mut current = name.as_str();
            for _ in 0..objects.len() {
                let Some(def) = objects.get(current) else {
                    break;
                };
                ancestors.push(def.name.as_str());
                current = &def.parent;
            }
            (name.as_str(), ancestors)
        })
        .collect()
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

/// Original sprite frame count used by independent Animation End events.
pub fn sprite_frame_count(name: &str) -> usize {
    sprite_frames()
        .get(name)
        .map_or(0, |sprite| sprite.frames.len())
}

/// Unscaled first-frame sprite mask at its original source origin.
pub fn sprite_bounds(name: &str, pos: Vec2) -> Rect {
    static BOUNDS: OnceLock<BTreeMap<String, Rect>> = OnceLock::new();
    let bounds = BOUNDS
        .get_or_init(|| {
            catalog()
                .sprites
                .keys()
                .map(|name| {
                    (
                        name.clone(),
                        transformed_sprite_bounds(
                            name,
                            Vec2::default(),
                            0.0,
                            Vec2::new(1.0, 1.0),
                            0.0,
                        ),
                    )
                })
                .collect()
        })
        .get(name)
        .copied()
        .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
    Rect::new(pos.x + bounds.x, pos.y + bounds.y, bounds.w, bounds.h)
}

/// Envelope of default masks in a source object family, used only to select
/// tile origins for a broad phase. Explicit per-instance sprite overrides are
/// queried separately, so this envelope never limits their collision extent.
pub fn group_bounds(group: &str) -> Option<Rect> {
    static GROUPS: OnceLock<BTreeMap<String, Rect>> = OnceLock::new();
    GROUPS
        .get_or_init(|| {
            let mut groups: BTreeMap<String, Rect> = BTreeMap::new();
            for def in catalog().objects.values() {
                let r = bounds(&def.name, Vec2::default());
                if !r.is_valid() {
                    continue;
                }
                let mut current = def.name.as_str();
                for _ in 0..catalog().objects.len() {
                    let Some(parent) = object(current) else {
                        break;
                    };
                    groups
                        .entry(current.to_owned())
                        .and_modify(|envelope| {
                            let left = envelope.x.min(r.x);
                            let top = envelope.y.min(r.y);
                            let right = (envelope.x + envelope.w).max(r.x + r.w);
                            let bottom = (envelope.y + envelope.h).max(r.y + r.h);
                            *envelope = Rect::new(left, top, right - left, bottom - top);
                        })
                        .or_insert(r);
                    current = &parent.parent;
                }
            }
            groups
        })
        .get(group)
        .copied()
}

#[derive(Deserialize)]
struct SpriteFrames {
    frames: Vec<FrameBounds>,
    mask: SpriteMask,
}

#[derive(Deserialize)]
struct FrameBounds {
    bounds: Option<[i32; 4]>,
    mask_offset: Option<usize>,
    mask_stride: Option<usize>,
    mask_width: Option<usize>,
    mask_height: Option<usize>,
}

#[derive(Deserialize)]
struct SpriteMask {
    separate: bool,
    shape: String,
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
    if animation == 0.0 && scale == Vec2::new(1.0, 1.0) && angle == 0.0 {
        return sprite_bounds(mask_sprite(mask, current_sprite), pos);
    }
    transformed_sprite_bounds(
        mask_sprite(mask, current_sprite),
        pos,
        animation,
        scale,
        angle,
    )
}

/// Source sprite mask and its instance transform. Bounding-box queries remain
/// independent from precise queries, exactly as Classic's `prec` arguments.
pub struct CollisionMask {
    bounds: Rect,
    /// Source sprite envelope, independent of the selected mask frame.
    event_bounds: Option<[i32; 4]>,
    frame: Option<&'static FrameBounds>,
    precise: bool,
    rectangle: bool,
    rectangle_bounds: Option<[i32; 4]>,
    origin: [i32; 2],
    pos: Vec2,
    scale: Vec2,
    angle: f64,
}

pub fn instance_mask(
    object: &str,
    current_sprite: &str,
    pos: Vec2,
    animation: f64,
    scale: Vec2,
    angle: f64,
) -> CollisionMask {
    let sprite = mask_sprite(object_mask(object), current_sprite);
    let frames = sprite_frames().get(sprite);
    let frame = frames.and_then(|frames| {
        (!frames.frames.is_empty() && animation.is_finite()).then(|| {
            let index = animation.floor().rem_euclid(frames.frames.len() as f64) as usize;
            &frames.frames[index]
        })
    });
    CollisionMask {
        bounds: instance_sprite_bounds(object, current_sprite, pos, animation, scale, angle),
        event_bounds: sprite_def(sprite).map(|sprite| sprite.bounds),
        frame,
        precise: frames.is_some_and(|frames| frames.mask.shape == "PRECISE"),
        rectangle: frames.is_some_and(|frames| frames.mask.shape == "RECTANGLE"),
        rectangle_bounds: frames.and_then(|frames| {
            if frames.mask.separate {
                frame.and_then(|frame| frame.bounds)
            } else {
                sprite_def(sprite).map(|sprite| sprite.bounds)
            }
        }),
        origin: sprite_def(sprite).map_or([0, 0], |sprite| sprite.origin),
        pos,
        scale,
        angle,
    }
}

impl CollisionMask {
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Precise rasterization is verified for the source's unrotated unit-scale
    /// masks and their horizontal/vertical mirrors. Other transforms retain
    /// the existing bounding-box behavior pending GM8 rasterization evidence.
    pub fn supports_precise_pixels(&self) -> bool {
        self.angle == 0.0
            && self.scale.x.abs() == 1.0
            && self.scale.y.abs() == 1.0
            && self.pos.x.fract() == 0.0
            && self.pos.y.fract() == 0.0
    }

    fn pixel(&self, x: i32, y: i32) -> bool {
        let image_x = self.origin[0] + if self.scale.x > 0.0 { x } else { -x };
        let image_y = self.origin[1] + if self.scale.y > 0.0 { y } else { -y };
        self.image_pixel(image_x, image_y)
    }

    fn image_pixel(&self, image_x: i32, image_y: i32) -> bool {
        let Some(frame) = self.frame else {
            return false;
        };
        if self.rectangle {
            // GM8 materializes rectangle masks as boolean pixels filling the
            // selected mask's inclusive bounds, regardless of image alpha.
            return self
                .rectangle_bounds
                .is_some_and(|[left, top, right, bottom]| {
                    image_x >= left && image_x <= right && image_y >= top && image_y <= bottom
                });
        }
        let (Some(offset), Some(stride), Some(width), Some(height)) = (
            frame.mask_offset,
            frame.mask_stride,
            frame.mask_width,
            frame.mask_height,
        ) else {
            return false;
        };
        if image_x < 0 || image_y < 0 || image_x as usize >= width || image_y as usize >= height {
            return false;
        }
        let index = offset + image_y as usize * stride + image_x as usize / 8;
        include_bytes!("../../../assets/collision.bin")
            .get(index)
            .is_some_and(|byte| byte & (1 << (image_x as usize % 8)) != 0)
    }

    /// collision_point uses the cached aggregate integer bbox, unlike smooth
    /// sprite geometry or the explicit rectangle query rasterizer. Reuse the
    /// independently verified bbox updater for its supported transforms only.
    fn point_bounds(&self) -> Rect {
        if self.frame.is_none() {
            return self.bounds;
        }
        self.event_geometry()
            .map_or(self.bounds, |(bounds, _)| bounds)
    }

    pub fn contains(&self, point: Vec2, precision: CollisionPrecision) -> bool {
        // Canonical 0x4AC73F compares the ORIGINAL doubles to integer bbox
        // edges. Rounding the query first would admit points just outside it.
        if !self.point_bounds().contains(point) {
            return false;
        }
        if precision == CollisionPrecision::BoundingBox
            || !self.precise
            || !self.supports_precise_pixels()
        {
            return true;
        }
        self.pixel(
            (point.x - self.pos.x).floor() as i32,
            (point.y - self.pos.y).floor() as i32,
        )
    }

    pub fn intersects(&self, rect: Rect, precision: CollisionPrecision) -> bool {
        if !self.bounds.intersects(rect) {
            return false;
        }
        if precision == CollisionPrecision::BoundingBox
            || !self.precise
            || !self.supports_precise_pixels()
        {
            return true;
        }
        let left = (rect.x.max(self.bounds.x) - self.pos.x).floor() as i32;
        let top = (rect.y.max(self.bounds.y) - self.pos.y).floor() as i32;
        let right =
            ((rect.x + rect.w).min(self.bounds.x + self.bounds.w) - self.pos.x).ceil() as i32;
        let bottom =
            ((rect.y + rect.h).min(self.bounds.y + self.bounds.h) - self.pos.y).ceil() as i32;
        for y in top..bottom {
            for x in left..right {
                if self.pixel(x, y) {
                    return true;
                }
            }
        }
        false
    }

    fn event_geometry(&self) -> Option<(Rect, Vec2)> {
        // The only source nonunit collision transform is oArrowTrapTest's
        // positive integral X stretch. Its Y scale and angle remain 1 and 0.
        let horizontal_stretch = self.angle == 0.0
            && self.scale.x >= 1.0
            && self.scale.x.fract() == 0.0
            && self.scale.y == 1.0;
        if !self.angle.is_finite()
            || (!horizontal_stretch && (self.scale.x.abs() != 1.0 || self.scale.y.abs() != 1.0))
            || !self.pos.x.is_finite()
            || !self.pos.y.is_finite()
            || ((self.angle != 0.0 || self.pos.x.fract() != 0.0 || self.pos.y.fract() != 0.0)
                && self.scale != Vec2::new(1.0, 1.0)
                && !horizontal_stretch)
        {
            return None;
        }
        let [left, top, right, bottom] = self.event_bounds?;
        if self.angle != 0.0 {
            // Canonical 0x4AAAC1..0x4AADBF transforms the aggregate source
            // sprite envelope's inclusive endpoints, then rounds each extreme
            // independently. Its positive GML angle is our negative screen
            // angle. The instance origin is rounded separately for sampling.
            let x0 = f64::from(left.min(right) - self.origin[0]);
            let x1 = f64::from(left.max(right) - self.origin[0]);
            let y0 = f64::from(top.min(bottom) - self.origin[1]);
            let y1 = f64::from(top.max(bottom) - self.origin[1]);
            let (sin, cos) = (-self.angle).sin_cos();
            let left = (self.pos.x + (cos * x0).min(cos * x1) + (sin * y0).min(sin * y1))
                .round_ties_even();
            let right = (self.pos.x + (cos * x0).max(cos * x1) + (sin * y0).max(sin * y1))
                .round_ties_even();
            let top = (self.pos.y + (cos * y0).min(cos * y1) - (sin * x0).max(sin * x1))
                .round_ties_even();
            let bottom = (self.pos.y + (cos * y0).max(cos * y1) - (sin * x0).min(sin * x1))
                .round_ties_even();
            return Some((
                Rect::new(left, top, right - left + 1.0, bottom - top + 1.0),
                Vec2::new(self.pos.x.round_ties_even(), self.pos.y.round_ties_even()),
            ));
        }
        // Canonical GM8 rounds each translated inclusive bbox edge before
        // broad-phase rejection, and rounds the instance origin independently
        // for pixel indexing. Rounding the origin alone differs at .5 ties.
        let x0 = self.pos.x + f64::from(left - self.origin[0]) * self.scale.x;
        let y0 = self.pos.y + f64::from(top - self.origin[1]) * self.scale.y;
        let x1 = self.pos.x + f64::from(right - self.origin[0] + 1) * self.scale.x - 1.0;
        let y1 = self.pos.y + f64::from(bottom - self.origin[1] + 1) * self.scale.y - 1.0;
        let left = x0.min(x1).round_ties_even();
        let top = y0.min(y1).round_ties_even();
        let right = x0.max(x1).round_ties_even();
        let bottom = y0.max(y1).round_ties_even();
        Some((
            Rect::new(left, top, right - left + 1.0, bottom - top + 1.0),
            Vec2::new(self.pos.x.round_ties_even(), self.pos.y.round_ties_even()),
        ))
    }

    /// Collision-event overlap of the two loaded source masks, including
    /// rectangle/precise pairs. Explicit shape-query precision is unchanged.
    /// Positive unit-scale rotations use canonical inverse pixel sampling.
    /// Source detectors' unrotated positive integral X stretches use the
    /// canonical inverse scale sampler. Other nonunit scales, rotated mirrors
    /// and fractional mirrors retain their bbox fallback.
    pub fn intersects_mask(&self, other: &Self) -> bool {
        if !(self.precise || self.rectangle) || !(other.precise || other.rectangle) {
            return self.bounds.intersects(other.bounds);
        }
        let (Some((bounds, pos)), Some((other_bounds, other_pos))) =
            (self.event_geometry(), other.event_geometry())
        else {
            return self.bounds.intersects(other.bounds);
        };
        if !bounds.intersects(other_bounds) {
            return false;
        }
        let left = bounds.x.max(other_bounds.x) as i32;
        let top = bounds.y.max(other_bounds.y) as i32;
        let right = (bounds.x + bounds.w).min(other_bounds.x + other_bounds.w) as i32;
        let bottom = (bounds.y + bounds.h).min(other_bounds.y + other_bounds.h) as i32;
        let (sin, cos) = if self.angle == 0.0 {
            (0.0, 1.0)
        } else {
            self.angle.sin_cos()
        };
        let (other_sin, other_cos) = if other.angle == 0.0 {
            (0.0, 1.0)
        } else {
            other.angle.sin_cos()
        };
        for y in top..bottom {
            for x in left..right {
                if self.event_pixel(x - pos.x as i32, y - pos.y as i32, sin, cos)
                    && other.event_pixel(
                        x - other_pos.x as i32,
                        y - other_pos.y as i32,
                        other_sin,
                        other_cos,
                    )
                {
                    return true;
                }
            }
        }
        false
    }

    fn event_pixel(&self, x: i32, y: i32, sin: f64, cos: f64) -> bool {
        if self.angle == 0.0 {
            if self.scale.x > 1.0 {
                // Canonical 0x48A6E6..0x48A78D divides the integer world
                // offset from the independently rounded instance origin,
                // adds the sprite origin, then floors before both mask tests.
                let image_x =
                    (f64::from(x) / self.scale.x + f64::from(self.origin[0])).floor() as i32;
                let image_y = y + self.origin[1];
                return self.image_pixel(image_x, image_y);
            }
            return self.pixel(x, y);
        }
        // Canonical 0x48A88A..0x48A987 samples each world pixel inversely,
        // flooring after rotation and sprite-origin addition. This branch is
        // restricted above to the source Arrow/FishBone's positive unit scale.
        let image_x =
            (f64::from(x) * cos + f64::from(y) * sin + f64::from(self.origin[0])).floor() as i32;
        let image_y =
            (f64::from(y) * cos - f64::from(x) * sin + f64::from(self.origin[1])).floor() as i32;
        self.image_pixel(image_x, image_y)
    }
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
    if scale == Vec2::new(1.0, 1.0) && angle == 0.0 {
        return Rect::new(
            pos.x + f64::from(left - ox),
            pos.y + f64::from(top - oy),
            f64::from(right - left + 1),
            f64::from(bottom - top + 1),
        );
    }
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

    fn original_is_a(objects: &BTreeMap<String, ObjectDef>, name: &str, parent: &str) -> bool {
        let mut current = name;
        for _ in 0..objects.len() {
            let Some(def) = objects.get(current) else {
                return false;
            };
            if def.name == parent {
                return true;
            }
            current = &def.parent;
        }
        false
    }

    #[test]
    fn collision_point_uses_aggregate_integer_bounds_without_rounding_query() {
        let mask = instance_mask(
            "oPushBlock",
            "sBlock",
            Vec2::new(16.0, 80.25),
            0.0,
            Vec2::new(1.0, 1.0),
            0.0,
        );
        // Canonical collision_point passes raw doubles into the cached integer
        // bbox predicate: L <= x < R+1, T <= y < B+1 (0x4AC73F).
        assert!(mask.contains(Vec2::new(24.0, 80.1), CollisionPrecision::BoundingBox));
        assert!(!mask.contains(Vec2::new(24.0, 96.1), CollisionPrecision::BoundingBox));
        assert!(!mask.contains(Vec2::new(15.9995, 88.0), CollisionPrecision::BoundingBox));
        assert_eq!(
            mask.bounds(),
            Rect::new(16.0, 80.25, 16.0, 16.0),
            "smooth sprite geometry remains fractional"
        );
        let lava = instance_mask(
            "oLava",
            "sLavaTop",
            Vec2::new(80.0, 80.0),
            0.0,
            Vec2::new(1.0, 1.0),
            0.0,
        );
        assert_eq!(lava.bounds().y, 81.0);
        assert!(
            lava.contains(Vec2::new(80.0, 80.0), CollisionPrecision::BoundingBox),
            "point broadphase uses aggregate sprite bbox, independent of current subimage"
        );
    }

    #[test]
    fn cached_ancestry_matches_original_for_every_object_pair_and_unknowns() {
        let objects = &catalog().objects;
        let mut names: Vec<_> = objects.keys().map(String::as_str).collect();
        names.extend(["", "missing", "oNotInTheCatalog"]);
        for name in &names {
            for parent in &names {
                assert_eq!(
                    is_a(name, parent),
                    original_is_a(objects, name, parent),
                    "{name:?} -> {parent:?}"
                );
            }
        }
    }

    #[test]
    fn cached_ancestry_preserves_cycles_dangling_parents_and_distinct_names() {
        let mut objects = BTreeMap::new();
        for (key, name, parent) in [
            ("a", "a", "b"),
            ("b", "b", "a"),
            ("self", "self", "self"),
            ("leaf", "display_name", "dangling"),
            ("tail", "tail", "leaf"),
        ] {
            let mut def = object("oBlock").unwrap().clone();
            def.name = name.into();
            def.parent = parent.into();
            objects.insert(key.into(), def);
        }
        let cached = ancestry(&objects);
        assert_eq!(cached.len(), objects.len());
        assert_eq!(cached["a"], ["a", "b", "a", "b", "a"]);
        assert_eq!(cached["self"].len(), objects.len());
        assert_eq!(cached["leaf"], ["display_name"]);
        for name in [
            "a",
            "b",
            "self",
            "leaf",
            "tail",
            "display_name",
            "dangling",
            "",
        ] {
            for parent in [
                "a",
                "b",
                "self",
                "leaf",
                "tail",
                "display_name",
                "dangling",
                "",
            ] {
                assert_eq!(
                    cached
                        .get(name)
                        .is_some_and(|ancestors| ancestors.contains(&parent)),
                    original_is_a(&objects, name, parent),
                    "{name:?} -> {parent:?}"
                );
            }
        }
        assert!(ancestry(&BTreeMap::new()).is_empty());
    }

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

    #[test]
    fn precise_queries_use_alpha_but_source_rectangles_keep_their_corners() {
        let precise = CollisionPrecision::Precise;
        let bbox = CollisionPrecision::BoundingBox;
        let pos = Vec2::new(80.0, 80.0);
        let mask = instance_mask("oDarkFall", "sDarkFall", pos, 0.0, Vec2::new(1.0, 1.0), 0.0);
        // Original PNG row 0 is .##############. at alpha tolerance zero.
        assert!(mask.contains(pos, bbox));
        assert!(!mask.contains(pos, precise));
        assert!(mask.contains(Vec2::new(81.0, 80.0), precise));
        assert!(!mask.intersects(Rect::new(80.0, 80.0, 1.0, 1.0), precise));
        assert!(mask.intersects(Rect::new(81.0, 80.0, 1.0, 1.0), precise));
        // The boulder PNG has transparent corners, but its source mask is
        // explicitly RECTANGLE. Even prec=1 must retain those corners.
        let boulder = instance_mask("oBoulder", "sBoulder", pos, 0.0, Vec2::new(1.0, 1.0), 0.0);
        assert!(boulder.contains(Vec2::new(64.0, 64.0), precise));
    }

    #[test]
    fn precise_masks_select_current_frames_and_mirror_source_pixel_indices() {
        let precise = CollisionPrecision::Precise;
        let unit = Vec2::new(1.0, 1.0);
        let p = Vec2::new(13.0, 1.0);
        let frame0 = instance_mask(
            "oAlienBoss",
            "sAlienBossDie",
            Vec2::default(),
            0.0,
            unit,
            0.0,
        );
        let frame1 = instance_mask(
            "oAlienBoss",
            "sAlienBossDie",
            Vec2::default(),
            1.0,
            unit,
            0.0,
        );
        assert!(frame0.contains(p, CollisionPrecision::BoundingBox));
        assert!(!frame0.contains(p, precise));
        assert!(frame1.contains(p, precise));
        let pos = Vec2::new(80.0, 80.0);
        let mirror = instance_mask(
            "oDarkFall",
            "sDarkFall",
            pos,
            0.0,
            Vec2::new(-1.0, 1.0),
            0.0,
        );
        assert!(!mirror.contains(pos, precise));
        assert!(mirror.contains(Vec2::new(79.0, 80.0), precise));
        assert!(mirror.intersects(Rect::new(79.0, 80.0, 1.0, 1.0), precise));
        assert!(!mirror.intersects(Rect::new(80.0, 80.0, 1.0, 1.0), precise));
        let vertical = instance_mask(
            "oDarkFall",
            "sDarkFall",
            pos,
            0.0,
            Vec2::new(1.0, -1.0),
            0.0,
        );
        assert!(!vertical.contains(pos, precise));
        assert!(vertical.contains(Vec2::new(80.0, 79.0), precise));
    }

    #[test]
    fn collision_events_require_a_shared_opaque_pixel_for_precise_pairs() {
        let unit = Vec2::new(1.0, 1.0);
        let web = instance_mask(
            "oWebBall",
            "sWebBall",
            Vec2::new(80.0, 80.0),
            0.0,
            unit,
            0.0,
        );
        let corner = instance_mask("oRock", "sRock", Vec2::new(74.0, 72.0), 0.0, unit, 0.0);
        // Each alpha mask touches the other bounding box, but no opaque
        // pixels coincide. Checking only one partner's mask is insufficient.
        assert!(web.intersects(corner.bounds(), CollisionPrecision::Precise));
        assert!(corner.intersects(web.bounds(), CollisionPrecision::Precise));
        assert!(!web.intersects_mask(&corner));
        assert!(!corner.intersects_mask(&web));
        let contact = instance_mask("oRock", "sRock", Vec2::new(74.0, 74.0), 0.0, unit, 0.0);
        assert!(web.intersects_mask(&contact));
        assert!(contact.intersects_mask(&web));
        // The corresponding pixels must agree after both source mirrors.
        let mirror = Vec2::new(-1.0, -1.0);
        let web = instance_mask(
            "oWebBall",
            "sWebBall",
            Vec2::new(80.0, 80.0),
            0.0,
            mirror,
            0.0,
        );
        let corner = instance_mask("oRock", "sRock", Vec2::new(86.0, 88.0), 0.0, mirror, 0.0);
        let contact = instance_mask("oRock", "sRock", Vec2::new(86.0, 86.0), 0.0, mirror, 0.0);
        assert!(!web.intersects_mask(&corner));
        assert!(web.intersects_mask(&contact));
        let frame0 = instance_mask(
            "oAlienBoss",
            "sAlienBossDie",
            Vec2::default(),
            0.0,
            unit,
            0.0,
        );
        let frame1 = instance_mask(
            "oAlienBoss",
            "sAlienBossDie",
            Vec2::default(),
            1.0,
            unit,
            0.0,
        );
        // Spark frame 3 has just two opaque pixels, placed at (13,1)/(14,1).
        // Both are absent in boss frame 0 and present in frame 1.
        let spark = instance_mask(
            "oFlareSpark",
            "sFlareSpark",
            Vec2::new(14.0, 2.0),
            3.0,
            unit,
            0.0,
        );
        assert!(frame0.bounds().intersects(spark.bounds()));
        assert!(!frame0.intersects_mask(&spark));
        assert!(frame1.intersects_mask(&spark));
    }

    #[test]
    fn stretched_detector_events_inverse_sample_both_masks_and_round_edges() {
        let unit = Vec2::new(1.0, 1.0);
        for scale in [1.0, 2.0, 5.0, 6.0] {
            let detector = instance_mask(
                "oArrowTrapTest",
                "sRed",
                Vec2::new(96.0, 80.0),
                0.0,
                Vec2::new(scale, 1.0),
                0.0,
            );
            assert_eq!(
                detector.event_geometry(),
                Some((
                    Rect::new(96.0, 81.0, 16.0 * scale, 14.0),
                    Vec2::new(96.0, 80.0)
                )),
            );
            for (rock_x, hit) in [(93.0, false), (94.0, true)] {
                // Only the Rock's lower-right corner reaches the detector in
                // the first case; its next bottom-row pixel is opaque.
                let rock = instance_mask("oRock", "sRock", Vec2::new(rock_x, 78.0), 0.0, unit, 0.0);
                assert!(detector.bounds().intersects(rock.bounds()));
                assert_eq!(detector.intersects_mask(&rock), hit);
                assert_eq!(rock.intersects_mask(&detector), hit);
                assert!(detector.intersects(rock.bounds(), CollisionPrecision::BoundingBox));
            }
        }
        let fractional = instance_mask(
            "oArrowTrapTest",
            "sRed",
            Vec2::new(97.5, 80.5),
            0.0,
            Vec2::new(2.0, 1.0),
            0.0,
        );
        assert_eq!(
            fractional.event_geometry(),
            Some((Rect::new(98.0, 82.0, 31.0, 13.0), Vec2::new(98.0, 80.0))),
        );
        // Canonical origin rounding would permit image pixel15 at world129,
        // but the independently rounded aggregate bbox ends at128. The bbox
        // must reject it before the inverse scale sampler is consulted.
        for (spark_x, hit) in [(129.0, true), (130.0, false)] {
            let spark = instance_mask(
                "oFlareSpark",
                "sFlareSpark",
                Vec2::new(spark_x, 83.0),
                3.0,
                unit,
                0.0,
            );
            assert_eq!(fractional.intersects_mask(&spark), hit);
            assert_eq!(spark.intersects_mask(&fractional), hit);
        }
    }

    #[test]
    fn rotated_events_inverse_sample_alpha_with_independently_rounded_bounds() {
        let unit = Vec2::new(1.0, 1.0);
        let angle = -0.2_f64.atan();
        let arrow = instance_mask(
            "oArrow",
            "sArrowRight",
            Vec2::new(100.0, 100.0),
            0.0,
            unit,
            angle,
        );
        // Golden values from canonical 0x4AAAC1..0x4AADBF and the standalone
        // source-bit oracle: integer inclusive bbox [96,98,104,104].
        assert_eq!(
            arrow.event_geometry(),
            Some((Rect::new(96.0, 98.0, 9.0, 7.0), Vec2::new(100.0, 100.0))),
        );
        for (pos, bounds, origin) in [
            (
                Vec2::new(100.5, 100.5),
                Rect::new(96.0, 99.0, 9.0, 6.0),
                Vec2::new(100.0, 100.0),
            ),
            (
                Vec2::new(-100.5, -100.5),
                Rect::new(-105.0, -102.0, 9.0, 6.0),
                Vec2::new(-100.0, -100.0),
            ),
        ] {
            let fractional = instance_mask("oArrow", "sArrowRight", pos, 0.0, unit, angle);
            assert_eq!(fractional.event_geometry(), Some((bounds, origin)));
        }
        for (pos, expected) in [
            (Vec2::new(94.0, 95.0), false),
            (Vec2::new(105.0, 95.0), false),
            (Vec2::new(100.0, 100.0), true),
            (Vec2::new(94.0, 105.0), true),
        ] {
            let web = instance_mask("oWebBall", "sWebBall", pos, 0.0, unit, 0.0);
            assert!(arrow.bounds().intersects(web.bounds()));
            assert_eq!(arrow.intersects_mask(&web), expected, "WebBall at {pos:?}");
            assert_eq!(web.intersects_mask(&arrow), expected);
            // Source precision-zero queries and the explicit rotated shape
            // query fallback retain their established rectangular behavior.
            assert!(arrow.intersects(web.bounds(), CollisionPrecision::BoundingBox));
            assert!(arrow.intersects(web.bounds(), CollisionPrecision::Precise));
        }
        let mirrored = instance_mask(
            "oArrow",
            "sArrowRight",
            arrow.pos,
            0.0,
            Vec2::new(-1.0, 1.0),
            angle,
        );
        assert!(mirrored.event_geometry().is_none());
    }

    #[test]
    fn event_precision_preserves_explicit_queries_and_unsupported_transform_fallbacks() {
        let unit = Vec2::new(1.0, 1.0);
        let web = instance_mask(
            "oWebBall",
            "sWebBall",
            Vec2::new(80.0, 80.0),
            0.0,
            unit,
            0.0,
        );
        let rectangle = instance_mask("oBrick", "sBrick", Vec2::new(60.0, 60.0), 0.0, unit, 0.0);
        assert!(web.bounds().intersects(rectangle.bounds()));
        assert!(!web.intersects(rectangle.bounds(), CollisionPrecision::Precise));
        assert!(!web.intersects_mask(&rectangle));
        let corner = instance_mask("oRock", "sRock", Vec2::new(72.0, 72.0), 0.0, unit, 0.0);
        assert!(!web.intersects_mask(&corner));
        let fractional = instance_mask(
            "oWebBall",
            "sWebBall",
            Vec2::new(80.0, 80.25),
            0.0,
            unit,
            0.0,
        );
        assert!(!fractional.supports_precise_pixels());
        assert!(!fractional.intersects_mask(&corner));
        // The newly traced rounding belongs to Collision events only. The
        // explicit point/rectangle query transform contract is unchanged.
        assert!(fractional.intersects(corner.bounds(), CollisionPrecision::Precise));
        let scaled = instance_mask(
            "oWebBall",
            "sWebBall",
            Vec2::new(80.0, 80.25),
            0.0,
            Vec2::new(2.0, 2.0),
            0.0,
        );
        assert!(scaled.intersects_mask(&corner));
        let multiframe = instance_mask(
            "oAlienBoss",
            "sAlienBossDie",
            Vec2::new(0.25, 0.0),
            0.0,
            unit,
            0.0,
        );
        assert!(multiframe.event_geometry().is_some());
        let fractional_mirror = instance_mask(
            "oWebBall",
            "sWebBall",
            Vec2::new(80.25, 80.0),
            0.0,
            Vec2::new(-1.0, 1.0),
            0.0,
        );
        assert!(fractional_mirror.event_geometry().is_none());
        assert!(web.intersects(corner.bounds(), CollisionPrecision::BoundingBox));
    }

    #[test]
    fn mixed_events_test_filled_rectangle_pixels_against_precise_alpha() {
        let unit = Vec2::new(1.0, 1.0);
        let web = instance_mask(
            "oWebBall",
            "sWebBall",
            Vec2::new(100.0, 100.0),
            0.0,
            unit,
            0.0,
        );
        for (x, y, hit) in [
            (81.0, 80.0, false),
            (82.0, 81.0, true),
            (81.0, 81.0, false),
            (82.0, 80.0, false),
        ] {
            let snake = instance_mask("oSnake", "sSnakeLeft", Vec2::new(x, y), 0.0, unit, 0.0);
            assert!(snake.bounds().intersects(web.bounds()));
            assert_eq!(snake.intersects_mask(&web), hit, "snake at ({x},{y})");
            assert_eq!(web.intersects_mask(&snake), hit);
            let mirrored_snake = instance_mask(
                "oSnake",
                "sSnakeLeft",
                Vec2::new(-x, -y),
                0.0,
                Vec2::new(-1.0, -1.0),
                0.0,
            );
            let mirrored_web = instance_mask(
                "oWebBall",
                "sWebBall",
                Vec2::new(-100.0, -100.0),
                0.0,
                Vec2::new(-1.0, -1.0),
                0.0,
            );
            assert_eq!(mirrored_snake.intersects_mask(&mirrored_web), hit);
        }
        // Boulder PNG corners are transparent; its source RECTANGLE mask
        // still fills them, so a tiny precise spark contacts the corner.
        let boulder = instance_mask(
            "oBoulder",
            "sBoulder",
            Vec2::new(80.0, 80.0),
            0.0,
            unit,
            0.0,
        );
        let spark = instance_mask(
            "oFlareSpark",
            "sFlareSpark",
            Vec2::new(65.0, 65.0),
            3.0,
            unit,
            0.0,
        );
        assert!(boulder.intersects_mask(&spark));
    }

    #[test]
    fn event_envelopes_are_aggregate_but_rectangle_pixels_follow_current_frames() {
        let unit = Vec2::new(1.0, 1.0);
        for (web_pos, expected) in [
            (Vec2::new(80.0, 98.0), [true, false, false]),
            (Vec2::new(85.0, 80.0), [false, false, true]),
            (Vec2::new(85.0, 90.0), [true, true, true]),
        ] {
            let web = instance_mask("oWebBall", "sWebBall", web_pos, 0.0, unit, 0.0);
            for (frame, expected) in expected.into_iter().enumerate() {
                let bat = instance_mask(
                    "oBat",
                    "sBatLeft",
                    Vec2::new(80.0, 80.0),
                    frame as f64,
                    unit,
                    0.0,
                );
                assert_eq!(
                    bat.intersects_mask(&web),
                    expected,
                    "frame={frame}, web={web_pos:?}"
                );
            }
        }
        // Original engine's rounded aggregate bbox retains (82,93). Rounding
        // only frame 1's bbox would clip that real shared pixel away.
        let bat = instance_mask("oBat", "sBatLeft", Vec2::new(81.5, 81.5), 1.0, unit, 0.0);
        let web = instance_mask(
            "oWebBall",
            "sWebBall",
            Vec2::new(79.0, 96.0),
            0.0,
            unit,
            0.0,
        );
        assert_eq!(
            bat.event_geometry().unwrap().0,
            Rect::new(82.0, 84.0, 15.0, 11.0)
        );
        assert!(bat.intersects_mask(&web));
    }

    #[test]
    fn fractional_events_round_translated_edges_and_origins_independently() {
        let unit = Vec2::new(1.0, 1.0);
        for (position, expected_left, expected_origin) in [
            (80.25, 75.0, 80.0),
            (80.5, 76.0, 80.0),
            (80.75, 76.0, 81.0),
            (81.5, 76.0, 82.0),
            (-80.25, -85.0, -80.0),
            (-80.5, -86.0, -80.0),
            (-80.75, -86.0, -81.0),
            (-81.5, -86.0, -82.0),
        ] {
            let web = instance_mask(
                "oWebBall",
                "sWebBall",
                Vec2::new(position, position),
                0.0,
                unit,
                0.0,
            );
            let (bounds, origin) = web.event_geometry().unwrap();
            assert_eq!(bounds, Rect::new(expected_left, expected_left, 9.0, 9.0));
            assert_eq!(origin, Vec2::new(expected_origin, expected_origin));
        }
        // Even-width source boxes can grow or shrink at translated .5 ties.
        for (x, left, width) in [
            (80.5, 76.0, 9.0),
            (81.5, 78.0, 7.0),
            (-80.5, -84.0, 7.0),
            (-81.5, -86.0, 9.0),
        ] {
            let arrow = instance_mask("oArrow", "sArrowRight", Vec2::new(x, 80.0), 0.0, unit, 0.0);
            let (bounds, _) = arrow.event_geometry().unwrap();
            assert_eq!(bounds.x, left);
            assert_eq!(bounds.w, width);
        }
        // Independently traced canonical bboxes are disjoint. Rounding only
        // origins would instead put opaque pixels from these masks together.
        let rock = instance_mask("oRock", "sRock", Vec2::new(71.5, 80.0), 0.0, unit, 0.0);
        let web = instance_mask(
            "oWebBall",
            "sWebBall",
            Vec2::new(80.5, 80.0),
            0.0,
            unit,
            0.0,
        );
        assert_eq!(
            rock.event_geometry().unwrap().0,
            Rect::new(68.0, 77.0, 7.0, 7.0)
        );
        assert_eq!(
            web.event_geometry().unwrap().0,
            Rect::new(76.0, 75.0, 9.0, 9.0)
        );
        assert!(!rock.intersects_mask(&web));
    }

    #[test]
    fn bundled_precise_masks_have_valid_portable_bitset_ranges() {
        let data = include_bytes!("../../../assets/collision.bin");
        let mut precise_sprites = 0;
        let mut precise_frames = 0;
        for sprite in sprite_frames().values() {
            if sprite.mask.shape != "PRECISE" {
                continue;
            }
            precise_sprites += 1;
            for frame in &sprite.frames {
                let offset = frame.mask_offset.expect("precise frame offset");
                let stride = frame.mask_stride.expect("precise frame stride");
                let width = frame.mask_width.expect("precise frame width");
                let height = frame.mask_height.expect("precise frame height");
                assert_eq!(stride, width.div_ceil(8));
                assert!(offset + stride * height <= data.len());
                precise_frames += 1;
            }
        }
        assert_eq!(
            (precise_sprites, precise_frames, data.len()),
            (373, 1039, 117054)
        );
    }
}
