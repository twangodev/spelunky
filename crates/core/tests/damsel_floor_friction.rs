//! Source-derived Damsel floor friction after ordinary pickup and throw.
//! All actors and terrain are authored before simulation. Thereafter only
//! ordinary InputFrame actions are supplied; neither actor nor RNG is edited.
use spelunky_core::{Game, InputFrame, RngAlgorithm, RulesPreset, Vec2, level::Level};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
const ALGORITHMS: [RngAlgorithm; 2] = [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1];

fn throw_from_floor(rules: RulesPreset, algorithm: RngAlgorithm, ledge: bool) -> (Game, u64) {
    let mut level = Level::default();
    level.set_tile(8, 8, '@').unwrap();
    level.set_tile(34, 8, 'X').unwrap();
    // World floor is y160. The control ends at x160, supporting the player
    // at x152 while leaving the rightward flight and landing point clear.
    for x in 0..if ledge {
        9
    } else {
        spelunky_core::level::LEVEL_WIDTH
    } {
        level.set_tile(x, 9, '1').unwrap();
    }
    let mut game = Game::shell_with_algorithm(23, rules, algorithm).unwrap();
    game.load_custom(&level, "damsel-floor-friction", "");
    assert_eq!(game.player.pos, Vec2::new(152.0, 152.0));
    let id = game.spawn("oDamsel", game.player.pos);
    // Let the initial Falling player land through the ordinary controller.
    for _ in 0..2 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    assert_eq!(game.world.entity(id).unwrap().state, 0);
    game.step(&InputFrame {
        down: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(id), "ordinary down+attack pickup");
    assert!(game.world.entity(id).unwrap().held);
    game.step(&InputFrame::default());
    game.step(&InputFrame {
        attack_pressed: true,
        ..Default::default()
    });
    let actor = game.world.entity(id).unwrap();
    assert!(game.player.held.is_none());
    assert!(!actor.held);
    assert_eq!((actor.health, actor.state), (4, 2));
    assert_eq!(actor.vel.x, 4.0, "ordinary heavy throw establishes vx4");
    assert!(actor.vel.y < 0.0, "throw starts airborne");
    assert_eq!(actor.value("colBot"), 0.0);
    (game, id)
}

#[test]
fn naturally_thrown_damsel_first_floor_bounce_applies_inherited_friction_once() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            let (mut game, id) = throw_from_floor(rules, algorithm, false);
            let mut contacted = false;
            for _ in 0..32 {
                game.step(&InputFrame::default());
                let actor = game.world.entity(id).expect("healthy throw survives");
                assert_eq!((actor.health, actor.state), (4, 2));
                assert_eq!(
                    (actor.value("colLeft"), actor.value("colRight")),
                    (0.0, 0.0)
                );
                if actor.value("colBot") == 0.0 {
                    continue;
                }
                assert_eq!(actor.pos.y, 152.0, "first contact is authored floor y160");
                assert!(
                    actor.vel.y < -1.0,
                    "descending flight produced a real rebound"
                );
                assert!(
                    (actor.vel.x - 1.2).abs() < 1e-12,
                    "source inherited floor friction is 4 * .3, actual {}",
                    actor.vel.x
                );
                contacted = true;
                break;
            }
            assert!(
                contacted,
                "ordinary thrown trajectory must reach its first floor bounce"
            );
        }
    }
}

#[test]
fn naturally_thrown_damsel_over_floor_edge_retains_airborne_horizontal_speed() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            let (mut game, id) = throw_from_floor(rules, algorithm, true);
            let launch_x = game.world.entity(id).unwrap().pos.x;
            for _ in 0..12 {
                game.step(&InputFrame::default());
                let actor = game.world.entity(id).expect("airborne control survives");
                assert_eq!((actor.health, actor.state), (4, 2));
                assert_eq!(actor.value("colBot"), 0.0, "no inherited floor friction");
                assert_eq!(
                    (actor.value("colLeft"), actor.value("colRight")),
                    (0.0, 0.0)
                );
                assert_eq!(actor.vel.x, 4.0, "airborne child must not damp the throw");
            }
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.pos.x, launch_x + 48.0);
            assert!(
                actor.pos.x - 4.0 > 160.0,
                "full body is beyond the floor edge"
            );
            assert!(
                actor.pos.y > 152.0,
                "control fell through the absent landing floor"
            );
        }
    }
}
