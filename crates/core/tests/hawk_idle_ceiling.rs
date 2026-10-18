//! Hawkman source Step72-86: wall hop, then an independent ceiling clamp.
//! These authored rooms use source-default births and one ordinary neutral Step.
//! This checks stored state, not a demonstrated visible displacement bug.
use spelunky_core::{Area, Game, InputFrame, Rect, RngAlgorithm, RulesPreset, Vec2, World};

fn check_first_step(roof: bool, wall: bool, expected_y_velocity: f64) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
            game.world = World::new(24, 20, Area::Temple, 12);
            game.world.name = "rLevel".into();
            for x in 0..game.world.width {
                game.world.set_tile(x, 10, "oHardBlock");
            }
            if wall {
                game.world.set_tile(8, 9, "oHardBlock");
            }
            if roof {
                game.world.set_tile(7, 8, "oHardBlock");
            }
            game.player.pos = Vec2::new(48.0, 152.0);
            game.player.previous_pos = game.player.pos;
            game.world.entrance = game.player.pos;
            let id = game.spawn("oHawkman", Vec2::new(112.0, 144.0));
            let born = game.world.entity(id).unwrap();
            assert_eq!((born.health, born.state, born.timer), (4, 0, 0));
            assert_eq!(born.facing, 1);
            assert_eq!(born.vel, Vec2::new(2.5, 0.0));
            assert_eq!(born.bounds(), Rect::new(114.0, 144.0, 12.0, 16.0));
            assert!(!born.held);
            assert_eq!(born.value("sight_timer"), 0.0);

            // Literal source rand(0,1), using each supported RNG adapter's
            // bounded-integer mapping, without calling production range/chance.
            let mut expected_rng = game.rng.clone();
            let raw = expected_rng.next_u64();
            let selector = match algorithm {
                RngAlgorithm::Gm8Lcg32V1 => (raw * 2) >> 32,
                RngAlgorithm::LegacySplitMix64V1 => raw % 2,
            };
            let expected_facing = if selector == 0 { -1 } else { 1 };
            assert_eq!(game.engine_time(), 1);

            // No state, pose, velocity, timer, terrain, or RNG writes after this.
            game.step(&InputFrame::default());
            let hawk = game
                .world
                .entity(id)
                .expect("uncrushed default actor survives");
            assert_eq!(game.engine_time(), 2);
            // Source vx2.5 requests three pixels on this even clock. The wall
            // stops the third pixel at x114; the open corridor reaches x115.
            let expected_x = if wall { 114.0 } else { 115.0 };
            assert_eq!(hawk.pos, Vec2::new(expected_x, 144.0));
            assert_eq!(hawk.previous_pos, Vec2::new(112.0, 144.0));
            assert_eq!(
                hawk.bounds(),
                Rect::new(expected_x + 2.0, 144.0, 12.0, 16.0)
            );
            assert!(hawk.alive && hawk.engine_active && !hawk.held);
            assert_eq!(hawk.health, 4);
            assert_eq!((hawk.state, hawk.timer), (1, 0));
            assert_eq!(hawk.facing, expected_facing);
            assert_eq!(hawk.value("sight_timer"), 5.0);
            assert_eq!(hawk.sprite, "sHawkRunLeft");
            let expected_x_velocity = if wall { 0.9 } else { 2.4 };
            assert!((hawk.vel.x - expected_x_velocity).abs() < 1e-12);
            assert_eq!(
                hawk.vel.y, expected_y_velocity,
                "SOURCE_HAWK_IDLE_UPWARD_CEILING_CLAMP"
            );
            // Independently guard the authored source probe geometry. The top
            // line is y143 and the floor line y160, with unchanged body bounds.
            assert_eq!(
                game.world.solid_line_except(
                    Vec2::new(expected_x + 2.0, 143.0),
                    Vec2::new(expected_x + 13.0, 143.0),
                    id,
                ),
                roof
            );
            assert!(game.world.solid_line_except(
                Vec2::new(expected_x + 2.0, 160.0),
                Vec2::new(expected_x + 13.0, 160.0),
                id,
            ));
            assert_eq!(game.world.solid_at(expected_x + 16.0, 144.0), wall);
            assert!(!game.world.solid_at(expected_x + 8.0, 152.0));
            assert_eq!(game.player.health, 4.0);
            assert!(game.player.held.is_none());
            assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
            assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
        }
    }
}

#[test]
fn default_hawk_wall_hop_is_clamped_by_low_roof_in_its_first_step() {
    check_first_step(true, true, 0.0);
}

#[test]
fn default_hawk_open_roof_retains_source_wall_hop() {
    check_first_step(false, true, -6.0);
}

#[test]
fn default_hawk_low_roof_without_wall_stays_grounded() {
    check_first_step(true, false, 0.0);
}
