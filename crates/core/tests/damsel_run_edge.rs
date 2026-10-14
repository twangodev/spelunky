//! Source-derived configured-state probe.
//! RUN/facing are authored initially; no claim of natural pickup/throw coverage.
//! All subsequent behavior uses one neutral actual Game.step.
use spelunky_core::*;

#[test]
fn configured_damsel_run_turns_at_literal_custom_edge_not_origin_offset() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            for (facing, x, wall_col, source_hit) in [
                (1, 152.0, 10, false),
                (1, 155.0, 10, true),
                (-1, 167.0, 9, false),
                (-1, 165.0, 9, true),
            ] {
                let mut g = Game::shell_with_algorithm(7, rules, algorithm).unwrap();
                g.world = World::new(30, 20, Area::Mines, 1);
                for col in 0..g.world.width {
                    g.world.set_tile(col, 10, "oBlock");
                }
                g.world.set_tile(wall_col, 9, "oBlock");
                g.player.pos = Vec2::new(64.0, 152.0);
                g.player.previous_pos = g.player.pos;
                g.player.state = PlayerState::Standing;
                g.set_simulation_view_extent(480.0, 320.0).unwrap();
                let id = g.spawn("oDamsel", Vec2::new(x, 152.0));
                let e = g.world.entity_mut(id).unwrap();
                assert_eq!(e.health, 4);
                assert!(!e.held);
                e.state = 1; // authored source status RUN, not a natural state claim
                e.facing = facing;
                e.data.insert("cost".into(), 0.0);
                let b = e.bounds();
                assert_eq!((b.x, b.y, b.w, b.h), (x - 4.0, 148.0, 8.0, 12.0));
                let edge = if facing > 0 {
                    x + 4.0 + 2.0 - 1.0
                } else {
                    x - 4.0 - 2.0
                };
                // Literal isCollisionRight/Left(2): full custom-height precise line.
                assert_eq!(
                    g.world.solid_line_except(
                        Vec2::new(edge.round_ties_even(), 148.0),
                        Vec2::new(edge.round_ties_even(), 159.0),
                        id
                    ),
                    source_hit
                );
                // Current point predicate sees the wall in all four scenes.
                assert!(g.world.solid_at(x + f64::from(facing) * 8.0, 152.0));
                g.step(&InputFrame::default());
                let e = g.world.entity(id).expect("healthy RUN actor must survive");
                assert_eq!(e.health, 4);
                assert_eq!(e.state, 1);
                assert_eq!(
                    e.pos,
                    Vec2::new(x, 152.0),
                    "own Step follows zero-speed inherited body"
                );
                let expected_facing = if source_hit { -facing } else { facing };
                assert_eq!(e.facing, expected_facing);
                assert_eq!(e.vel.x, f64::from(expected_facing) * 1.5);
            }
        }
    }
}
