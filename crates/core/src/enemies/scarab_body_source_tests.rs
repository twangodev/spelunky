//! Authored incoming-velocity controls through the actual dispatcher.
//! oScarab inherits Enemy Create, then replaces its movement rectangle with
//! setCollisionBounds(4,4,12,12). Its Step replaces Enemy Step entirely.
use super::*;

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(17, rules);
    game.world = World::new(24, 20, Area::Temple, 13);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.world.entrance = game.player.pos;
    game
}

#[test]
fn scarab_actual_step_uses_create_movement_bounds_before_decay_and_wall_probes() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (origin, velocity, wall, expected_pos, expected_velocity) in [
            (
                Vec2::new(97.0, 80.0),
                Vec2::new(2.0, 0.0),
                (7, 5),
                Vec2::new(99.0, 80.0),
                Vec2::new(1.5, 0.0),
            ),
            (
                Vec2::new(80.0, 80.0),
                Vec2::new(-2.0, 0.0),
                (4, 5),
                Vec2::new(78.0, 80.0),
                Vec2::new(-1.5, 0.0),
            ),
            (
                Vec2::new(80.0, 79.0),
                Vec2::new(0.0, -2.0),
                (5, 4),
                Vec2::new(80.0, 77.0),
                Vec2::new(0.0, -1.5),
            ),
            (
                Vec2::new(80.0, 98.0),
                Vec2::new(0.0, 2.0),
                (5, 7),
                Vec2::new(80.0, 100.0),
                Vec2::new(0.0, -1.5),
            ),
        ] {
            let mut game = room(rules);
            game.world.set_tile(wall.0, wall.1, "oHardBlock");
            let id = game.spawn("oScarab", origin);
            let scarab = game.world.entity_mut(id).unwrap();
            scarab.vel = velocity;
            let timer = scarab.timer;
            let rng = game.rng.state_bits();
            game.step(&InputFrame::default());
            let scarab = game.world.entity(id).unwrap();
            assert_eq!(scarab.pos, expected_pos, "incoming={velocity:?}");
            assert_eq!(scarab.vel, expected_velocity, "incoming={velocity:?}");
            assert_eq!(scarab.timer, timer, "moving Scarab does not count down");
            assert_eq!(
                game.rng.state_bits(),
                rng,
                "moving Scarab does not relaunch"
            );
            // Movement-controller offsets must leave the separate source
            // sprite collision mask, used by collection callbacks, unchanged.
            assert_eq!(
                scarab.bounds(),
                Rect::new(expected_pos.x + 4.0, expected_pos.y + 4.0, 8.0, 8.0)
            );
            assert_eq!(
                scarab.sprite_bounds(),
                Rect::new(expected_pos.x + 1.0, expected_pos.y + 2.0, 13.0, 11.0)
            );
        }
    }
}
