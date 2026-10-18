use spelunky_core::{Area, Game, GameEvent, InputFrame, RulesPreset, Vec2, World};

#[test]
fn actual_bomb_arrow_item_step_then_explosion_orders_enemy_and_damsel_hits() {
    let mut game = Game::shell(19, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;

    let caveman = game.spawn("oCaveman", Vec2::new(120.0, 80.0));
    let enemy = game.world.entity_mut(caveman).unwrap();
    enemy.state = 0;
    enemy.vel = Vec2::default();
    enemy.data.insert("sight_timer".into(), 1000.0);
    let damsel = game.spawn("oDamsel", Vec2::new(128.0, 88.0));
    let damsel_entity = game.world.entity_mut(damsel).unwrap();
    damsel_entity.state = 0;
    damsel_entity.timer = 1000;

    let arrow = game.spawn("oArrow", Vec2::new(120.0, 88.0));
    let projectile = game.world.entity_mut(arrow).unwrap();
    projectile.sprite = "sBombArrowRight".into();
    projectile.vel = Vec2::new(4.0, 0.0);
    projectile.data.insert("gravity".into(), 0.0);

    game.step(&InputFrame::default());

    assert!(game.world.entity(arrow).is_none());
    assert_eq!(game.world.entity(caveman).unwrap().health, -28);
    assert_eq!(game.world.entity(caveman).unwrap().state, 98);
    assert_eq!(game.world.entity(caveman).unwrap().value("burning"), 50.0);
    assert_eq!(game.world.entity(damsel).unwrap().health, -97);
    assert_eq!(game.world.entity(damsel).unwrap().state, 2);
    assert_eq!(game.world.entity(damsel).unwrap().value("burning"), 50.0);
}

#[test]
fn actual_armed_bomb_runs_inherited_keeper_hit_before_own_terrorist_warning() {
    // Classic Items/oBomb.events/Step.xml calls action_inherited() before its
    // own warning, so Basis/oItem.events/Step.xml records sndHit first here.
    let mut game = Game::shell(23, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;

    let keeper = game.spawn("oShopkeeper", Vec2::new(120.0, 80.0));
    let shopkeeper = game.world.entity_mut(keeper).unwrap();
    shopkeeper.state = 0;
    shopkeeper.vel = Vec2::default();
    shopkeeper.data.insert("has_gun".into(), 0.0);
    shopkeeper.data.insert("sight_timer".into(), 1000.0);

    let bomb = game.spawn("oBomb", Vec2::new(120.0, 88.0));
    let projectile = game.world.entity_mut(bomb).unwrap();
    projectile.sprite = "sBombArmed".into();
    projectile.vel = Vec2::new(4.0, 0.0);
    projectile.data.insert("armed".into(), 1.0);
    projectile.data.insert("gravity".into(), 0.0);
    projectile.alarms[0] = -1;
    projectile.alarms[1] = -1;

    game.step(&InputFrame::default());

    let keeper = game.world.entity(keeper).unwrap();
    assert_eq!(keeper.health, 19);
    assert_eq!(keeper.state, 2);
    assert_eq!(game.data["thiefLevel"], 2.0);
    let hit = game
        .events
        .iter()
        .position(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHit"));
    let warning = game.events.iter().position(
        |event| matches!(event, GameEvent::Dialogue(dialogue) if dialogue.first == "TERRORIST!"),
    );
    assert!(hit.is_some_and(|hit| warning.is_some_and(|warning| hit < warning)));
}

#[test]
fn actual_explosion_precedes_web_when_both_overlap_a_shopkeeper() {
    // Dispatcher resource order runs Explosion-Shopkeeper and then
    // Explosion-Web before the later Shopkeeper-Web pair. Explosion damage
    // kills the Web first, so its later velocity-zeroing callback never runs.
    let mut game = Game::shell(29, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;

    let keeper = game.spawn("oShopkeeper", Vec2::new(120.0, 80.0));
    let shopkeeper = game.world.entity_mut(keeper).unwrap();
    shopkeeper.state = 0;
    shopkeeper.vel = Vec2::default();
    shopkeeper.data.insert("has_gun".into(), 0.0);
    shopkeeper.data.insert("sight_timer".into(), 1000.0);
    let web = game.spawn("oWeb", Vec2::new(120.0, 80.0));
    let _explosion = game.spawn("oExplosion", Vec2::new(120.0, 80.0));

    game.step(&InputFrame::default());

    let keeper = game.world.entity(keeper).unwrap();
    assert_eq!(keeper.health, -10);
    assert_eq!(keeper.state, 0, "later Web collision must see hp <= 0");
    assert_eq!(keeper.vel.x, -6.0);
    assert_eq!(keeper.vel.y, -6.0);
    assert!(game.world.entity(web).is_none());
}
