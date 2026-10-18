//! Prepared source-facing controls. No compilation or runtime claim.
use spelunky_core::*;

fn movement(direction: f64, down: bool, run: bool) -> InputFrame {
    InputFrame {
        left: direction < 0.0,
        right: direction > 0.0,
        down,
        run,
        ..Default::default()
    }
}

fn room(rules: RulesPreset, option: Option<bool>) -> Game {
    let mut level = level::Level::default();
    level.set_tile(20, 8, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    // Existing public serialization deliberately configures the initial game.
    // Before the option exists the old deserializer ignores this extra key;
    // therefore the same behavioral tests compile against the old controller.
    if let Some(enabled) = option {
        let mut json = serde_json::to_value(&game).unwrap();
        json["down_to_run"] = serde_json::Value::Bool(enabled);
        game = serde_json::from_value(json).unwrap();
    }
    game.load_custom(&level, "down-to-run", "");
    game
}

fn warm_walk(game: &mut Game, direction: f64, fast: bool) -> f64 {
    for _ in 0..6 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded, "FIXTURE_GROUND");
    for _ in 0..12 {
        game.step(&movement(direction, false, false));
        let speed = game.player.vel.x.abs();
        if (fast && (2.0..3.0).contains(&speed)) || (!fast && speed > 0.1 && speed < 2.0) {
            assert_eq!(game.player.state, PlayerState::Running);
            assert!(game.player.grounded);
            assert_eq!(game.player.health, 4.0);
            let steps = if direction < 0.0 {
                "left_steps"
            } else {
                "right_steps"
            };
            assert!(game.player.data[steps] >= 3.0);
            return game.player.vel.x;
        }
    }
    panic!("FIXTURE_WALK_SPEED: {:?}", game.player);
}

fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-10,
        "actual={actual} expected={expected}"
    );
}

#[test]
fn natural_walk_then_down_runs_by_default_in_both_directions_and_rules() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1.0, 1.0] {
            let mut game = room(rules, None);
            let incoming = warm_walk(&mut game, direction, true);
            let start = game.player.pos;
            game.step(&movement(direction, true, false));
            // Source: high-speed Duck/Left-or-Right arm adds 0.1, limit6 and
            // friction0.98. Normal horizontal acceleration remains3 here.
            let expected = ((incoming + direction * 0.1 + direction * 3.0) * 0.98).clamp(-6.0, 6.0);
            near(game.player.vel.x, expected);
            assert!(game.player.vel.x.abs() > 3.0);
            assert!((game.player.pos.x - start.x) * direction > 0.0);
            assert!(game.player.grounded);
            assert_eq!(game.player.health, 4.0);
        }
    }
}

#[test]
fn disabled_down_run_keeps_literal_high_speed_fallback_including_left_asymmetry() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1.0, 1.0] {
            let mut game = room(rules, Some(false));
            let incoming = warm_walk(&mut game, direction, true);
            game.step(&movement(direction, true, false));
            // Canonical fallback tests signed xVel<0.5 after multiplying0.8.
            let damped = incoming * 0.8;
            let damped = if damped < 0.5 { 0.0 } else { damped };
            near(game.player.vel.x, (damped + direction * 3.0) * 0.2);
            assert!(game.player.vel.x.abs() < 2.0);
            assert_eq!(game.player.health, 4.0);
        }
    }
}

#[test]
fn low_speed_duck_stays_crawl_for_both_option_values() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for enabled in [false, true] {
            for direction in [-1.0, 1.0] {
                let mut game = room(rules, Some(enabled));
                let incoming = warm_walk(&mut game, direction, false);
                game.step(&movement(direction, true, false));
                near(game.player.vel.x, (incoming + direction * 3.0) * 0.2);
                assert!(game.player.vel.x.abs() < 2.0);
            }
        }
    }
}

#[test]
fn both_directions_choose_source_left_duck_run_branch_without_horizontal_acceleration() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1.0, 1.0] {
            let mut game = room(rules, None);
            let incoming = warm_walk(&mut game, direction, true);
            game.step(&InputFrame {
                left: true,
                right: true,
                down: true,
                ..Default::default()
            });
            near(game.player.vel.x, (incoming - 0.1) * 0.98);
            assert_eq!(game.player.state, PlayerState::Ducking);
        }
    }
}

#[test]
fn down_with_no_horizontal_key_keeps_source_fallback_even_when_enabled() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1.0, 1.0] {
            let mut game = room(rules, None);
            let incoming = warm_walk(&mut game, direction, true);
            game.step(&movement(0.0, true, false));
            let damped = incoming * 0.8;
            near(
                game.player.vel.x,
                if damped < 0.5 { 0.0 } else { damped * 0.2 },
            );
        }
    }
}

#[test]
fn shift_run_remains_available_with_down_run_disabled() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1.0, 1.0] {
            let mut game = room(rules, Some(false));
            let incoming = warm_walk(&mut game, direction, true);
            game.step(&movement(direction, true, true));
            near(
                game.player.vel.x,
                ((incoming + direction * 0.1 + direction * 3.0) * 0.98).clamp(-6.0, 6.0),
            );
            assert!(game.player.vel.x.abs() > 3.0);
        }
    }
}

#[test]
fn exact_two_initial_speed_uses_fast_duck_arm_and_small_speed_uses_crawl() {
    // Controlled source-valid initial platform state, not a natural route.
    // Every assignment precedes the first Game.step.
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1.0, 1.0] {
            for magnitude in [1.999, 2.0] {
                let mut game = room(rules, None);
                game.player.state = PlayerState::Running;
                game.player.grounded = true;
                game.player.vel = Vec2::new(direction * magnitude, 0.0);
                game.step(&movement(direction, true, false));
                let expected = if magnitude < 2.0 {
                    direction * magnitude * 0.2
                } else {
                    (direction * magnitude + direction * 0.1) * 0.98
                };
                near(game.player.vel.x, expected);
                assert_eq!(game.player.state, PlayerState::Ducking);
            }
        }
    }
}

#[test]
fn initial_fast_duck_preserves_prior_animation_rate_below_four_but_slow_arm_sets_point_eight() {
    // Controlled initial speed and animation, with no after-start edits.
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1.0, 1.0] {
            for magnitude in [1.999, 2.0] {
                let mut game = room(rules, None);
                game.player.state = PlayerState::Running;
                game.player.grounded = true;
                game.player.vel = Vec2::new(direction * magnitude, 0.0);
                game.player.set_image_speed(0.35);
                game.step(&movement(direction, true, false));
                assert_eq!(game.player.state, PlayerState::Ducking);
                assert!(game.player.vel.x.abs() < 4.0);
                near(
                    game.player.image_speed(),
                    if magnitude < 2.0 { 0.8 } else { 0.35 },
                );
            }
        }
    }
}

#[test]
fn movement_setting_defaults_true_and_false_survives_public_room_transitions() {
    let mut old = serde_json::to_value(Game::shell(42, RulesPreset::Classic)).unwrap();
    old.as_object_mut().unwrap().remove("down_to_run");
    let restored: Game = serde_json::from_value(old).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap()["down_to_run"], true);
    let mut game = room(RulesPreset::Classic, Some(false));
    assert_eq!(serde_json::to_value(&game).unwrap()["down_to_run"], false);
    game.load_hub(hub::HubKind::Title);
    assert_eq!(serde_json::to_value(&game).unwrap()["down_to_run"], false);
    game.load_custom(&level::Level::default(), "other", "");
    assert_eq!(serde_json::to_value(&game).unwrap()["down_to_run"], false);
}
