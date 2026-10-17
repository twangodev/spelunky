//! Authored source-query controls, not untouched campaign trajectories.
//! Piranha/Jaws use active bounds; Monkey Step42 requests precise mask pixels.
use super::*;
use crate::{InputFrame, RulesPreset};

fn room() -> Game {
    let mut game = Game::shell(17, RulesPreset::Classic);
    game.world = World::new(24, 20, Area::Jungle, 5);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game
}

#[test]
fn jungle_water_actual_steps_use_rounded_surface_mask_at_fractional_origin() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oPiranha", "oJaws"] {
            for (query_x, inside) in [
                (79.875, false),
                (80.125, true),
                (95.875, true),
                (96.125, false),
            ] {
                let mut game = room();
                game.rules = rules;
                // Explicit fractional instance origin isolates the difference
                // between cached source bounds and unrounded movement bounds.
                let fluid = game.spawn("oWaterSwim", Vec2::new(80.25, 80.25));
                game.world
                    .entity_mut(fluid)
                    .unwrap()
                    .set_sprite("sWaterTop");
                let probe = Vec2::new(query_x, 84.0);
                let water = game.world.entity(fluid).unwrap();
                assert!(water.alive && water.engine_active);
                assert_eq!(water.sprite, "sWaterTop");
                assert_eq!(
                    water.mask_contains(probe, CollisionPrecision::BoundingBox),
                    inside
                );
                assert_eq!(water.bounds().contains(probe), query_x >= 80.25);
                let pos = if object == "oPiranha" {
                    Vec2::new(query_x - 4.0, 80.0)
                } else {
                    Vec2::new(query_x - 8.0, 68.0)
                };
                let id = game.spawn(object, pos);
                let actor = game.world.entity_mut(id).unwrap();
                // Source PAUSE/TURN entries prevent locomotion so the real
                // dispatcher checks the exact initial mouth/center point.
                actor.state = if object == "oPiranha" { 2 } else { 3 };
                actor.timer = 10;
                actor.data.insert("bubble_timer".into(), 10.0);
                game.step(&InputFrame::default());
                if object == "oPiranha" {
                    assert_eq!(game.world.entity(id).is_some(), inside);
                    assert_eq!(
                        game.world
                            .entities
                            .iter()
                            .filter(|e| e.alive && e.object == "oFishBone")
                            .count(),
                        usize::from(!inside)
                    );
                } else {
                    assert_eq!(
                        game.world.entity(id).unwrap().health,
                        if inside { 40 } else { 39 }
                    );
                }
                assert!(game.world.entity(fluid).unwrap().engine_active);
            }
        }
    }
}

#[test]
fn jungle_water_query_respects_inactive_tiles_and_dynamic_instances() {
    for dynamic in [false, true] {
        for active in [false, true] {
            let mut game = room();
            let point = Vec2::new(88.0, 88.0);
            if dynamic {
                let id = game.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
                game.world.entity_mut(id).unwrap().engine_active = active;
            } else {
                game.world.set_tile(5, 5, "oWaterSwim");
                game.world.tiles[5 + 5 * game.world.width].engine_active = active;
            }
            assert_eq!(water(&game.world, point), active, "dynamic={dynamic}");
        }
    }
}

#[test]
fn monkey_water_step_uses_precise_current_mask_pixels() {
    let mut game = room();
    let fluid = game.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
    game.world.entity_mut(fluid).unwrap().set_sprite("sWeb");
    let water_entity = game.world.entity(fluid).unwrap();
    let transparent = (80..96)
        .flat_map(|y| (80..96).map(move |x| Vec2::new(f64::from(x), f64::from(y))))
        .find(|point| {
            water_entity.mask_contains(*point, CollisionPrecision::BoundingBox)
                && !water_entity.mask_contains(*point, CollisionPrecision::Precise)
        })
        .expect("sWeb has transparent pixels inside its bounding box");
    assert!(water(&game.world, transparent));
    assert!(!precise_water(&game.world, transparent));

    let monkey = game.spawn(
        "oMonkey",
        Vec2::new(transparent.x - 8.0, transparent.y - 8.0),
    );
    let actor = game.world.entity_mut(monkey).unwrap();
    actor.state = 5;
    actor.timer = 20;
    actor.data.insert("swimming".into(), 0.0);
    game.step(&InputFrame::default());
    assert_eq!(game.world.entity(monkey).unwrap().value("swimming"), 0.0);
}
