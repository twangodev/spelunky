//! Original sprite coordinates and origins, exported without resampling.
use std::collections::BTreeMap;

use serde::Deserialize;

#[cfg(test)]
use crate::renderer::DrawQuad;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasFrame {
    pub uv: [f32; 4],
    pub size: [f32; 2],
    pub origin: [f32; 2],
    /// Inclusive original collision bounds, before subtracting the origin.
    pub bounds: [i32; 4],
}

#[derive(Deserialize)]
struct Frame {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    bounds: Option<[i32; 4]>,
}

#[derive(Deserialize)]
struct Resource {
    origin: [i32; 2],
    bounds: Option<[i32; 4]>,
    frames: Vec<Frame>,
}

#[derive(Deserialize)]
struct Manifest {
    version: u32,
    width: u32,
    height: u32,
    sprites: BTreeMap<String, Resource>,
    backgrounds: BTreeMap<String, Resource>,
}

pub struct Atlas {
    pub width: u32,
    pub height: u32,
    pixels: Vec<u8>,
    resources: BTreeMap<String, Vec<AtlasFrame>>,
}

impl Atlas {
    pub fn load() -> Result<Self, String> {
        Self::decode(
            include_bytes!("../../../assets/atlas.json"),
            include_bytes!("../../../assets/atlas.png"),
        )
    }

    fn decode(json: &[u8], png: &[u8]) -> Result<Self, String> {
        let manifest: Manifest = serde_json::from_slice(json).map_err(|error| error.to_string())?;
        if manifest.version != 1 || manifest.width == 0 || manifest.height == 0 {
            return Err("Unsupported or empty sprite atlas".into());
        }
        let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)
            .map_err(|error| error.to_string())?
            .into_rgba8();
        if image.dimensions() != (manifest.width, manifest.height) {
            return Err("Sprite atlas image and manifest dimensions differ".into());
        }
        let mut resources = BTreeMap::new();
        for (name, resource) in manifest.sprites.into_iter().chain(manifest.backgrounds) {
            if resource.frames.is_empty() {
                return Err(format!("Sprite {name} has no frames"));
            }
            let mut frames = Vec::with_capacity(resource.frames.len());
            for frame in resource.frames {
                if frame.w == 0
                    || frame.h == 0
                    || frame
                        .x
                        .checked_add(frame.w)
                        .is_none_or(|x| x > manifest.width)
                    || frame
                        .y
                        .checked_add(frame.h)
                        .is_none_or(|y| y > manifest.height)
                {
                    return Err(format!("Sprite {name} extends outside its atlas"));
                }
                frames.push(AtlasFrame {
                    uv: [
                        frame.x as f32 / manifest.width as f32,
                        frame.y as f32 / manifest.height as f32,
                        frame.w as f32 / manifest.width as f32,
                        frame.h as f32 / manifest.height as f32,
                    ],
                    size: [frame.w as f32, frame.h as f32],
                    origin: resource.origin.map(|coordinate| coordinate as f32),
                    bounds: frame.bounds.or(resource.bounds).unwrap_or([
                        0,
                        0,
                        frame.w as i32 - 1,
                        frame.h as i32 - 1,
                    ]),
                });
            }
            resources.insert(name, frames);
        }
        Ok(Self {
            width: manifest.width,
            height: manifest.height,
            pixels: image.into_raw(),
            resources,
        })
    }

    pub fn rgba(&self) -> &[u8] {
        &self.pixels
    }

    /// Animation frame is authoritative simulation state, never presentation time.
    pub fn sprite(&self, name: &str, frame: f64) -> Option<AtlasFrame> {
        let frames = self.resources.get(name)?;
        let index = if frame.is_finite() {
            frame.floor().rem_euclid(frames.len() as f64) as usize
        } else {
            0
        };
        frames.get(index).copied()
    }

    /// Resolve a fractional display index to the adjacent frames for a smooth
    /// presentation blend. The game's image index and Animation End events
    /// remain on their authoritative fixed simulation ticks.
    pub fn sprite_transition(
        &self,
        name: &str,
        frame: f64,
    ) -> Option<(AtlasFrame, AtlasFrame, f32)> {
        let current = self.sprite(name, frame)?;
        if !frame.is_finite() {
            return Some((current, current, 0.0));
        }
        let mix = (frame - frame.floor()) as f32;
        if mix == 0.0 {
            return Some((current, current, 0.0));
        }
        let next = self.sprite(name, frame.floor() + 1.0)?;
        // A shared destination rectangle keeps both atlas samples registered
        // to the same sprite origin while blending their pixels.
        if current.size != next.size || current.origin != next.origin {
            return Some((current, current, 0.0));
        }
        if current.uv == next.uv {
            return Some((current, current, 0.0));
        }
        Some((current, next, mix))
    }

    /// A source tile_add rectangle within a background image.
    pub fn region(&self, name: &str, source: [u32; 4]) -> Option<AtlasFrame> {
        let mut frame = self.sprite(name, 0.0)?;
        let [x, y, width, height] = source;
        if width == 0
            || height == 0
            || x.checked_add(width)? > frame.size[0] as u32
            || y.checked_add(height)? > frame.size[1] as u32
        {
            return None;
        }
        frame.uv[0] += x as f32 / self.width as f32;
        frame.uv[1] += y as f32 / self.height as f32;
        frame.uv[2] = width as f32 / self.width as f32;
        frame.uv[3] = height as f32 / self.height as f32;
        frame.size = [width as f32, height as f32];
        frame.origin = [0.0; 2];
        frame.bounds = [0, 0, width as i32 - 1, height as i32 - 1];
        Some(frame)
    }
}

/// Match the source frame UV in a rendered quad, including horizontal mirroring.
#[cfg(test)]
pub(crate) fn quad_matches_sprite_frame(
    atlas: &Atlas,
    uv: [f32; 4],
    sprite: &str,
    animation: f64,
    mirrored: bool,
) -> bool {
    let Some(frame) = atlas.sprite(sprite, animation) else {
        return false;
    };
    let mut expected = frame.uv;
    if mirrored {
        expected[0] += expected[2];
        expected[2] = -expected[2];
    }
    uv == expected
}

/// Match the precise two-frame presentation sample emitted by the scene
/// builder. Tests use this instead of identifying animated sprites by only
/// the first atlas UV, which remains unchanged while the blended pose moves
/// through a 30 Hz source interval.
#[cfg(test)]
pub(crate) fn quad_matches_sprite_transition(
    atlas: &Atlas,
    quad: &DrawQuad,
    sprite: &str,
    animation: f64,
    mirrored: bool,
) -> bool {
    let Some((current, next, mix)) = atlas.sprite_transition(sprite, animation) else {
        return false;
    };
    let mut current_uv = current.uv;
    let mut next_uv = next.uv;
    if mirrored {
        current_uv[0] += current_uv[2];
        current_uv[2] = -current_uv[2];
        next_uv[0] += next_uv[2];
        next_uv[2] = -next_uv[2];
    }
    quad.uv == current_uv && quad.next_uv == next_uv && (quad.frame_mix - mix).abs() <= f32::EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_atlas_loads_and_retains_origins() {
        let atlas = Atlas::load().unwrap();
        assert_eq!(
            atlas.rgba().len(),
            atlas.width as usize * atlas.height as usize * 4
        );
        assert_eq!(atlas.sprite("sStandLeft", 0.0).unwrap().origin, [8.0, 8.0]);
        assert_eq!(atlas.sprite("bgCave", 0.0).unwrap().size, [64.0, 64.0]);
        assert_eq!(&atlas.rgba()[..4], &[255; 4]);
        assert!(atlas.sprite("missing", 0.0).is_none());
    }

    #[test]
    fn animation_wraps_and_handles_invalid_frame_numbers() {
        let atlas = Atlas::load().unwrap();
        let count = atlas.resources["sRunLeft"].len() as f64;
        assert_eq!(
            atlas.sprite("sRunLeft", 0.0),
            atlas.sprite("sRunLeft", count)
        );
        assert_eq!(
            atlas.sprite("sRunLeft", 0.0),
            atlas.sprite("sRunLeft", f64::NAN)
        );
        assert_eq!(
            atlas.sprite("sRunLeft", -1.0),
            atlas.sprite("sRunLeft", count - 1.0)
        );
    }

    #[test]
    fn fractional_display_index_blends_the_next_registered_subimage() {
        let atlas = Atlas::load().unwrap();
        let first = atlas.sprite("sRunLeft", 2.0).unwrap();
        let next = atlas.sprite("sRunLeft", 3.0).unwrap();
        let (actual, actual_next, mix) = atlas.sprite_transition("sRunLeft", 2.25).unwrap();
        assert_eq!(actual.uv, first.uv);
        assert_eq!(actual_next.uv, next.uv);
        assert_eq!(mix, 0.25);

        let (wrapped, wrapped_next, mix) = atlas.sprite_transition("sRunLeft", 5.5).unwrap();
        assert_eq!(wrapped.uv, atlas.sprite("sRunLeft", 5.0).unwrap().uv);
        assert_eq!(wrapped_next.uv, atlas.sprite("sRunLeft", 0.0).unwrap().uv);
        assert_eq!(mix, 0.5);
        assert_eq!(
            atlas.sprite_transition("sRunLeft", f64::NAN).unwrap().2,
            0.0
        );
    }
}
