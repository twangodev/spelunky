//! Opt-in gameplay view geometry. Coordinates remain original game pixels.
use crate::{Rect, VIEW_HEIGHT, VIEW_WIDTH, Vec2};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub enum SimulationView {
    #[default]
    Original,
    Adaptive(SimulationViewExtent),
}

impl<'de> Deserialize<'de> for SimulationView {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        enum Policy {
            Original,
            Adaptive(SimulationViewExtent),
        }
        match Policy::deserialize(deserializer)? {
            Policy::Original => Ok(Self::Original),
            Policy::Adaptive(extent) => {
                Self::from_extent(extent.width, extent.height).map_err(serde::de::Error::custom)
            }
        }
    }
}

/// Validated logical extent, supplied at a simulation boundary by the host.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SimulationViewExtent {
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationViewError;
impl std::fmt::Display for SimulationViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Simulation view must have finite width >= 320 and height >= 240")
    }
}
impl std::error::Error for SimulationViewError {}
impl<'de> Deserialize<'de> for SimulationViewExtent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Extent {
            width: f64,
            height: f64,
        }
        let extent = Extent::deserialize(deserializer)?;
        Self::new(extent.width, extent.height).map_err(serde::de::Error::custom)
    }
}
impl SimulationViewExtent {
    pub fn new(width: f64, height: f64) -> Result<Self, SimulationViewError> {
        if width.is_finite() && height.is_finite() && width >= VIEW_WIDTH && height >= VIEW_HEIGHT {
            Ok(Self { width, height })
        } else {
            Err(SimulationViewError)
        }
    }
}
impl SimulationView {
    pub fn from_extent(width: f64, height: f64) -> Result<Self, SimulationViewError> {
        let extent = SimulationViewExtent::new(width, height)?;
        if width == VIEW_WIDTH && height == VIEW_HEIGHT {
            Ok(Self::Original)
        } else {
            Ok(Self::Adaptive(extent))
        }
    }
    pub fn is_original(&self) -> bool {
        match self {
            Self::Original => true,
            Self::Adaptive(extent) => extent.width == VIEW_WIDTH && extent.height == VIEW_HEIGHT,
        }
    }
    /// Same centering/clamping as app Viewport::camera, using live camera only.
    pub fn rect(self, camera: Vec2, room_width: f64, room_height: f64) -> Rect {
        let Self::Adaptive(extent) = self else {
            return Rect::new(camera.x, camera.y, VIEW_WIDTH, VIEW_HEIGHT);
        };
        // An explicitly deserialized 320x240 extent is also the exact original
        // camera, including authored cinematics or cameras beyond room edges.
        if extent.width == VIEW_WIDTH && extent.height == VIEW_HEIGHT {
            return Rect::new(camera.x, camera.y, VIEW_WIDTH, VIEW_HEIGHT);
        }
        fn axis(origin: f64, extent: f64, original_extent: f64, room: f64) -> f64 {
            if room <= extent {
                (room - extent) * 0.5
            } else {
                (origin - (extent - original_extent) * 0.5).clamp(0.0, room - extent)
            }
        }
        Rect::new(
            axis(camera.x, extent.width, VIEW_WIDTH, room_width),
            axis(camera.y, extent.height, VIEW_HEIGHT, room_height),
            extent.width,
            extent.height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Game, RulesPreset};

    #[test]
    fn adaptive_geometry_matches_logical_view_at_edges_and_small_rooms() {
        // These are the f32 logical extents returned by the app's real viewport.
        let width = f64::from(426.66666_f32);
        let wide = SimulationView::from_extent(width, 240.0).unwrap();
        let middle = wide.rect(Vec2::new(160.0, 120.0), 640.0, 480.0);
        assert_eq!(
            middle,
            Rect::new(160.0 - (width - 320.0) * 0.5, 120.0, width, 240.0)
        );
        assert_eq!(wide.rect(Vec2::default(), 640.0, 480.0).x, 0.0);
        assert_eq!(
            wide.rect(Vec2::new(320.0, 240.0), 640.0, 480.0).x,
            640.0 - width
        );
        assert_eq!(
            wide.rect(Vec2::default(), 160.0, 120.0),
            Rect::new((160.0 - width) * 0.5, -60.0, width, 240.0)
        );
        let portrait = SimulationView::from_extent(320.0, 640.0).unwrap();
        assert_eq!(
            portrait.rect(Vec2::new(200.0, 180.0), 672.0, 544.0),
            Rect::new(200.0, -48.0, 320.0, 640.0)
        );
    }

    #[test]
    fn original_codec_is_omitted_and_invalid_updates_are_atomic() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        let before = serde_json::to_vec(&game).unwrap();
        assert!(
            !serde_json::to_value(&game)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("simulation_view")
        );
        game.set_simulation_view_extent(320.0, 240.0).unwrap();
        assert_eq!(serde_json::to_vec(&game).unwrap(), before);
        for (w, h) in [
            (0.0, 240.0),
            (320.0, 239.0),
            (f64::NAN, 240.0),
            (320.0, f64::INFINITY),
        ] {
            assert!(game.set_simulation_view_extent(w, h).is_err());
            assert_eq!(serde_json::to_vec(&game).unwrap(), before);
        }
        let historical: Game = serde_json::from_slice(&before).unwrap();
        assert_eq!(historical.simulation_view, SimulationView::Original);
        assert_eq!(serde_json::to_vec(&historical).unwrap(), before);
        let normalized: SimulationView =
            serde_json::from_str(r#"{"Adaptive":{"width":320,"height":240}}"#).unwrap();
        assert_eq!(normalized, SimulationView::Original);
        game.set_simulation_view_extent(480.0, 240.0).unwrap();
        let restored: Game = serde_json::from_slice(&serde_json::to_vec(&game).unwrap()).unwrap();
        assert_eq!(restored.simulation_view, game.simulation_view);
        assert!(
            serde_json::from_str::<SimulationView>(r#"{"Adaptive":{"width":319,"height":240}}"#)
                .is_err()
        );
        assert!(
            serde_json::from_str::<SimulationView>(
                r#"{"Adaptive":{"width":480,"height":240,"extra":1}}"#
            )
            .is_err()
        );
    }

    #[test]
    fn callback_geometry_reads_live_camera_without_changing_authoritative_pose() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.set_simulation_view_extent(480.0, 240.0).unwrap();
        game.camera = Vec2::new(160.0, 100.0);
        game.previous_camera = Vec2::new(150.0, 99.0);
        let player = game.player.clone();
        let rng = game.rng.clone();
        assert_eq!(
            game.simulation_view_rect(),
            Rect::new(80.0, 100.0, 480.0, 240.0)
        );
        game.camera.x = 180.0;
        assert_eq!(
            game.simulation_view_rect(),
            Rect::new(100.0, 100.0, 480.0, 240.0)
        );
        assert_eq!(game.previous_camera, Vec2::new(150.0, 99.0));
        assert_eq!(
            serde_json::to_value(&game.player).unwrap(),
            serde_json::to_value(player).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&game.rng).unwrap(),
            serde_json::to_value(rng).unwrap()
        );
        game.flags.insert("hub".into());
        assert_eq!(
            game.simulation_view_rect(),
            Rect::new(180.0, 100.0, 320.0, 240.0)
        );
    }
}
