use spelunky_core::*;

fn room_with_active_arrow_detector() -> Game {
    let mut game = Game::shell(17, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Jungle, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(24.0, 184.0);
    game.player.previous_pos = game.player.pos;
    for x in 0..30 {
        game.world.set_tile(x, 12, "oBlock");
    }

    // With no blocking wall, oArrowTrapLeft Alarm 1 stretches its active
    // sRed detector to its 96-pixel limit. Water lets oDeadFish cross the
    // sensor boundary using moveTo(1, 0), which preserves source xVel/yVel.
    game.world.set_tile(9, 5, "oWaterSwim");
    game.world.set_tile(10, 5, "oWaterSwim");
    game.world.set_tile(11, 5, "oWaterSwim");
    game.spawn("oArrowTrapLeft", Vec2::new(256.0, 80.0));
    game.step(&InputFrame::default());

    assert!(
        game.world
            .entities
            .iter()
            .any(|entity| entity.alive && entity.object == "oArrowTrapTest")
    );
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|entity| entity.alive && entity.object == "oArrow")
    );
    game
}

fn arrows(game: &Game) -> usize {
    game.world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oArrow")
        .count()
}

#[test]
fn dead_fish_direct_motion_keeps_source_velocity_gate_closed() {
    let mut fish_game = room_with_active_arrow_detector();
    let fish_id = fish_game.spawn("oDeadFish", Vec2::new(150.0, 80.0));
    assert_eq!(
        fish_game.world.entity(fish_id).unwrap().vel,
        Vec2::default()
    );

    // Source oDeadFish/Create sets xVel=yVel=0. Its Step moves through
    // water with moveTo(1, 0), while the ArrowTrapTest collision only checks
    // other.xVel/other.yVel (Collision with oEnemy.xml:20).
    let start = fish_game.world.entity(fish_id).unwrap().pos;
    let detector_id = fish_game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oArrowTrapTest")
        .unwrap()
        .id;
    let detector_left = fish_game.world.entity(detector_id).unwrap().pos.x;
    assert!(
        start.x + 8.0 < detector_left,
        "fish must start outside the sensor"
    );
    fish_game.step(&InputFrame::default());
    fish_game.step(&InputFrame::default());

    let fish = fish_game.world.entity(fish_id).unwrap();
    assert!(
        fish.pos.x > start.x,
        "fish must actually move into the detector"
    );
    assert_eq!(
        fish.vel,
        Vec2::default(),
        "moveTo arguments preserve xVel/yVel"
    );
    assert!(
        fish_game
            .world
            .entity(detector_id)
            .unwrap()
            .event_intersects(fish)
    );
    assert_eq!(
        arrows(&fish_game),
        0,
        "zero source velocity must not fire the trap"
    );
    assert!(
        fish_game
            .world
            .entities
            .iter()
            .any(|entity| entity.alive && entity.object == "oArrowTrapTest")
    );

    // A real moving enemy is the positive control for the same source gate.
    let mut enemy_game = room_with_active_arrow_detector();
    let scarab_id = enemy_game.spawn("oScarab", Vec2::new(150.0, 80.0));
    {
        let scarab = enemy_game.world.entity_mut(scarab_id).unwrap();
        scarab.state = 1; // ATTACK: source Step leaves its xVel/yVel intact.
        scarab.vel.x = 2.0;
    }
    let start = enemy_game.world.entity(scarab_id).unwrap().pos;
    enemy_game.step(&InputFrame::default());

    let scarab = enemy_game.world.entity(scarab_id).unwrap();
    assert!(
        scarab.pos.x > start.x,
        "positive-control enemy must actually move"
    );
    assert_ne!(scarab.vel, Vec2::default());
    assert_eq!(arrows(&enemy_game), 1, "nonzero source velocity fires once");
}
