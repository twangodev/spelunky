//! Ordinary-input source sprite branches; no state, frame, or velocity injection.
use spelunky_core::*;

fn scene(rules: RulesPreset, character: characters::Character, floating: bool) -> Game {
    let mut room = level::Level::default();
    room.set_tile(8, if floating { 3 } else { 6 }, '@').unwrap();
    for x in 0..40 {
        room.set_tile(x, if floating { 20 } else { 7 }, '1')
            .unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&room, "source-sprite-history", "");
    characters::apply(&mut game, character);
    game
}

#[test]
fn stationary_up_then_horizontal_input_uses_the_looking_up_character_branch() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (character, stationary, moving) in [
            (characters::Character::Explorer, "sLookLeft", "sLookRunL"),
            (characters::Character::Damsel, "sDamselLookL", "sDamselRunL"),
            (
                characters::Character::TunnelMan,
                "sTunnelLookL",
                "sTunnelRunL",
            ),
        ] {
            let mut game = scene(rules, character, false);
            let health = game.player.health;
            for _ in 0..4 {
                game.step(&InputFrame::default());
            }
            assert_eq!(game.player.state, PlayerState::Standing);
            assert!(game.player.grounded);
            assert_eq!(game.player.vel, Vec2::default());
            game.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(game.player.state, PlayerState::LookingUp);
            assert_eq!(game.player.sprite, stationary);
            for _ in 0..2 {
                game.step(&InputFrame {
                    up: true,
                    right: true,
                    ..Default::default()
                });
                assert_eq!(game.player.state, PlayerState::LookingUp);
                assert_eq!(game.player.vel.x, 0.0);
                assert_eq!(game.player.sprite, stationary);
            }
            game.step(&InputFrame {
                up: true,
                right: true,
                ..Default::default()
            });
            assert_eq!(game.player.state, PlayerState::LookingUp);
            assert!((game.player.vel.x - 1.8).abs() < 1e-12);
            assert_eq!(game.player.vel.y, 0.0);
            assert!(game.player.grounded);
            assert_eq!(
                game.player.sprite, moving,
                "SOURCE_LOOKING_UP_SPRITE: {rules:?}/{character:?}"
            );
            assert_eq!(game.player.health, health);
            assert!(!game.player.is_stunned());
            assert_eq!(game.state, RunState::Playing);

            // Releasing Up returns to the ordinary running branch. Reapplying
            // Up while already Running intentionally selects Tunnel LookRun.
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            assert_eq!(game.player.state, PlayerState::Running);
            game.step(&InputFrame {
                up: true,
                right: true,
                ..Default::default()
            });
            assert_eq!(game.player.state, PlayerState::Running);
            assert_eq!(
                game.player.sprite,
                match character {
                    characters::Character::Explorer => "sLookRunL",
                    characters::Character::Damsel => "sDamselRunL",
                    characters::Character::TunnelMan => "sTunnelLookRunL",
                }
            );
            assert_eq!(game.player.health, health);
        }
    }
}

#[test]
fn a_fresh_unsupported_custom_entrance_starts_with_both_falling_history_states() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (character, falling, standing) in [
            (characters::Character::Explorer, "sFallLeft", "sStandLeft"),
            (characters::Character::Damsel, "sDamselFallL", "sDamselLeft"),
            (
                characters::Character::TunnelMan,
                "sTunnelFallL",
                "sTunnelLeft",
            ),
        ] {
            let mut game = scene(rules, character, true);
            let health = game.player.health;
            assert_eq!(game.player.pos, Vec2::new(152.0, 72.0));
            assert_eq!(game.player.state, PlayerState::Falling);
            assert_eq!(game.player.sprite, standing);
            // Create initializes statePrev and statePrevPrev to FALLING. The
            // first completed controller is already eligible for its Fall art.
            for (age, (y, vy)) in [(73.0, 1.0), (75.0, 2.0), (78.0, 3.0)]
                .into_iter()
                .enumerate()
            {
                game.step(&InputFrame::default());
                assert_eq!(game.level_tick, age as u64 + 1);
                assert_eq!(game.player.pos, Vec2::new(152.0, y));
                assert_eq!(game.player.vel, Vec2::new(0.0, vy));
                assert_eq!(game.player.state, PlayerState::Falling);
                assert!(!game.player.grounded);
                assert!(
                    !game
                        .world
                        .solid_at(game.player.pos.x, game.player.pos.y + 8.0)
                );
                assert_eq!(
                    game.player.sprite, falling,
                    "SOURCE_CREATE_FALL_HISTORY: {rules:?}/{character:?}/age{age}"
                );
                assert_eq!(game.player.health, health);
                assert!(!game.player.is_stunned());
                assert_eq!(game.state, RunState::Playing);
            }
            let mut supported = scene(rules, character, false);
            for _ in 0..4 {
                supported.step(&InputFrame::default());
            }
            assert_eq!(supported.player.state, PlayerState::Standing);
            assert_eq!(supported.player.sprite, standing);
            assert_eq!(supported.player.data.get("fall_state_ticks"), Some(&0.0));
            assert!(supported.player.grounded);
        }
    }
}
