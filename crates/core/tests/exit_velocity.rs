//! Classic's exit snap retains the velocity read by later room actors.
use spelunky_core::*;

#[test]
fn walking_into_exit_retains_source_velocity_and_historical_control() {
    for revision in [
        GenerationRevision::SourceClassicV2,
        GenerationRevision::HistoricalV1,
    ] {
        let mut game =
            Game::shell_with_algorithm(7, RulesPreset::Classic, RngAlgorithm::Gm8Lcg32V1).unwrap();
        game.generation_revision = revision;
        game.world = World::new(30, 20, Area::Mines, 1);
        for col in 0..game.world.width {
            game.world.set_tile(col, 10, "oBlock");
        }
        game.player.pos = Vec2::new(160.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.set_simulation_view_extent(480.0, 320.0).unwrap();
        game.spawn("oExit", Vec2::new(232.0, 144.0));
        for _ in 0..80 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            if game.player.pos.x >= 232.0 {
                break;
            }
        }
        assert!(game.player.vel.x > 0.0, "walking must establish momentum");
        game.step(&InputFrame {
            right: true,
            up: true,
            ..Default::default()
        });
        assert!(game.player.is_exiting());
        if revision == GenerationRevision::SourceClassicV2 {
            assert!(
                game.player.vel.x > 0.0,
                "Classic exit leaves walking velocity intact"
            );
            let snapped_pos = game.player.pos;
            assert_eq!(snapped_pos, Vec2::new(240.0, 152.0));
            for _ in 0..8 {
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
                assert!(game.player.is_exiting(), "exit animation remains live");
                assert!(!game.player.is_stunned());
                assert_eq!(game.player.pos, snapped_pos, "inactive exit must not drift");
                assert!(
                    game.player.vel.x > 0.0,
                    "inactive exit animation keeps velocity"
                );
            }
        } else {
            assert_eq!(
                game.player.vel,
                Vec2::default(),
                "historical behavior remains unchanged"
            );
        }
    }
}
