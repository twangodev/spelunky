//! Source-grounded ordinary trap recovery in an authored tunnel.
use spelunky_core::*;

fn description(game: &Game) -> String {
    format!(
        "tick={} pos={:?} vel={:?} hp={} state={:?} sprite={} stun={} data={:?}",
        game.tick,
        game.player.pos,
        game.player.vel,
        game.player.health,
        game.player.state,
        game.player.sprite,
        game.player.stun,
        game.player.data
    )
}

fn recovery(rules: RulesPreset, historical: bool, keep_down: bool) {
    let mut game = if historical {
        Game::legacy_shell(42, rules)
    } else {
        Game::shell(42, rules)
    };
    game.world = World::new(20, 15, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..20 {
        game.world.set_tile(x, 8, "oHardBlock");
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.world.set_tile(7, 9, "oHardBlock");
    let trap = game.spawn("oArrowTrapRight", Vec2::new(48., 144.));
    game.player.pos = Vec2::new(104., 152.);
    game.player.previous_pos = game.player.pos;
    let held = InputFrame {
        right: true,
        down: true,
        ..Default::default()
    };
    let mut saw_arrow = false;
    let mut hit = false;
    for _ in 0..120 {
        let incoming = game.player.clone();
        game.step(&held);
        saw_arrow |= game.world.entities.iter().any(|e| e.object == "oArrow");
        assert_eq!(game.state, RunState::Playing, "{}", description(&game));
        if game.player.health < 4.0 {
            assert_eq!(game.player.health, 2.0, "actual trap Arrow damage");
            assert!(
                saw_arrow,
                "observed original Arrow birth before the hurt branch"
            );
            assert_eq!(
                incoming.state,
                PlayerState::Ducking,
                "real Arrow interrupts ordinary Ducking"
            );
            assert_eq!(incoming.data.get("duck_to_hang").copied().unwrap_or(0.), 0.);
            assert!(game.player.is_stunned());
            assert_eq!(game.player.stun, 20);
            assert!(game.world.entity(trap).is_some());
            hit = true;
            break;
        }
    }
    assert!(
        hit,
        "finite ordinary Down+Right route must trigger its actual trap"
    );
    let mut recovered = false;
    for _ in 0..240 {
        if game.player.stun == 1 && game.player.sprite == "sStunL" {
            assert_eq!(game.player.health, 2.0);
            assert!(game.player.is_stunned());
            assert_eq!(
                game.player.vel,
                Vec2::default(),
                "stationary real physical stun: {}",
                description(&game)
            );
            assert_eq!(game.player.pos.y, 152.);
            assert_eq!(game.player.data.get("source_platform_grounded"), Some(&1.));
            assert_eq!(game.player.data.get("source_k_down"), Some(&1.));
            assert_eq!(game.player.data.get("previous_right"), Some(&1.));
            assert_eq!(game.data.get("input_frame_right"), Some(&1.));
            let bounds = game.player.bounds();
            assert!(
                game.world.solid_line_except(
                    Vec2::new(bounds.x, bounds.y - 1.),
                    Vec2::new(bounds.x + bounds.w - 1., bounds.y - 1.),
                    0
                ),
                "fresh real Source Top1 blocks the tunnel ceiling"
            );
            assert!(
                game.world.solid_line_except(
                    Vec2::new(bounds.x, bounds.y + bounds.h),
                    Vec2::new(bounds.x + bounds.w - 1., bounds.y + bounds.h),
                    0
                ),
                "fresh real Source Bottom1 prevents the earlier walk-off branch from erasing Ducking"
            );
            game.step(&InputFrame {
                right_released: true,
                down: keep_down,
                ..Default::default()
            });
            assert_eq!(game.player.health, 2.0);
            assert!(!game.player.is_stunned());
            assert_eq!(game.player.stun, 0);
            assert_eq!(
                game.player.data.get("source_k_down"),
                Some(&f64::from(keep_down))
            );
            assert_eq!(game.player.data.get("previous_right"), Some(&0.));
            let expected = if keep_down {
                0.1
            } else if historical {
                0.3
            } else {
                0.
            };
            assert_eq!(
                game.player.vel.x,
                expected,
                "SOURCE_DUCK_RELEASE_AFTER_REAL_STUN_CLEARS_RELEASE_ACCEL rules={rules:?} historical={historical} keep_down={keep_down}: {}",
                description(&game)
            );
            assert_eq!(
                game.player.state,
                if keep_down {
                    PlayerState::Ducking
                } else if historical {
                    PlayerState::Running
                } else {
                    PlayerState::Standing
                }
            );
            recovered = true;
            break;
        }
        game.step(&held);
        assert_eq!(
            game.player.health,
            2.0,
            "healthy real physical stun: {}",
            description(&game)
        );
    }
    assert!(
        recovered,
        "real stationary stun reaches its last timer before ordinary release"
    );
}

#[test]
fn natural_ducked_arrow_stun_keeps_the_source_release_reset_on_recovery() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        recovery(rules, false, false);
    }
}
#[test]
fn natural_ducked_arrow_stun_real_right_release_still_accelerates_when_down_stays_held() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        recovery(rules, false, true);
    }
}
#[test]
fn historical_ducked_arrow_stun_retains_the_prior_standing_recovery_impulse() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        recovery(rules, true, false);
    }
}
