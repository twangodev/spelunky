//! Presentation-only view. Core camera and activation retain Classic's 320×240.
use spelunky_core::Vec2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub scale: f64,
    pub surface_width: u32,
    pub surface_height: u32,
}

impl Viewport {
    pub fn canonical() -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            scale: 1.0,
            surface_width: 320,
            surface_height: 240,
        }
    }

    pub fn from_surface(width: u32, height: u32) -> Self {
        if width == 0 || height == 0 {
            return Self {
                surface_width: width,
                surface_height: height,
                ..Self::canonical()
            };
        }
        let scale = (f64::from(width) / 320.0).min(f64::from(height) / 240.0);
        Self {
            width: (f64::from(width) / scale) as f32,
            height: (f64::from(height) / scale) as f32,
            scale,
            surface_width: width,
            surface_height: height,
        }
    }

    pub fn ui_origin(self) -> [f32; 2] {
        [(self.width - 320.0) * 0.5, (self.height - 240.0) * 0.5]
    }

    /// Inverse of the centered authored UI's uniform physical projection.
    pub fn logical_pointer(self, x: f64, y: f64) -> [f32; 2] {
        if self.surface_width == 0 || self.surface_height == 0 || !x.is_finite() || !y.is_finite() {
            return [f32::NEG_INFINITY; 2];
        }
        let [ox, oy] = self.ui_origin();
        [(x / self.scale) as f32 - ox, (y / self.scale) as f32 - oy]
    }

    pub fn camera(self, canonical: Vec2, room_width: f64, room_height: f64) -> Vec2 {
        if self.width == 320.0 && self.height == 240.0 {
            return canonical;
        }
        fn axis(origin: f64, extent: f64, canonical_extent: f64, room: f64) -> f64 {
            if room <= extent {
                (room - extent) * 0.5
            } else {
                (origin - (extent - canonical_extent) * 0.5).clamp(0.0, room - extent)
            }
        }
        Vec2::new(
            axis(canonical.x, f64::from(self.width), 320.0, room_width),
            axis(canonical.y, f64::from(self.height), 240.0, room_height),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expanded_views_fill_surface_with_square_pixels_and_dpr_invariance() {
        for (w, h) in [
            (1920, 1080),
            (800, 800),
            (600, 1200),
            (3440, 1440),
            (160, 120),
        ] {
            let view = Viewport::from_surface(w, h);
            assert!((f64::from(view.width) * view.scale - f64::from(w)).abs() < 0.001);
            assert!((f64::from(view.height) * view.scale - f64::from(h)).abs() < 0.001);
            let double = Viewport::from_surface(w * 2, h * 2);
            assert_eq!((view.width, view.height), (double.width, double.height));
            assert_eq!(double.scale, view.scale * 2.0);
            let [x, y] = view.ui_origin();
            let pointer = view.logical_pointer(
                f64::from(x + 123.5) * view.scale,
                f64::from(y + 87.25) * view.scale,
            );
            assert!((pointer[0] - 123.5).abs() < 0.0001 && (pointer[1] - 87.25).abs() < 0.0001);
        }
        assert_eq!(
            Viewport::from_surface(0, 100).logical_pointer(0.0, 0.0),
            [f32::NEG_INFINITY; 2]
        );
    }
    #[test]
    fn expanded_camera_clamps_edges_and_centers_small_rooms() {
        let view = Viewport::from_surface(1920, 1080);
        assert_eq!(
            view.camera(Vec2::new(0.0, 0.0), 640.0, 480.0),
            Vec2::new(0.0, 0.0)
        );
        let center = view.camera(Vec2::new(160.0, 120.0), 640.0, 480.0);
        assert!((center.x - 106.66667175292969).abs() < 0.001);
        assert_eq!(center.y, 120.0);
        let tiny = view.camera(Vec2::new(0.0, 0.0), 160.0, 120.0);
        assert!((tiny.x - (160.0 - f64::from(view.width)) * 0.5).abs() < 0.001);
        assert_eq!(tiny.y, -60.0);
    }
}
