use spelunky_core::{Game, InputFrame, RulesPreset, Vec2};

fn game() -> Game {
    let mut game = Game::shell(7, RulesPreset::Classic);
    game.player.pos = Vec2::new(160.0, 160.0);
    game
}

#[test]
fn game_step_explosion_preserves_source_invincible_lava_solid_terrain() {
    let mut game = game();
    game.world.set_tile(5, 5, "oLavaSolid");
    game.world.spawn("oExplosion", Vec2::new(88.0, 88.0));

    game.step(&InputFrame::default());

    assert_eq!(game.world.tile(5, 5).unwrap().object, "oLavaSolid");
}

#[test]
fn game_step_explosion_preserves_source_invincible_lava_solid_instances() {
    let mut game = game();
    let lava_solid = game.spawn("oLavaSolid", Vec2::new(80.0, 80.0));
    assert_eq!(game.world.entity(lava_solid).unwrap().value("invincible"), 1.0);
    game.world.spawn("oExplosion", Vec2::new(88.0, 88.0));

    game.step(&InputFrame::default());

    assert!(game.world.entity(lava_solid).is_some());
}
