//! Explicitly configured Animation End/automatic-collision phase controls.
//! These establish callback semantics, not an ordinary timed Yeti encounter.
use super::*;
use crate::characters::{self, Character};

fn pending(rules: RulesPreset, character: Character, facing: i8) -> Game {
    let mut game = Game::shell(42, rules);
    characters::apply(&mut game, character);
    game.player.pos = Vec2::new(239.0, 312.0);
    game.player.previous_pos = game.player.pos;
    game.player.facing = facing;
    game.player.state = PlayerState::Ducking;
    game.player.data.insert("duck_to_hang".into(), 1.0);
    game.player.data.insert("xAcc".into(), 3.0);
    game.player.data.insert("yAcc".into(), -4.0);
    game.player.set_sprite(match character {
        Character::Explorer => "sDuckToHangL",
        Character::Damsel => "sDamselDtHL",
        Character::TunnelMan => "sTunnelDtHL",
    });
    game.player.set_image_speed(0.8);
    game
}

#[test]
fn configured_animation_end_clears_both_velocities_and_source_accelerations() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for facing in [-1, 1] {
                for ladder in [false, true] {
                    let mut game = pending(rules, character, facing);
                    game.player.vel = Vec2::new(4.0, -2.0);
                    if ladder {
                        game.spawn(
                            "oLadderOrange",
                            Vec2::new(231.0 + f64::from(facing) * 8.0, 320.0),
                        );
                    }
                    let sprite = game.player.sprite.clone();
                    animation_end(&mut game);
                    assert_eq!(
                        game.player.vel,
                        Vec2::default(),
                        "SOURCE_DTH_CALLBACK_MOTION"
                    );
                    assert_eq!(game.player.data.get("xAcc"), Some(&0.0));
                    assert_eq!(game.player.data.get("yAcc"), Some(&0.0));
                    assert_eq!(
                        game.player.state,
                        if ladder {
                            PlayerState::Climbing
                        } else {
                            PlayerState::Hanging
                        }
                    );
                    assert_eq!(
                        game.player.pos,
                        Vec2::new(
                            if ladder {
                                239.0 + f64::from(facing) * 8.0
                            } else if facing < 0 {
                                234.0
                            } else {
                                245.0
                            },
                            328.0,
                        )
                    );
                    assert_eq!(game.player.previous_pos, game.player.pos);
                    assert_eq!(game.player.facing, if ladder { facing } else { -facing });
                    assert_eq!(game.player.sprite, sprite);
                    assert_eq!(game.player.data.get("duck_to_hang"), Some(&0.0));
                    assert_eq!(game.player.data.get("gravity"), Some(&0.0));
                    assert!(!game.player.is_stunned());
                }
            }
        }
    }
}

#[test]
fn configured_terminal_yeti_throw_keeps_zero_timer_stun_after_platform_state_changes() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            let mut game = pending(rules, character, 1);
            for x in 0..game.world.width {
                game.world.set_tile(x, 22, "oBlock");
            }
            let yeti = game.spawn("oYeti", Vec2::new(231.0, 304.0));
            let frames = crate::catalog::sprite_frame_count(&game.player.sprite);
            assert_eq!(
                frames,
                if character == Character::TunnelMan {
                    10
                } else {
                    9
                },
                "original DT H sprite frame counts"
            );
            // Initial configured last-frame phase; no state/frame writes occur
            // after this setup. The normal Step must perform the real collision.
            game.player.animation = frames as f64 - 0.4;
            let health = game.player.health;
            let sprite = game.player.sprite.clone();
            assert!(!game.player.is_stunned());
            assert_eq!(game.player.stun, 0);
            game.step(&InputFrame::default());
            assert_eq!(
                game.world.entity(yeti).unwrap().state,
                3,
                "actual Yeti entered THROW"
            );
            let draw = &game.presentation().player;
            assert_eq!(
                draw.sprite, sprite,
                "automatic throw leaves DT H sprite for Animation End"
            );
            assert_eq!(draw.state, PlayerState::Stunned);
            assert_eq!(draw.stun, 0, "Yeti assigns no stun timer");
            assert_eq!(draw.vel.y, -6.0);
            assert_eq!(draw.vel.x.abs(), 6.0);
            let throwing_yeti = game.world.entity(yeti).unwrap();
            assert_eq!(
                draw.pos,
                Vec2::new(
                    throwing_yeti.pos.x + if throwing_yeti.facing > 0 { 0.0 } else { 16.0 },
                    throwing_yeti.pos.y,
                ),
                "source Yeti THROW first relocates the Player to its own origin"
            );
            let expected_pos = Vec2::new(
                draw.pos.x + 6.0,
                ((draw.pos.y + 16.0) / 8.0).round_ties_even() * 8.0,
            );
            assert!(
                source_hanging(&game),
                "Source Animation End assigns HANGING independently of stunned"
            );
            assert_eq!(game.player.pos, expected_pos);
            assert_eq!(game.player.previous_pos, expected_pos);
            assert_eq!(game.player.facing, -1);
            assert_eq!(
                game.player.vel,
                Vec2::default(),
                "SOURCE_TERMINAL_DTH_ZERO_RECOIL"
            );
            assert_eq!(game.player.data.get("xAcc"), Some(&0.0));
            assert_eq!(game.player.data.get("yAcc"), Some(&0.0));
            assert_eq!(game.player.stun, 0);
            assert!(
                game.player.is_stunned(),
                "SOURCE_TERMINAL_DTH_INDEPENDENT_STUN"
            );
            assert_eq!(game.player.health, health);
            assert_eq!(game.player.data.get("duck_to_hang"), Some(&0.0));

            game.step(&InputFrame::default());
            assert!(
                game.player.is_stunned(),
                "following Player Step owns stun motion"
            );
            assert_eq!(game.player.state, PlayerState::Stunned);
            assert_eq!(
                game.player.sprite,
                match character {
                    Character::Explorer => "sStunL",
                    Character::Damsel => "sDamselStunL",
                    Character::TunnelMan => "sTunnelStunL",
                }
            );
            game.step(&InputFrame::default());
            assert!(
                !game.player.is_stunned(),
                "stationary stun sprite clears zero timer flag"
            );
            assert_eq!(game.player.health, health);
            assert_eq!(game.state, RunState::Playing);
        }
    }
}
