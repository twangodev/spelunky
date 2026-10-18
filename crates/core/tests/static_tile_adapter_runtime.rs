//! Runtime checks for terrain and climbable objects adapted from Classic's
//! static instance families.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn empty_room(area: Area, level: u32) -> Game {
    let mut game = Game::shell(7, RulesPreset::Classic);
    game.world = World::new(20, 20, area, level);
    game.flags.insert("custom".into());
    game.player.pos = Vec2::new(120.0, 120.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game
}

#[test]
fn generated_alien_ship_terrain_uses_solid_destroy_and_own_debris_events() {
    for object in ["oAlienShip", "oAlienShipFloor"] {
        let mut game = empty_room(Area::Ice, 9);
        game.world.set_tile(5, 5, object);
        let fixture = game.spawn("oTikiTorch", Vec2::new(80.0, 64.0));
        let explosion = game.spawn("oExplosion", Vec2::new(88.0, 88.0));
        assert!(game.world.tiles_point(Vec2::new(88.0, 88.0), "oSolid"));
        assert!(game.world.entity(explosion).is_some());

        let mut expected_rng = game.rng.clone();
        let expected_positions: Vec<_> = (0..3)
            .map(|_| {
                Vec2::new(
                    88.0 + f64::from(expected_rng.range(9))
                        - (f64::from(expected_rng.range(9)) - 8.0),
                    88.0 + f64::from(expected_rng.range(9))
                        - (f64::from(expected_rng.range(9)) - 8.0),
                )
            })
            .collect();

        game.step(&InputFrame::default());

        assert_eq!(game.world.tile(5, 5).unwrap().object, "", "{object}");
        assert!(
            game.world.entity(fixture).is_none(),
            "inherited Solid Destroy"
        );
        let debris: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|entity| {
                entity.alive && matches!(entity.object.as_str(), "oRubbleDark" | "oRubbleDarkSmall")
            })
            .collect();
        assert_eq!(debris.len(), 3, "{object}");
        assert_eq!(debris[0].object, "oRubbleDark", "{object}");
        assert_eq!(
            debris.iter().map(|entity| entity.pos).collect::<Vec<_>>(),
            expected_positions,
            "{object} source Destroy rand calls"
        );
        assert_eq!(game.rng.next_u64(), expected_rng.next_u64(), "{object}");
        assert!(game.world.fluid_dirty, "inherited Solid Destroy");
    }
}

#[test]
fn vine_and_vine_top_retain_ladder_controls_in_actual_game_steps() {
    for object in ["oVine", "oVineTop"] {
        let mut game = empty_room(Area::Jungle, 5);
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.world.set_tile(5, 9, object);
        game.world.set_tile(5, 8, object);
        game.player.pos = Vec2::new(89.0, 152.0);
        game.player.previous_pos = game.player.pos;

        game.step(&InputFrame {
            up: true,
            ..InputFrame::default()
        });
        assert_eq!(game.player.state, PlayerState::Climbing, "{object}");
        assert_eq!(game.player.pos.x, 88.0, "{object}");
        game.step(&InputFrame {
            up: true,
            ..InputFrame::default()
        });
        assert!((game.player.vel.y + 0.36).abs() < 1e-12, "{object}");
    }
}
