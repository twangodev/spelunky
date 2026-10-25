use spelunky_core::statistics::EnemyKill;
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, InputFrame, Rect, RulesPreset, Vec2, World,
};

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
fn actual_fire_frog_bomb_runs_inherited_keeper_hit_before_own_warning() {
    // Classic Items/oFireFrogBomb.events/Step.xml calls action_inherited()
    // before its warning, then handles water entry in the same own event.
    let mut game = Game::shell(31, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Jungle, 1);
    game.world.name = "rLevel".into();
    game.world.set_tile(7, 5, "oWaterSwim");
    game.player.pos = Vec2::new(400.0, 280.0);
    game.player.previous_pos = game.player.pos;

    let keeper = game.spawn("oShopkeeper", Vec2::new(120.0, 80.0));
    let shopkeeper = game.world.entity_mut(keeper).unwrap();
    shopkeeper.state = 0;
    shopkeeper.vel = Vec2::default();
    shopkeeper.data.insert("has_gun".into(), 0.0);
    shopkeeper.data.insert("sight_timer".into(), 1000.0);

    let bomb = game.spawn("oFireFrogBomb", Vec2::new(120.0, 88.0));
    let projectile = game.world.entity_mut(bomb).unwrap();
    projectile.vel = Vec2::new(4.0, 0.0);
    projectile.data.insert("armed".into(), 1.0);
    projectile.data.insert("gravity".into(), 0.0);

    game.step(&InputFrame::default());

    let keeper = game.world.entity(keeper).unwrap();
    assert_eq!(keeper.health, 19);
    assert_eq!(keeper.state, 2);
    let hit = game
        .events
        .iter()
        .position(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHit"));
    let warning = game.events.iter().position(
        |event| matches!(event, GameEvent::Dialogue(dialogue) if dialogue.first == "TERRORIST!"),
    );
    let splash = game
        .events
        .iter()
        .position(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndSplash"));
    assert!(hit.is_some_and(|hit| {
        warning
            .is_some_and(|warning| splash.is_some_and(|splash| hit < warning && warning < splash))
    }));
}

#[test]
fn actual_bomb_arrow_death_is_credited_once_after_source_death_step() {
    // Basis/oItem creates oExplosion from the enemy branch; Explosion's
    // Collision only applies hp/velocity/burning. Caveman Step credits the
    // kill later, after its stunned body settles and reaches DEAD.
    let mut game = Game::shell(37, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 12, "oBrick");
    }
    game.player.pos = Vec2::new(24.0, 176.0);
    game.player.previous_pos = game.player.pos;
    game.flags.insert("low_graphics".into());

    let caveman = game.spawn("oCaveman", Vec2::new(240.0, 176.0));
    let enemy = game.world.entity_mut(caveman).unwrap();
    enemy.state = 0;
    enemy.vel = Vec2::default();
    enemy.data.insert("sight_timer".into(), 1000.0);
    let arrow = game.spawn("oArrow", Vec2::new(240.0, 184.0));
    let projectile = game.world.entity_mut(arrow).unwrap();
    projectile.sprite = "sBombArrowRight".into();
    projectile.vel = Vec2::new(4.0, 0.0);
    projectile.data.insert("gravity".into(), 0.0);

    game.step(&InputFrame::default());
    assert_eq!(game.world.entity(caveman).unwrap().health, -28);
    assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
    assert_eq!(
        game.lifetime_stats.enemy_kills[EnemyKill::Caveman as usize],
        0
    );

    for _ in 0..120 {
        if game.lifetime_stats.enemy_kills[EnemyKill::Caveman as usize] != 0 {
            break;
        }
        game.step(&InputFrame::default());
    }
    assert_eq!(
        game.data.get("kills").copied().unwrap_or(0.0),
        1.0,
        "state={:?}, player_hp={}, player_pos={:?}, enemy={:?}, flags={:?}",
        game.state,
        game.player.health,
        game.player.pos,
        game.world.entity(caveman),
        game.flags
    );
    assert_eq!(game.data.get("cavemen").copied().unwrap_or(0.0), 1.0);
    assert_eq!(
        game.lifetime_stats.enemy_kills[EnemyKill::Caveman as usize],
        1
    );
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 1.0);
    assert_eq!(
        game.lifetime_stats.enemy_kills[EnemyKill::Caveman as usize],
        1
    );
}

#[test]
fn actual_item_step_with_four_overlapping_damsels_stops_at_first_invincible_target() {
    // oItem Step's Damsel collision_rectangle returns the first active overlap
    // and tests eligibility afterward; an invincible first match is not a
    // request to try the other three overlapping Damsels.
    let mut game = Game::shell(41, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 6, "oBrick");
    }
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;

    let mut targets = Vec::new();
    for index in 0..4 {
        let target = game.spawn("oDamsel", Vec2::new(88.0, 88.0));
        let damsel = game.world.entity_mut(target).unwrap();
        damsel.state = 0;
        damsel.vel = Vec2::default();
        damsel.timer = 1000;
        damsel.data.insert("hit_cooldown".into(), 0.0);
        damsel
            .data
            .insert("invincible".into(), f64::from(index == 0));
        targets.push(target);
    }
    let rock = game.spawn("oRock", Vec2::new(83.0, 88.0));
    let projectile = game.world.entity_mut(rock).unwrap();
    projectile.vel = Vec2::new(4.0, 0.0);
    projectile.data.insert("gravity".into(), 0.0);
    assert!(
        targets
            .iter()
            .all(|id| game.world.entity(*id).unwrap().mask_intersects(
                Rect::new(85.0, 86.0, 4.0, 4.0),
                CollisionPrecision::BoundingBox
            ))
    );

    game.step(&InputFrame::default());

    assert!(game.world.entity(rock).is_some());
    for target in targets {
        let damsel = game.world.entity(target).unwrap();
        assert_eq!(damsel.health, 4);
        assert_eq!(damsel.state, 0);
    }
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|entity| entity.alive && entity.object == "oBlood")
    );
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHit"))
    );
}

#[test]
fn actual_item_step_nearest_enemy_query_checks_all_four_overlapping_shopkeepers() {
    // oItem Step first uses collision_rectangle as an overlap gate, then
    // instance_nearest(oEnemy) across the active family. The unique nearest
    // match is invincible, so the other overlapping Shopkeepers stay untouched.
    let mut game = Game::shell(43, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(400.0, 280.0);
    game.player.previous_pos = game.player.pos;

    let mut targets = Vec::new();
    for x in [108.0, 111.0, 114.0, 117.0] {
        let target = game.spawn("oShopkeeper", Vec2::new(x, 80.0));
        let keeper = game.world.entity_mut(target).unwrap();
        keeper.state = 0;
        keeper.vel = Vec2::default();
        keeper.data.insert("has_gun".into(), 0.0);
        keeper.data.insert("sight_timer".into(), 1000.0);
        keeper
            .data
            .insert("invincible".into(), f64::from(x == 117.0));
        targets.push(target);
    }
    let rock = game.spawn("oRock", Vec2::new(119.0, 88.0));
    let projectile = game.world.entity_mut(rock).unwrap();
    projectile.vel = Vec2::new(4.0, 0.0);
    projectile.data.insert("gravity".into(), 0.0);
    assert!(targets.iter().all(|id| {
        game.world.entity(*id).unwrap().mask_intersects(
            Rect::new(121.0, 86.0, 4.0, 4.0),
            CollisionPrecision::BoundingBox,
        )
    }));

    game.step(&InputFrame::default());

    for target in targets {
        assert_eq!(game.world.entity(target).unwrap().health, 20);
    }
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|entity| entity.alive && entity.object == "oBlood")
    );
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHit"))
    );
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

#[test]
fn actual_explosion_born_magmas_hit_later_shopkeeper_pair_in_same_step() {
    // Resource pairs visit Explosion391/MagmaMan648 before Magma649/
    // Shopkeeper666. The first callback creates two Magmas at x+8,y-4;
    // newborn sync must register them before the later Magma-Shopkeeper pair.
    let mut game = Game::shell(41, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Temple, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.world.set_tile(5, 9, "oHardBlock");
    game.world.set_tile(6, 9, "oHardBlock");

    let magma_man = game.spawn("oMagmaMan", Vec2::new(80.0, 128.0));
    let actor = game.world.entity_mut(magma_man).unwrap();
    actor.state = 0;
    actor.timer = 10;
    actor.vel = Vec2::default();

    let keeper = game.spawn("oShopkeeper", Vec2::new(88.0, 124.0));
    let shopkeeper = game.world.entity_mut(keeper).unwrap();
    shopkeeper.state = 0;
    shopkeeper.vel = Vec2::default();
    shopkeeper.data.insert("has_gun".into(), 0.0);
    shopkeeper.data.insert("sight_timer".into(), 1000.0);

    // Frame zero's opaque center hits Magma Man at its center. The resulting
    // Magma origin (88,124) overlaps the Shopkeeper's rectangular mask.
    game.spawn("oExplosion", Vec2::new(88.0, 136.0));

    game.step(&InputFrame::default());

    assert!(game.world.entity(magma_man).is_none());
    let magmas: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oMagma")
        .collect();
    assert_eq!(magmas.len(), 2, "Explosion's MagmaMan callback creates two");
    assert!(
        magmas
            .iter()
            .all(|magma| magma.pos == Vec2::new(88.0, 124.0))
    );

    let keeper = game.world.entity(keeper).unwrap();
    assert!(magmas.iter().all(|magma| magma.event_intersects(keeper)));
    assert_eq!(keeper.health, -14, "Explosion hits first, then both Magmas");
    assert_eq!(keeper.state, 98, "later newborn pair applies Magma stun");
    assert_eq!(keeper.timer, 50);
    assert_eq!(keeper.value("burning"), 100.0);
}
