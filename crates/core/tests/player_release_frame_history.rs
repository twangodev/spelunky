//! Ordinary trap recovery plus explicit configured skipped-callback input history.
//! No player/equipment/health/timer edits after the first simulated frame.
use spelunky_core::*;

fn floor(rules: RulesPreset) -> Game {
    let mut level = level::Level::default();
    level.set_tile(4, 20, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 21, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "release-frame-history", "");
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    game
}
fn held(left: bool) -> InputFrame {
    InputFrame {
        left,
        right: !left,
        ..Default::default()
    }
}
fn released(left: bool) -> InputFrame {
    InputFrame {
        left_released: left,
        right_released: !left,
        ..Default::default()
    }
}
#[test]
fn immediate_held_only_release_and_independent_device_edges_keep_source_half_acceleration() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for left in [false, true] {
            let expected = if left { -0.3 } else { 0.3 };
            let mut game = floor(rules);
            game.step(&held(left));
            assert_eq!(game.player.vel.x, 0.);
            game.step(&InputFrame::default());
            assert_eq!(
                game.player.vel.x, expected,
                "held-only API transition retains source acceleration"
            );
            let mut game = floor(rules);
            let mut input = held(left);
            input.left_released = left;
            input.right_released = !left;
            game.step(&input);
            assert_eq!(
                game.player.vel.x, expected,
                "explicit other-device release survives merged held=true"
            );
        }
    }
}
#[test]
fn configured_skipped_player_callback_does_not_replay_the_previous_frames_release() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for left in [false, true] {
            let mut game = Game::shell(42, rules);
            game.world = World::new(40, 32, Area::Mines, 1);
            game.world.name = "rLevel".into();
            for x in 0..40 {
                game.world.set_tile(x, 3, "oBlock");
            }
            // Initial persisted-source context: last controller and prior
            // actual input were held, but the Player starts inactive. Level553
            // reactivates Characters after the skipped Player319 callback.
            game.player.engine_active = false;
            game.player.data.insert(
                if left {
                    "previous_left"
                } else {
                    "previous_right"
                }
                .into(),
                1.,
            );
            game.data.insert(
                if left {
                    "input_frame_left"
                } else {
                    "input_frame_right"
                }
                .into(),
                1.,
            );
            let key = if left {
                "previous_left"
            } else {
                "previous_right"
            };
            let origin = game.player.pos;
            assert_eq!(origin, Vec2::new(40., 40.));
            assert_eq!(game.player.health, 4.0);
            game.step(&InputFrame::default());
            assert_eq!(
                game.player.pos, origin,
                "configured Player callback was skipped"
            );
            assert_eq!(game.player.data.get(key), Some(&1.));
            assert!(
                game.player.engine_active,
                "source Level reactivates Characters"
            );
            game.step(&InputFrame::default());
            assert_eq!(game.player.data.get(key), Some(&0.));
            assert_eq!(game.player.health, 4.0);
            assert_eq!(game.state, RunState::Playing);
            assert!(game.player.grounded);
            assert_eq!(
                game.player.vel.x, 0.,
                "SOURCE_SKIPPED_FRAME_RELEASE_EXPIRES_BEFORE_REACTIVATION"
            );
        }
    }
}
fn trap_room(rules: RulesPreset, character: characters::Character, left: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(if left { 29 } else { 10 }, 8, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    for x in if left { 26..=28 } else { 11..=13 } {
        for y in 7..=8 {
            level.set_tile(x, y, '1').unwrap();
        }
    }
    level
        .set_tile(if left { 33 } else { 6 }, 7, if left { '<' } else { '>' })
        .unwrap();
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "release-during-trap-stun", "");
    characters::apply(&mut game, character);
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    game
}
#[test]
fn real_trap_stun_consumes_a_release_without_inventing_a_recovery_impulse() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
        ] {
            for left in [false, true] {
                let mut game = trap_room(rules, character, left);
                let mut hanging = false;
                let mut arrow = false;
                for tick in 0..60 {
                    let was_hanging = game.player.state == PlayerState::Hanging;
                    let mut input = held(left);
                    input.jump = true;
                    input.jump_pressed = tick == 0;
                    game.step(&input);
                    arrow |= game.world.entities.iter().any(|e| e.object == "oArrow");
                    if game.player.health < 4.0 {
                        assert!(
                            hanging && was_hanging && arrow,
                            "source trap must hit a natural hanging player"
                        );
                        break;
                    }
                    hanging |= game.player.state == PlayerState::Hanging;
                }
                assert_eq!(game.player.health, 2.0);
                assert!(game.player.is_stunned());
                let key = if left {
                    "previous_left"
                } else {
                    "previous_right"
                };
                assert_eq!(game.player.data.get(key), Some(&1.));
                game.step(&released(left));
                assert!(
                    game.player.is_stunned(),
                    "actual release is consumed during the physical stun branch"
                );
                assert_eq!(
                    game.player.data.get(key),
                    Some(&1.),
                    "source controller key is still cached"
                );
                let mut resumed = false;
                for _ in 0..240 {
                    let incoming = game.player.clone();
                    game.step(&InputFrame::default());
                    assert_eq!(game.player.health, 2.0);
                    if game.player.data.get(key) == Some(&0.) {
                        // Source recovers from the stationary stun sprite and
                        // timer, even while its geometric bounce is airborne.
                        assert!(incoming.is_stunned());
                        assert_eq!(incoming.stun, 1);
                        assert_eq!(
                            incoming.data.get("source_platform_grounded"),
                            Some(&1.),
                            "SOURCE_SKIPPED_STUN_CONTROLLER_RETAINS_STANDING_PLATFORM_STATE"
                        );
                        assert_eq!(incoming.data.get(key), Some(&1.));
                        assert_eq!(
                            incoming.sprite,
                            match character {
                                characters::Character::Explorer => "sStunL",
                                characters::Character::Damsel => "sDamselStunL",
                                characters::Character::TunnelMan => unreachable!(),
                            },
                            "SOURCE_STATIONARY_STUN_SPRITE_QUALIFIES_FIRST_CONTROLLER_SAMPLE"
                        );
                        assert_eq!(
                            incoming.vel.x, 0.,
                            "SOURCE_RECOVERY_MUST_BEGIN_FROM_ACTUAL_ZERO_HORIZONTAL_VELOCITY"
                        );
                        assert_eq!(
                            game.player.vel.x, 0.,
                            "SOURCE_RELEASE_DURING_STUN_IS_NOT_A_FRESH_RECOVERY_EDGE"
                        );
                        resumed = true;
                        break;
                    }
                    assert!(game.player.is_stunned());
                }
                assert!(
                    resumed,
                    "natural source stun must recover within the fixed bound"
                );
            }
        }
    }
}
