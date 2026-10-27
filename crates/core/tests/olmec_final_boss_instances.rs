use spelunky_core::*;

#[test]
fn controller_state_is_per_instance_and_works_outside_boss_area() {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(42, 55, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(320.0, 320.0);
    game.player.previous_pos = game.player.pos;
    let first = game.spawn("oFinalBoss", Vec2::default());
    let second = game.spawn("oFinalBoss", Vec2::default());
    game.world
        .entity_mut(first)
        .unwrap()
        .data
        .insert("olmecDead".into(), 1.0);
    game.step(&InputFrame::default());
    assert_eq!(game.world.entity(first).unwrap().value("doorOpen"), 1.0);
    assert_eq!(game.world.entity(second).unwrap().value("doorOpen"), 0.0);
    let count = |g: &Game| {
        g.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oXEnd")
            .count()
    };
    assert_eq!(count(&game), 1);
    game.world
        .entity_mut(second)
        .unwrap()
        .data
        .insert("olmecDead".into(), 1.0);
    game.step(&InputFrame::default());
    assert_eq!(game.world.entity(second).unwrap().value("doorOpen"), 1.0);
    assert_eq!(count(&game), 2);
    game.step(&InputFrame::default());
    assert_eq!(count(&game), 2);
}

#[test]
fn lava_death_assigns_only_first_active_controller() {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(42, 55, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(320.0, 320.0);
    game.player.previous_pos = game.player.pos;
    let first = game.spawn("oFinalBoss", Vec2::default());
    let second = game.spawn("oFinalBoss", Vec2::default());
    game.spawn("oLava", Vec2::new(64.0, 80.0));
    let boss = game.spawn("oOlmec", Vec2::new(64.0, 96.0));
    game.step(&InputFrame::default());
    assert!(game.world.entity(boss).is_none());
    assert_eq!(game.world.entity(first).unwrap().value("olmecDead"), 1.0);
    assert_eq!(game.world.entity(second).unwrap().value("olmecDead"), 0.0);
    assert_eq!(game.world.entity(first).unwrap().value("doorOpen"), 1.0);
    assert_eq!(game.world.entity(second).unwrap().value("doorOpen"), 0.0);
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oXEnd")
            .count(),
        1
    );
}

#[test]
fn lava_death_uses_reactivated_controller_order() {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(42, 55, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(320.0, 320.0);
    game.player.previous_pos = game.player.pos;
    let first = game.spawn("oFinalBoss", Vec2::new(16.0, 16.0));
    let second = game.spawn("oFinalBoss", Vec2::new(300.0, 16.0));
    game.world.entity_mut(first).unwrap().engine_active = false;
    dispatcher::activate_region(&mut game, Rect::new(290.0, 0.0, 40.0, 40.0));
    dispatcher::activate_object(&mut game, "oFinalBoss");
    assert_eq!(
        dispatcher::descendant_instances(&game, "oFinalBoss"),
        vec![
            dispatcher::Instance::Entity(second),
            dispatcher::Instance::Entity(first)
        ]
    );
    game.spawn("oLava", Vec2::new(64.0, 80.0));
    game.spawn("oOlmec", Vec2::new(64.0, 96.0));
    game.step(&InputFrame::default());
    assert_eq!(game.world.entity(first).unwrap().value("olmecDead"), 0.0);
    assert_eq!(game.world.entity(second).unwrap().value("olmecDead"), 1.0);
}

#[test]
fn historical_flags_retain_old_door_behavior_and_controller_shape() {
    for opened in [false, true] {
        let mut game = Game::legacy_shell(42, RulesPreset::Classic);
        game.world = World::new(42, 55, Area::Boss, 1);
        game.world.name = "rOlmec".into();
        let controller = game.spawn("oFinalBoss", Vec2::default());
        game.flags.insert("olmec_dead".into());
        if opened {
            game.flags.insert("boss_door_open".into());
        }
        let encoded = serde_json::to_string(&game).unwrap();
        let mut game: Game = serde_json::from_str(&encoded).unwrap();
        game.step(&InputFrame::default());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oXEnd")
                .count(),
            usize::from(!opened)
        );
        let data = &game.world.entity(controller).unwrap().data;
        assert!(!data.contains_key("olmecDead"));
        assert!(!data.contains_key("doorOpen"));
    }

    // The historical public adapter ran the global handler without requiring
    // an oFinalBoss instance, unlike the authoritative instance callback.
    let mut game = Game::legacy_shell(42, RulesPreset::Classic);
    game.world = World::new(42, 55, Area::Boss, 1);
    game.flags.insert("olmec_dead".into());
    progression::tick(&mut game, &InputFrame::default());
    assert!(game.flags.contains("boss_door_open"));
}

#[test]
fn historical_lava_death_does_not_add_controller_keys() {
    let mut game = Game::legacy_shell(42, RulesPreset::Classic);
    game.world = World::new(42, 55, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(320.0, 320.0);
    game.player.previous_pos = game.player.pos;
    let controller = game.spawn("oFinalBoss", Vec2::default());
    game.spawn("oLava", Vec2::new(64.0, 80.0));
    game.spawn("oOlmec", Vec2::new(64.0, 96.0));
    game.step(&InputFrame::default());
    assert!(game.flags.contains("olmec_dead"));
    assert!(!game.flags.contains("boss_door_open"));
    let data = &game.world.entity(controller).unwrap().data;
    assert!(!data.contains_key("olmecDead"));
    assert!(!data.contains_key("doorOpen"));
}
