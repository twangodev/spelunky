use super::*;
use spelunky_core::{InputFrame, RulesPreset, hub::HubKind};

#[test]
fn real_living_moon_timeout_draw_and_music_use_source_reveal_boundaries() {
    let atlas = Atlas::load().unwrap();
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(7, rules);
        game.load_hub(HubKind::Moon);
        let mut quads = Vec::new();
        scene::game_over_overlay(&mut quads, &atlas, &game, "UNUSED ACKNOWLEDGMENT");
        assert!(quads.is_empty());
        for _ in 0..1900 {
            game.step(&InputFrame::default());
            if game.data["timer"] < 0.0 {
                break;
            }
        }
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.player.health, 8.0);
        assert_eq!(game_music(&game), "boss");
        let hash = game.state_hash();
        let mut rng = game.rng.clone();
        let next_rng = rng.next_u64();
        // Actual source Draw snapshot, not a manually forced Dead game.
        scene::game_over_overlay(&mut quads, &atlas, &game, "UNUSED ACKNOWLEDGMENT");
        assert!(!quads.is_empty(), "{rules:?}");
        assert_eq!(quads[0].rect, [88., 48., 16., 16.]);
        assert!(quads.iter().all(|q| q.rect[1] != 80. && q.rect[1] != 120.));
        for _ in 0..8 {
            quads.clear();
            scene::game_over_overlay(&mut quads, &atlas, &game, "UNUSED ACKNOWLEDGMENT");
        }
        assert_eq!(game.state_hash(), hash);
        assert_eq!(game.rng.clone().next_u64(), next_rng);
        for _ in 0..29 {
            game.step(&InputFrame::default());
            assert_eq!(game_music(&game), "boss");
        }
        game.step(&InputFrame::default());
        assert_eq!(game_music(&game), "");
        assert!(stop_music_immediately(&game, game_music(&game)));
        quads.clear();
        scene::game_over_overlay(&mut quads, &atlas, &game, "UNUSED ACKNOWLEDGMENT");
        let line: Vec<_> = quads.iter().filter(|q| q.rect[1] == 80.).collect();
        assert_eq!(line.len(), "BETTER LUCK NEXT TIME...".len());
        assert!(quads.iter().all(|q| q.rect[1] != 88. && q.rect[1] != 120.));
        // Existing host room-entry reset must prevent a completed challenge
        // from muting the next room. Core room swap does not own audio policy.
        game.load_hub(HubKind::Scores);
        resume_room_music(&mut game);
        quads.clear();
        scene::game_over_overlay(&mut quads, &atlas, &game, "UNUSED ACKNOWLEDGMENT");
        assert!(quads.is_empty());
        assert!(!game.flags.contains("source_music_stopped"));
    }
}
