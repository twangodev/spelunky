use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

fn room_with_detector(right: bool, scale: usize) -> (Game, u64, usize) {
    let mut game = Game::shell(1, RulesPreset::Classic);
    game.world = World::new(32, 15, Area::Mines, 1);
    game.player.pos = Vec2::new(24.0, 184.0);
    game.player.previous_pos = game.player.pos;
    for x in 0..32 {
        game.world.set_tile(x, 12, "oBlock");
    }
    let wall_x = if right { 11 + scale } else { 9 - scale };
    game.world.set_tile(wall_x, 5, "oBlock");
    game.world.spawn(
        if right {
            "oArrowTrapRight"
        } else {
            "oArrowTrapLeft"
        },
        Vec2::new(160.0, 80.0),
    );
    // Real Alarm 1 computes the span and creates the scaled detector.
    game.step(&InputFrame::default());
    let detector = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oArrowTrapTest")
        .unwrap();
    assert_eq!(detector.value("image_xscale"), scale as f64);
    let id = detector.id;
    assert!(!game.world.entities.iter().any(|e| e.object == "oArrow"));
    (game, id, wall_x)
}

#[test]
fn scaled_detectors_preserve_source_transparent_boundary_rows() {
    for right in [false, true] {
        for scale in [1, 2, 5] {
            for (rock_y, fires) in [(77.0, false), (78.0, true), (97.0, true), (98.0, false)] {
                let (mut game, detector_id, _) = room_with_detector(right, scale);
                let rock_x = game.world.entity(detector_id).unwrap().pos.x + 8.0;
                let rock_id = game.world.spawn("oRock", Vec2::new(rock_x, rock_y));
                game.world.entity_mut(rock_id).unwrap().vel.x = 1.0;

                game.step(&InputFrame::default());

                assert_eq!(
                    game.world.entity(rock_id).unwrap().pos,
                    Vec2::new(rock_x + 1.0, rock_y)
                );
                assert_eq!(
                    game.world.entities.iter().any(|e| e.object == "oArrow"),
                    fires,
                    "right={right}, scale={scale}, rock_y={rock_y}"
                );
            }
        }
    }
}

#[test]
fn scaled_detectors_test_target_alpha_at_their_retained_far_edge() {
    for right in [false, true] {
        for scale in [1, 2, 5] {
            for opaque in [false, true] {
                let (mut game, detector_id, wall_x) = room_with_detector(right, scale);
                let detector = game.world.entity(detector_id).unwrap();
                let origin = detector.pos;
                // Source Alarm 1 fixes the detector span. A destroyed wall
                // does not resize it; removing this fixture wall exposes its
                // far edge without embedding the moving item in terrain.
                game.world.set_tile(wall_x, 5, "");
                let (rock_x, velocity) = if right {
                    (
                        origin.x + 16.0 * scale as f64 + if opaque { 2.0 } else { 3.0 },
                        -1.0,
                    )
                } else {
                    (origin.x - if opaque { 3.0 } else { 4.0 }, 1.0)
                };
                let rock_id = game.world.spawn("oRock", Vec2::new(rock_x, 78.0));
                game.world.entity_mut(rock_id).unwrap().vel.x = velocity;

                game.step(&InputFrame::default());

                assert_eq!(
                    game.world.entity(rock_id).unwrap().pos,
                    Vec2::new(rock_x + velocity, 78.0)
                );
                assert_eq!(
                    game.world.entities.iter().any(|e| e.object == "oArrow"),
                    opaque,
                    "right={right}, scale={scale}, opaque={opaque}"
                );
            }
        }
    }
}
