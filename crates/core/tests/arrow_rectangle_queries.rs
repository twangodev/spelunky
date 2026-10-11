//! Source imprecise rectangle queries against naturally rotating trap arrows.
use spelunky_core::*;

fn room(rules: RulesPreset) -> Game {
    let mut level = level::Level::default();
    level.set_tile(23, 9, '@').unwrap();
    level.set_tile(25, 9, '<').unwrap();
    for x in 0..40 {
        level.set_tile(x, 10, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "rotating-arrow-rectangle", "");
    game
}

#[test]
fn walking_away_from_trap_waits_for_source_integer_arrow_bounds() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let input = InputFrame {
            left: true,
            ..Default::default()
        };
        for tick in 0..7 {
            // Source Player319 Step runs before Arrow465 Step. The final
            // iteration queries the existing diagonal arrow at (390,165).
            if tick == 6 {
                let arrow = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.object == "oArrow")
                    .unwrap();
                assert_eq!(arrow.pos, Vec2::new(390.0, 165.0));
                assert_eq!(arrow.vel.x, -8.0);
                assert!((arrow.vel.y - 1.4).abs() < 1e-12);
                assert!(
                    (arrow.value("rotation") + std::f64::consts::PI + (1.4_f64 / 8.0).atan()).abs()
                        < 1e-12
                );
            }
            game.step(&input);
            assert_eq!(
                game.player.health, 4,
                "{rules:?}/tick{tick}: no source bbox contact yet"
            );
        }
        assert_eq!(game.player.pos, Vec2::new(378.0, 168.0));
        assert!(!game.player.is_stunned());
        // Native bbox updater4AA958 gives [387,161,394,167] before the last
        // Player Step. Its raw rectangle ends at x386, so the query is false.
        // The next real frame closes that gap; genuine contact still hurts.
        game.step(&input);
        assert_eq!(
            game.player.health, 2,
            "{rules:?}: next real arrow contact must hurt"
        );
        assert!(game.player.is_stunned());
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oArrow")
        );
    }
}

#[test]
fn triggered_arrow_still_hits_when_player_stops() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        // The entrance is outside the detector at rest. Walk into its real
        // range, let automatic Collision emit the Arrow, and then stop.
        for _ in 0..4 {
            game.step(&InputFrame {
                left: true,
                ..Default::default()
            });
        }
        assert_eq!(game.player.health, 4);
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oArrow")
        );
        let mut hurt = false;
        for _ in 0..15 {
            game.step(&InputFrame::default());
            if game.player.health == 2 {
                hurt = true;
                assert!(game.player.is_stunned());
                break;
            }
        }
        assert!(hurt, "{rules:?}: source collision must remain enabled");
    }
}
