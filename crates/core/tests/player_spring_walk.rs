use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn ice_spring() -> (Game, u64) {
    let mut game = Game::shell(17, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Ice, 1);
    game.world.name = "ice-spring-walk".into();
    for x in 0..20 {
        game.world.set_tile(x, 6, "oDark");
    }
    let spring = game.spawn("oSpringTrap", Vec2::new(80.0, 80.0));
    (game, spring)
}

#[test]
fn walking_onto_ice_spring_launches_player_and_clamps_on_next_step() {
    let (mut game, spring) = ice_spring();
    game.player.pos = Vec2::new(48.0, 88.0);
    let right = InputFrame {
        right: true,
        ..Default::default()
    };
    for _ in 0..13 {
        game.step(&right);
        assert_eq!(game.world.entity(spring).unwrap().state, 0);
    }
    game.step(&right);
    let trap = game.world.entity(spring).unwrap();
    assert_eq!((trap.state, trap.timer), (1, 10));
    assert_eq!(trap.sprite, "sSpringTrapSprung");
    assert_eq!(game.player.pos, Vec2::new(83.0, 72.0));
    assert_eq!(game.player.vel.y, -16.0);
    assert_eq!(game.player.state, PlayerState::Running);

    game.step(&right);
    assert_eq!(game.player.pos.y, 62.0);
    assert_eq!(game.player.vel.y, -10.0);
    assert_eq!(game.player.state, PlayerState::Jumping);
    assert_eq!(game.world.entity(spring).unwrap().timer, 9);
}

#[test]
fn distant_player_does_not_trigger_ice_spring() {
    let (mut game, spring) = ice_spring();
    game.player.pos = Vec2::new(48.0, 88.0);
    for _ in 0..10 {
        game.step(&InputFrame::default());
    }
    let trap = game.world.entity(spring).unwrap();
    assert_eq!((trap.state, trap.timer), (0, 0));
    assert_eq!(game.player.pos, Vec2::new(48.0, 88.0));
    assert_eq!(game.player.vel.y, 0.0);
}
