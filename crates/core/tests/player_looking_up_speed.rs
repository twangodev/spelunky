//! SOURCE-ONLY PROPOSAL: neither compiled nor executed.
//! Reach stationary LookingUp through an authored room and ordinary input.
//! No actor position/velocity/state/image-speed/index injection after loading.
use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, characters, level::Level};

fn floor_room(rules: RulesPreset, character: characters::Character) -> Game {
    let mut level = Level::default();
    level.set_tile(4, 8, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 10, '1').unwrap();
    }
    let mut game = Game::legacy_shell(42, rules);
    game.load_custom(&level, "looking-up-speed", "");
    characters::apply(&mut game, character);
    for _ in 0..16 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded, "fixture did not settle on its floor");
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.player.vel.x, 0.0);
    assert_eq!(game.player.attack_timer, 0);
    assert!(!game.player.is_stunned());
    game
}

fn look_art(character: characters::Character) -> &'static str {
    match character {
        characters::Character::Explorer => "sLookLeft",
        characters::Character::Damsel => "sDamselLookL",
        characters::Character::TunnelMan => "sTunnelLookL",
    }
}

fn assert_stationary_look(game: &Game, character: characters::Character) {
    assert_eq!(game.player.state, PlayerState::LookingUp);
    assert_eq!(game.player.vel.x, 0.0);
    assert!(game.player.grounded);
    assert_eq!(game.player.sprite, look_art(character));
    assert!((game.player.image_speed() - 0.1).abs() < 1e-12);
    let drawn = game.presentation();
    assert_eq!(drawn.player.state, PlayerState::LookingUp);
    assert_eq!(drawn.player.sprite, look_art(character));
    assert!((drawn.player.image_speed() - 0.1).abs() < 1e-12);
}

#[test]
fn supported_up_sets_source_speed_instead_of_inheriting_standing_speed() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            let mut game = floor_room(rules, character);
            assert!(
                (game.player.image_speed() - 0.1).abs() > 1e-12,
                "fixture must begin with a distinct inherited speed"
            );
            let position = game.player.pos;
            for _ in 0..6 {
                game.step(&InputFrame {
                    up: true,
                    ..Default::default()
                });
                assert_stationary_look(&game, character);
                assert_eq!(game.player.pos, position);
            }
            game.step(&InputFrame::default());
            assert_eq!(game.player.state, PlayerState::Standing);
            assert!(game.player.grounded);
        }
    }
}

#[test]
fn natural_running_release_then_up_sets_source_speed_when_motion_stops() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            let mut game = floor_room(rules, character);
            for _ in 0..12 {
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            assert_eq!(game.player.state, PlayerState::Running);
            assert!(game.player.vel.x > 0.0);
            assert!(game.player.image_speed() > 0.1);
            let mut stopped = false;
            for tick in 0..64 {
                game.step(&InputFrame {
                    up: true,
                    right_released: tick == 0,
                    ..Default::default()
                });
                if game.player.state == PlayerState::LookingUp {
                    assert_stationary_look(&game, character);
                    stopped = true;
                    break;
                }
            }
            assert!(stopped, "fixture never reached stationary LookingUp");
        }
    }
}
