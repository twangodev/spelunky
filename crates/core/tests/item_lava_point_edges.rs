use spelunky_core::{Area, CollisionPrecision, Game, InputFrame, RulesPreset, Vec2, World};

fn fixture() -> Game {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(400.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.world.set_tile(6, 6, "oLava");
    game
}

#[test]
fn arrow_flare_and_rope_throw_point_lava_queries_use_half_open_bounds() {
    for object in ["oArrow", "oFlare", "oRopeThrow"] {
        for (x, expected_hit) in [(111.9995, true), (112.0, false)] {
            let mut game = fixture();
            let origin = Vec2::new(x, 101.0);
            let lava_point = Vec2::new(origin.x, origin.y - 5.0);
            assert_eq!(
                game.world
                    .tile_collision_mask(6, 6)
                    .contains(lava_point, CollisionPrecision::BoundingBox),
                expected_hit
            );

            let id = game.spawn(object, origin);
            game.step(&InputFrame::default());
            assert_eq!(
                game.world.entity(id).is_none(),
                expected_hit,
                "source point lava boundary: {object} at x={x}"
            );
        }
    }
}
