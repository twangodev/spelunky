//! Prior controller states advance during attacks even when characterSprite skips art.
use spelunky_core::*;

fn scene(rules: RulesPreset, character: characters::Character, edge: bool) -> Game {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..40 {
        game.world.set_tile(x, 20, "oHardBlock");
        if !edge || (4..=7).contains(&x) {
            game.world.set_tile(x, 10, "oHardBlock");
        }
    }
    characters::apply(&mut game, character);
    game.player.pos = Vec2::new(125.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game
}

fn characters() -> [characters::Character; 3] {
    [
        characters::Character::Explorer,
        characters::Character::Damsel,
        characters::Character::TunnelMan,
    ]
}

fn fall_art(character: characters::Character) -> &'static str {
    match character {
        characters::Character::Explorer => "sFallLeft",
        characters::Character::Damsel => "sDamselFallL",
        characters::Character::TunnelMan => "sTunnelFallL",
    }
}

fn standing_art(character: characters::Character) -> &'static str {
    match character {
        characters::Character::Explorer => "sStandLeft",
        characters::Character::Damsel => "sDamselLeft",
        characters::Character::TunnelMan => "sTunnelLeft",
    }
}

fn healthy(game: &Game, health: i32) {
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, health);
    assert!(!game.player.is_stunned());
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
}

#[test]
fn falling_attack_records_prior_states_before_its_first_normal_controller() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in characters() {
            let mut game = scene(rules, character, true);
            let health = game.player.health;
            let mut falling_attack_steps = 0;
            let mut ended = false;
            for frame in 0..32 {
                let was_attacking = game.player.attack_timer > 0;
                game.step(&InputFrame {
                    right: true,
                    run: true,
                    attack: frame == 0,
                    attack_pressed: frame == 0,
                    attack_released: frame == 1,
                    ..Default::default()
                });
                healthy(&game, health);
                if frame == 0 {
                    assert!(game.player.attack_timer > 0, "natural attack did not start");
                }
                if game.player.attack_timer > 0 && game.player.state == PlayerState::Falling {
                    falling_attack_steps += 1;
                }
                if was_attacking && game.player.attack_timer == 0 {
                    assert!(
                        falling_attack_steps >= 3,
                        "FIXTURE_TIMING: attack ended too early"
                    );
                    assert_eq!(game.player.state, PlayerState::Falling);
                    assert!(!game.player.grounded);
                    ended = true;
                    break;
                }
            }
            assert!(ended, "FIXTURE_TIMING: no natural attack Animation End");
            // This is the first controller after source Animation End cleared
            // whipping. Its preceding two controller states were FALLING even
            // though those Steps kept the attack sprite.
            game.step(&InputFrame {
                right: true,
                run: true,
                ..Default::default()
            });
            healthy(&game, health);
            assert_eq!(game.player.state, PlayerState::Falling);
            assert!(
                !game.player.grounded,
                "FIXTURE_TIMING: landed before the art check"
            );
            assert_eq!(game.player.sprite, fall_art(character));
        }
    }
}

#[test]
fn ordinary_departure_keeps_the_source_three_falling_controller_delay() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in characters() {
            let mut game = scene(rules, character, true);
            let health = game.player.health;
            let mut consecutive_falling = 0;
            let mut checked = false;
            for _ in 0..24 {
                game.step(&InputFrame {
                    right: true,
                    run: true,
                    ..Default::default()
                });
                healthy(&game, health);
                assert_eq!(game.player.attack_timer, 0);
                if game.player.state == PlayerState::Falling {
                    consecutive_falling += 1;
                    if consecutive_falling < 3 {
                        assert_ne!(game.player.sprite, fall_art(character));
                    } else {
                        assert_eq!(game.player.sprite, fall_art(character));
                        checked = true;
                        break;
                    }
                } else {
                    consecutive_falling = 0;
                }
            }
            assert!(checked, "FIXTURE_TIMING: no ordinary three-Step fall");
        }
    }
}

#[test]
fn supported_attack_returns_to_standing_without_inventing_falling_history() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in characters() {
            let mut game = scene(rules, character, false);
            let health = game.player.health;
            let mut ended = false;
            for frame in 0..32 {
                let was_attacking = game.player.attack_timer > 0;
                game.step(&InputFrame {
                    attack: frame == 0,
                    attack_pressed: frame == 0,
                    attack_released: frame == 1,
                    ..Default::default()
                });
                healthy(&game, health);
                assert!(game.player.grounded);
                assert_eq!(game.player.pos, Vec2::new(125.0, 152.0));
                if frame == 0 {
                    assert!(game.player.attack_timer > 0);
                }
                if was_attacking && game.player.attack_timer == 0 {
                    ended = true;
                    break;
                }
            }
            assert!(ended, "FIXTURE_TIMING: supported attack did not finish");
            game.step(&InputFrame::default());
            healthy(&game, health);
            assert_eq!(game.player.attack_timer, 0);
            assert!(game.player.grounded);
            assert_eq!(game.player.state, PlayerState::Standing);
            assert_eq!(game.player.sprite, standing_art(character));
        }
    }
}
