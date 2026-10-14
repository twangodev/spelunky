//! Source-only proposal: Hawk WALK/ATTACK query full-height side contact,
//! whereas movement queries exclude the upper five pixels. No production edit.
//! Actors and native HardBlock geometry are authored before play; two neutral
//! public Steps supply the trajectory without state/sprite/mask/velocity writes.
use spelunky_core::{Area, Game, InputFrame, Rect, RngAlgorithm, RulesPreset, Vec2, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Geometry {
    HeadCorner,
    NoWall,
    FullHeightWall,
}

fn select(raw: u64, upper: u64, algorithm: RngAlgorithm) -> u32 {
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => ((raw * upper) >> 32) as u32,
        RngAlgorithm::LegacySplitMix64V1 => (raw % upper) as u32,
    }
}

fn check(object: &str, geometry: Geometry) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            let mut game = Game::shell_with_algorithm(16, rules, algorithm).unwrap();
            game.world = World::new(24, 20, Area::Temple, 12);
            game.world.name = "rLevel".into();
            for x in 0..game.world.width {
                game.world.set_tile(x, 10, "oHardBlock");
            }
            match geometry {
                Geometry::HeadCorner => game.world.set_tile(8, 8, "oHardBlock"),
                Geometry::FullHeightWall => game.world.set_tile(8, 9, "oHardBlock"),
                Geometry::NoWall => {}
            }
            game.player.pos = Vec2::new(48.0, 152.0);
            game.player.previous_pos = game.player.pos;
            game.world.entrance = game.player.pos;
            let id = game.spawn(object, Vec2::new(112.0, 140.0));
            let initial_hp = if object == "oHawkman" { 4 } else { 3 };
            let born = game.world.entity(id).unwrap();
            assert_eq!((born.health, born.state, born.timer), (initial_hp, 0, 0));
            assert_eq!(born.pos, Vec2::new(112.0, 140.0));
            assert_eq!(born.bounds(), Rect::new(114.0, 140.0, 12.0, 16.0));
            assert_eq!(born.vel, Vec2::new(2.5, 0.0));
            assert_eq!(born.facing, 1);
            assert!(!born.held);
            assert_eq!(game.rng.state_bits(), 16);
            assert_eq!(game.engine_time(), 1);

            // Finite literal seed16 draws, independently projected from the
            // canonical LCG and legacy SplitMix adapter; no range/chance call.
            let raw = match algorithm {
                RngAlgorithm::Gm8Lcg32V1 => [2_156_413_009, 1_314_588_054, 1_398_305_775],
                RngAlgorithm::LegacySplitMix64V1 => [
                    6_764_836_397_866_521_095,
                    1_466_504_191_324_546_123,
                    15_650_902_688_141_952_628,
                ],
            };
            let mut oracle = game.rng.clone();
            assert_eq!(oracle.next_u64(), raw[0]);
            assert_eq!(select(raw[0], 2, algorithm), 1);
            let expected_counter = 20 + select(raw[1], 31, algorithm);
            assert_ne!(select(raw[2], 100, algorithm), 0);

            // All later changes come from ordinary neutral InputFrame only.
            game.step(&InputFrame::default());
            let first = game.world.entity(id).unwrap().clone();
            let first_x = if geometry == Geometry::FullHeightWall {
                114.0
            } else {
                115.0
            };
            assert_eq!(game.engine_time(), 2);
            assert_eq!(game.tick, 1);
            assert_eq!(first.pos, Vec2::new(first_x, 140.0));
            assert_eq!((first.health, first.state, first.timer), (initial_hp, 1, 0));
            assert_eq!(first.facing, 1);
            assert!((first.vel.x - 2.4).abs() < 1e-12);
            assert_eq!(first.vel.y, 0.6);
            assert_eq!(first.value("sight_timer"), 5.0);
            assert_eq!(game.rng.state_bits(), oracle.state_bits());
            assert_eq!(game.player.pos, Vec2::new(48.0, 152.0));
            assert_eq!(game.player.health, 4);

            assert_eq!(oracle.next_u64(), raw[1]);
            assert_eq!(oracle.next_u64(), raw[2]);
            game.step(&InputFrame::default());
            let second = game
                .world
                .entity(id)
                .expect("ordinary falling actor survives");
            // Stored 2.5 - 0.1 is 2.3999999999999999, not literal 2.4.
            // Its fractional reciprocal is 2.5000000000000004, which
            // source round maps to 3; room clock3 therefore adds one pixel.
            let expected_x = if geometry == Geometry::FullHeightWall {
                114.0
            } else {
                118.0
            };
            assert_eq!(game.engine_time(), 3);
            assert_eq!(game.tick, 2);
            assert_eq!(second.pos, Vec2::new(expected_x, 140.0));
            assert_eq!(second.previous_pos, first.pos);
            assert_eq!(
                second.bounds(),
                Rect::new(expected_x + 2.0, 140.0, 12.0, 16.0)
            );
            assert!(second.alive && second.engine_active && !second.held);
            assert_eq!(second.health, initial_hp);
            assert_eq!((second.state, second.timer), (0, expected_counter));
            assert_eq!(second.vel.y, 1.2);
            assert_eq!(second.value("sight_timer"), 4.0);
            assert_eq!(game.rng.state_bits(), oracle.state_bits());
            assert_eq!(game.player.pos, Vec2::new(48.0, 152.0));
            assert_eq!(game.player.health, 4);
            assert!(game.player.held.is_none());
            assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));

            // Native fixed controller bounds remain2,0,14,16. The full side
            // line hits a head tile; the movement strip starts five pixels lower.
            let right_x = expected_x + 14.0;
            assert_eq!(
                game.world.solid_line_except(
                    Vec2::new(expected_x + 1.0, 140.0),
                    Vec2::new(expected_x + 1.0, 155.0),
                    id,
                ),
                false
            );
            assert_eq!(
                game.world.solid_line_except(
                    Vec2::new(right_x, 140.0),
                    Vec2::new(right_x, 155.0),
                    id,
                ),
                geometry != Geometry::NoWall
            );
            assert_eq!(
                game.world.solid_line_except(
                    Vec2::new(right_x, 145.0),
                    Vec2::new(right_x, 155.0),
                    id,
                ),
                geometry == Geometry::FullHeightWall
            );
            assert!(!game.world.solid_at(expected_x + 8.0, 148.0));
            assert!(!game.world.solid_at(expected_x - 1.0, 156.0));
            assert_eq!(
                game.world.solid_at(expected_x + 16.0, 156.0),
                geometry == Geometry::FullHeightWall
            );

            // These prerequisites precede the new source facing oracle. No
            // health/status/clock/shape/RNG failure can masquerade as a turn bug.
            let expected_facing = if geometry == Geometry::NoWall { 1 } else { -1 };
            assert_eq!(
                second.facing, expected_facing,
                "SOURCE_HAWK_FULL_HEIGHT_SIDE_TURN object={object} geometry={geometry:?} rules={rules:?} algorithm={algorithm:?}"
            );
            let expected_vx = if object == "oCaveman" { -1.4 } else { 0.0 };
            assert!((second.vel.x - expected_vx).abs() < 1e-12);
        }
    }
}

#[test]
fn default_hawk_falling_head_corner_turns_before_ledge_pause() {
    check("oHawkman", Geometry::HeadCorner);
}

#[test]
fn default_hawk_without_head_tile_preserves_direction() {
    check("oHawkman", Geometry::NoWall);
}

#[test]
fn default_hawk_full_height_wall_agrees_with_movement_collision() {
    check("oHawkman", Geometry::FullHeightWall);
}

#[test]
fn default_caveman_head_corner_already_uses_full_height_flags() {
    check("oCaveman", Geometry::HeadCorner);
}
