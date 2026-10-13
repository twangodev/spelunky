//! Literal collision_point(..., family, 0, 0) controls for shared world events.
use super::*;
use crate::{CollisionPrecision, RulesPreset};

#[test]
fn world_points_use_raw_queries_against_cached_fractional_target_bounds() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, family) in [("oBlock", "oSolid"), ("oWaterSwim", "oWater")] {
            let mut game = Game::shell(42, rules);
            let id = game.spawn(object, Vec2::new(80.25, 80.25));
            let target = game.world.entity(id).unwrap();
            // Default 16px source sprites at fractional origins cache 80..95
            // inclusive. The raw point remains unrounded and the upper edge
            // is 96, rather than the smooth sprite's 96.25.
            assert_eq!(
                target.collision_mask().point_bounds(),
                Rect::new(80., 80., 16., 16.)
            );
            assert!(!target.sprite_bounds().contains(Vec2::new(80., 88.)));
            assert!(target.sprite_bounds().contains(Vec2::new(96., 88.)));
            for (x, expected) in [(79.9995, false), (80., true), (95.75, true), (96., false)] {
                assert_eq!(
                    world_point_family(&game.world, Vec2::new(x, 88.), family),
                    expected
                );
            }
        }
    }
}

#[test]
fn world_points_exclude_inactive_and_retired_targets_but_include_held_family_members() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (alive, active, held, expected) in [
            (true, true, false, true),
            (true, false, false, false),
            (false, true, false, false),
            (true, true, true, true),
        ] {
            let mut game = Game::shell(42, rules);
            let id = game.spawn("oBlock", Vec2::new(80., 80.));
            let target = game.world.entity_mut(id).unwrap();
            target.alive = alive;
            target.engine_active = active;
            target.held = held;
            target.solid = false; // Family membership is independent of this flag.
            assert_eq!(
                world_point_family(&game.world, Vec2::new(88., 88.), "oSolid"),
                expected
            );
        }
        let mut game = Game::shell(42, rules);
        let id = game.spawn("oRubbleSmall", Vec2::new(80., 80.));
        game.world.entity_mut(id).unwrap().solid = true;
        assert!(!world_point_family(
            &game.world,
            Vec2::new(88., 88.),
            "oSolid"
        ));
    }
}

#[test]
fn world_tile_points_respect_active_state_and_raw_bbox_instead_of_precise_alpha() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(42, rules);
        game.world.set_tile(5, 5, "oWaterSwim");
        let point = Vec2::new(95.75, 88.);
        assert!(
            !game
                .world
                .tile_collision_mask(5, 5)
                .contains(point, CollisionPrecision::Precise)
        );
        assert!(world_point_family(&game.world, point, "oWater"));
        game.world.tiles[5 * game.world.width + 5].engine_active = false;
        assert!(!world_point_family(&game.world, point, "oWater"));
        // The compatibility query retains its separate all-terrain contract.
        assert!(game.world.tiles_point(point, "oWater"));
    }
}

#[test]
fn real_burn_step_queries_cached_solid_bounds_after_default_create_motion() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (active, expected_alive) in [(true, false), (false, true)] {
            let mut game = Game::shell(42, rules);
            let block = game.spawn("oBlock", Vec2::new(80.25, 80.25));
            game.world.entity_mut(block).unwrap().engine_active = active;
            let burn = game.spawn("oBurn", Vec2::new(80., 88.));
            let before = game.world.entity(burn).unwrap().clone();
            let rng = game.rng.state_bits();
            step_instance(&mut game, burn);
            let after = game.world.entities.iter().find(|e| e.id == burn).unwrap();
            assert_eq!(after.pos, Vec2::new(80., 88. + before.vel.y));
            assert_eq!(after.alive, expected_alive);
            assert_eq!(game.rng.state_bits(), rng);
        }
    }
}
