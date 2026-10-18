use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

#[test]
fn bullet_hit_knocks_back_snake_then_its_next_step_runs_death_callback() {
    // Classic source: oBullet Collision with oEnemy sets xVel/yVel before
    // reducing hp (references/classic-1.1/Objects/Effects/oBullet.events/Collision with oEnemy.xml).
    // Snake's own Step checks hp < 1, emits its death blood, and records the
    // Mines-family kill (references/classic-1.1/Objects/Enemies/oSnake.events/Step.xml).
    let mut game = Game::shell(17, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(200.0, 96.0);
    game.player.previous_pos = game.player.pos;

    let snake = game.spawn("oSnake", Vec2::new(128.0, 144.0));
    let bullet = game.spawn("oBullet", Vec2::new(130.0, 150.0));
    game.world.entity_mut(bullet).unwrap().vel = Vec2::default();

    game.step(&InputFrame::default());

    let snake_after_hit = game
        .world
        .entity(snake)
        .expect("automatic collision follows Snake Step, so its death callback is next tick");
    assert_eq!(snake_after_hit.health, -3);
    assert_eq!(snake_after_hit.vel, Vec2::new(0.0, -4.0));
    assert!(game.world.entity(bullet).is_none());
    assert_eq!(game.data.get("kills"), None);

    game.step(&InputFrame::default());

    assert!(game.world.entity(snake).is_none());
    assert_eq!(game.data.get("kills"), Some(&1.0));
    assert_eq!(game.data.get("snakes"), Some(&1.0));
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|entity| entity.object == "oBlood" && entity.alive)
            .count(),
        4,
        "the lethal bullet emits one hit drop and Snake's own Step emits three death drops"
    );
}
