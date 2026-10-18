//! Configured initial scenes, one actual neutral Game.step per case.
//! Player Step1719–1737 gates hurt by rectangle, then separately chooses
//! instance_nearest from the current active family. Native nearest's strict
//! improvement keeps the first family entry on equal origin distances.
//! Activation/direction/active0/bloodless1 are initial fixture authoring;
//! this is not an input-only encounter or original-runtime replay claim.
use spelunky_core::{
    dispatcher::{self, Instance},
    *,
};

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|rules| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (rules, algorithm))
        })
}

fn game(rules: RulesPreset, algorithm: RngAlgorithm) -> Game {
    let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
    game.player.pos = Vec2::new(160.0, 160.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.health = 40.0;
    game.player.data.insert("active".into(), 0.0);
    game.player.data.insert("bloodless".into(), 1.0);
    game.world.entrance = game.player.pos;
    game.set_simulation_view_extent(
        game.world.width as f64 * 16.0,
        game.world.height as f64 * 16.0,
    )
    .unwrap();
    let sprite = catalog::sprite_def("sSmashTrap").unwrap();
    assert_eq!((sprite.origin, sprite.bounds), ([0, 0], [0, 0, 15, 15]));
    game
}

fn trap(game: &mut Game, point: Vec2, direction: f64) -> u64 {
    let id = game.spawn("oSmashTrap", point);
    let actor = game.world.entity_mut(id).unwrap();
    assert_eq!(actor.state, 0);
    assert_eq!(actor.vel, Vec2::default());
    assert_eq!(actor.value("moving_context_initialized"), 1.0);
    actor.data.insert("direction".into(), direction);
    id
}

fn active_family(game: &Game) -> Vec<Instance> {
    dispatcher::descendant_instances(game, "oSmashTrap")
        .into_iter()
        .filter(|instance| dispatcher::is_active(game, *instance))
        .collect()
}

fn ordered_scene(rules: RulesPreset, algorithm: RngAlgorithm, b_x: f64) -> (Game, u64, u64, u64) {
    let mut game = game(rules, algorithm);
    // Birth/storage order C,B,A. Sequential deactivation appends A then B to
    // the inactive source array; reactivation yields active C,A,B. No Vec
    // reversal or dispatcher-array authoring establishes this difference.
    let c = trap(&mut game, Vec2::new(304.0, 166.0), 0.0);
    let b = trap(&mut game, Vec2::new(b_x, 166.0), 0.0);
    let a = trap(&mut game, Vec2::new(148.0, 166.0), 1.0);
    game.world.entity_mut(a).unwrap().engine_active = false;
    dispatcher::activate_region(&mut game, Rect::new(300.0, 162.0, 24.0, 24.0));
    game.world.entity_mut(b).unwrap().engine_active = false;
    dispatcher::activate_region(&mut game, Rect::new(300.0, 162.0, 24.0, 24.0));
    dispatcher::activate_object(&mut game, "oSmashTrap");
    assert_eq!(
        active_family(&game),
        vec![
            Instance::Entity(c),
            Instance::Entity(a),
            Instance::Entity(b)
        ]
    );
    let storage: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|actor| actor.object == "oSmashTrap")
        .map(|actor| actor.id)
        .collect();
    assert_eq!(storage, vec![c, b, a]);
    // Raw source rectangle is154,154..166,167. A's y166 lower edge enters
    // that gate, but Player center160,160 lies above every trap: no Crush.
    // B's origin170/172 is beyond x166, so B never owns the gate.
    assert_eq!(game.world.entity(a).unwrap().pos, Vec2::new(148.0, 166.0));
    assert!(b_x > 166.0);
    assert!(
        game.world
            .entities
            .iter()
            .all(|actor| { actor.object != "oSmashTrap" || actor.pos.y > game.player.pos.y })
    );
    (game, a, b, c)
}

fn hurt_once(game: &mut Game, expected_x_sign: f64, expected_y: f64) {
    game.step(&InputFrame::default());
    assert_eq!(
        game.player.pos,
        Vec2::new(160.0, 160.0),
        "active0 skips only controller motion"
    );
    assert_eq!(
        game.player.health, 30.0,
        "one Smash hurt, no Crush or unrelated hit"
    );
    assert_eq!(
        game.events
            .iter()
            .filter(|event| matches!(event, GameEvent::Damage { amount: 10 }))
            .count(),
        1
    );
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { amount: 99 }))
    );
    assert!((4.0..=6.0).contains(&game.player.vel.x.abs()));
    assert_eq!(
        game.player.vel.x.signum(),
        expected_x_sign,
        "SMASH_NEAREST_SOURCE_FAMILY_MUST_SELECT_HORIZONTAL_KNOCKBACK"
    );
    assert_eq!(
        game.player.vel.y, expected_y,
        "SMASH_NEAREST_SOURCE_FAMILY_MUST_SELECT_INITIAL_DIRECTION"
    );
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|actor| actor.object == "oBlood")
    );
}

#[test]
fn equal_origin_distance_smash_hurt_keeps_first_reactivated_active_family_entry() {
    for (rules, algorithm) in modes() {
        let (mut game, a, b, _) = ordered_scene(rules, algorithm, 172.0);
        // Independent integer squared distances to160,160: both12²+6².
        assert_eq!(game.world.entity(a).unwrap().pos.x - 160.0, -12.0);
        assert_eq!(game.world.entity(b).unwrap().pos.x - 160.0, 12.0);
        hurt_once(&mut game, -1.0, 4.0);
    }
}

#[test]
fn strictly_nearer_smash_wins_even_when_only_farther_actor_owns_hurt_gate() {
    for (rules, algorithm) in modes() {
        let (mut game, _, b, _) = ordered_scene(rules, algorithm, 170.0);
        // B:10²+6²=136 < A:12²+6²=180, despite source order C,A,B.
        assert_eq!(game.world.entity(b).unwrap().pos, Vec2::new(170.0, 166.0));
        hurt_once(&mut game, 1.0, -6.0);
    }
}

#[test]
fn far_smash_without_any_rectangle_gate_does_not_hurt() {
    for (rules, algorithm) in modes() {
        let mut game = game(rules, algorithm);
        let far = trap(&mut game, Vec2::new(304.0, 166.0), 1.0);
        assert_eq!(active_family(&game), vec![Instance::Entity(far)]);
        game.step(&InputFrame::default());
        assert_eq!(game.player.pos, Vec2::new(160.0, 160.0));
        assert_eq!(game.player.health, 40.0);
        assert_eq!(game.player.vel, Vec2::default());
        assert!(
            !game
                .events
                .iter()
                .any(|event| matches!(event, GameEvent::Damage { .. }))
        );
    }
}

#[test]
fn initially_inactive_closer_smash_is_skipped_by_player_hurt_before_level_reactivation() {
    for (rules, algorithm) in modes() {
        let (mut game, a, b, c) = ordered_scene(rules, algorithm, 170.0);
        game.world.entity_mut(b).unwrap().engine_active = false;
        dispatcher::activate_region(&mut game, Rect::new(300.0, 162.0, 24.0, 24.0));
        assert_eq!(
            active_family(&game),
            vec![Instance::Entity(c), Instance::Entity(a)]
        );
        assert!(
            game.dispatcher
                .hidden_entities()
                .iter()
                .any(|actor| actor.id == b && actor.alive && !actor.engine_active)
        );
        // Player319 runs before Level553. The later Level activation may
        // reactivate B, but cannot change the Player's already selected hurt.
        hurt_once(&mut game, -1.0, 4.0);
    }
}

#[test]
fn raw_world_newborn_smash_create_and_registration_complete_before_player_nearest_hurt() {
    for (rules, algorithm) in modes() {
        let mut game = game(rules, algorithm);
        let far = trap(&mut game, Vec2::new(304.0, 166.0), 0.0);
        let a = trap(&mut game, Vec2::new(148.0, 166.0), 1.0);
        assert_eq!(
            active_family(&game),
            vec![Instance::Entity(far), Instance::Entity(a)]
        );
        // Initial raw birth follows the registered actors. Actual Game.step
        // finishes pending Create and registers it before Player319. This is
        // a bootstrap/newborn control, not a late event-frame birth claim.
        let born = game.world.spawn("oSmashTrap", Vec2::new(170.0, 166.0));
        let actor = game.world.entity_mut(born).unwrap();
        assert_eq!(actor.value("moving_context_initialized"), 0.0);
        actor.data.insert("direction".into(), 0.0);
        hurt_once(&mut game, 1.0, -6.0);
        let actor = game.world.entity(born).unwrap();
        assert_eq!(actor.value("moving_context_initialized"), 1.0);
        assert!(active_family(&game).contains(&Instance::Entity(born)));
    }
}
