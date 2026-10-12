//! Default source-initialized King birth in an authored flat room.
//! Only setup precedes Step; subsequent gameplay uses neutral ordinary inputs.
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

fn room(rules: RulesPreset, algorithm: RngAlgorithm, player: Vec2) -> (Game, u64) {
    room_at(rules, algorithm, player, Vec2::new(80.0, 128.0))
}

#[test]
fn default_idle_lead_survives_until_walk_assigns_its_speed() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            // Above: neither guard qualifies; dy from King+16 exceeds32;
            // turn dy from King+32 exceeds16. Both stay inside the view gate.
            let (mut game, id) = room(rules, algorithm, Vec2::new(192.0, 72.0));
            let rng = game.rng.state_bits();
            game.step(&InputFrame::default());
            assert_eq!(game.engine_time(), 2);
            let king = game.world.entity(id).unwrap();
            assert_eq!(king.pos, Vec2::new(83.0, 128.0), "TIME2_2_5_MOVES3");
            assert_eq!(king.state, 1, "NATURAL_IDLE_TO_WALK");
            let idle_exit_vx = king.vel.x;
            assert_eq!(king.health, 30);
            assert_eq!(game.player.health, 4);

            game.step(&InputFrame::default());
            assert_eq!(game.engine_time(), 3);
            let king = game.world.entity(id).unwrap();
            assert_eq!(king.pos, Vec2::new(85.0, 128.0), "TIME3_2_5_MOVES2");
            assert_eq!(idle_exit_vx, 2.5, "IDLE_HAS_NO_HORIZONTAL_WRITE");
            assert_eq!((king.state, king.vel.x), (1, 1.0), "WALK_RETAINS_OWN_SPEED");

            game.step(&InputFrame::default());
            assert_eq!(game.engine_time(), 4);
            let king = game.world.entity(id).unwrap();
            assert_eq!(king.pos, Vec2::new(86.0, 128.0), "WALK_NEXT_MOVE_IS1");
            assert_eq!((king.health, king.state, king.vel.x), (30, 1, 1.0));
            assert_eq!(game.player.health, 4);
            assert_eq!(game.rng.state_bits(), rng, "NO_SOURCE_RANDOM_PRODUCER");
        }
    }
}

#[test]
fn naturally_entered_attack_and_turn_keep_their_own_velocity_writes() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            for (player, state, sprite) in [
                (Vec2::new(192.0, 152.0), 3, "sYetiKingYellR"),
                (Vec2::new(32.0, 152.0), 2, "sYetiKingTurnL"),
            ] {
                let (mut game, id) = room(rules, algorithm.clone(), player);
                let rng = game.rng.state_bits();
                game.step(&InputFrame::default());
                assert_eq!(game.world.entity(id).unwrap().state, 1);
                game.step(&InputFrame::default());
                let king = game.world.entity(id).unwrap();
                assert_eq!(king.state, state, "ORDINARY_PLAYER_GEOMETRY_STATE_ENTRY");
                assert_eq!(king.sprite, sprite);
                assert_eq!(king.pos, Vec2::new(85.0, 128.0), "SOURCE_SECOND_LEAD_MOVE");
                assert_eq!(king.vel.x, if state == 3 { 0.0 } else { 2.5 });
                let position = king.pos;
                // Inspect the actual ATTACK/TURN branch, rather than treating
                // its preceding WALK transition as that branch's velocity oracle.
                game.step(&InputFrame::default());
                let king = game.world.entity(id).unwrap();
                if state == 3 {
                    assert_eq!(king.pos, position, "ATTACK_ENTRY_STOPS_BEFORE_NEXT_MOVE");
                } else {
                    assert_eq!(
                        king.pos,
                        Vec2::new(88.0, 128.0),
                        "TURN_MOVES_INCOMING2_5_BEFORE_STOP"
                    );
                }
                assert_eq!((king.health, king.state, king.vel.x), (30, state, 0.0));
                assert_eq!(king.sprite, sprite);
                assert_eq!(game.player.health, 4);
                if state == 3 {
                    assert_eq!(king.value("attack_timer"), 100.0);
                    assert_eq!(king.image_speed(), 0.5);
                } else {
                    assert_eq!(king.value("counter"), 30.0);
                }
                assert_eq!(game.rng.state_bits(), rng, "BEFORE_YELL_ICE_WINDOW");
            }
        }
    }
}

#[test]
fn source_tomb_lord_fly_hit_preserves_the_kings_attack_sprite() {
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
fn configured_initial_negative_and_fractional_counters_keep_literal_source_values() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            for value in [-1.0, 0.5] {
                let (mut game, id) = room(rules, algorithm.clone(), Vec2::new(192.0, 152.0));
                // Explicit controlled INITIAL metadata; no ordinary producer
                // or campaign reachability claim for these counter values.
                for key in ["counter", "attack_timer", "hit_cooldown"] {
                    game.world
                        .entity_mut(id)
                        .unwrap()
                        .data
                        .insert(key.into(), value);
                }
                let expected = if value > 0.0 { value - 1.0 } else { value };
                game.step(&InputFrame::default());
                let king = game.world.entity(id).unwrap();
                assert_eq!(king.state, 1, "IDLE_COUNTER_LE_ZERO");
                assert_eq!(king.vel.x, 2.5);
                for key in ["counter", "attack_timer", "hit_cooldown"] {
                    assert_eq!(king.value(key), expected, "SOURCE_POSITIVE_ONLY_DECREMENT");
                }
                game.step(&InputFrame::default());
                let king = game.world.entity(id).unwrap();
                assert_eq!((king.health, king.state, king.vel.x), (30, 1, 1.0));
                for key in ["counter", "attack_timer", "hit_cooldown"] {
                    assert_eq!(
                        king.value(key),
                        expected,
                        "NEGATIVE_VALUES_ARE_NOT_NORMALIZED"
                    );
                }
                assert_eq!(game.player.health, 4);
            }
        }
    }
}

#[test]
fn natural_walk_checks_the_wall_beside_its_final_position() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            let (mut game, id) = room(rules, algorithm, Vec2::new(192.0, 72.0));
            // Initial authored wall, no edits after the first actual Step.
            // King right bound starts106, then109/111/112 at times2/3/4.
            game.world.set_tile(7, 8, "oHardBlock");
            let rng = game.rng.state_bits();
            for x in [83.0, 85.0] {
                game.step(&InputFrame::default());
                let king = game.world.entity(id).unwrap();
                assert_eq!(king.pos, Vec2::new(x, 128.0));
                assert_eq!((king.health, king.state), (30, 1));
            }
            game.step(&InputFrame::default());
            let king = game.world.entity(id).unwrap();
            assert_eq!(king.pos, Vec2::new(86.0, 128.0));
            assert_eq!(king.state, 2, "FRESH_RIGHT_LINE_SEES_WALL112");
            assert_eq!(king.sprite, "sYetiKingTurnL");
            assert_eq!(king.value("counter"), 30.0);
            assert_eq!(king.vel.x, 1.0, "TURN_ENTRY_RETAINS_WALK_VELOCITY");
            game.step(&InputFrame::default());
            let king = game.world.entity(id).unwrap();
            assert_eq!(king.pos, Vec2::new(86.0, 128.0), "WALL_BLOCKS_INCOMING1");
            assert_eq!((king.health, king.state, king.vel.x), (30, 2, 0.0));
            assert_eq!(king.sprite, "sYetiKingTurnL");
            assert_eq!(game.player.health, 4);
            assert_eq!(game.rng.state_bits(), rng);
        }
    }
}
