//! Original GameMaker object defaults and sprite collision rectangles.
//!
//! Positions use the original instance origin. Sprite bounds are inclusive in
//! the source; [`bounds`] converts them to the core's half-open rectangles.

mod source_real;

use crate::{CollisionPrecision, Rect, Vec2};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, VecDeque};
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
    static ANCESTORS: OnceLock<HashMap<&'static str, Vec<&'static str>>> = OnceLock::new();
    ANCESTORS
        .get_or_init(|| ancestry(&catalog().objects).into_iter().collect())
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

/// Full unscaled canvas dimensions of the first sprite frame, independent of
/// its origin and collision mask. Returns `None` for an unknown or empty sprite.
pub fn sprite_dimensions(name: &str) -> Option<(u32, u32)> {
    let frame = sprite_frames().get(name)?.frames.first()?;
    Some((frame.w, frame.h))
}

/// Unscaled first-frame sprite mask at its original source origin.
pub fn sprite_bounds(name: &str, pos: Vec2) -> Rect {
    static BOUNDS: OnceLock<HashMap<String, Rect>> = OnceLock::new();
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

/// Conservative envelope for a unit, unrotated terrain mask. Unlike
/// `sprite_bounds`, this includes every animation frame of a separate mask.
pub(crate) fn terrain_mask_envelope(object: &str, sprite: &str, pos: Vec2) -> Rect {
    static ENVELOPES: OnceLock<HashMap<&'static str, Rect>> = OnceLock::new();
    let bounds = ENVELOPES
        .get_or_init(|| {
            catalog()
                .sprites
                .iter()
                .map(|(name, sprite)| {
                    let [left, top, right, bottom] = sprite.bounds;
                    let bounds = if right < left || bottom < top {
                        Rect::new(0.0, 0.0, 0.0, 0.0)
                    } else {
                        Rect::new(
                            f64::from(left - sprite.origin[0]),
                            f64::from(top - sprite.origin[1]),
                            f64::from(right - left + 1),
                            f64::from(bottom - top + 1),
                        )
                    };
                    (name.as_str(), bounds)
                })
                .collect()
        })
        .get(mask_sprite(object_mask(object), sprite))
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
    w: u32,
    h: u32,
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
    source_image_angle_degrees: Option<f64>,
}

pub fn instance_mask(
    object: &str,
    current_sprite: &str,
    pos: Vec2,
    animation: f64,
    scale: Vec2,
    angle: f64,
) -> CollisionMask {
    instance_mask_with_source_angle(object, current_sprite, pos, animation, scale, angle, None)
}

pub(crate) fn instance_mask_with_source_angle(
    object: &str,
    current_sprite: &str,
    pos: Vec2,
    animation: f64,
    scale: Vec2,
    angle: f64,
    source_image_angle_degrees: Option<f64>,
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
        source_image_angle_degrees,
    }
}

impl CollisionMask {
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Whether the legacy smooth-Rect adapter can index an unrotated unit mask.
    /// Source-specific point, raw-corner rectangle and axis-line queries use
    /// their separately verified fractional-placement sampler below.
    pub fn supports_precise_pixels(&self) -> bool {
        self.angle == 0.0
            && self.scale.x.abs() == 1.0
            && self.scale.y.abs() == 1.0
            && self.pos.x.fract() == 0.0
            && self.pos.y.fract() == 0.0
    }

    pub(crate) fn supports_source_query_pixels(&self) -> bool {
        self.frame.is_some()
            && self.event_bounds.is_some()
            && (self.precise || self.rectangle)
            && self.scale.x.is_finite()
            && self.scale.y.is_finite()
            && self.pos.x.is_finite()
            && self.pos.y.is_finite()
            && self.source_query_geometry().is_some()
    }

    /// The source mattock adapter still admits only unrotated mask metadata.
    /// Widening explicit collision query transforms must not change that
    /// adapter's unsupported-transform fallback.
    pub(crate) fn supports_source_unrotated_query_pixels(&self) -> bool {
        self.angle == 0.0 && self.supports_source_query_pixels()
    }

    // Original 4AA958 unrotated cache and 48A9F4/48AB50 inverse sampling
    // use PC64 intermediates, independently rounded coordinates and both scales.
    // Integer unit placements with bounded metadata retain the exact cheap
    // path. Fractional/large placements use the original PC64 cache arithmetic.
    fn source_query_geometry(&self) -> Option<(Rect, Vec2)> {
        if self.angle != 0.0 {
            return self.event_geometry();
        }
        if self.scale.x.abs() == 1.0
            && self.scale.y.abs() == 1.0
            && self.pos.x.fract() == 0.0
            && self.pos.y.fract() == 0.0
            && self.pos.x.abs() <= 1073741824.0
            && self.pos.y.abs() <= 1073741824.0
            && self.origin.iter().all(|v| i64::from(*v).abs() <= 1048576)
            && self
                .event_bounds
                .is_some_and(|bounds| bounds.iter().all(|v| i64::from(*v).abs() <= 1048576))
        {
            return self.unrotated_unit_event_geometry();
        }
        let [left, top, right, bottom] = self.event_bounds?;
        let px = source_real::Real::from_f64(self.pos.x)?;
        let py = source_real::Real::from_f64(self.pos.y)?;
        let sx = source_real::Real::from_f64(self.scale.x)?;
        let sy = source_real::Real::from_f64(self.scale.y)?;
        let one = source_real::Real::from_i32(1);
        let x0 = px
            .add(source_real::Real::from_i32(left.wrapping_sub(self.origin[0])).mul(sx))
            .round_i32()?;
        let x1 = px
            .add(
                source_real::Real::from_i32(right.wrapping_sub(self.origin[0]).wrapping_add(1))
                    .mul(sx),
            )
            .sub(one)
            .round_i32()?;
        let y0 = py
            .add(source_real::Real::from_i32(top.wrapping_sub(self.origin[1])).mul(sy))
            .round_i32()?;
        let y1 = py
            .add(
                source_real::Real::from_i32(bottom.wrapping_sub(self.origin[1]).wrapping_add(1))
                    .mul(sy),
            )
            .sub(one)
            .round_i32()?;
        Some((
            Rect::new(
                f64::from(x0.min(x1)),
                f64::from(y0.min(y1)),
                f64::from(x0.max(x1)) - f64::from(x0.min(x1)) + 1.0,
                f64::from(y0.max(y1)) - f64::from(y0.min(y1)) + 1.0,
            ),
            Vec2::new(f64::from(px.round_i32()?), f64::from(py.round_i32()?)),
        ))
    }

    fn source_query_round(value: f64) -> Option<i32> {
        // Original 403498 returns the LOW word of a signed64 FISTP result.
        source_real::Real::from_f64(value)?.round_i32()
    }

    fn source_query_exclusive_edges(&self, bounds: Rect) -> (f64, f64) {
        if self.angle == 0.0 && self.frame.is_some() && self.source_query_geometry().is_some() {
            // Original 4AC745/4AD063 INC the signed32 cached R/B before FILD.
            let right = ((bounds.x + bounds.w - 1.0) as i32).wrapping_add(1);
            let bottom = ((bounds.y + bounds.h - 1.0) as i32).wrapping_add(1);
            (f64::from(right), f64::from(bottom))
        } else {
            (bounds.x + bounds.w, bounds.y + bounds.h)
        }
    }

    fn source_query_pixel(&self, x: i32, y: i32) -> bool {
        if self.angle != 0.0 {
            // Explicit shape queries keep their established binary64
            // transform. Automatic object-event sampling below models the
            // executable's stored QWORD trig coefficients and x87 inverse.
            if self.angle.abs() == std::f64::consts::PI
                && self.scale.x.abs() == 1.0
                && self.scale.y.abs() == 1.0
            {
                return self.half_turn_source_pixel(x, y);
            }
            let (sin, cos) = self.angle.sin_cos();
            let image_x = ((f64::from(x) * cos + f64::from(y) * sin) / self.scale.x
                + f64::from(self.origin[0]))
            .floor() as i32;
            let image_y = ((f64::from(y) * cos - f64::from(x) * sin) / self.scale.y
                + f64::from(self.origin[1]))
            .floor() as i32;
            return self.image_pixel(image_x, image_y);
        }
        if self.scale.x.abs() == 1.0 && self.scale.y.abs() == 1.0 {
            let ix = self.origin[0].wrapping_add(if self.scale.x > 0.0 {
                x
            } else {
                x.wrapping_neg()
            });
            let iy = self.origin[1].wrapping_add(if self.scale.y > 0.0 {
                y
            } else {
                y.wrapping_neg()
            });
            return self.image_pixel(ix, iy);
        }
        let inverse_pixel = |relative: i32, scale: f64, origin: i32| -> Option<i32> {
            if scale == 0.0 {
                // Native 48A9F4 still performs FILD/Fdiv/FADD for signed
                // zero. The following 4238E8 floor adapter converts the
                // masked NaN/Inf FISTP result (low word 0), sees an unordered
                // fractional comparison, and decrements the final image
                // coordinate to -1, which is outside the mask.
                return Some(-1);
            }
            source_real::Real::from_i32(relative)
                .div(source_real::Real::from_f64(scale)?)
                .map(|n| n.add(source_real::Real::from_i32(origin)))?
                .floor_i32()
        };
        let Some(ix) = inverse_pixel(x, self.scale.x, self.origin[0]) else {
            return false;
        };
        let Some(iy) = inverse_pixel(y, self.scale.y, self.origin[1]) else {
            return false;
        };
        self.image_pixel(ix, iy)
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
    pub(crate) fn point_bounds(&self) -> Rect {
        if self.frame.is_none() {
            return self.bounds;
        }
        self.source_query_geometry()
            .map_or(self.bounds, |(bounds, _)| bounds)
    }

    pub fn contains(&self, point: Vec2, precision: CollisionPrecision) -> bool {
        // Canonical 0x4AC73F compares the ORIGINAL doubles to integer bbox
        // edges. Rounding the query first would admit points just outside it.
        let bounds = self.point_bounds();
        let (right, bottom) = self.source_query_exclusive_edges(bounds);
        if !bounds.is_valid()
            || point.x < bounds.x
            || point.y < bounds.y
            || point.x >= right
            || point.y >= bottom
            || !point.x.is_finite()
            || !point.y.is_finite()
        {
            return false;
        }
        if precision == CollisionPrecision::BoundingBox || !self.supports_source_query_pixels() {
            return true;
        }
        // 0x4AC7D2..802 rounds the query and instance origins separately.
        // Unlike the rectangle kernel, point sampling does not clamp the
        // rounded query to the cached bbox a second time. RECTANGLE masks
        // are selected boolean pixels too, not a precise-query shortcut.
        let (Some(qx), Some(qy), Some(px), Some(py)) = (
            Self::source_query_round(point.x),
            Self::source_query_round(point.y),
            Self::source_query_round(self.pos.x),
            Self::source_query_round(self.pos.y),
        ) else {
            return true;
        }; // Unsupported native FISTP exception domain retains bbox fallback.
        self.source_query_pixel(qx.wrapping_sub(px), qy.wrapping_sub(py))
    }

    /// The source `collision_rectangle(..., 0, ...)` predicate. Query corners
    /// remain raw inclusive endpoints; the target uses its cached aggregate
    /// integer bounds for the verified source transforms, as point queries do.
    /// This also permits reversed corners and zero-area query rectangles.
    pub fn intersects_rectangle_bbox(&self, first: Vec2, second: Vec2) -> bool {
        if !first.x.is_finite()
            || !first.y.is_finite()
            || !second.x.is_finite()
            || !second.y.is_finite()
        {
            return false;
        }
        let bounds = self.point_bounds();
        let (right, bottom) = self.source_query_exclusive_edges(bounds);
        // Canonical 0x4AC834..0x4AC923 compares the raw endpoints against
        // inclusive cached edges. In particular, do not add one to query max.
        bounds.is_valid()
            && right > first.x.min(second.x)
            && bounds.x <= first.x.max(second.x)
            && bottom > first.y.min(second.y)
            && bounds.y <= first.y.max(second.y)
    }

    /// Source `collision_rectangle` with raw inclusive corners. Unlike the
    /// legacy smooth-Rect adapter, zero-area and reversed source arguments
    /// remain queries, and precise RECTANGLE masks sample selected pixels.
    /// Unsupported transforms retain the legacy Rect adapter for precision.
    pub fn intersects_rectangle(
        &self,
        first: Vec2,
        second: Vec2,
        precision: CollisionPrecision,
    ) -> bool {
        if !first.x.is_finite()
            || !first.y.is_finite()
            || !second.x.is_finite()
            || !second.y.is_finite()
        {
            return false;
        }
        if precision == CollisionPrecision::BoundingBox {
            return self.intersects_rectangle_bbox(first, second);
        }
        if !self.supports_source_query_pixels() {
            return self.intersects(
                Rect::new(
                    first.x.min(second.x),
                    first.y.min(second.y),
                    (first.x - second.x).abs() + 1.0,
                    (first.y - second.y).abs() + 1.0,
                ),
                precision,
            );
        }
        // 0x4AC834 performs the raw bbox gate before any endpoint conversion.
        if !self.intersects_rectangle_bbox(first, second) {
            return false;
        }
        let Some((bounds, origin)) = self.source_query_geometry() else {
            return false;
        };
        // 0x4AC9C6..4ACA35 independently rounds sorted query corners and the
        // instance origin; 0x48AB50 clips the resulting inclusive pixel box.
        let (Some(left), Some(right), Some(top), Some(bottom)) = (
            Self::source_query_round(first.x.min(second.x)),
            Self::source_query_round(first.x.max(second.x)),
            Self::source_query_round(first.y.min(second.y)),
            Self::source_query_round(first.y.max(second.y)),
        ) else {
            return self.intersects(
                Rect::new(
                    first.x.min(second.x),
                    first.y.min(second.y),
                    (first.x - second.x).abs() + 1.0,
                    (first.y - second.y).abs() + 1.0,
                ),
                precision,
            );
        };
        // 419E08/407CD4 do not re-sort signed endpoints after low32 conversion.
        for y in top.max(bounds.y as i32)..=bottom.min((bounds.y + bounds.h - 1.0) as i32) {
            for x in left.max(bounds.x as i32)..=right.min((bounds.x + bounds.w - 1.0) as i32) {
                if self.source_query_pixel(
                    x.wrapping_sub(origin.x as i32),
                    y.wrapping_sub(origin.y as i32),
                ) {
                    return true;
                }
            }
        }
        false
    }

    /// Source axis-aligned `collision_line`: raw endpoints first meet the
    /// cached integer aggregate bbox, then clipped endpoints and instance
    /// origins round independently before selected-mask pixel sampling.
    /// Fractional unit-scale placement, both unit mirrors, and positive
    /// unit-scale rotations use the source query pixel sampler. Rotated mirrors
    /// and non-unit rotated scales retain the Rect adapter.
    pub fn intersects_axis_line(
        &self,
        first: Vec2,
        second: Vec2,
        precision: CollisionPrecision,
    ) -> bool {
        if !first.x.is_finite()
            || !first.y.is_finite()
            || !second.x.is_finite()
            || !second.y.is_finite()
            || (first.x != second.x && first.y != second.y)
        {
            return false;
        }
        let query = Rect::new(
            first.x.min(second.x),
            first.y.min(second.y),
            (first.x - second.x).abs() + 1.0,
            (first.y - second.y).abs() + 1.0,
        );
        if !self.supports_source_query_pixels() {
            return self.intersects(query, precision);
        }
        let Some((bounds, origin)) = self.source_query_geometry() else {
            return self.intersects(query, precision);
        };
        let (right_exclusive, bottom_exclusive) = self.source_query_exclusive_edges(bounds);
        // 0x4ACEEC rejects raw endpoints before any RTE conversion, using
        // inclusive cached edges. A query just outside must stay outside.
        if !bounds.is_valid()
            || right_exclusive <= first.x.min(second.x)
            || bounds.x > first.x.max(second.x)
            || bottom_exclusive <= first.y.min(second.y)
            || bounds.y > first.y.max(second.y)
        {
            return false;
        }
        // 0x4AD024 clips a horizontal segment to [L,R+1] before rounding.
        // A vertical segment surviving the raw bbox test needs no X clipping.
        if precision == CollisionPrecision::BoundingBox {
            return true;
        }
        let (Some(left), Some(right), Some(y0), Some(y1)) = (
            Self::source_query_round(first.x.min(second.x).max(bounds.x)),
            Self::source_query_round(first.x.max(second.x).min(right_exclusive)),
            Self::source_query_round(first.y),
            Self::source_query_round(second.y),
        ) else {
            return self.intersects(query, precision);
        };
        // 48B100 sorts ONLY converted signed Y. X remains the wrapper's
        // raw preordered/clipped pair, even if low32 conversion reverses it.
        let top = y0.min(y1);
        let bottom = y0.max(y1);
        // 0x48B0D4 delegates an axis line/point to the inclusive rectangle
        // pixel kernel0x48AB50, which clamps to the aggregate cached bounds.
        for y in top.max(bounds.y as i32)..=bottom.min((bounds.y + bounds.h - 1.0) as i32) {
            for x in left.max(bounds.x as i32)..=right.min((bounds.x + bounds.w - 1.0) as i32) {
                // Rectangle source masks are materialized boolean pixels too:
                // an aggregate bbox hit alone can include an empty half-tie row.
                if self.source_query_pixel(
                    x.wrapping_sub(origin.x as i32),
                    y.wrapping_sub(origin.y as i32),
                ) {
                    return true;
                }
            }
        }
        false
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
        if self.angle == 0.0 {
            return self.source_query_geometry();
        }
        if !self.angle.is_finite()
            || !self.pos.x.is_finite()
            || !self.pos.y.is_finite()
            || !self.scale.x.is_finite()
            || !self.scale.y.is_finite()
            || (self.angle != 0.0 && (self.scale.x == 0.0 || self.scale.y == 0.0))
        {
            return None;
        }
        let [left, top, right, bottom] = self.event_bounds?;
        if self.angle != 0.0 {
            // Canonical 0x4AAAC1..0x4AADBF transforms the aggregate source
            // sprite envelope's inclusive endpoints, then rounds each extreme
            // independently. Its positive GML angle is our negative screen
            // angle. The instance origin is rounded separately for sampling.
            // 0x4AAAC1..0x4AADBF scales the inclusive source bbox edges
            // before rotating. It represents the far edge as (max+1)*scale-1,
            // matching the native unrotated cache's pixel-inclusive extent.
            if let Some((inverse_sin, cos)) =
                source_event_trig(self.angle, self.source_image_angle_degrees)
            {
                return source_rotated_event_geometry(
                    [left, top, right, bottom],
                    self.origin,
                    self.pos,
                    self.scale,
                    -inverse_sin,
                    cos,
                );
            }
            let x0 = f64::from(left.min(right) - self.origin[0]) * self.scale.x;
            let x1 = f64::from(left.max(right) - self.origin[0] + 1) * self.scale.x - 1.0;
            let y0 = f64::from(top.min(bottom) - self.origin[1]) * self.scale.y;
            let y1 = f64::from(top.max(bottom) - self.origin[1] + 1) * self.scale.y - 1.0;
            let (x0, x1) = (x0.min(x1), x0.max(x1));
            let (y0, y1) = (y0.min(y1), y0.max(y1));
            let (sin, cos) = source_event_trig(self.angle, self.source_image_angle_degrees)
                .map(|(inverse_sin, cos)| (-inverse_sin, cos))
                .unwrap_or_else(|| (-self.angle).sin_cos());
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
        None
    }

    fn unrotated_unit_event_geometry(&self) -> Option<(Rect, Vec2)> {
        let [left, top, right, bottom] = self.event_bounds?;
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
    /// Finite nonzero scales and rotations use the native source cache and
    /// inverse pixel sampler, including fractional placements and mirrors.
    /// Zero-scale rotated transforms retain the bbox fallback because the
    /// native inverse sampler divides by each image scale.
    pub fn intersects_mask(&self, other: &Self) -> bool {
        if !(self.precise || self.rectangle) || !(other.precise || other.rectangle) {
            return self.bounds.intersects(other.bounds);
        }
        let (Some((bounds, pos)), Some((other_bounds, other_pos))) =
            (self.event_geometry(), other.event_geometry())
        else {
            return self.bounds.intersects(other.bounds);
        };
        if self.angle == 0.0 && other.angle == 0.0 {
            return self.intersects_unrotated_mask(other, bounds, pos, other_bounds, other_pos);
        }
        if !bounds.intersects(other_bounds) {
            return false;
        }
        let left = bounds.x.max(other_bounds.x) as i32;
        let top = bounds.y.max(other_bounds.y) as i32;
        let right = (bounds.x + bounds.w).min(other_bounds.x + other_bounds.w) as i32;
        let bottom = (bounds.y + bounds.h).min(other_bounds.y + other_bounds.h) as i32;
        let (sin, cos) = source_event_trig(self.angle, self.source_image_angle_degrees)
            .unwrap_or_else(|| self.angle.sin_cos());
        let (other_sin, other_cos) =
            source_event_trig(other.angle, other.source_image_angle_degrees)
                .unwrap_or_else(|| other.angle.sin_cos());
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

    fn intersects_unrotated_mask(
        &self,
        other: &Self,
        bounds: Rect,
        pos: Vec2,
        other_bounds: Rect,
        other_pos: Vec2,
    ) -> bool {
        // 4AD1AD..4AD1F3 compares the signed cached R/B successors; INC
        // wraps before the double gate. 48A51F..48A583 then clips inclusive
        // signed cache edges. Both origins were independently FISTP-rounded.
        let right = (bounds.x + bounds.w - 1.0) as i32;
        let bottom = (bounds.y + bounds.h - 1.0) as i32;
        let other_right = (other_bounds.x + other_bounds.w - 1.0) as i32;
        let other_bottom = (other_bounds.y + other_bounds.h - 1.0) as i32;
        if bounds.x >= f64::from(other_right.wrapping_add(1))
            || other_bounds.x >= f64::from(right.wrapping_add(1))
            || bounds.y >= f64::from(other_bottom.wrapping_add(1))
            || other_bounds.y >= f64::from(bottom.wrapping_add(1))
        {
            return false;
        }
        let left = (bounds.x as i32).max(other_bounds.x as i32);
        let top = (bounds.y as i32).max(other_bounds.y as i32);
        let right = right.min(other_right);
        let bottom = bottom.min(other_bottom);
        // Native SUB/JL uses SF xor OF (signed operand ordering), followed
        // by wrapping count/INC. The inclusive integer ranges visit the same
        // finite pixels; the selected mask checks have no gameplay callbacks.
        for x in left..=right {
            for y in top..=bottom {
                if self
                    .source_query_pixel(x.wrapping_sub(pos.x as i32), y.wrapping_sub(pos.y as i32))
                    && other.source_query_pixel(
                        x.wrapping_sub(other_pos.x as i32),
                        y.wrapping_sub(other_pos.y as i32),
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
            return self.source_query_pixel(x, y);
        }
        if self.angle.abs() == std::f64::consts::PI
            && self.scale.x.abs() == 1.0
            && self.scale.y.abs() == 1.0
        {
            // At a half turn, the source's x87 sin(-180 degrees) retains a
            // signed sub-ulp residual through the final floor. `f64::sin`
            // returns a much larger residual with the opposite sign, and
            // adding an integer origin can either erase or reverse that
            // one-pixel decision. Preserve the source residual's sign without
            // depending on host libm or x87 availability (including Wasm).
            return self.half_turn_source_pixel(x, y);
        }
        // 0x48A7DD..0x48A987 receives image_angle in degrees. Rebuild that
        // source angle from our screen-radian adapter, round its degree to
        // radian conversion once, then use the deterministic PC64 arithmetic
        // model for the source's x87 multiply/add/divide/floor sequence.
        let Some(image_x) = source_inverse_pixel(x, y, cos, sin, self.scale.x, self.origin[0])
        else {
            return false;
        };
        let Some(image_y) = source_inverse_pixel(y, x, cos, -sin, self.scale.y, self.origin[1])
        else {
            return false;
        };
        self.image_pixel(image_x, image_y)
    }

    fn half_turn_source_pixel(&self, x: i32, y: i32) -> bool {
        // At a half turn, the source's x87 sin(-180 degrees) retains a
        // signed sub-ulp residual through the final floor. `f64::sin`
        // returns a much larger residual with the opposite sign, and adding
        // an integer origin can either erase or reverse that one-pixel
        // decision. Preserve the source residual's sign portably.
        let sine_sign = if self.angle.is_sign_negative() {
            1_i64
        } else {
            -1_i64
        };
        let scale_x_sign = if self.scale.x > 0.0 { 1_i64 } else { -1_i64 };
        let scale_y_sign = if self.scale.y > 0.0 { 1_i64 } else { -1_i64 };
        let x_residual_sign = sine_sign * i64::from(y.signum()) * scale_x_sign;
        let y_residual_sign = -sine_sign * i64::from(x.signum()) * scale_y_sign;
        let mut image_x = self.origin[0].wrapping_add(if self.scale.x > 0.0 {
            x.wrapping_neg()
        } else {
            x
        });
        let mut image_y = self.origin[1].wrapping_add(if self.scale.y > 0.0 {
            y.wrapping_neg()
        } else {
            y
        });
        if x_residual_sign < 0 {
            image_x = image_x.wrapping_sub(1);
        }
        if y_residual_sign < 0 {
            image_y = image_y.wrapping_sub(1);
        }
        self.image_pixel(image_x, image_y)
    }
}

fn source_scaled_extent(
    low: i32,
    high: i32,
    origin: i32,
    scale: source_real::Real,
) -> Option<(source_real::Real, source_real::Real)> {
    let one = source_real::Real::from_i32(1);
    let first = source_real::Real::from_i32(low.wrapping_sub(origin)).mul(scale);
    let last = source_real::Real::from_i32(high.wrapping_sub(origin).wrapping_add(1))
        .mul(scale)
        .sub(one);
    Some(if first.cmp(last).is_le() {
        (first, last)
    } else {
        (last, first)
    })
}

fn source_rotated_extent(
    low: source_real::Real,
    high: source_real::Real,
    coefficient: source_real::Real,
) -> (source_real::Real, source_real::Real) {
    let first = low.mul(coefficient);
    let last = high.mul(coefficient);
    if first.cmp(last).is_le() {
        (first, last)
    } else {
        (last, first)
    }
}

fn source_rotated_event_geometry(
    bounds: [i32; 4],
    origin: [i32; 2],
    pos: Vec2,
    scale: Vec2,
    sin: f64,
    cos: f64,
) -> Option<(Rect, Vec2)> {
    let [left, top, right, bottom] = bounds;
    let scale_x = source_real::Real::from_f64(scale.x)?;
    let scale_y = source_real::Real::from_f64(scale.y)?;
    let sin = source_real::Real::from_f64(sin)?;
    let cos = source_real::Real::from_f64(cos)?;
    let (x0, x1) = source_scaled_extent(left, right, origin[0], scale_x)?;
    let (y0, y1) = source_scaled_extent(top, bottom, origin[1], scale_y)?;
    let (cos_x0, cos_x1) = source_rotated_extent(x0, x1, cos);
    let (sin_y0, sin_y1) = source_rotated_extent(y0, y1, sin);
    let (cos_y0, cos_y1) = source_rotated_extent(y0, y1, cos);
    let (sin_x0, sin_x1) = source_rotated_extent(x0, x1, sin);
    let pos_x = source_real::Real::from_f64(pos.x)?;
    let pos_y = source_real::Real::from_f64(pos.y)?;
    let left = pos_x.add(cos_x0).add(sin_y0).round_i32()?;
    let right = pos_x.add(cos_x1).add(sin_y1).round_i32()?;
    let top = pos_y.add(cos_y0).sub(sin_x1).round_i32()?;
    let bottom = pos_y.add(cos_y1).sub(sin_x0).round_i32()?;
    let rounded_x = pos_x.round_i32()?;
    let rounded_y = pos_y.round_i32()?;
    Some((
        Rect::new(
            f64::from(left),
            f64::from(top),
            f64::from(right.wrapping_sub(left).wrapping_add(1)),
            f64::from(bottom.wrapping_sub(top).wrapping_add(1)),
        ),
        Vec2::new(f64::from(rounded_x), f64::from(rounded_y)),
    ))
}

type SourceTrigKey = (u8, u64);

#[derive(Default)]
struct SourceTrigCache {
    values: HashMap<SourceTrigKey, (f64, f64)>,
    insertion_order: VecDeque<SourceTrigKey>,
}

impl SourceTrigCache {
    fn touch(&mut self, key: SourceTrigKey) {
        if let Some(position) = self.insertion_order.iter().position(|entry| *entry == key) {
            self.insertion_order.remove(position);
        }
        self.insertion_order.push_back(key);
    }

    fn get(&mut self, key: SourceTrigKey) -> Option<(f64, f64)> {
        let coefficients = self.values.get(&key).copied()?;
        self.touch(key);
        Some(coefficients)
    }

    fn insert(&mut self, key: SourceTrigKey, coefficients: (f64, f64)) {
        const CAPACITY: usize = 256;
        if self.values.contains_key(&key) {
            self.values.insert(key, coefficients);
            self.touch(key);
            return;
        }
        if self.values.len() == CAPACITY {
            if let Some(oldest) = self.insertion_order.pop_front() {
                self.values.remove(&oldest);
            }
        }
        self.insertion_order.push_back(key);
        self.values.insert(key, coefficients);
    }
}

/// Reconstruct degrees only when the source image angle is unavailable, then
/// compute QWORD coefficients using the pinned portable x87 profile.
fn source_event_trig(
    screen_angle: f64,
    source_image_angle_degrees: Option<f64>,
) -> Option<(f64, f64)> {
    if !screen_angle.is_finite() {
        return None;
    }
    if screen_angle == 0.0 && source_image_angle_degrees.is_none() {
        return Some((0.0, 1.0));
    }
    let source_degrees = source_image_angle_degrees.unwrap_or_else(|| -screen_angle.to_degrees());
    if !source_degrees.is_finite() || source_degrees.abs() > 360.0 {
        return None;
    }
    let key = source_image_angle_degrees.map_or((0, screen_angle.to_bits()), |degrees| {
        (1, degrees.to_bits())
    });
    static CACHE: OnceLock<std::sync::Mutex<SourceTrigCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(SourceTrigCache::default()));
    if let Ok(mut cache) = cache.lock() {
        if let Some(coefficients) = cache.get(key) {
            return Some(coefficients);
        }
    }
    let coefficients = source_degree_trig(source_degrees)?;
    if let Ok(mut cache) = cache.lock() {
        cache.insert(key, coefficients);
    }
    Some(coefficients)
}

fn source_degree_trig(source_degrees: f64) -> Option<(f64, f64)> {
    let radians = source_degrees_to_radians(source_degrees)?;
    crate::source_x87_trig::qword_sin_cos(radians).ok()
}

/// GameMaker's `sin` argument is in degrees. Use the same pinned source trig
/// path as collision geometry for the bounded angles produced by the game;
/// retain the host fallback for out-of-profile values.
pub(crate) fn source_sin_degrees(source_degrees: f64) -> f64 {
    source_degree_trig(source_degrees)
        .map(|(sine, _)| sine)
        .unwrap_or_else(|| source_degrees.to_radians().sin())
}

fn source_degrees_to_radians(source_degrees: f64) -> Option<crate::finite_pc64::F128> {
    use crate::finite_pc64 as f;

    // The inverse-mask trace and pinned standalone-x87 profile are supported
    // over one full turn in either direction. Other finite angles retain the
    // existing host-libm fallback selected by the caller.
    if !source_degrees.is_finite() || source_degrees.abs() > 360.0 {
        return None;
    }
    let degrees = f::from_f64(source_degrees).ok()?;
    // 0x48A9E4 loads the executable's binary80 PI constant.
    let pi = f::from_ext80(f::Ext80 {
        sign_exp: 0x4000,
        significand: 0xc90f_daa2_2168_c235,
    })
    .ok()?;
    let product = f::mul64_ne(degrees, pi).ok()?;
    f::div64_ne(product, f::from_f64(180.0).ok()?).ok()
}

/// Apply the source's x87 PC=11 inverse coordinate sequence with finite
/// operands. The y path passes a negative sine into this same multiply/add
/// helper to preserve the source's `y*cos - x*sin` evaluation order.
fn source_inverse_pixel(
    first_axis: i32,
    second_axis: i32,
    first_trig: f64,
    second_trig: f64,
    scale: f64,
    origin: i32,
) -> Option<i32> {
    source_inverse_coordinate(
        first_axis,
        second_axis,
        first_trig,
        second_trig,
        scale,
        origin,
    )?
    .floor_i32()
}

fn source_inverse_coordinate(
    first_axis: i32,
    second_axis: i32,
    first_trig: f64,
    second_trig: f64,
    scale: f64,
    origin: i32,
) -> Option<source_real::Real> {
    let first =
        source_real::Real::from_i32(first_axis).mul(source_real::Real::from_f64(first_trig)?);
    let second =
        source_real::Real::from_i32(second_axis).mul(source_real::Real::from_f64(second_trig)?);
    first
        .add(second)
        .div(source_real::Real::from_f64(scale)?)
        .map(|coordinate| coordinate.add(source_real::Real::from_i32(origin)))
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
    use crate::{Area, Game, InputFrame, RulesPreset, Vec2, World};
    use std::collections::BTreeSet;

    #[test]
    fn sprite_dimensions_match_canonical_png_canvases_and_animation_variants() {
        let cases: [(&str, (u32, u32), &[u8]); 6] = [
            (
                "sGiantSpider",
                (32, 32),
                include_bytes!(
                    "../../../references/classic-1.1/Sprites/Enemies/GiantSpider/sGiantSpider.images/image 0.png"
                ),
            ),
            (
                "sGiantSpiderHang",
                (32, 16),
                include_bytes!(
                    "../../../references/classic-1.1/Sprites/Enemies/GiantSpider/sGiantSpiderHang.images/image 0.png"
                ),
            ),
            (
                "sHawkLeft",
                (16, 16),
                include_bytes!(
                    "../../../references/classic-1.1/Sprites/Enemies/Hawkman/sHawkLeft.images/image 0.png"
                ),
            ),
            (
                "sHawkStunL",
                (24, 16),
                include_bytes!(
                    "../../../references/classic-1.1/Sprites/Enemies/Hawkman/sHawkStunL.images/image 0.png"
                ),
            ),
            (
                "sManTrapEatL",
                (32, 32),
                include_bytes!(
                    "../../../references/classic-1.1/Sprites/Enemies/ManTrap/sManTrapEatL.images/image 0.png"
                ),
            ),
            (
                "sYetiDeadL",
                (24, 16),
                include_bytes!(
                    "../../../references/classic-1.1/Sprites/Enemies/Yeti/sYetiDeadL.images/image 0.png"
                ),
            ),
        ];
        for (name, expected, png) in cases {
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "{name}");
            assert_eq!(&png[12..16], b"IHDR", "{name}");
            let png_dimensions = (
                u32::from_be_bytes(png[16..20].try_into().unwrap()),
                u32::from_be_bytes(png[20..24].try_into().unwrap()),
            );
            assert_eq!(png_dimensions, expected, "canonical PNG: {name}");
            assert_eq!(sprite_dimensions(name), Some(png_dimensions), "{name}");
        }
    }

    #[test]
    fn sprite_dimensions_are_canvas_sizes_rather_than_collision_mask_sizes() {
        for (name, canvas, mask_size) in [
            ("sGiantSpider", (32, 32), (30, 15)),
            ("sHawkStunL", (24, 16), (20, 16)),
        ] {
            let [left, top, right, bottom] = sprite_def(name).unwrap().bounds;
            let actual_mask_size = ((right - left + 1) as u32, (bottom - top + 1) as u32);
            assert_eq!(actual_mask_size, mask_size, "{name}");
            assert_ne!(canvas, actual_mask_size, "{name}");
            assert_eq!(sprite_dimensions(name), Some(canvas), "{name}");
        }
    }

    #[test]
    fn bundled_sprite_canvases_are_positive_and_consistent_across_frames() {
        // The API specifies the first frame. Validate the current source corpus
        // without requiring future sprite data to have uniform frame canvases.
        for (name, sprite) in sprite_frames() {
            let first = sprite.frames.first().expect("bundled sprite has a frame");
            assert!(first.w > 0 && first.h > 0, "{name}");
            assert_eq!(sprite_dimensions(name), Some((first.w, first.h)), "{name}");
            for frame in &sprite.frames {
                assert_eq!((frame.w, frame.h), (first.w, first.h), "{name}");
            }
        }
    }

    #[test]
    fn sprite_dimensions_unknown_names_do_not_fall_back_to_object_sprites() {
        assert!(sprite("oGiantSpider").is_some());
        for name in ["", "missing", "oGiantSpider"] {
            assert_eq!(sprite_dimensions(name), None, "{name}");
        }
    }

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
        assert!(mirrored.event_geometry().is_some());
    }

    #[test]
    fn rotated_drip_x87_sine_rounding_keeps_a_source_mask_overlap() {
        // The measured standalone x87 FSIN result for this retained raw
        // image_angle rounds to bfe2c89f471e63fc. The adjacent Taylor QWORD
        // changes the bottom bbox edge from 1 to 0 and rejects this overlap.
        let source_degrees = 35.9437_f64;
        let a = instance_mask_with_source_angle(
            "oDrip",
            "sDrip",
            Vec2::new(0.0, -0.08699001207708024),
            0.0,
            Vec2::new(1.0, 1.0),
            -source_degrees.to_radians(),
            Some(source_degrees),
        );
        let b = instance_mask(
            "oDrip",
            "sDrip",
            Vec2::new(1.0, 2.0),
            0.0,
            Vec2::new(1.0, 1.0),
            0.0,
        );

        assert_eq!(
            source_event_trig(a.angle, a.source_image_angle_degrees)
                .unwrap()
                .0
                .to_bits(),
            0xbfe2_c89f_471e_63fc,
            "source FSIN QWORD coefficient"
        );
        assert_eq!(
            a.event_geometry().unwrap().0,
            Rect::new(-1.0, -1.0, 2.0, 3.0)
        );
        assert!(a.intersects_mask(&b));
    }

    #[test]
    fn source_degree_rounding_keeps_the_x87_boundary_on_the_transparent_row() {
        let image_angle = 216.86989764584402_f64;
        let source_radians = source_degrees_to_radians(image_angle).unwrap();
        assert_eq!(
            crate::finite_pc64::to_f64_nearest(source_radians).unwrap(),
            3.7850937623830778_f64
        );

        let screen_angle = -(std::f64::consts::PI + 0.75_f64.atan());
        assert_eq!(-screen_angle.to_degrees(), image_angle);
        let (sin, cos) = source_event_trig(screen_angle, Some(image_angle)).unwrap();
        assert_eq!(sin, 0.6);
        assert_eq!(cos, -0.8);
        let image_x = source_inverse_coordinate(-4, -2, cos, sin, 1.0, 4).unwrap();
        let image_y = source_inverse_coordinate(-2, -4, cos, -sin, 1.0, 4).unwrap();
        let source_x = source_real::Real::from_i32(6)
            .add(source_real::Real::from_f64(2.220446049250313e-16).unwrap());
        assert_eq!(image_x, source_x);
        assert_eq!(image_y, source_real::Real::from_i32(8));
        assert_eq!(image_x.floor_i32(), Some(6));
        assert_eq!(image_y.floor_i32(), Some(8));

        let arrow = instance_mask(
            "oArrow",
            "sArrowRight",
            Vec2::default(),
            0.0,
            Vec2::new(1.0, 1.0),
            screen_angle,
        );
        assert!(arrow.image_pixel(6, 7));
        assert!(!arrow.image_pixel(6, 8));
    }

    #[test]
    fn deterministic_source_trig_rounds_generic_quadrant_and_nonquadrant_vectors() {
        // The retained source vector and quadrantal residual exercise the
        // pinned standalone-FSIN/FCOS profile; the other values cover generic
        // angles in every quadrant and check the QWORD coefficients passed to
        // mask sampling.
        let vectors: [(f64, f64, f64); 7] = [
            (45.0, -0.7071067811865476, 0.7071067811865476),
            (135.0, -0.7071067811865476, -0.7071067811865476),
            (225.0, 0.7071067811865476, -0.7071067811865476),
            (315.0, 0.7071067811865476, 0.7071067811865476),
            (30.0, -0.5, 0.8660254037844386),
            (90.0, -1.0, f64::from_bits(0xbbe0_0000_0000_0000)),
            (216.86989764584402, 0.6, -0.8),
        ];
        for (degrees, expected_sine, expected_cosine) in vectors {
            let (sine, cosine) = source_degree_trig(degrees).unwrap();
            assert_eq!(
                sine.to_bits(),
                expected_sine.to_bits(),
                "{degrees} degrees sine"
            );
            assert_eq!(
                cosine.to_bits(),
                expected_cosine.to_bits(),
                "{degrees} degrees cosine: got {cosine:.18e} ({:#018x})",
                cosine.to_bits()
            );
        }
    }

    #[test]
    fn source_image_zero_angle_preserves_standalone_fsin_zero_sign() {
        assert_eq!(
            source_event_trig(0.0, Some(0.0)).unwrap().0.to_bits(),
            0x8000_0000_0000_0000
        );
        assert_eq!(
            source_event_trig(0.0, Some(-0.0)).unwrap().0.to_bits(),
            0x0000_0000_0000_0000
        );
        assert_eq!(
            source_event_trig(0.0, Some(0.0)).unwrap().1.to_bits(),
            0x3ff0_0000_0000_0000
        );
    }

    #[test]
    fn entity_and_player_masks_keep_non_roundtripping_source_image_angles() {
        for source_degrees in [12.34567890123457_f64] {
            let screen_angle = -source_degrees.to_radians();
            let reconstructed_degrees = -screen_angle.to_degrees();
            assert_ne!(source_degrees.to_bits(), reconstructed_degrees.to_bits());

            let expected = source_degree_trig(source_degrees).unwrap();
            let reconstructed = source_event_trig(screen_angle, None).unwrap();
            let preserved = source_event_trig(screen_angle, Some(source_degrees)).unwrap();
            assert_eq!(preserved.0.to_bits(), expected.0.to_bits());
            assert_eq!(preserved.1.to_bits(), expected.1.to_bits());
            assert_ne!(preserved.0.to_bits(), reconstructed.0.to_bits());

            let mut entity = crate::Entity::new(1, "oArrow", Vec2::default());
            entity.data.insert("image_angle".into(), source_degrees);
            let entity_mask = entity.collision_mask();
            assert_eq!(entity_mask.source_image_angle_degrees, Some(source_degrees));

            let mut game = Game::shell(5, RulesPreset::Classic);
            game.player
                .data
                .insert("image_angle".into(), source_degrees);
            let player_mask = game.player.collision_mask();
            assert_eq!(player_mask.source_image_angle_degrees, Some(source_degrees));
        }

        let screen_rotation = -12.34567890123457_f64.to_radians();
        let mut arrow = crate::Entity::new(2, "oArrow", Vec2::default());
        arrow.data.insert("rotation".into(), screen_rotation);
        assert_eq!(
            arrow.collision_mask().source_image_angle_degrees,
            Some(-screen_rotation.to_degrees())
        );
    }

    #[test]
    fn source_trig_cache_evicts_least_recently_used_and_keeps_new_keys_warm() {
        let mut cache = SourceTrigCache::default();
        for index in 0_u64..256 {
            let coefficients = (index as f64, -(index as f64));
            cache.insert((0, index), coefficients);
            assert_eq!(cache.get((0, index)), Some(coefficients));
        }
        assert_eq!(cache.get((0, 0)), Some((0.0, -0.0)));
        let coefficients = (256.0, -256.0);
        cache.insert((0, 256), coefficients);
        assert_eq!(cache.get((0, 256)), Some(coefficients));
        assert_eq!(cache.values.len(), 256);
        assert_eq!(cache.get((0, 0)), Some((0.0, -0.0)));
        assert_eq!(cache.get((0, 1)), None);
    }

    #[test]
    fn falling_leftward_arrow_does_not_web_a_ball_on_an_x87_boundary_pixel() {
        let mut game = Game::shell(5, RulesPreset::Classic);
        game.world = World::new(20, 15, Area::Mines, 1);
        game.player.pos = Vec2::new(24.0, 184.0);
        game.player.previous_pos = game.player.pos;
        for x in 0..20 {
            game.world.set_tile(x, 12, "oBlock");
        }

        let arrow = game.world.spawn("oArrow", Vec2::new(100.0, 100.0));
        game.world.entity_mut(arrow).unwrap().vel = Vec2::new(-4.0, 2.8);
        let web_id = game.world.spawn("oWebBall", Vec2::new(90.0, 100.0));
        {
            let web = game.world.entity_mut(web_id).unwrap();
            web.vel = Vec2::new(1.0, -2.0);
            web.data.insert("life".into(), 50.0);
        }

        game.step(&InputFrame::default());

        let arrow = game.world.entity(arrow).unwrap();
        assert_eq!(arrow.pos, Vec2::new(96.0, 103.0));
        assert_eq!(arrow.vel, Vec2::new(-4.0, 3.0));
        assert_eq!(
            arrow.value("rotation"),
            -(std::f64::consts::PI + 0.75_f64.atan())
        );
        let web = game.world.entity(web_id).unwrap();
        assert_eq!(web.pos, Vec2::new(91.0, 98.0));
        assert_eq!(web.state, 0);
    }

    #[test]
    fn rotated_scaled_events_use_inclusive_edges_and_inverse_mask_scales() {
        // Native 4AAAC1..4AADBF scales [0,15] to [0,31]/[0,47], then
        // rotates the inclusive edges; 48A7DD..48A987 inverse-divides each
        // sampled axis by its signed scale. A +90-degree GML image_angle is
        // stored here as -PI/2 screen radians.
        let unit = Vec2::new(1.0, 1.0);
        let stretched = instance_mask(
            "oBlock",
            "sBlock",
            Vec2::new(80.0, 80.0),
            0.0,
            Vec2::new(2.0, 3.0),
            -std::f64::consts::FRAC_PI_2,
        );
        assert_eq!(
            stretched.event_geometry(),
            Some((Rect::new(80.0, 49.0, 48.0, 32.0), Vec2::new(80.0, 80.0)))
        );

        // The transformed bounds include x=127, but canonical automatic
        // event sampling stores FCOS(90 degrees) as a small negative QWORD.
        // Its inverse floor selects image x=-1 at this exact edge; x=128
        // remains outside the half-open bounds as well.
        let edge = instance_mask("oBlock", "sBlock", Vec2::new(127.0, 80.0), 0.0, unit, 0.0);
        let outside = instance_mask("oBlock", "sBlock", Vec2::new(128.0, 80.0), 0.0, unit, 0.0);
        assert!(!stretched.intersects_mask(&edge));
        assert!(!edge.intersects_mask(&stretched));
        assert!(!stretched.intersects_mask(&outside));
        assert!(!outside.intersects_mask(&stretched));

        // Explicit point/rectangle queries intentionally keep their existing
        // binary64 transform; they are separate from automatic event masks.
        assert!(stretched.contains(Vec2::new(127.0, 80.0), CollisionPrecision::Precise));
        assert!(!stretched.contains(Vec2::new(128.0, 80.0), CollisionPrecision::BoundingBox));
        assert!(stretched.intersects_rectangle(
            Vec2::new(127.0, 80.0),
            Vec2::new(127.0, 80.0),
            CollisionPrecision::Precise,
        ));
        assert!(!stretched.intersects_axis_line(
            Vec2::new(128.0, 80.0),
            Vec2::new(128.0, 95.0),
            CollisionPrecision::BoundingBox,
        ));

        // Nonuniform scaling and a rotation must use the same inverse mapping
        // for the precise alpha path; bbox precision remains rectangular.
        let arrow = instance_mask(
            "oArrow",
            "sArrowRight",
            Vec2::new(100.0, 100.0),
            0.0,
            Vec2::new(2.0, 3.0),
            -0.4,
        );
        assert!(arrow.supports_source_query_pixels());
        let (sin, cos) = arrow.angle.sin_cos();
        let bounds = arrow.point_bounds();
        let mut alpha_differs_from_bbox = false;
        for y in bounds.y as i32..(bounds.y + bounds.h) as i32 {
            for x in bounds.x as i32..(bounds.x + bounds.w) as i32 {
                let point = Vec2::new(f64::from(x), f64::from(y));
                let precise = arrow.contains(point, CollisionPrecision::Precise);
                if !precise {
                    assert!(arrow.contains(point, CollisionPrecision::BoundingBox));
                    alpha_differs_from_bbox = true;
                }
                let relative_x = x - arrow.pos.x.round_ties_even() as i32;
                let relative_y = y - arrow.pos.y.round_ties_even() as i32;
                let image_x = ((f64::from(relative_x) * cos + f64::from(relative_y) * sin)
                    / arrow.scale.x
                    + f64::from(arrow.origin[0]))
                .floor() as i32;
                let image_y = ((f64::from(relative_y) * cos - f64::from(relative_x) * sin)
                    / arrow.scale.y
                    + f64::from(arrow.origin[1]))
                .floor() as i32;
                assert_eq!(
                    precise,
                    arrow.image_pixel(image_x, image_y),
                    "source inverse transform at ({x},{y})"
                );
            }
        }
        assert!(
            alpha_differs_from_bbox,
            "rotated precise mask became bbox-only"
        );
    }

    #[test]
    fn rotated_source_queries_inverse_sample_alpha_but_bbox_queries_stay_rectangular() {
        // These are explicit shape-query APIs, not automatic object-event
        // mask overlaps. Their established binary64 inverse transform remains
        // separate from the executable-aligned QWORD event coefficients.
        let arrow = instance_mask(
            "oArrow",
            "sArrowRight",
            Vec2::new(100.0, 100.0),
            0.0,
            Vec2::new(1.0, 1.0),
            -std::f64::consts::FRAC_PI_2,
        );
        assert!(arrow.supports_source_query_pixels());

        // image_angle=90° rotates the arrow upward. The source inverse sampler
        // maps (99,97) to transparent image pixel (7,3), while (99,98) maps
        // to opaque image pixel (6,3).
        let transparent = Vec2::new(99.0, 97.0);
        let opaque = Vec2::new(99.0, 98.0);
        for point in [transparent, opaque] {
            assert!(arrow.contains(point, CollisionPrecision::BoundingBox));
            assert!(arrow.intersects_rectangle(point, point, CollisionPrecision::BoundingBox));
            assert!(arrow.intersects_axis_line(point, point, CollisionPrecision::BoundingBox));
        }
        assert!(!arrow.contains(transparent, CollisionPrecision::Precise));
        assert!(!arrow.intersects_rectangle(transparent, transparent, CollisionPrecision::Precise));
        assert!(!arrow.intersects_axis_line(transparent, transparent, CollisionPrecision::Precise));
        assert!(arrow.contains(opaque, CollisionPrecision::Precise));
        assert!(arrow.intersects_rectangle(opaque, opaque, CollisionPrecision::Precise));
        assert!(arrow.intersects_axis_line(opaque, opaque, CollisionPrecision::Precise));
    }

    #[test]
    fn rotated_query_support_does_not_expand_the_unrotated_mattock_adapter() {
        let unit = Vec2::new(1.0, 1.0);
        for angle in [-std::f64::consts::FRAC_PI_2, -std::f64::consts::TAU] {
            let mask = instance_mask(
                "oBlock",
                "sBlock",
                Vec2::new(100.0, 100.0),
                0.0,
                unit,
                angle,
            );
            assert!(mask.supports_source_query_pixels());
            assert!(!mask.supports_source_unrotated_query_pixels());
        }
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
        assert_eq!(
            fractional_mirror.event_geometry(),
            Some((Rect::new(75.0, 75.0, 11.0, 9.0), Vec2::new(80.0, 80.0)))
        );
        assert!(!fractional_mirror.supports_precise_pixels());
        assert!(web.intersects(corner.bounds(), CollisionPrecision::BoundingBox));
    }

    #[test]
    fn fractional_unit_mirror_native_bbox_origin_and_rectangle_samples() {
        // Native PE4AA958/4AD1A0/48A4F0 proof963bfed2: independently rounded
        // mirrored cached edges include the low-side pad; origin rounds alone.
        for (x, left, right, origin, first_opaque, last_opaque) in [
            (40.0, 33.0, 46.0, 40.0, 35, 46),
            (40.25, 33.0, 46.0, 40.0, 35, 46),
            (40.5, 34.0, 46.0, 40.0, 35, 46),
            (40.75, 34.0, 47.0, 41.0, 36, 47),
            (-40.0, -47.0, -34.0, -40.0, -45, -34),
            (-40.25, -47.0, -34.0, -40.0, -45, -34),
            (-40.5, -48.0, -34.0, -40.0, -45, -34),
            (-40.75, -48.0, -35.0, -41.0, -46, -35),
        ] {
            let mask = instance_mask(
                "oPlayer1",
                "sStandLeft",
                Vec2::new(x, 40.0),
                0.0,
                Vec2::new(-1.0, 1.0),
                0.0,
            );
            assert!(
                mask.event_geometry().is_some(),
                "SOURCE_FRACTIONAL_MIRROR_BBOX {x}"
            );
            let (bounds, rounded_origin) = mask.event_geometry().unwrap();
            assert_eq!(bounds, Rect::new(left, 32.0, right - left + 1.0, 16.0));
            assert_eq!(rounded_origin, Vec2::new(origin, 40.0));
            assert!(mask.contains(Vec2::new(left, 40.0), CollisionPrecision::BoundingBox));
            assert!(!mask.contains(
                Vec2::new(right + 1.0, 40.0),
                CollisionPrecision::BoundingBox
            ));
            for world_x in (left as i32 - 1)..=(right as i32 + 1) {
                assert_eq!(
                    mask.event_pixel(world_x - origin as i32, 0, 0.0, 1.0),
                    (first_opaque..=last_opaque).contains(&world_x),
                    "{x}/{world_x}"
                );
            }
        }
        let both = instance_mask(
            "oPlayer1",
            "sStandLeft",
            Vec2::new(40.5, -40.5),
            0.0,
            Vec2::new(-1.0, -1.0),
            0.0,
        );
        assert_eq!(
            both.event_geometry(),
            Some((Rect::new(34.0, -50.0, 13.0, 19.0), Vec2::new(40.0, -40.0)))
        );
        assert!(!both.event_pixel(-6, -10, 0.0, 1.0));
        assert!(both.event_pixel(-5, -7, 0.0, 1.0));
    }

    #[test]
    fn controlled_fractional_mirrored_damsel_rejects_empty_web_corner() {
        // Controlled exact values, not an ordinary-input timing witness.
        // Source sDamselDieLR RECTANGLE frame0 occupies x99..111/y88..103.
        // Web116,85 shares only(111,88): its source image(3,11) is transparent.
        // Web114,85 reaches image(5,11), an opaque positive control.
        for y in [96.05, 96.25, 96.5] {
            let damsel = instance_mask(
                "oDamsel",
                "sDamselDieLR",
                Vec2::new(104.0, y),
                0.0,
                Vec2::new(-1.0, 1.0),
                0.0,
            );
            let empty = instance_mask(
                "oWebBall",
                "sWebBall",
                Vec2::new(116.0, 85.0),
                0.0,
                Vec2::new(1.0, 1.0),
                0.0,
            );
            let opaque = instance_mask(
                "oWebBall",
                "sWebBall",
                Vec2::new(114.0, 85.0),
                0.0,
                Vec2::new(1.0, 1.0),
                0.0,
            );
            assert!(damsel.bounds().intersects(empty.bounds()));
            assert!(damsel.intersects_mask(&opaque));
            assert!(!damsel.intersects_mask(&empty));
            assert!(!empty.intersects_mask(&damsel));
            // Explicit precise shape-query sampler remains its old adapter.
            assert!(damsel.intersects(empty.bounds(), CollisionPrecision::Precise));
        }
    }

    #[test]
    fn rotated_and_scaled_finite_nonzero_transforms_have_source_event_geometry() {
        for (scale, angle) in [
            (Vec2::new(-1.0, 1.0), 0.2),
            (Vec2::new(1.0, -1.0), 0.2),
            (Vec2::new(2.0, 1.0), 0.2),
            (Vec2::new(1.0, 2.0), 0.2),
            (Vec2::new(-2.0, 1.0), 0.2),
            (Vec2::new(-2.0, 1.0), 0.0),
            (Vec2::new(1.0, 2.0), 0.0),
        ] {
            let mask = instance_mask(
                "oDamsel",
                "sDamselDieLR",
                Vec2::new(104.25, 96.25),
                0.0,
                scale,
                angle,
            );
            assert!(mask.event_geometry().is_some());
        }
        let zero_scale = instance_mask(
            "oDamsel",
            "sDamselDieLR",
            Vec2::new(104.25, 96.25),
            0.0,
            Vec2::new(0.0, 1.0),
            0.2,
        );
        assert!(zero_scale.event_geometry().is_none());
    }

    #[test]
    fn source_precise_point_unit_fractional_and_mirror_controls() {
        // Native point raw gate then independent RTE; no second bbox clamp.
        // Controlled API inputs, not an ordinary source collision_point caller.
        for (object, sprite, frame, x, y, sx, sy, qx, qy, bbox, precise) in [
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.0, 1.0, 1.0, 24.0, 88.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.0, 1.0, 1.0, 31.75, 88.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.0, 1.0, 1.0, 15.75, 88.0, false, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.25, 80.0, 1.0, 1.0, 16.1, 88.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.5, 80.0, 1.0, 1.0, 31.75, 88.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 17.5, 80.0, 1.0, 1.0, 32.75, 88.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.25, 80.0, -1.0, 1.0, -0.75, 88.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.25, 80.0, -1.0, 1.0, 0.75, 88.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.0, -1.0, 1.0, 1.0, 88.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.0, -1.0, 1.0, 17.0, 88.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.5, 1.0, -1.0, 24.0, 64.5, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.5, 1.0, -1.0, 24.0, 64.75, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, -16.25, -80.5, -1.0, -1.0, -32.0, -96.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, -16.25, -80.5, -1.0, -1.0, -31.0, -95.0, true, true,
            ),
            (
                "oPlayer1",
                "sStandLeft",
                0.0,
                41.5,
                40.0,
                1.0,
                1.0,
                46.75,
                40.0,
                true,
                true,
            ),
            (
                "oPlayer1",
                "sStandLeft",
                0.0,
                40.5,
                40.0,
                -1.0,
                1.0,
                34.49,
                40.0,
                true,
                false,
            ),
            (
                "oPlayer1",
                "sStandLeft",
                0.0,
                40.5,
                40.0,
                -1.0,
                1.0,
                34.51,
                40.0,
                true,
                true,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.0, 80.0, 1.0, 1.0, 76.75, 75.0, true, true,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, 1.0, 1.0, 75.0, 75.0, true, false,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, 1.0, 1.0, 77.0, 75.0, true, true,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, -1.0, 1.0, 85.0, 75.0, true, false,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, -1.0, 1.0, 83.0, 75.0, true, true,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.75, -1.0, -1.0, 85.0, 86.0, true, false,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.75, -1.0, -1.0, 83.0, 86.0, true, true,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                0.0,
                104.0,
                104.0,
                1.0,
                1.0,
                97.0,
                104.0,
                true,
                false,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                4.0,
                104.0,
                104.0,
                1.0,
                1.0,
                97.0,
                104.0,
                true,
                true,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                0.0,
                104.25,
                104.5,
                -1.0,
                1.0,
                111.0,
                104.0,
                true,
                false,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                4.0,
                104.25,
                104.5,
                -1.0,
                1.0,
                111.0,
                104.0,
                true,
                true,
            ),
        ] {
            let mask = instance_mask(
                object,
                sprite,
                Vec2::new(x, y),
                frame,
                Vec2::new(sx, sy),
                0.0,
            );
            assert_eq!(
                mask.contains(Vec2::new(qx, qy), CollisionPrecision::BoundingBox),
                bbox
            );
            assert_eq!(
                mask.contains(Vec2::new(qx, qy), CollisionPrecision::Precise),
                precise,
                "SOURCE_PRECISE_POINT {sprite}/{frame}/{x}/{y}/{sx}/{sy}/{qx}/{qy}"
            );
        }
    }

    #[test]
    fn source_precise_axis_unit_mirror_and_selected_frame_controls() {
        // Native raw gate, endpoint RTE, aggregate pixel-box clip, boolean byte.
        for (object, sprite, frame, x, y, sx, sy, ax, ay, bx, by, bbox, precise) in [
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.0, 1.0, 1.0, 16.0, 88.0, 31.0, 88.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.5, 1.0, 1.0, 16.0, 96.0, 31.0, 96.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 17.5, 80.0, 1.0, 1.0, 32.75, 88.0, 32.75, 88.0, true,
                false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.0, -1.0, 1.0, 1.0, 80.0, 1.0, 95.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.0, -1.0, 1.0, 0.0, 80.0, 0.0, 95.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.0, -1.0, 1.0, 17.0, 80.0, 17.0, 95.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.25, 80.0, -1.0, 1.0, 0.0, 88.0, 0.0, 88.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.25, 80.0, -1.0, 1.0, 0.75, 88.0, 0.75, 88.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.75, 1.0, -1.0, 16.0, 65.0, 31.0, 65.0, true,
                false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.75, 1.0, -1.0, 16.0, 81.0, 31.0, 81.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.75, -1.0, -1.0, 17.0, 65.0, 17.0, 81.0, true,
                true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.75, -1.0, -1.0, 1.0, 65.0, 1.0, 81.0, true,
                false,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, 1.0, 1.0, 75.0, 75.0, 77.0, 75.0, true,
                true,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, 1.0, 1.0, 75.0, 75.0, 76.0, 75.0, true,
                false,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, -1.0, 1.0, 83.0, 75.0, 85.0, 75.0, true,
                true,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, -1.0, 1.0, 84.0, 75.0, 85.0, 75.0, true,
                false,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.75, -1.0, -1.0, 83.0, 86.0, 85.0, 86.0,
                true, true,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.75, -1.0, -1.0, 84.0, 86.0, 85.0, 86.0,
                true, false,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                0.0,
                104.0,
                104.0,
                1.0,
                1.0,
                97.0,
                96.0,
                97.0,
                111.0,
                true,
                false,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                4.0,
                104.0,
                104.0,
                1.0,
                1.0,
                97.0,
                96.0,
                97.0,
                111.0,
                true,
                true,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                0.0,
                104.25,
                104.5,
                -1.0,
                1.0,
                111.0,
                96.0,
                111.0,
                111.0,
                true,
                false,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                4.0,
                104.25,
                104.5,
                -1.0,
                1.0,
                111.0,
                96.0,
                111.0,
                111.0,
                true,
                true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.0, -1.0, 1.0, -0.25, 88.0, -0.25, 88.0, false,
                false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.0, 1.0, 1.0, 15.75, 80.0, 15.75, 95.0, false,
                false,
            ),
            (
                "oBlock", "sBlock", 0.0, -16.25, -80.5, -1.0, -1.0, -31.0, -95.0, -16.0, -95.0,
                true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, -16.25, -80.5, -1.0, -1.0, -31.0, -96.0, -16.0, -96.0,
                true, false,
            ),
        ] {
            let mask = instance_mask(
                object,
                sprite,
                Vec2::new(x, y),
                frame,
                Vec2::new(sx, sy),
                0.0,
            );
            let first = Vec2::new(ax, ay);
            let second = Vec2::new(bx, by);
            assert_eq!(
                mask.intersects_axis_line(first, second, CollisionPrecision::BoundingBox),
                bbox
            );
            assert_eq!(
                mask.intersects_axis_line(first, second, CollisionPrecision::Precise),
                precise,
                "SOURCE_PRECISE_AXIS {sprite}/{frame}/{x}/{y}/{sx}/{sy}/{ax}/{ay}/{bx}/{by}"
            );
            assert_eq!(
                mask.intersects_axis_line(second, first, CollisionPrecision::Precise),
                precise
            );
        }
    }

    #[test]
    fn source_raw_precise_rectangle_after_only_controls() {
        // New raw-corner API: after-only oracle, not a missing-API old failure.
        for (object, sprite, frame, x, y, sx, sy, ax, ay, bx, by, bbox, precise) in [
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.0, 1.0, 1.0, 16.0, 88.0, 31.0, 88.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.5, 1.0, 1.0, 16.0, 96.0, 31.0, 96.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 17.5, 80.0, 1.0, 1.0, 32.75, 88.0, 32.75, 88.0, true,
                false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.0, -1.0, 1.0, 0.0, 80.0, 0.0, 95.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.0, -1.0, 1.0, 1.0, 80.0, 1.0, 95.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.0, -1.0, 1.0, 17.0, 80.0, 17.0, 95.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.25, 80.0, -1.0, 1.0, 0.0, 88.0, 0.0, 88.0, true, false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.25, 80.0, -1.0, 1.0, 0.75, 88.0, 0.75, 88.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.75, 1.0, -1.0, 16.0, 65.0, 31.0, 65.0, true,
                false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.75, 1.0, -1.0, 16.0, 81.0, 31.0, 81.0, true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.75, -1.0, -1.0, 17.0, 65.0, 17.0, 81.0, true,
                true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.75, -1.0, -1.0, 1.0, 65.0, 1.0, 81.0, true,
                false,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, 1.0, 1.0, 75.0, 75.0, 77.0, 75.0, true,
                true,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, 1.0, 1.0, 75.0, 75.0, 76.0, 75.0, true,
                false,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, -1.0, 1.0, 83.0, 75.0, 85.0, 75.0, true,
                true,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, -1.0, 1.0, 84.0, 75.0, 85.0, 75.0, true,
                false,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.75, -1.0, -1.0, 83.0, 86.0, 85.0, 86.0,
                true, true,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.75, -1.0, -1.0, 84.0, 86.0, 85.0, 86.0,
                true, false,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                0.0,
                104.0,
                104.0,
                1.0,
                1.0,
                97.0,
                96.0,
                97.0,
                111.0,
                true,
                false,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                4.0,
                104.0,
                104.0,
                1.0,
                1.0,
                97.0,
                96.0,
                97.0,
                111.0,
                true,
                true,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                0.0,
                104.25,
                104.5,
                -1.0,
                1.0,
                111.0,
                96.0,
                111.0,
                111.0,
                true,
                false,
            ),
            (
                "oDamsel",
                "sDamselYellL",
                4.0,
                104.25,
                104.5,
                -1.0,
                1.0,
                111.0,
                96.0,
                111.0,
                111.0,
                true,
                true,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.75, 80.0, -1.0, 1.0, -0.25, 88.0, -0.25, 88.0, false,
                false,
            ),
            (
                "oBlock", "sBlock", 0.0, 16.0, 80.0, 1.0, 1.0, 15.75, 80.0, 15.75, 95.0, false,
                false,
            ),
            (
                "oBlock", "sBlock", 0.0, -16.25, -80.5, -1.0, -1.0, -31.0, -95.0, -16.0, -95.0,
                true, true,
            ),
            (
                "oBlock", "sBlock", 0.0, -16.25, -80.5, -1.0, -1.0, -31.0, -96.0, -16.0, -96.0,
                true, false,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, 1.0, 1.0, 75.0, 75.0, 76.0, 76.0, true,
                true,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, 1.0, 1.0, 75.0, 75.0, 75.0, 76.0, true,
                false,
            ),
            (
                "oWebBall", "sWebBall", 0.0, 80.25, 80.25, -1.0, 1.0, 85.0, 75.0, 84.0, 76.0, true,
                true,
            ),
        ] {
            let mask = instance_mask(
                object,
                sprite,
                Vec2::new(x, y),
                frame,
                Vec2::new(sx, sy),
                0.0,
            );
            let first = Vec2::new(ax, ay);
            let second = Vec2::new(bx, by);
            assert_eq!(
                mask.intersects_rectangle(first, second, CollisionPrecision::BoundingBox),
                bbox
            );
            assert_eq!(
                mask.intersects_rectangle(first, second, CollisionPrecision::Precise),
                precise,
                "SOURCE_RAW_RECTANGLE {sprite}/{frame}/{x}/{y}/{sx}/{sy}/{ax}/{ay}/{bx}/{by}"
            );
            assert_eq!(
                mask.intersects_rectangle(second, first, CollisionPrecision::Precise),
                precise
            );
        }
        let mask = instance_mask(
            "oPlayer1",
            "sStandLeft",
            Vec2::new(41.5, 40.0),
            0.0,
            Vec2::new(1.0, 1.0),
            0.0,
        );
        let q = Vec2::new(46.75, 40.0);
        assert!(mask.contains(q, CollisionPrecision::Precise));
        assert!(!mask.intersects_rectangle(q, q, CollisionPrecision::Precise));
        assert!(!mask.intersects_axis_line(q, q, CollisionPrecision::Precise));
        // This is a native route distinction, not a rounding approximation:
        // point samples pixel47 beyond cached R46; rectangle/line clips it out.
    }

    #[test]
    fn source_query_extension_preserves_legacy_and_supports_rotated_scales() {
        let mask = instance_mask(
            "oBlock",
            "sBlock",
            Vec2::new(16.0, 80.0),
            0.0,
            Vec2::new(1.0, 1.0),
            0.0,
        );
        let q = Vec2::new(31.75, 88.0);
        assert!(mask.intersects(Rect::new(q.x, q.y, 0.1, 0.1), CollisionPrecision::Precise));
        assert!(!mask.intersects_rectangle(q, q, CollisionPrecision::Precise));
        for (scale, angle) in [
            (Vec2::new(-1.0, 1.0), 0.2),
            (Vec2::new(1.0, -1.0), 0.2),
            (Vec2::new(2.0, 1.0), 0.0),
            (Vec2::new(1.0, 2.0), 0.0),
            (Vec2::new(-2.0, 1.0), 0.0),
            (Vec2::new(1.0, 1.0), 0.2),
        ] {
            let mask = instance_mask(
                "oWebBall",
                "sWebBall",
                Vec2::new(80.25, 80.25),
                0.0,
                scale,
                angle,
            );
            let supports_source_pixels = scale.x != 0.0 && scale.y != 0.0;
            assert_eq!(mask.supports_source_query_pixels(), supports_source_pixels);
            // Every finite nonzero rotation/scale pair uses the native
            // inverse sampler. The legacy smooth-Rect adapter stays narrower.
            let first = Vec2::new(75.0, 75.0);
            let second = Vec2::new(77.0, 77.0);
            let p = Vec2::new(80.0, 80.0);
            if angle == 0.0 {
                assert_eq!(
                    mask.intersects_rectangle(first, second, CollisionPrecision::Precise),
                    mask.intersects(Rect::new(75.0, 75.0, 3.0, 3.0), CollisionPrecision::Precise)
                );
                assert_eq!(
                    mask.contains(p, CollisionPrecision::Precise),
                    mask.contains(p, CollisionPrecision::BoundingBox)
                );
            }
        }
        let fractional = instance_mask(
            "oWebBall",
            "sWebBall",
            Vec2::new(80.25, 80.25),
            0.0,
            Vec2::new(-1.0, 1.0),
            0.0,
        );
        assert!(!fractional.supports_precise_pixels()); // Legacy smooth-Rect capability unchanged.
        assert!(fractional.supports_source_query_pixels());
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

    #[test]
    fn nonunit_source_point_queries_use_native_cache_and_selected_arrow_pixels() {
        // Controlled source API vectors: exact native25 probe rows0..11,
        // not ordinary campaign transform producers. First opaque control
        // precedes the scale2 transparent pixel; no guessed trig values.
        for (sx, sy, x, y, edges, precise, bbox) in [
            (2.0, 1.0, 72.0, 79.0, [72.0, 79.0, 87.0, 83.0], true, true),
            (2.0, 1.0, 76.0, 79.0, [72.0, 79.0, 87.0, 83.0], false, true),
            (0.5, 1.0, 78.0, 79.0, [78.0, 79.0, 81.0, 83.0], true, true),
            (0.5, 1.0, 79.0, 79.0, [78.0, 79.0, 81.0, 83.0], false, true),
            (-2.0, 1.0, 88.0, 79.0, [71.0, 79.0, 88.0, 83.0], true, true),
            (-2.0, 1.0, 84.0, 79.0, [71.0, 79.0, 88.0, 83.0], false, true),
            (1.0, 2.0, 76.0, 78.0, [76.0, 78.0, 83.0, 87.0], true, true),
            (1.0, 2.0, 78.0, 78.0, [76.0, 78.0, 83.0, 87.0], false, true),
            (1.0, -0.5, 76.0, 80.5, [76.0, 77.0, 83.0, 80.0], true, true),
            (1.0, -0.5, 78.0, 80.5, [76.0, 77.0, 83.0, 80.0], true, true),
            (2.0, 0.5, 72.0, 79.5, [72.0, 80.0, 87.0, 81.0], false, false),
            (2.0, 0.5, 76.0, 79.5, [72.0, 80.0, 87.0, 81.0], false, false),
        ] {
            let mask = instance_mask(
                "oArrow",
                "sArrowRight",
                Vec2::new(80.0, 80.0),
                0.0,
                Vec2::new(sx, sy),
                0.0,
            );
            assert_eq!(
                mask.contains(Vec2::new(x, y), CollisionPrecision::BoundingBox),
                bbox,
                "SOURCE_NATIVE_NONUNIT_RAW_CACHE {sx}/{sy}/{x}/{y}"
            );
            assert_eq!(
                mask.contains(Vec2::new(x, y), CollisionPrecision::Precise),
                precise,
                "SOURCE_NATIVE_NONUNIT_POINT_PIXEL {sx}/{sy}/{x}/{y}"
            );
            assert_eq!(
                mask.point_bounds(),
                Rect::new(
                    edges[0],
                    edges[1],
                    edges[2] - edges[0] + 1.0,
                    edges[3] - edges[1] + 1.0
                ),
                "SOURCE_NATIVE_NONUNIT_CACHE_EDGES {sx}/{sy}"
            );
        }
    }

    #[test]
    fn nonunit_source_rectangle_and_axis_queries_use_native_arrow_pixels() {
        // Original raw inclusive equal-corner rectangle and equal-axis paths
        // have actual native results matching these twelve source vectors.
        for (sx, sy, x, y, _edges, precise, bbox) in [
            (2.0, 1.0, 72.0, 79.0, [72.0, 79.0, 87.0, 83.0], true, true),
            (2.0, 1.0, 76.0, 79.0, [72.0, 79.0, 87.0, 83.0], false, true),
            (0.5, 1.0, 78.0, 79.0, [78.0, 79.0, 81.0, 83.0], true, true),
            (0.5, 1.0, 79.0, 79.0, [78.0, 79.0, 81.0, 83.0], false, true),
            (-2.0, 1.0, 88.0, 79.0, [71.0, 79.0, 88.0, 83.0], true, true),
            (-2.0, 1.0, 84.0, 79.0, [71.0, 79.0, 88.0, 83.0], false, true),
            (1.0, 2.0, 76.0, 78.0, [76.0, 78.0, 83.0, 87.0], true, true),
            (1.0, 2.0, 78.0, 78.0, [76.0, 78.0, 83.0, 87.0], false, true),
            (1.0, -0.5, 76.0, 80.5, [76.0, 77.0, 83.0, 80.0], true, true),
            (1.0, -0.5, 78.0, 80.5, [76.0, 77.0, 83.0, 80.0], true, true),
            (2.0, 0.5, 72.0, 79.5, [72.0, 80.0, 87.0, 81.0], false, false),
            (2.0, 0.5, 76.0, 79.5, [72.0, 80.0, 87.0, 81.0], false, false),
        ] {
            let mask = instance_mask(
                "oArrow",
                "sArrowRight",
                Vec2::new(80.0, 80.0),
                0.0,
                Vec2::new(sx, sy),
                0.0,
            );
            let point = Vec2::new(x, y);
            assert_eq!(
                mask.intersects_rectangle(point, point, CollisionPrecision::BoundingBox),
                bbox
            );
            assert_eq!(
                mask.intersects_rectangle(point, point, CollisionPrecision::Precise),
                precise,
                "SOURCE_NATIVE_NONUNIT_RECTANGLE_PIXEL {sx}/{sy}/{x}/{y}"
            );
            assert_eq!(
                mask.intersects_axis_line(point, point, CollisionPrecision::BoundingBox),
                bbox
            );
            assert_eq!(
                mask.intersects_axis_line(point, point, CollisionPrecision::Precise),
                precise,
                "SOURCE_NATIVE_NONUNIT_AXIS_PIXEL {sx}/{sy}/{x}/{y}"
            );
        }
    }

    #[test]
    fn nonunit_rectangle_masks_sample_pixels_after_independent_half_rounding() {
        // Original sBlock RECTANGLE origin0/aggregate[0,0,15,15]. At x16.5,
        // scale2 cache is[16,48], originRTE16; x47 samples15, x48 samples16.
        // This is controlled nonunit geometry, not a mirrored Solid producer.
        let right = instance_mask(
            "oBlock",
            "sBlock",
            Vec2::new(16.5, 80.0),
            0.0,
            Vec2::new(2.0, 1.0),
            0.0,
        );
        for (x, expected) in [(47.0, true), (48.0, false), (47.5, false), (47.49, true)] {
            let q = Vec2::new(x, 80.0);
            assert!(right.contains(q, CollisionPrecision::BoundingBox));
            assert_eq!(
                right.contains(q, CollisionPrecision::Precise),
                expected,
                "SOURCE_NATIVE_NONUNIT_RECTANGLE_MASK_POINT {x}"
            );
            assert_eq!(
                right.intersects_rectangle(q, q, CollisionPrecision::Precise),
                expected
            );
            assert_eq!(
                right.intersects_axis_line(q, q, CollisionPrecision::Precise),
                expected
            );
        }
        // Negative scale reverses AFTER rounding cache endpoints; origin16
        // and query16.5 both round16, selecting opaque imagecolumn0.
        let left = instance_mask(
            "oBlock",
            "sBlock",
            Vec2::new(16.5, 80.0),
            0.0,
            Vec2::new(-2.0, 1.0),
            0.0,
        );
        for (x, expected) in [(16.5, true), (-16.0, false)] {
            let q = Vec2::new(x, 80.0);
            assert!(left.contains(q, CollisionPrecision::BoundingBox));
            assert_eq!(
                left.contains(q, CollisionPrecision::Precise),
                expected,
                "SOURCE_NATIVE_NEGATIVE_NONUNIT_RECTANGLE_MASK_POINT {x}"
            );
            assert_eq!(
                left.intersects_rectangle(q, q, CollisionPrecision::Precise),
                expected
            );
            assert_eq!(
                left.intersects_axis_line(q, q, CollisionPrecision::Precise),
                expected
            );
        }
    }

    #[test]
    fn native_zero_scale_cache_keeps_bbox_but_rejects_invalid_precise_pixels() {
        // Classic 1.1 PE 4AA958 rounds and sorts both cached edges even when
        // image scale is signed zero. 48A9F4 then divides the rounded pixel
        // offset by zero; 4238E8/4034A4 produce -1 from the masked invalid
        // conversion, which the mask kernel rejects as an image coordinate.
        for (pos, scale, bounds, point) in [
            (
                Vec2::new(80.0, 80.0),
                Vec2::new(0.0, 0.0),
                Rect::new(79.0, 79.0, 2.0, 2.0),
                Vec2::new(79.0, 79.0),
            ),
            (
                Vec2::new(80.0, 80.0),
                Vec2::new(-0.0, -0.0),
                Rect::new(79.0, 79.0, 2.0, 2.0),
                Vec2::new(80.0, 80.0),
            ),
            (
                Vec2::new(80.5, 80.5),
                Vec2::new(-0.0, 0.0),
                Rect::new(80.0, 80.0, 1.0, 1.0),
                Vec2::new(80.0, 80.0),
            ),
        ] {
            let mask = instance_mask("oWebBall", "sWebBall", pos, 0.0, scale, 0.0);
            assert_eq!(mask.point_bounds(), bounds, "pos={pos:?}, scale={scale:?}");
            assert!(mask.contains(point, CollisionPrecision::BoundingBox));
            assert!(mask.intersects_rectangle_bbox(point, point));
            assert!(mask.intersects_rectangle(point, point, CollisionPrecision::BoundingBox));
            assert!(mask.intersects_axis_line(point, point, CollisionPrecision::BoundingBox));
            assert!(!mask.contains(point, CollisionPrecision::Precise));
            assert!(!mask.intersects_rectangle(point, point, CollisionPrecision::Precise));
            assert!(!mask.intersects_axis_line(point, point, CollisionPrecision::Precise));
            assert!(!mask.contains(
                Vec2::new(bounds.x + bounds.w, point.y),
                CollisionPrecision::Precise
            ));
        }
    }

    #[test]
    fn source_query_word_boundaries_preserve_native_conversion_and_raw_gate_order() {
        // Controlled API positions/scales, not campaign reachability claims.
        // Native FISTP64 returns low32 BEFORE independent signed32 subtraction.
        let block = instance_mask(
            "oBlock",
            "sBlock",
            Vec2::new(2147483648.0, 80.0),
            0.0,
            Vec2::new(2.0, 1.0),
            0.0,
        );
        let q = Vec2::new(-2147483648.0, 80.0);
        assert_eq!(
            block.contains(q, CollisionPrecision::BoundingBox),
            true,
            "SOURCE_FISTP64_LOW_WORD_CACHE_GATE"
        );
        assert!(
            block.contains(q, CollisionPrecision::Precise),
            "SOURCE_FISTP64_LOW_WORD_ORIGIN"
        );
        assert!(block.intersects_rectangle(q, q, CollisionPrecision::Precise));
        assert!(block.intersects_axis_line(q, q, CollisionPrecision::Precise));
        let arrow = instance_mask(
            "oArrow",
            "sArrowRight",
            Vec2::new(4294967376.0, 80.0),
            0.0,
            Vec2::new(2.0, 1.0),
            0.0,
        );
        assert!(arrow.contains(Vec2::new(80.0, 80.0), CollisionPrecision::Precise));
        let arrow = instance_mask(
            "oArrow",
            "sArrowRight",
            Vec2::new(80.0, 80.0),
            0.0,
            Vec2::new(536870912.0, 1.0),
            0.0,
        );
        let q = Vec2::new(-2147483569.0, 79.0);
        assert!(arrow.contains(q, CollisionPrecision::BoundingBox));
        assert!(
            !arrow.contains(q, CollisionPrecision::Precise),
            "SOURCE_WRAPPING_OFFSET_SELECTS_EMPTY_PIXEL_7_3"
        );
        assert!(!arrow.intersects_rectangle(q, q, CollisionPrecision::Precise));
        assert!(!arrow.intersects_axis_line(q, q, CollisionPrecision::Precise));
        // R==MAX makes signed32 R+1==MIN before native raw-double comparisons.
        let arrow = instance_mask(
            "oArrow",
            "sArrowRight",
            Vec2::new(0.0, 80.0),
            0.0,
            Vec2::new(536870912.0, 1.0),
            0.0,
        );
        let q = Vec2::new(0.0, 80.0);
        for precision in [CollisionPrecision::BoundingBox, CollisionPrecision::Precise] {
            assert!(
                !arrow.contains(q, precision),
                "SOURCE_SIGNED_CACHE_SUCCESSOR_RAW_GATE"
            );
            assert!(!arrow.intersects_rectangle(q, q, precision));
            assert!(!arrow.intersects_axis_line(q, q, precision));
        }
        // SUB/JL compares signed original endpoints (SF != OF), rather than
        // the sign bit of the wrapped difference. The native full-width count
        // is zero after INC, representing 2^32 DEC/JNZ iterations. Its FIRST
        // selected pixel is opaque, so this control does not run a giant loop.
        assert!(
            arrow.intersects_rectangle(
                Vec2::new(-6442450944.0, 80.0),
                Vec2::new(2147483647.0, 80.0),
                CollisionPrecision::Precise
            ),
            "SOURCE_SIGNED_JL_FULL_WIDTH_FIRST_PIXEL"
        );
        // These raw ordered inputs convert to signed MAX/MIN. The constructor
        // does not resort them; SUB has overflow, and JL rejects R < L.
        assert!(
            !arrow.intersects_rectangle(
                Vec2::new(-2147483649.0, 80.0),
                Vec2::new(2147483648.0, 80.0),
                CollisionPrecision::Precise
            ),
            "SOURCE_SIGNED_JL_REVERSED_LOW_WORDS"
        );
        // Source PC64 preserves the input epsilon through the bbox expression.
        let unit = instance_mask(
            "oBlock",
            "sBlock",
            Vec2::new(255.50000000000003, 80.0),
            0.0,
            Vec2::new(1.0, 1.0),
            0.0,
        );
        assert_eq!(unit.point_bounds(), Rect::new(256.0, 80.0, 16.0, 16.0));
        let q = Vec2::new(270.75, 80.0);
        assert!(unit.intersects_rectangle(q, q, CollisionPrecision::Precise));
        assert!(unit.intersects_axis_line(q, q, CollisionPrecision::Precise));
    }
    #[test]
    fn unrotated_event_nonunit_cache_and_mask_controls_from_original_bytes() {
        // Controlled native primitive inputs, not ordinary stock transform producers.
        // Positive source case15 first; empty .3-scale case20 exposes old bbox fallback.
        for (case, x, y, sx, sy, left, top, right, bottom, hit) in [
            (15, 80.0, 80.0, 3.0, 1.0, 68.0, 79.0, 91.0, 83.0, true),
            (20, 80.0, 80.0, 0.3, 1.0, 79.0, 79.0, 80.0, 83.0, false),
            (21, 80.0, 80.0, 1.0, 0.3, 76.0, 80.0, 83.0, 80.0, false),
            (16, 80.0, 80.0, 1.0, 3.0, 76.0, 77.0, 83.0, 91.0, true),
            (17, 80.0, 80.0, -3.0, 1.0, 67.0, 79.0, 92.0, 83.0, true),
            (18, 80.0, 80.0, 1.0, -3.0, 76.0, 67.0, 83.0, 83.0, true),
            (19, 80.0, 80.0, 3.0, 3.0, 68.0, 77.0, 91.0, 91.0, true),
            (22, 80.5, 80.5, 1.5, -2.5, 74.0, 70.0, 86.0, 83.0, true),
            (23, 80.25, 80.75, -0.3, 1.5, 78.0, 79.0, 81.0, 86.0, false),
            (24, 80.5, 80.5, 3.0, 3.0, 68.0, 78.0, 92.0, 92.0, true),
        ] {
            let arrow = instance_mask(
                "oArrow",
                "sArrowRight",
                Vec2::new(x, y),
                0.0,
                Vec2::new(sx, sy),
                0.0,
            );
            let web = instance_mask(
                "oWeb",
                "sWeb",
                Vec2::new(68.0, 64.0),
                0.0,
                Vec2::new(1.0, 1.0),
                0.0,
            );
            let expected_bounds = Rect::new(left, top, right - left + 1.0, bottom - top + 1.0);
            assert_eq!(
                arrow.point_bounds(),
                expected_bounds,
                "already proved query cache, case {case}"
            );
            assert_eq!(
                arrow.intersects_mask(&web),
                hit,
                "NATIVE_UNROTATED_EVENT_PAIR case {case}"
            );
            assert_eq!(
                web.intersects_mask(&arrow),
                hit,
                "NATIVE_UNROTATED_REVERSE_EVENT_PAIR case {case}"
            );
            assert_eq!(arrow.event_geometry().unwrap().0, expected_bounds);
        }
    }

    #[test]
    fn unrotated_event_native_lowword_offsets_and_cached_successor_controls() {
        // Source-law API controls with valid signed64 conversions, not campaign positions.
        let unit = Vec2::new(1.0, 1.0);
        let large = instance_mask(
            "oArrow",
            "sArrowRight",
            Vec2::new(4294967376.0, 80.0),
            0.0,
            Vec2::new(2.0, 1.0),
            0.0,
        );
        let web = instance_mask("oWeb", "sWeb", Vec2::new(68.0, 64.0), 0.0, unit, 0.0);
        assert_eq!(large.point_bounds(), Rect::new(72.0, 79.0, 16.0, 5.0));
        assert!(
            large.intersects_mask(&web),
            "NATIVE_EVENT_INDEPENDENT_LOWWORD_ORIGIN"
        );
        assert!(web.intersects_mask(&large));
        let edge = instance_mask(
            "oArrow",
            "sArrowRight",
            Vec2::new(0.0, 80.0),
            0.0,
            Vec2::new(536870912.0, 1.0),
            0.0,
        );
        let web = instance_mask("oWeb", "sWeb", Vec2::new(0.0, 76.0), 0.0, unit, 0.0);
        assert_eq!(
            edge.point_bounds(),
            Rect::new(-2147483648.0, 79.0, 4294967296.0, 5.0)
        );
        assert!(
            !edge.intersects_mask(&web),
            "NATIVE_EVENT_SIGNED_CACHED_RIGHT_SUCCESSOR"
        );
        assert!(!web.intersects_mask(&edge));
    }
}
