//! Ordinary crawl entry completes the source controller tail before animation.
use spelunky_core::*;

fn room(rules: RulesPreset, character: characters::Character, direction: i8) -> Game {
    let mut level = level::Level::default();
    level.set_tile(13, 18, '@').unwrap();
    let floor = if direction < 0 { 13..=23 } else { 3..=13 };
    for x in floor {
        level.set_tile(x, 19, '1').unwrap();
    }
    for x in 0..40 {
        level.set_tile(x, 22, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "ledge-controller-tail", "");
    characters::apply(&mut game, character);
    game
}

fn entered(game: &Game) -> bool {
    game.player.data.get("duck_to_hang") == Some(&1.0)
}

fn entry(rules: RulesPreset, character: characters::Character, direction: i8) -> (Game, f64) {
    let mut game = room(rules, character, direction);
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded, "initial authored support");
    let health = game.player.health;
    let key = if direction < 0 {
        "left_steps"
    } else {
        "right_steps"
    };
    for _ in 0..80 {
        let previous_steps = game.player.data.get(key).copied().unwrap_or(0.0);
        let position = game.player.pos;
        let rng = game.rng.clone().next_u64();
        game.step(&InputFrame {
            down: true,
            left: direction < 0,
            right: direction > 0,
            ..Default::default()
        });
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.player.health, health);
        assert!(!game.player.is_stunned());
        if entered(&game) {
            assert!(previous_steps > 2.0, "real crawl precedes ledge entry");
            assert_eq!(game.player.pos, position, "source xPrev/yPrev hold");
            assert_eq!(game.player.vel, Vec2::default());
            assert_eq!(game.player.facing, direction);
            assert!(matches!(
                game.player.sprite.as_str(),
                "sDuckToHangL" | "sDamselDtHL" | "sTunnelDtHL"
            ));
            assert_eq!(game.player.image_speed(), 0.8);
            assert_eq!(game.rng.clone().next_u64(), rng);
            return (game, previous_steps);
        }
    }
    panic!("ordinary crawl did not enter ledge animation");
}

fn cases(mut check: impl FnMut(Game, f64, i8)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1, 1] {
            for character in [
                characters::Character::Explorer,
                characters::Character::Damsel,
                characters::Character::TunnelMan,
            ] {
                let (game, previous_steps) = entry(rules, character, direction);
                check(game, previous_steps, direction);
            }
        }
    }
}

#[test]
fn ordinary_ledge_entry_persists_direction_counter_before_animation_gate() {
    cases(|game, previous_steps, direction| {
        let (held, released) = if direction < 0 {
            ("left_steps", "right_steps")
        } else {
            ("right_steps", "left_steps")
        };
        assert_eq!(
            game.player.data.get(held),
            Some(&(previous_steps + 1.0)),
            "entry controller counter must persist"
        );
        assert_eq!(game.player.data.get(released), Some(&0.0));
        assert_eq!(game.player.data.get("source_platform_grounded"), Some(&0.0));
        assert_eq!(game.player.data.get("source_platform_climbing"), Some(&0.0));
        assert_eq!(game.player.data.get("fall_state_ticks"), Some(&0.0));
    });
}

#[test]
fn ordinary_ledge_entry_persists_zero_gravity_and_collision_tail() {
    cases(|game, _, _| {
        assert_eq!(
            game.player.data.get("gravity"),
            Some(&0.0),
            "entry DUCKTOHANG zero gravity must persist"
        );
        assert!(!game.player.grounded);
        assert_eq!(game.player.data.get("collision_top"), Some(&-8.0));
        assert_eq!(game.player.data.get("collision_half_width"), Some(&5.0));
    });
}

#[test]
fn ordinary_ledge_animation_skips_later_controllers_then_allows_hang_jump() {
    cases(|mut game, _, direction| {
        let health = game.player.health;
        let key = if direction < 0 {
            "left_steps"
        } else {
            "right_steps"
        };
        let count = game.player.data.get(key).copied();
        let position = game.player.pos;
        let mut completed = false;
        for _ in 0..16 {
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, health);
            assert!(!game.player.is_stunned());
            assert_eq!(
                game.player.data.get(key).copied(),
                count,
                "already-DUCKTOHANG excludes controller"
            );
            if entered(&game) {
                assert_eq!(game.player.pos, position);
                assert_eq!(game.player.vel, Vec2::default());
            } else {
                assert_eq!(game.player.state, PlayerState::Hanging);
                assert_eq!(game.player.facing, -direction);
                completed = true;
                break;
            }
        }
        assert!(completed, "natural Animation End must reach hanging");
        game.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        // The two-pixel jump shift detaches this floor edge. The source's
        // independent support check then selects FALLING and adds -1 to -4.
        assert_eq!(game.player.state, PlayerState::Falling);
        assert_eq!(game.player.vel.y, -5.0);
        assert_eq!(game.player.data.get("hang_count"), Some(&4.0));
        assert_eq!(game.player.health, health);
        assert_eq!(game.state, RunState::Playing);
        game.step(&InputFrame {
            jump: true,
            ..Default::default()
        });
        assert_eq!(game.player.state, PlayerState::Jumping);
        assert!(game.player.vel.y < 0.0);
        assert_eq!(game.player.health, health);
        assert_eq!(game.state, RunState::Playing);
    });
}
