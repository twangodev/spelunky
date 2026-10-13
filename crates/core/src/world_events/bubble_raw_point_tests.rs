//! Configured Bubble own-Step point contracts; no ordinary fractional-producer claim.
use super::*;
use crate::{CollisionPrecision, RulesPreset};

fn bubble_step(game: &mut Game, point: Vec2, expected_alive: bool) {
    let id = game.spawn("oBubble", point);
    let before = game.world.entity(id).unwrap().clone();
    assert!(before.alive && before.engine_active);
    assert_eq!(before.value("world_effect_initialized"), 1.0);
    assert_eq!(before.vel.x, 0.0);
    let speed = (-before.vel.y * 10.0).round() as u32;
    assert!((1..=3).contains(&speed));
    assert!((before.vel.y + f64::from(speed) * 0.1).abs() < 1e-12);
    let next_rng = game.rng.clone().next_u64();
    // Original own Step first adds the default Create yVel, then probes its
    // resulting raw origin. No engine animation or full-room phase is claimed.
    step_instance(game, id);
    let after = game
        .world
        .entities
        .iter()
        .find(|entity| entity.id == id)
        .expect("stored Step receiver remains identifiable before dispatcher cleanup");
    assert_eq!(after.pos.x, point.x);
    assert_eq!(after.pos.y, point.y + before.vel.y);
    assert_eq!(after.vel, before.vel);
    assert_eq!(after.alive, expected_alive, "SOURCE_BUBBLE_RAW_WATER_POINT");
    assert_eq!(
        game.rng.clone().next_u64(),
        next_rng,
        "point lookup and repeated Create marker consume no RNG"
    );
}

#[test]
fn configured_bubble_uses_cached_fractional_water_bounds_at_both_horizontal_edges() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (water_x, bubble_x, expected) in [
            (80.4, 80.0, true),
            (80.4, 96.0, false),
            (80.0, 88.0, true),
            (80.0, 79.9995, false),
            (80.0, 96.0, false),
        ] {
            let mut game = Game::shell(42, rules);
            let water = game.spawn("oWaterSwim", Vec2::new(water_x, 80.0));
            let target = game.world.entity(water).unwrap();
            assert_eq!(
                target.collision_mask().point_bounds(),
                Rect::new(80.0, 80.0, 16.0, 16.0)
            );
            assert_eq!(
                target.mask_contains(Vec2::new(bubble_x, 88.0), CollisionPrecision::BoundingBox),
                expected
            );
            if water_x == 80.4 && bubble_x == 80.0 {
                assert!(!target.sprite_bounds().contains(Vec2::new(bubble_x, 88.0)));
            }
            if water_x == 80.4 && bubble_x == 96.0 {
                assert!(target.sprite_bounds().contains(Vec2::new(bubble_x, 88.0)));
            }
            bubble_step(&mut game, Vec2::new(bubble_x, 88.0), expected);
        }
    }
}

#[test]
fn configured_bubble_point_filters_retired_and_inactive_but_includes_held_water_family() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Lava is an original oWater descendant too. The source asks for that
        // family, not only oWaterSwim, regardless of the Player's lava damage.
        for object in ["oWaterSwim", "oLava"] {
            for (alive, active, held, expected) in [
                (true, true, false, true),
                (false, true, false, false),
                (true, false, false, false),
                (true, true, true, true),
            ] {
                let mut game = Game::shell(42, rules);
                let id = game.spawn(object, Vec2::new(80.0, 80.0));
                let target = game.world.entity_mut(id).unwrap();
                target.alive = alive;
                target.engine_active = active;
                target.held = held;
                assert!(
                    target.mask_contains(Vec2::new(88.0, 88.0), CollisionPrecision::BoundingBox)
                );
                bubble_step(&mut game, Vec2::new(88.0, 88.0), expected);
            }
        }
    }
}

#[test]
fn configured_bubble_tile_water_uses_the_active_raw_point_mask() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (active, x, expected) in [
            (true, 88.0, true),
            (false, 88.0, false),
            (true, 79.9995, false),
            (true, 96.0, false),
        ] {
            let mut game = Game::shell(42, rules);
            game.world.set_tile(5, 5, "oWaterSwim");
            let index = 5 * game.world.width + 5;
            game.world.tiles[index].engine_active = active;
            assert_eq!(
                game.world.tile_collision_mask(5, 5).point_bounds(),
                Rect::new(80.0, 80.0, 16.0, 16.0)
            );
            bubble_step(&mut game, Vec2::new(x, 88.0), expected);
        }
    }
}
