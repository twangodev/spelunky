//! Ordinary input route; initial geometry/cost/player pose only.
//! Damsel is freshly created in default IDLE. No mid-route actor writes.
use spelunky_core::*;

#[test]
fn real_pickup_throw_and_grounded_recovery_reaches_source_run_wall_turns() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            let mut g = Game::shell_with_algorithm(7, rules, algorithm).unwrap();
            g.world = World::new(30, 20, Area::Mines, 1);
            for col in 0..g.world.width {
                g.world.set_tile(col, 10, "oBlock");
            }
            for row in 0..10 {
                g.world.set_tile(2, row, "oBlock");
                g.world.set_tile(10, row, "oBlock");
            }
            g.player.pos = Vec2::new(100.0, 152.0);
            g.player.previous_pos = g.player.pos;
            g.player.state = PlayerState::Standing;
            g.player.grounded = true;
            g.player.facing = 1;
            g.set_simulation_view_extent(480.0, 320.0).unwrap();
            let id = g.spawn("oDamsel", Vec2::new(104.0, 152.0));
            g.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("cost".into(), 0.0);
            assert_eq!(g.world.entity(id).unwrap().state, 0);
            assert_eq!(g.world.entity(id).unwrap().health, 4);
            g.step(&InputFrame {
                down: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(g.player.held, Some(id), "real pickup prerequisite");
            assert!(g.world.entity(id).unwrap().held);
            g.step(&InputFrame::default());
            g.step(&InputFrame {
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(g.player.held, None, "real throw prerequisite");
            assert_eq!(g.world.entity(id).unwrap().state, 2);
            assert_eq!(g.world.entity(id).unwrap().timer, 120);
            let mut recovery_decrements = 0;
            let mut running_steps = 0;
            let mut differs_from_old_point = false;
            let mut observed_turn = false;
            for _ in 0..600 {
                let before = g
                    .world
                    .entity(id)
                    .expect("healthy tossed actor must survive")
                    .clone();
                g.step(&InputFrame::default());
                let e = g
                    .world
                    .entity(id)
                    .expect("healthy recovered actor must survive");
                assert_eq!(e.health, 4);
                assert_eq!(g.player.health, 4, "route stays clear of unrelated damage");
                if before.state == 2 && e.timer < before.timer {
                    recovery_decrements += before.timer - e.timer;
                }
                if before.state != 1 {
                    continue;
                }
                assert_eq!(
                    recovery_decrements, 120,
                    "source supported recovery before RUN"
                );
                assert_eq!(e.state, 1);
                running_steps += 1;
                // Own RUN changes facing/velocity, not position. The preceding
                // inherited Item movement cannot alter facing in this static room.
                let b = e.bounds();
                let top = b.y.round_ties_even();
                let bottom = (b.y + b.h - 1.0).round_ties_even();
                let left = (b.x - 2.0).round_ties_even();
                let right = (b.x + b.w + 1.0).round_ties_even();
                let mut source_facing = before.facing;
                if source_facing < 0
                    && g.world
                        .solid_line_except(Vec2::new(left, top), Vec2::new(left, bottom), id)
                {
                    source_facing = 1;
                }
                if source_facing > 0
                    && g.world.solid_line_except(
                        Vec2::new(right, top),
                        Vec2::new(right, bottom),
                        id,
                    )
                {
                    source_facing = -1;
                }
                let old_facing = if g
                    .world
                    .solid_at(e.pos.x + f64::from(before.facing) * 8.0, e.pos.y)
                {
                    -before.facing
                } else {
                    before.facing
                };
                differs_from_old_point |= source_facing != old_facing;
                observed_turn |= source_facing != before.facing;
                assert_eq!(
                    e.facing, source_facing,
                    "literal post-parent RUN edge oracle"
                );
                assert_eq!(e.vel.x, f64::from(source_facing) * 1.5);
                if differs_from_old_point && observed_turn && running_steps >= 40 {
                    break;
                }
            }
            assert_eq!(recovery_decrements, 120);
            assert!(running_steps >= 40, "actual RUN transition prerequisite");
            assert!(observed_turn, "real wall turnaround prerequisite");
            assert!(
                differs_from_old_point,
                "route must expose the old point predicate"
            );
        }
    }
}
