use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

#[test]
fn grounded_player_above_zombie_receives_source_vertical_contact_knockback() {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(20, 16, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oBlock");
    }
    // Keep the player grounded on a narrow ledge to the Zombie's right. The
    // source generic oEnemy Character callback does not classify Standing as
    // a stomp, but still applies yVel=-6 because player.y < enemy.y.
    game.world.set_tile(7, 9, "oBlock");
    game.player.pos = Vec2::new(109.0, 136.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.vel = Vec2::default();
    let zombie = game.spawn("oZombie", Vec2::new(96.0, 142.0));
    assert_eq!(game.world.entity(zombie).unwrap().health, 1);
    assert!(game.player.pos.y < game.world.entity(zombie).unwrap().pos.y);

    game.step(&InputFrame::default());

    assert_eq!(game.player.health, 3.0, "Zombie contact deals one damage");
    assert_eq!(
        game.world.entity(zombie).unwrap().health,
        1,
        "contact is not a stomp"
    );
    assert_eq!(game.player.invincible, 30);
    assert_eq!(game.player.vel.x, 6.0);
    assert_eq!(game.player.vel.y, -6.0, "SOURCE_PARENT_Y_KNOCKBACK");
}
