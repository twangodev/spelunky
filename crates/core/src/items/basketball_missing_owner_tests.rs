//! Configured dormant own-Step error paths, not ordinary stock Moon scenes.
//! Existing utility Step entry publishes prior child writes even on block abort.
use super::*;

fn configured(
    rules: RulesPreset,
    algorithm: RngAlgorithm,
    controller: bool,
    basket_mode: u8,
    held: bool,
) -> (Game, u64, Option<u64>) {
    let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rMoon".into();
    if controller {
        game.flags.extend(["hub".into(), "hub_moon".into()]);
    }
    game.player.pos = Vec2::new(24.0, 200.0);
    game.player.previous_pos = game.player.pos;
    game.data.insert("baskets".into(), 17.0);
    game.data.insert("hub_alarm0".into(), 7.0);
    game.spawn("oRim", Vec2::new(156.0, 100.0));
    let basket = if basket_mode == 0 {
        None
    } else {
        let id = game.spawn("oBasket", Vec2::new(156.0, 104.0));
        let e = game.world.entity_mut(id).unwrap();
        e.engine_active = basket_mode != 2;
        e.alive = basket_mode != 3;
        Some(id)
    };
    let ball = game.spawn("oBasketball", Vec2::new(160.0, 101.0));
    let e = game.world.entity_mut(ball).unwrap();
    e.vel = Vec2::new(2.0, 1.0);
    e.held = held;
    e.data.insert("bounces".into(), 1.0);
    e.data.insert("wallBounce".into(), 1.0);
    // Reachable prior-frame dribble art proves the final sprite write is skipped.
    e.set_sprite("sDribble");
    if held {
        game.player.held = Some(ball);
        game.player.pickup_item_type = "Basketball".into();
    }
    assert!(source_point_bbox(&game, Vec2::new(160.0, 101.0), "oRim"));
    assert!(!source_rectangle_bbox(
        &game,
        Vec2::new(159.0, 105.0),
        Vec2::new(161.0, 106.0),
        "oRimDeflect"
    ));
    assert_eq!(game.world.entity(ball).unwrap().value("goingIn"), 0.0);
    (game, ball, basket)
}

fn errors(game: &Game, field: &str) -> usize {
    game.events
        .iter()
        .filter(|event| {
            matches!(event,
        GameEvent::Message(message) if message.contains("SOURCE RUNTIME ERROR: oBasketball Step")
            && message.contains(field) && message.contains("code block aborted"))
        })
        .count()
}

#[test]
fn missing_basket_keeps_prefix_and_stops_before_moon_credit_held_and_sprite_tail() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            for controller in [false, true] {
                for mode in [0, 2, 3] {
                    for held in [false, true] {
                        let (mut game, ball, basket) =
                            configured(rules, algorithm, controller, mode, held);
                        let rng = game.rng.state_bits();
                        step_instance(&mut game, ball);
                        let e = game.world.entity(ball).unwrap();
                        assert_eq!(e.vel, Vec2::default());
                        assert_eq!(e.value("goingIn"), 1.0);
                        assert_eq!(e.value("bounces"), 0.0);
                        assert_eq!(e.value("obj"), -4.0);
                        assert_eq!(e.held, held);
                        assert_eq!(
                            e.sprite, "sDribble",
                            "post-fault final sprite assignment is suppressed"
                        );
                        assert_eq!(game.player.held, if held { Some(ball) } else { None });
                        assert_eq!(game.data["baskets"], 17.0);
                        assert_eq!(game.data["hub_alarm0"], if controller { 20.0 } else { 7.0 });
                        assert_eq!(
                            game.data
                                .get("source_runtime_errors")
                                .copied()
                                .unwrap_or(0.0),
                            1.0,
                            "MISSING_BASKET_ABORT {rules:?} {algorithm:?}"
                        );
                        assert_eq!(errors(&game, "obj.sprite_index"), 1);
                        assert_eq!(
                            errors(&game, "oMoonRoom.baskets"),
                            0,
                            "later error is not reached"
                        );
                        if let Some(id) = basket {
                            let e = game.world.entities.iter().find(|e| e.id == id).unwrap();
                            assert_eq!(e.sprite, "sBasket");
                        }
                        assert_eq!(game.rng.state_bits(), rng);
                    }
                }
            }
        }
    }
}

#[test]
fn missing_moon_controller_keeps_basket_effects_before_fault_and_does_not_clear_hand() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            for held in [false, true] {
                let (mut game, ball, basket) = configured(rules, algorithm, false, 1, held);
                let rng = game.rng.state_bits();
                step_instance(&mut game, ball);
                let e = game.world.entity(ball).unwrap();
                assert_eq!(e.vel, Vec2::default());
                assert_eq!(e.value("goingIn"), 1.0);
                assert_eq!(e.value("bounces"), 0.0);
                assert_eq!(e.value("obj"), basket.unwrap() as f64);
                assert_eq!(e.held, held);
                assert_eq!(e.sprite, "sDribble");
                assert_eq!(game.player.held, if held { Some(ball) } else { None });
                assert_eq!(
                    game.data["baskets"], 17.0,
                    "virtual data does not invent a controller instance"
                );
                assert_eq!(
                    game.data["hub_alarm0"], 7.0,
                    "empty with-body made no alarm write"
                );
                let basket = game.world.entity(basket.unwrap()).unwrap();
                assert_eq!(basket.sprite, "sBasketSwoosh");
                assert_eq!(
                    basket.animation, 0.0,
                    "Basket writes precede missing Moon getter"
                );
                assert_eq!(
                    game.data
                        .get("source_runtime_errors")
                        .copied()
                        .unwrap_or(0.0),
                    1.0,
                    "MISSING_MOON_CONTROLLER_ABORT {rules:?} {algorithm:?}"
                );
                assert_eq!(errors(&game, "oMoonRoom.baskets"), 1);
                assert_eq!(game.rng.state_bits(), rng);
            }
        }
    }
}

#[test]
fn valid_owners_complete_tail_and_fault_is_not_repeated_without_reentering_score_branch() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            for held in [false, true] {
                let (mut game, ball, _) = configured(rules, algorithm, true, 1, held);
                let rng = game.rng.state_bits();
                step_instance(&mut game, ball);
                let e = game.world.entity(ball).unwrap();
                assert!(!e.held);
                assert_eq!(e.sprite, "sBasketball");
                assert_eq!(game.player.held, None);
                assert_eq!(game.data["baskets"], if held { 18.0 } else { 20.0 });
                assert_eq!(
                    game.data
                        .get("source_runtime_errors")
                        .copied()
                        .unwrap_or(0.0),
                    0.0
                );
                assert_eq!(game.rng.state_bits(), rng);
            }
            let (mut game, ball, _) = configured(rules, algorithm, false, 0, false);
            step_instance(&mut game, ball);
            assert_eq!(game.data["source_runtime_errors"], 1.0);
            step_instance(&mut game, ball);
            let e = game.world.entity(ball).unwrap();
            assert_eq!(
                game.data["source_runtime_errors"], 1.0,
                "goingIn prevents another invalid assignment"
            );
            assert_eq!(
                e.sprite, "sBasketball",
                "next Step still runs; no invented fatal halt"
            );
            assert_eq!(game.data["baskets"], 17.0);
        }
    }
}
