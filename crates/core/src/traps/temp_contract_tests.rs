//! Configured initial actor branches; direct existing dispatcher-handler API.
use super::*;
use crate::{Area, Game, RulesPreset, World};

fn scene(rules: RulesPreset) -> (Game, u64) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(80.0, 200.0);
    let trap = game.spawn("oTemp", Vec2::new(192.0, 96.0));
    (game, trap)
}
#[test]
fn temp_own_queries_keep_nearest_guards_and_stationary_moveable_exception() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, pos, velocity, expected) in [
            ("oEnemy", Vec2::new(160.0, 104.0), Vec2::new(1.0, 0.0), true),
            ("oEnemy", Vec2::new(160.0, 104.0), Vec2::default(), false),
            (
                "oEnemy",
                Vec2::new(192.0, 104.0),
                Vec2::new(1.0, 0.0),
                false,
            ),
            (
                "oEnemy",
                Vec2::new(160.0, 112.0),
                Vec2::new(1.0, 0.0),
                false,
            ),
            ("oPushBlock", Vec2::new(160.0, 104.0), Vec2::default(), true),
            ("oRock", Vec2::new(160.0, 104.0), Vec2::new(1.0, 0.0), true),
            ("oRock", Vec2::new(160.0, 104.0), Vec2::default(), false),
        ] {
            let (mut game, trap) = scene(rules);
            let actor = game.spawn(object, pos);
            game.world.entity_mut(actor).unwrap().vel = velocity;
            step_instance(&mut game, Instance::Entity(trap));
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oArrow")
                    .count(),
                usize::from(expected),
                "{object}/{pos:?}/{velocity:?}: source nearest guard"
            );
            assert_eq!(
                game.world.entity(trap).unwrap().value("fired"),
                f64::from(expected)
            );
        }
    }
}
#[test]
fn temp_nearest_stationary_enemy_blocks_a_farther_moving_enemy() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, trap) = scene(rules);
        game.spawn("oEnemy", Vec2::new(180.0, 104.0));
        let farther = game.spawn("oEnemy", Vec2::new(160.0, 104.0));
        game.world.entity_mut(farther).unwrap().vel.x = 1.0;
        step_instance(&mut game, Instance::Entity(trap));
        assert_eq!(game.world.entity(trap).unwrap().value("fired"), 0.0);
        assert!(game.world.entities.iter().all(|e| e.object != "oArrow"));
    }
}

#[test]
fn temp_nearest_skips_retired_and_inactive_family_prefixes() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oEnemy", "oRock"] {
            let (mut game, trap) = scene(rules);
            let retired = game.spawn(object, Vec2::new(188.0, 104.0));
            let inactive = game.spawn(object, Vec2::new(184.0, 104.0));
            let target = game.spawn(object, Vec2::new(160.0, 104.0));
            // Initial configured membership controls, before this direct Step.
            game.world.entity_mut(retired).unwrap().alive = false;
            game.world.entity_mut(inactive).unwrap().engine_active = false;
            game.world.entity_mut(target).unwrap().vel.x = 1.0;
            step_instance(&mut game, Instance::Entity(trap));
            assert_eq!(
                game.world.entity(trap).unwrap().value("fired"),
                1.0,
                "live active farther member must survive nearer excluded prefixes"
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oArrow")
                    .count(),
                1
            );
        }
    }
}
