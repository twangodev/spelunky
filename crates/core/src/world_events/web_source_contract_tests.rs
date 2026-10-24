use super::{collision, handles};
use crate::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn room() -> Game {
    let mut game = Game::shell(17, RulesPreset::Classic);
    game.world = World::new(40, 30, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 14, "oHardBlock");
    }
    game.player.pos = Vec2::new(220.0, 208.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game
}

fn web(game: &mut Game, pos: Vec2) -> u64 {
    game.spawn("oWeb", pos)
}

fn target(game: &mut Game, object: &str, source_type: &str, pos: Vec2) -> u64 {
    let id = game.spawn(object, pos);
    game.world
        .entity_mut(id)
        .unwrap()
        .text
        .insert("type".into(), source_type.into());
    id
}

fn set_motion(game: &mut Game, id: u64, velocity: Vec2) {
    let entity = game.world.entity_mut(id).unwrap();
    entity.vel = velocity;
    entity.data.insert("xVel".into(), velocity.x);
    entity.data.insert("yVel".into(), velocity.y);
}

#[test]
fn source_create_and_step_fade_web_in_source_order() {
    let mut game = room();
    let id = web(&mut game, Vec2::new(120.0, 100.0));
    let created = game.world.entity(id).unwrap();
    assert_eq!(created.value("life"), 12.0);
    assert_eq!(created.value("dying"), 0.0);
    assert!(handles("oWeb"));

    // A fresh web does not age until WebBall's Animation End marks it dying.
    game.step(&InputFrame::default());
    assert_eq!(game.world.entity(id).unwrap().value("life"), 12.0);
    game.world
        .entity_mut(id)
        .unwrap()
        .data
        .insert("dying".into(), 1.0);

    game.step(&InputFrame::default());
    let first = game.world.entity(id).unwrap();
    assert_eq!(first.value("image_alpha"), 1.0);
    assert!((first.value("life") - 11.98).abs() < 1e-12);

    game.step(&InputFrame::default());
    let second = game.world.entity(id).unwrap();
    assert!((second.value("image_alpha") - 11.98 / 12.0).abs() < 1e-12);
    assert!((second.value("life") - 11.96).abs() < 1e-12);

    let mut ticks = 2;
    while game.world.entity(id).is_some() && ticks < 600 {
        game.step(&InputFrame::default());
        ticks += 1;
    }
    assert!(
        game.world.entity(id).is_none(),
        "source life expires in {ticks} ticks"
    );
    assert!(
        (550..=551).contains(&ticks),
        "unexpected expiry tick: {ticks}"
    );
}

#[test]
fn web_enemy_collision_freezes_most_enemies_but_preserves_spiders() {
    for (object, source_type, should_freeze) in [
        ("oCaveman", "Caveman", true),
        ("oSpider", "Spider", false),
        ("oGiantSpider", "Giant Spider", false),
    ] {
        let mut game = room();
        let web_id = web(&mut game, Vec2::new(120.0, 100.0));
        let enemy_id = target(&mut game, object, source_type, Vec2::new(120.0, 100.0));
        let velocity = Vec2::new(1.25, -0.75);
        set_motion(&mut game, enemy_id, velocity);

        collision(
            &mut game,
            web_id,
            crate::dispatcher::Instance::Entity(enemy_id),
        );

        let enemy = game.world.entity(enemy_id).unwrap();
        assert_eq!(
            enemy.vel,
            if should_freeze {
                Vec2::default()
            } else {
                velocity
            }
        );
        assert_eq!(
            enemy.value("xVel"),
            if should_freeze { 0.0 } else { velocity.x }
        );
        assert_eq!(
            enemy.value("yVel"),
            if should_freeze { 0.0 } else { velocity.y }
        );
        assert!(game.world.entity(web_id).is_some());
    }
}

#[test]
fn web_collision_destroys_it_on_magma_man_and_angers_living_shopkeeper() {
    let mut game = room();
    let web_id = web(&mut game, Vec2::new(120.0, 100.0));
    let magma_id = target(&mut game, "oMagmaMan", "Magma Man", Vec2::new(120.0, 100.0));
    set_motion(&mut game, magma_id, Vec2::new(2.0, 1.0));
    collision(
        &mut game,
        web_id,
        crate::dispatcher::Instance::Entity(magma_id),
    );
    assert!(game.world.entity(web_id).is_none());
    assert_eq!(game.world.entity(magma_id).unwrap().vel, Vec2::default());

    let mut game = room();
    let web_id = web(&mut game, Vec2::new(120.0, 100.0));
    let keeper_id = target(
        &mut game,
        "oShopkeeper",
        "Shopkeeper",
        Vec2::new(120.0, 100.0),
    );
    {
        let keeper = game.world.entity_mut(keeper_id).unwrap();
        keeper.health = 1;
        keeper.state = 0;
    }
    set_motion(&mut game, keeper_id, Vec2::new(2.0, 1.0));
    collision(
        &mut game,
        web_id,
        crate::dispatcher::Instance::Entity(keeper_id),
    );
    assert_eq!(game.world.entity(keeper_id).unwrap().state, 2);
    assert_eq!(game.world.entity(keeper_id).unwrap().vel, Vec2::default());

    let mut game = room();
    let web_id = web(&mut game, Vec2::new(120.0, 100.0));
    let keeper_id = target(
        &mut game,
        "oShopkeeper",
        "Shopkeeper",
        Vec2::new(120.0, 100.0),
    );
    {
        let keeper = game.world.entity_mut(keeper_id).unwrap();
        keeper.health = 0;
        keeper.state = 0;
    }
    collision(
        &mut game,
        web_id,
        crate::dispatcher::Instance::Entity(keeper_id),
    );
    assert_eq!(game.world.entity(keeper_id).unwrap().state, 0);
}

#[test]
fn web_collision_stops_unheld_items_and_treasure_but_not_rope_or_held_items() {
    for (object, source_type, held, should_freeze) in [
        ("oBombBag", "Bomb Bag", false, true),
        ("oRopeThrow", "Rope", false, false),
        ("oBombBag", "Bomb Bag", true, false),
    ] {
        let mut game = room();
        let web_id = web(&mut game, Vec2::new(120.0, 100.0));
        let item_id = target(&mut game, object, source_type, Vec2::new(120.0, 100.0));
        let velocity = Vec2::new(1.25, -0.75);
        set_motion(&mut game, item_id, velocity);
        game.world.entity_mut(item_id).unwrap().held = held;
        collision(
            &mut game,
            web_id,
            crate::dispatcher::Instance::Entity(item_id),
        );
        let item = game.world.entity(item_id).unwrap();
        assert_eq!(
            item.vel,
            if should_freeze {
                Vec2::default()
            } else {
                velocity
            }
        );
    }

    let mut game = room();
    let web_id = web(&mut game, Vec2::new(120.0, 100.0));
    let treasure_id = target(&mut game, "oRuby", "Ruby", Vec2::new(120.0, 100.0));
    set_motion(&mut game, treasure_id, Vec2::new(1.25, -0.75));
    collision(
        &mut game,
        web_id,
        crate::dispatcher::Instance::Entity(treasure_id),
    );
    assert_eq!(game.world.entity(treasure_id).unwrap().vel, Vec2::default());
}

#[test]
fn web_collision_freezes_rubble_and_destroys_web_on_laser_or_slash() {
    let mut game = room();
    let web_id = web(&mut game, Vec2::new(120.0, 100.0));
    let rubble_id = game.world.spawn("oRubble", Vec2::new(120.0, 100.0));
    {
        let rubble = game.world.entity_mut(rubble_id).unwrap();
        rubble.vel = Vec2::new(1.25, -0.75);
        rubble.set_image_speed(0.3);
    }
    collision(
        &mut game,
        web_id,
        crate::dispatcher::Instance::Entity(rubble_id),
    );
    let rubble = game.world.entity(rubble_id).unwrap();
    assert_eq!(rubble.vel, Vec2::default());
    assert_eq!(rubble.image_speed(), 0.0);

    for object in ["oLaser", "oSlash"] {
        let mut game = room();
        let web_id = web(&mut game, Vec2::new(120.0, 100.0));
        let other_id = game.spawn(object, Vec2::new(120.0, 100.0));
        collision(
            &mut game,
            web_id,
            crate::dispatcher::Instance::Entity(other_id),
        );
        assert!(game.world.entity(web_id).is_none(), "{object}");
    }
}
