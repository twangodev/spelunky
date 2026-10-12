//! Source Fly counter assignment; natural route and explicitly initial-phase controls.
use spelunky_core::*;

fn room_at(rules: RulesPreset, algorithm: RngAlgorithm, player: Vec2, origin: Vec2) -> (Game, u64) {
    let mut game = Game::shell_with_algorithm(0, rules, algorithm).unwrap();
    game.world = World::new(30, 20, Area::Ice, 9);
    for x in 0..30 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = player;
    game.player.previous_pos = player;
    let id = game.spawn("oYetiKing", origin);
    let king = game.world.entity(id).unwrap();
    assert_eq!(king.pos, origin);
    assert_eq!(king.vel, Vec2::new(2.5, 0.0));
    assert_eq!((king.health, king.state, king.facing), (30, 0, 1));
    assert_eq!(king.value("counter"), 0.0);
    assert_eq!(king.value("attack_timer"), 0.0);
    assert_eq!(king.image_speed(), 0.25);
    assert_eq!(game.engine_time(), 1);
    (game, id)
}

#[test]
fn natural_tomb_fly_writes_the_kings_source_counter() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room_at(
            rules,
            RngAlgorithm::Gm8Lcg32V1,
            Vec2::new(192.0, 152.0),
            Vec2::new(120.0, 128.0),
        );
        let tomb = game.spawn("oTombLord", Vec2::new(80.0, 128.0));
        let producer = game.world.entity(tomb).unwrap();
        assert_eq!(
            (producer.health, producer.state, producer.facing),
            (20, 0, 1)
        );
        assert_eq!(producer.vel, Vec2::new(2.5, 0.0));
        let mut first_fly = None;
        let mut hit = false;
        for _ in 0..28 {
            let before = game.world.entity(id).unwrap().clone();
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, 4, "NO_PLAYER_CONTACT_HIDING_ENEMY_HIT");
            if first_fly.is_none() {
                if let Some(fly) = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oFly")
                {
                    assert_eq!(
                        game.world.entity(tomb).unwrap().state,
                        3,
                        "NATURAL_TOMB_ATTACK_PRODUCER"
                    );
                    assert_eq!(fly.vel.x, 5.0, "SOURCE_FIRST_FLY_CALLER_LAUNCH");
                    assert_eq!(
                        fly.vel.y,
                        0.5 - 3.0 * (134775814.0 / 4294967296.0),
                        "SOURCE_FIRST_FLY_CREATE"
                    );
                    first_fly = Some(fly.id);
                }
            }
            let king = game.world.entity(id).unwrap();
            if king.health < before.health {
                assert_eq!(
                    (before.health, before.state),
                    (30, 3),
                    "REAL_KING_ATTACK_BEFORE_FLY"
                );
                assert_eq!(before.sprite, "sYetiKingYellR");
                assert_eq!(
                    (king.health, king.state),
                    (28, 98),
                    "ACTUAL_SINGLE_FLY_ENEMY_COLLISION"
                );
                assert_eq!(king.sprite, "sYetiKingYellR");
                assert_eq!(king.value("counter"), 20.0, "FLY_WRITES_SOURCE_COUNTER");
                let first = first_fly.expect("NATURAL_FLY_BIRTH_PRECEDES_HIT");
                assert!(game.world.entity(first).is_none(), "SOURCE_FLY_RETIRED");
                assert!(
                    game.events
                        .iter()
                        .any(|e| matches!(e, GameEvent::Sound{name,..} if name == "sndHit"))
                );
                game.step(&InputFrame::default());
                let king = game.world.entity(id).unwrap();
                assert_eq!((king.health, king.state), (28, 98));
                assert_eq!(
                    king.value("counter"),
                    20.0,
                    "STUNNED_HAS_NO_COUNTER_DECREMENT"
                );
                assert_eq!(
                    king.sprite, "sYetiKingYellR",
                    "STUNNED_HAS_NO_OWN_SPRITE_WRITE"
                );
                assert_eq!(game.player.health, 4);
                hit = true;
                break;
            }
        }
        assert!(hit, "BOUNDED_NATURAL_TOMB_FLY_KING_HIT_NOT_REACHED");
    }
}

#[test]
fn initial_turn_animation_end_retains_fly_counter_and_unhit_counter() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            for hit in [false, true] {
                let (mut game, id) = room_at(
                    rules,
                    algorithm.clone(),
                    Vec2::new(192.0, 72.0),
                    Vec2::new(120.0, 128.0),
                );
                // Explicit controlled INITIAL phase, not a generated-fight claim.
                // Source TURN retains counter30; its last frame changes only
                // status/facing. No actor writes follow the first Game::step.
                let king = game.world.entity_mut(id).unwrap();
                king.state = 2;
                king.vel = Vec2::default();
                king.set_sprite("sYetiKingTurnL");
                king.animation = 4.75;
                king.data.insert("counter".into(), 30.0);
                if hit {
                    let tomb = game.spawn("oTombLord", Vec2::new(96.0, 128.0));
                    let emitter = game.world.entity_mut(tomb).unwrap();
                    emitter.state = 3;
                    emitter.vel = Vec2::default();
                    emitter.set_sprite("sTombLordAttackR");
                    // Last inclusive source firing frame: one natural Fly
                    // Create/caller launch; the next12.5 frame emits none.
                    emitter.animation = 12.0;
                }
                game.step(&InputFrame::default());
                let king = game.world.entity(id).unwrap();
                assert_eq!(game.player.health, 4, "NO_PLAYER_CONTACT");
                assert_eq!(
                    king.health,
                    if hit { 28 } else { 30 },
                    "EMITTER_CONTACT_PREREQUISITE"
                );
                assert_eq!(
                    (king.state, king.facing),
                    (1, -1),
                    "TURN_ANIMATION_END_CHANGES_STATUS_ONLY"
                );
                assert_eq!(king.sprite, "sYetiKingTurnL");
                assert_eq!(
                    king.value("counter"),
                    if hit { 20.0 } else { 30.0 },
                    "TURN_END_RETAINS_SOURCE_COUNTER"
                );
                game.step(&InputFrame::default());
                let king = game.world.entity(id).unwrap();
                assert_eq!(
                    king.health,
                    if hit { 28 } else { 30 },
                    "NO_SECOND_FLY_CONTACT"
                );
                assert_eq!(king.state, 1);
                assert_eq!(
                    king.value("counter"),
                    if hit { 19.0 } else { 29.0 },
                    "WALK_DECREMENTS_RETAINED_SOURCE_COUNTER"
                );
            }
        }
    }
}
