//! Source FireFrogBomb collision_point(-1,-1) fractional edge controls.
//! Initially authored water/bomb scene; only ordinary neutral inputs follow.
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World,
};

fn splash_count(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndSplash"))
        .count()
}

#[test]
fn fire_frog_water_point_uses_bbox_for_tile_and_dynamic_source_water() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for dynamic in [false, true] {
            for (x, expected, precise) in [
                (95.75, true, false),
                (95.49, true, true),
                (96.0, false, false),
                (79.9995, false, false),
                (80.0, true, true),
            ] {
                let mut game = Game::shell(42, rules);
                game.world = World::new(40, 24, Area::Mines, 1);
                game.world.name = "rTitle".into();
                game.flags.insert("hub".into());
                game.flags.insert("hub_title".into());
                game.player.pos = Vec2::new(24.0, 152.0);
                game.player.previous_pos = game.player.pos;
                game.player.state = PlayerState::Standing;
                game.player.grounded = true;
                game.world.set_tile(1, 10, "oHardBlock");
                let point = Vec2::new(x, 104.0);
                if dynamic {
                    let water = game.spawn("oWaterSwim", Vec2::new(80.0, 96.0));
                    let water = game.world.entity(water).unwrap();
                    assert_eq!(water.sprite, "sWater");
                    assert!(water.engine_active);
                    assert_eq!(
                        water.mask_contains(point, CollisionPrecision::BoundingBox),
                        expected
                    );
                    assert_eq!(
                        water.mask_contains(point, CollisionPrecision::Precise),
                        precise
                    );
                } else {
                    game.world.set_tile(5, 6, "oWaterSwim");
                    let mask = game.world.tile_collision_mask(5, 6);
                    assert_eq!(
                        mask.contains(point, CollisionPrecision::BoundingBox),
                        expected
                    );
                    assert_eq!(mask.contains(point, CollisionPrecision::Precise), precise);
                }
                // Literal raw bbox admits [80,96); precise RTE95.75=96 samples
                // column16 outside default sWater. No custom mask or velocity.
                assert_eq!(x >= 80.0 && x < 96.0, expected);
                let bomb = game.spawn("oFireFrogBomb", point);
                assert_eq!(game.world.entity(bomb).unwrap().vel, Vec2::default());
                assert_eq!(game.world.entity(bomb).unwrap().value("swimming"), 0.0);
                assert_eq!(game.player.health, 4.0);
                game.step(&InputFrame::default());
                let receiver = game
                    .world
                    .entity(bomb)
                    .expect("default fuse has not expired");
                assert_eq!(
                    receiver.pos, point,
                    "FIXTURE: own query retains initial zero-displacement origin"
                );
                assert_eq!(
                    receiver.value("swimming"),
                    f64::from(expected),
                    "SOURCE_FIREFROG_BBOX {rules:?} dynamic={dynamic} x={x}"
                );
                assert_eq!(receiver.value("armed"), 0.0);
                assert!(receiver.alarms[1] > 0);
                assert_eq!(splash_count(&game), usize::from(expected));
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oSplash")
                        .count(),
                    usize::from(expected)
                );
                game.step(&InputFrame::default());
                let receiver = game.world.entity(bomb).unwrap();
                assert_eq!(receiver.pos.x, x);
                assert!((96.0..112.0).contains(&receiver.pos.y));
                assert_eq!(receiver.value("swimming"), f64::from(expected));
                assert_eq!(
                    splash_count(&game),
                    0,
                    "continuous immersion does not repeat entry sound"
                );
                assert_eq!(game.player.health, 4.0);
                assert!(!game.world.entities.iter().any(|e| e.object == "oExplosion"));
            }
        }
    }
}
