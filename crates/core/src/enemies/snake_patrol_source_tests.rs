//! Patrol decisions from oSnake Step.xml, exercised through the game dispatcher.
use super::*;
use crate::{Area, InputFrame, RulesPreset};

#[test]
fn snake_walk_reverses_at_ledge_and_wall_and_stops_on_isolated_floor() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for scenario in ["ledge", "wall", "isolated", "isolated_left_wall"] {
            let mut game = Game::shell(42, rules);
            game.world = World::new(20, 16, Area::Mines, 1);
            game.world.name = "rLevel".into();
            game.player.pos = Vec2::new(250.0, 100.0);
            game.player.previous_pos = game.player.pos;
            game.world.set_tile(5, 10, "oHardBlock");
            if scenario == "ledge" || scenario == "wall" {
                game.world.set_tile(4, 10, "oHardBlock");
            }
            if scenario == "isolated_left_wall" {
                game.world.set_tile(4, 9, "oHardBlock");
            }
            if scenario == "wall" {
                game.world.set_tile(6, 10, "oHardBlock");
                game.world.set_tile(6, 9, "oHardBlock");
            }
            let (x, facing) = if scenario.starts_with("isolated") {
                (81.0, -1)
            } else if scenario == "wall" {
                (82.0, 1)
            } else {
                (80.0, 1)
            };
            let id = game.spawn("oSnake", Vec2::new(x, 144.0));
            let snake = game.world.entity_mut(id).unwrap();
            // Enter WALK with the velocity produced by the preceding patrol Step.
            snake.state = 1;
            snake.facing = facing;
            snake.vel.x = f64::from(facing);

            game.step(&InputFrame::default());
            let snake = game.world.entity(id).unwrap();
            assert_eq!(
                snake.state, 1,
                "seed avoids the random idle branch: {scenario} {rules:?}"
            );
            match scenario {
                "ledge" => {
                    assert_eq!(snake.pos.x, 81.0);
                    assert_eq!((snake.facing, snake.vel.x), (-1, -1.0));
                }
                "wall" => {
                    assert_eq!(snake.pos.x, 82.0);
                    assert_eq!((snake.facing, snake.vel.x), (-1, -1.0));
                }
                "isolated" => {
                    assert_eq!(snake.pos.x, 80.0);
                    assert_eq!((snake.facing, snake.vel.x), (-1, 0.0));
                }
                _ => {
                    assert_eq!(snake.pos.x, 80.0);
                    assert_eq!((snake.facing, snake.vel.x), (1, 0.0));
                }
            }
            // The source moves using the previous Step's velocity before
            // calculating the next patrol decision.
            game.step(&InputFrame::default());
            let snake = game.world.entity(id).unwrap();
            let expected_x = match scenario {
                "ledge" => 80.0,
                "wall" => 81.0,
                _ => 80.0,
            };
            assert_eq!(snake.pos.x, expected_x, "{scenario} {rules:?}");
        }
    }
}
