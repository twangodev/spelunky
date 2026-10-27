//! Source characterStepEvent219-231: Jump leaves a ladder with the configured
//! lateral launch, upward launch, and five-tick reattach lockout. The ladder
//! is authored, then all movement and jump transitions come through Game::step.
use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, Vec2, characters, level::Level};

fn climbing(rules: RulesPreset, character: characters::Character, vine: bool) -> Game {
    let mut level = Level::default();
    level.set_tile(5, 8, '@').unwrap(); // Raw entrance at (104, 152).
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap(); // Support at y=160.
    }
    let mut game = Game::shell(42, rules);
    characters::apply(&mut game, character);
    game.load_custom(&level, "ladder-jump-off", "");
    for y in [112.0, 128.0, 144.0] {
        let object = if vine {
            if y == 112.0 { "oVineTop" } else { "oVine" }
        } else {
            "oLadderOrange"
        };
        game.spawn(object, Vec2::new(96.0, y));
    }
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.player.pos, Vec2::new(104.0, 152.0));
    assert!(game.player.grounded);

    game.step(&InputFrame {
        up: true,
        ..Default::default()
    });
    assert_eq!(game.player.state, PlayerState::Climbing);
    for _ in 0..10 {
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(game.player.state, PlayerState::Climbing);
        assert_eq!(game.player.health, 4.0);
        assert!(!game.player.is_stunned());
    }
    assert!(game.player.pos.y < 152.0);
    assert!(game.player.pos.y > 112.0);
    assert!(!game.player.grounded);
    game
}

#[test]
fn natural_ladder_jump_uses_source_launch_direction_and_locks_out_reattachment() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            for vine in [false, true] {
                for (left, right, expected_sign) in [(true, false, -1.0), (false, true, 1.0)] {
                    let mut game = climbing(rules, character, vine);
                    let before = game.player.pos;
                    game.step(&InputFrame {
                        left,
                        right,
                        jump: true,
                        jump_pressed: true,
                        ..Default::default()
                    });
                    assert_eq!(game.player.state, PlayerState::Jumping);
                    assert_eq!(game.player.data.get("ladder_timer"), Some(&5.0));
                    assert_eq!(game.player.facing, expected_sign as i8);
                    assert!(game.player.vel.x * expected_sign > 0.0);
                    assert!(game.player.vel.y < 0.0);
                    assert_eq!(game.player.health, 4.0);
                    assert!(!game.player.is_stunned());

                    // The launch is spatial, not just a state change. Continue
                    // with ordinary input after the source's one-step launch.
                    for _ in 0..4 {
                        game.step(&InputFrame::default());
                    }
                    assert_ne!(game.player.pos.x, before.x);
                    assert_eq!(game.player.state, PlayerState::Jumping);
                    assert!(game.player.vel.x * expected_sign > 0.0);
                    assert!(game.player.pos.y < before.y);
                    assert_eq!(game.player.health, 4.0);
                }
            }
        }
    }
}

#[test]
fn natural_vertical_ladder_jump_cannot_reattach_during_source_timer() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            for vine in [false, true] {
                let mut game = climbing(rules, character, vine);
                game.step(&InputFrame {
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                });
                assert_eq!(game.player.state, PlayerState::Jumping);
                assert_eq!(game.player.data.get("ladder_timer"), Some(&5.0));

                // Source236 decrements this timer before source610 tests for
                // another ladder attach. The first four Up samples must stay
                // airborne even while the player's mask still crosses a rung.
                for expected_timer in [4.0, 3.0, 2.0, 1.0] {
                    game.step(&InputFrame {
                        up: true,
                        ..Default::default()
                    });
                    assert_eq!(game.player.state, PlayerState::Jumping);
                    assert_eq!(game.player.data.get("ladder_timer"), Some(&expected_timer));
                    assert!(game.player.vel.y < 0.0);
                    assert_eq!(game.player.health, 4.0);
                    assert!(!game.player.is_stunned());
                }
            }
        }
    }
}

#[test]
fn natural_down_leaves_the_bottom_ladder_and_falls_to_the_next_solid_floor() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for vine in [false, true] {
            let mut level = Level::default();
            level.set_tile(5, 8, '@').unwrap();
            for x in 0..40 {
                level.set_tile(x, 12, '1').unwrap();
            }
            let mut game = Game::shell(42, rules);
            game.load_custom(&level, "ladder-bottom-release", "");
            for y in [112.0, 128.0, 144.0] {
                let object = if vine {
                    if y == 112.0 { "oVineTop" } else { "oVine" }
                } else {
                    "oLadderOrange"
                };
                game.spawn(object, Vec2::new(96.0, y));
            }
            for _ in 0..4 {
                game.step(&InputFrame::default());
            }

            let mut attached = false;
            for _ in 0..24 {
                game.step(&InputFrame {
                    up: true,
                    ..Default::default()
                });
                assert_eq!(game.player.health, 4.0);
                assert!(!game.player.is_stunned());
                if game.player.state == PlayerState::Climbing {
                    attached = true;
                    break;
                }
            }
            assert!(
                attached,
                "ordinary Up failed to attach: {rules:?}, vine={vine}"
            );

            let mut left_bottom = false;
            for _ in 0..100 {
                game.step(&InputFrame {
                    down: true,
                    ..Default::default()
                });
                assert_eq!(game.player.health, 4.0);
                assert!(!game.player.is_stunned());
                if game.player.state == PlayerState::Falling {
                    left_bottom = true;
                    break;
                }
            }
            assert!(left_bottom, "ordinary Down never left the ladder bottom");
            assert!(!game.player.grounded);

            for _ in 0..100 {
                game.step(&InputFrame::default());
            }
            assert!(game.player.grounded);
            assert_eq!(game.player.health, 4.0);
            assert!(!game.player.is_stunned());
        }
    }
}
