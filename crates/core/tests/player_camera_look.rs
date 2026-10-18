//! Configured authored rooms; only ordinary inputs after initial construction.
//! Source Player Step399-412 samples cached keys and pans after 31 quiet ticks.
use spelunky_core::*;

fn count(g: &Game) -> f64 {
    g.player.data.get("view_count").copied().unwrap_or(0.0)
}
fn floor(rules: RulesPreset, character: characters::Character, door: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(4, 20, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 21, '1').unwrap();
    }
    let mut g = Game::shell(42, rules);
    g.load_custom(&level, "player-look-pan", "");
    characters::apply(&mut g, character);
    if door {
        g.spawn("oExit", Vec2::new(g.player.pos.x - 8., g.player.pos.y - 8.));
    }
    for _ in 0..4 {
        g.step(&InputFrame::default());
    }
    assert_eq!(g.player.pos.y, 344.);
    assert!(g.player.grounded);
    assert_eq!(g.camera.y, 200.);
    assert_eq!(count(&g), 0.);
    g
}
fn down() -> InputFrame {
    InputFrame {
        down: true,
        ..Default::default()
    }
}
fn up() -> InputFrame {
    InputFrame {
        up: true,
        ..Default::default()
    }
}
#[test]
fn ordinary_down_and_up_pan_on_the_thirty_second_tick_and_horizontal_input_resets() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            let mut g = floor(rules, character, false);
            for _ in 0..31 {
                g.step(&down());
                assert_eq!(g.camera.y, 200., "SOURCE_THIRTY_ONE_QUIET_DOWN_TICKS");
            }
            g.step(&down());
            assert_eq!(g.camera.y, 204., "SOURCE_DOWN_PAN_ON_TICK_THIRTY_TWO");
            assert_eq!(count(&g), 31.);
            for _ in 0..11 {
                g.step(&down());
            }
            assert_eq!(
                g.camera.y, 248.,
                "source follow caps the downward pan at Player.y-96"
            );
            g.step(&InputFrame::default());
            assert_eq!(count(&g), 0., "release resets the delay");
            for _ in 0..31 {
                g.step(&up());
                assert_eq!(g.camera.y, 248.);
            }
            g.step(&up());
            assert_eq!(g.camera.y, 244., "SOURCE_UP_PAN_ON_TICK_THIRTY_TWO");
            g.step(&InputFrame {
                up: true,
                right: true,
                ..Default::default()
            });
            assert_eq!(
                count(&g),
                0.,
                "direction key excludes the look branch even before acceleration"
            );
        }
    }
}
#[test]
fn ordinary_exit_keeps_cached_up_for_the_skipped_inactive_controller() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            let mut g = floor(rules, character, true);
            for _ in 0..43 {
                g.step(&down());
            }
            assert_eq!(count(&g), 31.);
            assert_eq!(g.camera.y, 248.);
            g.step(&up());
            assert!(
                g.player.is_exiting(),
                "ordinary Up must enter the authored exit"
            );
            assert_eq!(g.player.data.get("active"), Some(&0.));
            assert_eq!(g.camera.y, 244.);
            g.step(&InputFrame::default());
            assert!(g.player.is_exiting());
            assert_eq!(
                count(&g),
                31.,
                "source has no exit exclusion or new key sampling"
            );
            assert_eq!(g.camera.y, 240., "SOURCE_INACTIVE_EXIT_USES_CACHED_UP");
        }
    }
}
#[test]
fn real_trap_stun_skips_look_until_the_normal_controller_recovers() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
        ] {
            let mut level = level::Level::default();
            level.set_tile(10, 8, '@').unwrap();
            for x in 0..40 {
                level.set_tile(x, 9, '1').unwrap();
            }
            for x in 11..=13 {
                for y in 7..=8 {
                    level.set_tile(x, y, '1').unwrap();
                }
            }
            level.set_tile(6, 7, '>').unwrap();
            let mut g = Game::shell(42, rules);
            g.load_custom(&level, "look-stun-recovery", "");
            characters::apply(&mut g, character);
            for _ in 0..4 {
                g.step(&InputFrame::default());
            }
            assert!(g.player.grounded);
            let mut hanging = false;
            let mut arrow = false;
            for tick in 0..60 {
                let was_hanging = g.player.state == PlayerState::Hanging;
                g.step(&InputFrame {
                    right: true,
                    jump: true,
                    jump_pressed: tick == 0,
                    ..Default::default()
                });
                arrow |= g.world.entities.iter().any(|e| e.object == "oArrow");
                if g.player.health < 4.0 {
                    assert!(
                        hanging && was_hanging && arrow,
                        "real source trap must interrupt a natural hang"
                    );
                    break;
                }
                hanging |= g.player.state == PlayerState::Hanging;
            }
            assert_eq!(g.player.health, 2.0);
            assert!(g.player.is_stunned());
            assert_eq!(count(&g), 0.);
            let mut recovered = false;
            for _ in 0..240 {
                g.step(&down());
                assert_eq!(g.player.health, 2.0);
                if g.player.is_stunned() {
                    assert_eq!(
                        count(&g),
                        0.,
                        "stun branch does not enter look or sample held Down"
                    );
                } else if g.player.grounded && count(&g) > 0. {
                    recovered = true;
                    break;
                }
            }
            assert!(
                recovered,
                "normal grounded controller must eventually sample held Down"
            );
            assert_eq!(count(&g), 1., "SOURCE_LOOK_STARTS_AFTER_REAL_STUN_RECOVERY");
        }
    }
}
