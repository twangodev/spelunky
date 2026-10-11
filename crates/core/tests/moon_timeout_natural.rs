use spelunky_core::{Game, InputFrame, RulesPreset, RunState, hub::HubKind};

fn authored_moon(rules: RulesPreset) -> Game {
    let mut game = Game::shell(7, rules);
    game.load_hub(HubKind::Moon);
    game
}

#[test]
fn authored_moon_alarm_zero_starts_countdown_in_its_same_alarm_pass() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = authored_moon(rules);
        for _ in 0..99 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.data["timer"], 60.0);
        game.step(&InputFrame::default());
        assert_eq!(game.level_tick, 100);
        assert_eq!(game.data["timer"], 59.0, "{rules:?}");
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.player.health, 8);
    }
}

#[test]
fn authored_moon_living_timeout_reveals_then_returns_through_real_scores_portal() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = authored_moon(rules);
        // The authored entrance platform (player40,88 / support32,96) is
        // separated from target xMid160/208/256 (late sway is at most32).
        // Neutral input survives naturally; no timer/HP/state writes occur.
        for _ in 0..1900 {
            game.step(&InputFrame::default());
            assert_eq!(game.state, RunState::Playing, "{rules:?}");
            assert_eq!(game.player.health, 8);
            if game.data["timer"] < 0.0 {
                break;
            }
        }
        assert!(game.data["timer"] < 0.0);
        assert_eq!(game.game_over.draw_status, 1, "{rules:?}");
        assert_eq!(game.level_tick, 1870);
        assert_eq!(game.data["mini2"], 0.0);
        assert_eq!(game.data["challenge_highscore"], 0.0);
        assert!(game.flags.contains("hub_challenge_finished"));
        assert!(!game.flags.contains("source_music_stopped"));
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBatTarget")
        );
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oEntrance")
        );
        let portal = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oXScores")
            .unwrap();
        let poofs: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oPoof")
            .collect();
        assert_eq!(poofs.len(), 6);
        assert!(
            poofs
                .iter()
                .all(|e| e.creation_serial < portal.creation_serial)
        );
        assert_eq!(poofs.iter().filter(|e| e.vel.y == -1.0).count(), 3);
        assert_eq!(poofs.iter().filter(|e| e.vel.y == 1.0).count(), 3);
        for _ in 0..29 {
            game.step(&InputFrame::default());
            assert_eq!(game.game_over.draw_status, 1);
            assert!(!game.flags.contains("source_music_stopped"));
        }
        game.step(&InputFrame::default());
        assert_eq!(game.game_over.draw_status, 3);
        assert!(game.flags.contains("source_music_stopped"));
        assert_eq!(game.state, RunState::Playing);
        // Living Moon has no invented attack-to-scores acknowledgment. Its
        // authored Scores door uses the ordinary Up/exit animation route.
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        for _ in 0..40 {
            if game.state == RunState::Transition {
                break;
            }
            game.step(&InputFrame::default());
        }
        assert_eq!(game.state, RunState::Transition);
        assert!(game.flags.contains("hub_destination:scores"));
        assert!(game.flags.contains("scores_start:2"));
    }
}

#[test]
fn moon_initial_authored_fatal_hazard_reveals_without_living_timeout_replacement() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = authored_moon(rules);
        // Initial authored hazard only: actual Explosion Create + ordinary
        // Game.step hurt query. No mid-run health, timer, or state injection.
        game.spawn("oExplosion", game.player.pos);
        game.step(&InputFrame::default());
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(game.data["timer"], -1.0);
        assert_eq!(game.data["mini2"], 0.0);
        for _ in 0..29 {
            game.step(&InputFrame::default());
        }
        assert!(game.game_over.draw_status < 3);
        game.step(&InputFrame::default());
        assert_eq!(game.game_over.draw_status, 3, "{rules:?}");
        assert!(game.flags.contains("source_music_stopped"));
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oEntrance")
        );
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oXScores")
        );
        for _ in 0..40 {
            game.step(&InputFrame::default());
        }
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBatTarget")
                .count(),
            3
        );
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oXScores")
        );
    }
}
