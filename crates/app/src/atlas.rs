//! Original sprite coordinates and origins, exported without resampling.
use std::collections::BTreeMap;

use serde::Deserialize;

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
}
