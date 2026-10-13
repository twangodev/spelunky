//! Configured raw point contracts, not natural fractional-actor producer claims.
use super::*;

#[test]
fn configured_water_points_use_cached_bounds_and_active_family_membership() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (active, alive, held, expected) in [
            (true, true, false, true),
            (false, true, false, false),
            (true, false, false, false),
            (true, true, true, true),
        ] {
            let mut game = Game::shell(42, rules);
            let id = game.spawn("oWaterSwim", Vec2::new(80.4, 80.0));
            let e = game.world.entity_mut(id).unwrap();
            e.engine_active = active;
            e.alive = alive;
            e.held = held;
            assert_eq!(
                e.collision_mask().point_bounds(),
                Rect::new(80.0, 80.0, 16.0, 16.0)
            );
            assert_eq!(
                water(&game.world, Vec2::new(80.0, 88.0)),
                expected,
                "SOURCE_ACTIVE_CACHED_WATER_POINT"
            );
            assert!(!water(&game.world, Vec2::new(79.9995, 88.0)));
            assert!(!water(&game.world, Vec2::new(96.0, 88.0)));
        }
    }
}

#[test]
fn configured_swimming_onset_rejects_tiny_rectangle_only_overlap() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(42, rules);
        game.player.pos = Vec2::new(79.9995, 88.0);
        let water = game.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
        assert!(
            !game
                .world
                .entity(water)
                .unwrap()
                .mask_contains(game.player.pos, CollisionPrecision::BoundingBox)
        );
        assert!(collision_rect(
            &game.world,
            Rect::new(79.9995, 88.0, 0.001, 0.001),
            "oWaterSwim"
        ));
        environmental_prelude(&mut game);
        assert_eq!(
            value(&game.player, "swimming", 0.0),
            0.0,
            "SOURCE_POINT_HAS_NO_AREA"
        );
        assert!(!game.world.entities.iter().any(|e| e.object == "oSplash"));
        assert!(
            !game
                .events
                .iter()
                .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndSplash"))
        );
    }
}

#[test]
fn configured_swimming_lava_guard_uses_the_raw_cached_point() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (active, alive, held, expected_swimming) in [
            (true, true, false, false),
            (false, true, false, true),
            (true, false, false, true),
            (true, true, true, false),
        ] {
            let mut game = Game::shell(42, rules);
            game.player.pos = Vec2::new(80.0, 88.0);
            game.player.data.insert("swimming".into(), 1.0);
            let id = game.spawn("oLava", Vec2::new(80.4, 80.0));
            let e = game.world.entity_mut(id).unwrap();
            e.engine_active = active;
            e.alive = alive;
            e.held = held;
            assert!(e.mask_contains(Vec2::new(80.0, 88.0), CollisionPrecision::BoundingBox));
            assert_eq!(
                swimming_without_lava(&game.world, &game.player),
                expected_swimming,
                "SOURCE_SWIMMING_LAVA_POINT"
            );
        }
    }
}
