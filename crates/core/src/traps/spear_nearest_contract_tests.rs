//! Supplemental direct source-Step controls; target activation is initial setup.
use super::*;
use crate::{Game, RulesPreset, Vec2};

#[test]
fn spear_nearest_ignores_inactive_members_and_includes_terrain() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for terrain in [false, true] {
            let mut game = Game::shell(42, rules);
            game.player.pos = Vec2::new(320.0, 104.0);
            let trap = game.spawn("oSpearTrapTop", Vec2::new(160.0, 96.0));
            game.world.set_tile(10, 7, "oHardBlock");
            let inactive = game.spawn("oPushBlock", Vec2::new(144.0, 112.0));
            game.world.entity_mut(inactive).unwrap().engine_active = false;
            if terrain {
                game.world.set_tile(8, 6, "oPushBlock");
            } else {
                game.spawn("oPushBlock", Vec2::new(128.0, 96.0));
            }
            crate::dispatcher::register_instances(&mut game);
            game.world.entity_mut(inactive).unwrap().engine_active = false;
            step_instance(&mut game, Instance::Entity(trap));
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oSpearsLeft")
                    .count(),
                1
            );
            assert_eq!(game.world.entity(trap).unwrap().value("fired"), 50.0);
        }
    }
}
