//! Source oPlayer1 Step's Item / else-if Enemy pickup selection.
use spelunky_core::{Game, InputFrame, RulesPreset, Vec2, level::Level};

fn room(rules: RulesPreset, caveman: bool) -> Game {
    let mut level = Level::default();
    level.set_tile(6, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    if caveman {
        level.set_tile(8, 7, 'h').unwrap();
    }
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "enemy-pickup-contract", "");
    game
}
fn pickup() -> InputFrame {
    InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    }
}
fn naturally_stunned(rules: RulesPreset) -> (Game, u64) {
    let mut game = room(rules, true);
    let id = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oCaveman")
        .unwrap()
        .id;
    for tick in 0..60 {
        let enemy = game.world.entity(id).unwrap();
        let right = tick > 22 && enemy.pos.x > game.player.pos.x + 4.0;
        let left = tick > 22 && enemy.pos.x < game.player.pos.x - 6.0;
        game.step(&InputFrame {
            right,
            left,
            attack: tick == 4,
            attack_pressed: tick == 4,
            ..Default::default()
        });
    }
    let enemy = game.world.entity(id).unwrap();
    assert_eq!(
        (enemy.health, enemy.state),
        (2, 98),
        "ordinary whip must stun a living caveman"
    );
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.attack_timer, 0);
    assert_eq!(game.player.held, None);
    assert!(enemy.pos.distance(game.player.pos) < 12.0);
    (game, id)
}
#[test]
fn naturally_whipped_caveman_can_be_picked_up_carried_and_thrown() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = naturally_stunned(rules);
        game.step(&pickup());
        assert_eq!(
            game.player.held,
            Some(id),
            "Down+Attack must select stunned Enemy fallback"
        );
        assert!(game.world.entity(id).unwrap().held);
        let before = game.player.pos.x;
        for _ in 0..12 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            let enemy = game.world.entity(id).unwrap();
            assert_eq!(game.player.held, Some(id));
            assert!(enemy.held);
            assert_eq!(
                enemy.pos,
                Vec2::new(game.player.pos.x - 4.0, game.player.pos.y - 12.0)
            );
            assert_eq!(game.player.health, 4);
        }
        assert!(game.player.pos.x > before + 12.0);
        let thrown_from = game.world.entity(id).unwrap().pos.x;
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, None);
        let enemy = game.world.entity(id).unwrap();
        assert!(!enemy.held);
        assert!(enemy.vel.x > 3.0);
        assert!(enemy.vel.y < 0.0);
        for _ in 0..6 {
            game.step(&InputFrame::default());
        }
        assert!(game.world.entity(id).unwrap().pos.x > thrown_from + 20.0);
        assert_eq!(game.player.health, 4);
    }
}

// These fixtures author initial actor state before the first simulation tick;
// the acquisition itself always goes through ordinary Game::step input.
fn body(game: &mut Game, object: &str, state: i32) -> u64 {
    let id = game.spawn(
        object,
        Vec2::new(game.player.pos.x - 4.0, game.player.pos.y - 8.0),
    );
    let enemy = game.world.entity_mut(id).unwrap();
    enemy.state = state;
    enemy.health = if state == 99 { 0 } else { 2 };
    enemy.timer = 200;
    enemy.vel = Vec2::default();
    id
}
#[test]
fn inherited_pickup_permission_preserves_boss_exclusions() {
    for object in [
        "oSnake",
        "oSpider",
        "oBat",
        "oCaveman",
        "oManTrap",
        "oYeti",
        "oHawkman",
        "oShopkeeper",
    ] {
        let mut game = room(RulesPreset::Classic, false);
        let id = body(&mut game, object, 98);
        assert_eq!(
            game.world.entity(id).unwrap().value("canPickUp"),
            1.0,
            "{object} inherits Enemy Create permission"
        );
    }
    for object in ["oYetiKing", "oTombLord", "oJaws", "oAlienBoss"] {
        let mut game = room(RulesPreset::Classic, false);
        let id = body(&mut game, object, 98);
        assert_eq!(
            game.world.entity(id).unwrap().value("canPickUp"),
            0.0,
            "{object} own Create overrides parent"
        );
        game.step(&pickup());
        assert_eq!(
            game.player.held, None,
            "{object} cannot be carried even when stunned"
        );
    }
}
#[test]
fn dead_caveman_is_pickable_but_awake_and_explicitly_unpickable_are_not() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (state, permission, expected) in
            [(99, true, true), (0, true, false), (98, false, false)]
        {
            let mut game = room(rules, false);
            let id = body(&mut game, "oCaveman", state);
            if !permission {
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("canPickUp".into(), 0.0);
            }
            game.player.invincible = 120;
            game.step(&pickup());
            assert_eq!(
                game.player.held,
                expected.then_some(id),
                "state={state}, permission={permission}"
            );
        }
    }
}
#[test]
fn item_overlap_preempts_enemy_even_when_nearest_item_cannot_be_picked_up() {
    for can_pick_up in [true, false] {
        let mut game = room(RulesPreset::Classic, false);
        let enemy = body(&mut game, "oCaveman", 98);
        let rock = game.spawn(
            "oRock",
            Vec2::new(game.player.pos.x, game.player.pos.y + 4.0),
        );
        game.world
            .entity_mut(rock)
            .unwrap()
            .data
            .insert("canPickUp".into(), f64::from(can_pick_up));
        game.step(&pickup());
        assert_eq!(game.player.held, can_pick_up.then_some(rock));
        assert!(!game.world.entity(enemy).unwrap().held);
    }
}
#[test]
fn enemy_nearest_is_global_and_single_ineligible_result_blocks_other_body() {
    let mut game = room(RulesPreset::Classic, false);
    let eligible = body(&mut game, "oCaveman", 98);
    game.world.entity_mut(eligible).unwrap().pos.x = game.player.pos.x - 20.0;
    let nearest = game.spawn(
        "oCaveman",
        Vec2::new(game.player.pos.x, game.player.pos.y - 17.0),
    );
    game.world.entity_mut(nearest).unwrap().state = 0;
    let area = spelunky_core::Rect::new(game.player.pos.x - 8.0, game.player.pos.y, 17.0, 9.0);
    let candidate = game.world.entity(eligible).unwrap();
    let awake = game.world.entity(nearest).unwrap();
    assert!(candidate.mask_intersects(area, spelunky_core::CollisionPrecision::BoundingBox));
    assert!(!awake.mask_intersects(area, spelunky_core::CollisionPrecision::BoundingBox));
    assert!(awake.pos.distance(game.player.pos) < candidate.pos.distance(game.player.pos));
    game.player.invincible = 120;
    game.step(&pickup());
    assert_eq!(game.player.held, None);
    assert!(!game.world.entity(eligible).unwrap().held);
}
#[test]
fn enemy_rectangle_gate_is_required_and_equal_distance_keeps_creation_order() {
    let mut far = room(RulesPreset::Classic, false);
    let id = body(&mut far, "oCaveman", 98);
    far.world.entity_mut(id).unwrap().pos.x += 80.0;
    far.step(&pickup());
    assert_eq!(far.player.held, None);

    let mut tied = room(RulesPreset::Classic, false);
    let first = body(&mut tied, "oCaveman", 98);
    let second = body(&mut tied, "oCaveman", 98);
    tied.world.entity_mut(second).unwrap().pos.x += 8.0;
    assert_eq!(
        tied.world
            .entity(first)
            .unwrap()
            .pos
            .distance(tied.player.pos),
        tied.world
            .entity(second)
            .unwrap()
            .pos
            .distance(tied.player.pos)
    );
    tied.step(&pickup());
    assert_eq!(tied.player.held, Some(first));
    assert!(!tied.world.entity(second).unwrap().held);
}
#[test]
fn invincible_enemy_does_not_gain_an_unsourced_pickup_exclusion() {
    let mut game = room(RulesPreset::Classic, false);
    let id = body(&mut game, "oCaveman", 98);
    game.world
        .entity_mut(id)
        .unwrap()
        .data
        .insert("invincible".into(), 100.0);
    game.step(&pickup());
    assert_eq!(game.player.held, Some(id));
}
