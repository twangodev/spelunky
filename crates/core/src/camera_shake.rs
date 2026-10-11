//! The level controller consumes scrShake requests at source object 553.
use crate::Game;

/// scrShake: retain the longer request, clamp negative durations, and capture
/// the current view even when the request does not extend the duration.
pub(crate) fn request(game: &mut Game, duration: f64) {
    let current = game.data.get("shake").copied().unwrap_or(0.0);
    game.data
        .insert("shake".into(), current.max(duration).max(0.0));
    game.data.insert("xview".into(), game.camera.x);
    game.data.insert("yview".into(), game.camera.y);
}

/// oLevel Step, before region activation. Expiry restores the follow border;
/// it deliberately retains the alternating phase and the current view Y.
pub(crate) fn level_step(game: &mut Game) {
    let duration = game.data.get("shake").copied().unwrap_or(0.0);
    let mut border = 96.0;
    if duration > 0.0 {
        if game.player.pos.y < 96.0 || game.player.pos.y > game.world.height as f64 * 16.0 - 96.0 {
            border = 0.0;
        }
        let toggle = game.data.get("shake_toggle").copied().unwrap_or(0.0) != 0.0;
        if toggle || game.camera.y <= 0.0 {
            game.camera.y += 3.0;
            game.data.insert("shake_toggle".into(), 0.0);
        } else {
            game.camera.y -= 3.0;
            game.data.insert("shake_toggle".into(), 1.0);
        }
        game.data.insert("shake".into(), duration - 1.0);
    }
    game.data.insert("camera_vborder".into(), border);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, RulesPreset, Vec2, World};

    #[test]
    fn request_retains_max_clamps_negative_and_always_captures_current_view() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.camera = Vec2::new(19., 41.);
        game.data.insert("shake".into(), 100.);
        game.data.insert("shake_toggle".into(), 1.);
        request(&mut game, 5.);
        assert_eq!(game.data["shake"], 100.);
        assert_eq!(game.data["xview"], 19.);
        assert_eq!(game.data["yview"], 41.);
        assert_eq!(game.data["shake_toggle"], 1.);
        game.data.insert("shake".into(), -5.);
        request(&mut game, -2.);
        assert_eq!(game.data["shake"], 0.);
    }

    #[test]
    fn inactive_olmec_follow_uses_level_border_rounded_origin_and_room_clamp() {
        // Configured view-follow controls for canonical 4BECA4, not a natural
        // boss-intro fixture. The existing temple test covers the real intro.
        for (target_y, border, expected) in [
            (552.5, None, 400.0), // Olmec Create's initial zero border.
            (552.5, Some(0.0), 400.0),
            (552.5, Some(96.0), 408.0), // Ties to even:552-(240-96).
            (553.5, Some(96.0), 410.0), // Ties to even:554-(240-96).
            (900.0, Some(96.0), 640.0), // Room880 clamps view240.
        ] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            game.world = World::new(42, 55, Area::Boss, 16);
            game.world.spawn("oOlmec", Vec2::new(304.5, target_y));
            game.player.data.insert("active".into(), 0.0);
            game.camera = Vec2::new(0.0, 400.0);
            if let Some(border) = border {
                game.data.insert("camera_vborder".into(), border);
            }
            let rng = game.rng.clone().next_u64();
            game.update_camera();
            assert_eq!(game.camera, Vec2::new(0.0, expected));
            assert_eq!(game.rng.clone().next_u64(), rng);
        }
    }

    #[test]
    fn level_step_strict_edge_borders_expiry_and_no_random_draws() {
        for (y, border) in [(95., 0.), (96., 96.), (416., 96.), (417., 0.)] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            game.world = World::new(64, 32, Area::Mines, 1);
            game.player.pos.y = y;
            game.camera.y = 100.;
            game.data.insert("shake".into(), 1.);
            let rng = game.rng.clone().next_u64();
            level_step(&mut game);
            assert_eq!(game.data["camera_vborder"], border);
            assert_eq!(game.data["shake"], 0.);
            assert_eq!(game.camera.y, 97.);
            assert_eq!(game.data["shake_toggle"], 1.);
            level_step(&mut game);
            assert_eq!(game.data["camera_vborder"], 96.);
            assert_eq!(game.camera.y, 97.);
            assert_eq!(game.data["shake_toggle"], 1.);
            assert_eq!(game.rng.clone().next_u64(), rng);
        }
    }
}
