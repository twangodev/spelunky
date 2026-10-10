use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

#[test]
fn webball_collision_events_test_precise_pixels_against_snake_rectangle_masks() {
    // oWebBall Collision with oEnemy uses the two actual mask predicates.
    // Snake Step selects its rectangular walking mask; the first placement
    // reaches only WebBall's transparent top-left corner, the second its body.
    for (snake_pos, expected_state) in [(Vec2::new(81.0, 80.0), 0), (Vec2::new(82.0, 81.0), 1)] {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(16, 12, Area::Mines, 1);
        game.player.pos = Vec2::new(24.0, 24.0);
        game.player.previous_pos = game.player.pos;
        let snake_id = game.world.spawn("oSnake", snake_pos);
        // A source IDLE snake can retain zero velocity after ending a walk.
        // Keep it stationary to isolate event geometry from Step ordering.
        let snake = game.world.entity_mut(snake_id).unwrap();
        snake.timer = 60;
        snake.vel = Vec2::default();
        let web_id = game.world.spawn("oWebBall", Vec2::new(99.0, 102.0));
        let web = game.world.entity_mut(web_id).unwrap();
        web.vel = Vec2::new(1.0, -2.0);
        web.data.insert("life".into(), 50.0);

        game.step(&InputFrame::default());

        let snake = game.world.entity(snake_id).unwrap();
        assert_eq!(snake.pos, snake_pos);
        assert_eq!(snake.sprite, "sSnakeWalkL");
        let web = game.world.entity(web_id).unwrap();
        assert_eq!(web.pos, Vec2::new(100.0, 100.0));
        assert_eq!(web.state, expected_state, "snake at {snake_pos:?}");
    }
}
