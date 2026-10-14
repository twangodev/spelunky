//! Configured own-event/native selector boundaries, not generated basketball.
use super::*;

fn game(rules: RulesPreset, algorithm: RngAlgorithm) -> Game {
    let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rMoon".into();
    game.flags.extend(["hub".into(), "hub_moon".into()]);
    game.data.insert("baskets".into(), 17.0);
    game.data.insert("hub_alarm0".into(), 7.0);
    game
}

fn faults(game: &Game, field: &str) -> usize {
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
fn native_nearest_uses_strict_squared_sentinel_and_first_active_family_ties() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            for distance in [99_999.0, 100_000.0, 100_001.0] {
                let mut game = game(rules, algorithm);
                let target = game.spawn("oBasket", Vec2::new(distance, 0.0));
                assert_eq!(
                    nearest_basketball_target(&game, Vec2::default(), "oBasket"),
                    if distance < 100_000.0 {
                        Some(target)
                    } else {
                        None
                    }
                );
            }
            for first_active in [false, true] {
                let mut game = game(rules, algorithm);
                let first = game.spawn("oRimDeflect", Vec2::new(3.0, 4.0));
                game.world.entity_mut(first).unwrap().engine_active = first_active;
                let second = game.spawn("oRimDeflect", Vec2::new(-3.0, 4.0));
                let retired = game.spawn("oRimDeflect", Vec2::default());
                game.world.entity_mut(retired).unwrap().alive = false;
                let inactive = game.spawn("oRimDeflect", Vec2::default());
                game.world.entity_mut(inactive).unwrap().engine_active = false;
                game.spawn("oRim", Vec2::default());
                assert_eq!(
                    nearest_basketball_target(&game, Vec2::default(), "oRimDeflect"),
                    Some(if first_active { first } else { second })
                );
            }
        }
    }
}

#[test]
fn distant_scaled_deflector_gates_but_noone_x_fault_preserves_only_source_prefix() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            let mut game = game(rules, algorithm);
            let deflector = game.spawn("oRimDeflect", Vec2::new(-99_840.0, 101.0));
            let target = game.world.entity_mut(deflector).unwrap();
            target.data.insert("image_xscale".into(), 10_000.0);
            target.data.insert("image_yscale".into(), 2.0);
            // Origin is exactly100000 away, while the scaled source bbox gates.
            assert!(source_rectangle_bbox(
                &game,
                Vec2::new(159.0, 105.0),
                Vec2::new(161.0, 106.0),
                "oRimDeflect"
            ));
            assert_eq!(
                nearest_basketball_target(&game, Vec2::new(160.0, 101.0), "oRimDeflect"),
                None
            );
            game.spawn("oRim", Vec2::new(156.0, 100.0));
            let basket = game.spawn("oBasket", Vec2::new(156.0, 104.0));
            let ball = game.spawn("oBasketball", Vec2::new(160.0, 101.0));
            let entity = game.world.entity_mut(ball).unwrap();
            entity.vel = Vec2::new(2.0, 1.0);
            entity.held = true;
            entity.data.insert("colLeft".into(), 1.0);
            entity.data.insert("colBot".into(), 1.0);
            entity.data.insert("bounces".into(), 3.0);
            entity.set_sprite("sDribble");
            game.player.state = PlayerState::Falling;
            game.player.held = Some(ball);
            let rng = game.rng.state_bits();
            // Own event boundary only: inherited action has already supplied
            // this cached contact/velocity snapshot; no full Step claim.
            step_instance(&mut game, ball);
            let entity = game.world.entity(ball).unwrap();
            assert_eq!(entity.value("wallBounce"), 1.0);
            assert_eq!(entity.vel, Vec2::new(2.0, -0.8));
            assert_eq!(entity.value("obj"), -4.0);
            assert_eq!(
                entity.value("bounces"),
                3.0,
                "fault precedes BOTH increment and decrement"
            );
            assert_eq!(entity.value("goingIn"), 0.0);
            assert_eq!(entity.sprite, "sDribble", "final sprite tail unexecuted");
            assert!(entity.held);
            assert_eq!(game.player.held, Some(ball));
            assert_eq!(game.data["baskets"], 17.0);
            assert_eq!(game.data["hub_alarm0"], 7.0);
            assert_eq!(game.world.entity(basket).unwrap().sprite, "sBasket");
            assert_eq!(game.data["source_runtime_errors"], 1.0);
            assert_eq!(faults(&game, "obj.x"), 1);
            assert_eq!(faults(&game, "obj.sprite_index"), 0);
            assert_eq!(game.rng.state_bits(), rng);
        }
    }
}

#[test]
fn basket_at_finite_nearest_cutoff_faults_after_rim_prefix_before_credit_and_hand_clear() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            for distance in [99_999.0, 100_000.0, 100_001.0] {
                let mut game = game(rules, algorithm);
                game.spawn("oRim", Vec2::new(156.0, 100.0));
                let basket = game.spawn("oBasket", Vec2::new(160.0 + distance, 101.0));
                let ball = game.spawn("oBasketball", Vec2::new(160.0, 101.0));
                let entity = game.world.entity_mut(ball).unwrap();
                entity.vel = Vec2::new(2.0, 1.0);
                entity.held = true;
                entity.data.insert("bounces".into(), 1.0);
                entity.set_sprite("sDribble");
                game.player.state = PlayerState::Falling;
                game.player.held = Some(ball);
                let rng = game.rng.state_bits();
                step_instance(&mut game, ball);
                let entity = game.world.entity(ball).unwrap();
                assert_eq!(entity.vel, Vec2::default());
                assert_eq!(entity.value("goingIn"), 1.0);
                assert_eq!(
                    entity.value("bounces"),
                    0.0,
                    "decrement precedes scoring prefix"
                );
                assert_eq!(game.data["hub_alarm0"], 20.0);
                if distance < 100_000.0 {
                    assert_eq!(entity.value("obj"), basket as f64);
                    assert!(!entity.held);
                    assert_eq!(entity.sprite, "sBasketball");
                    assert_eq!(game.player.held, None);
                    assert_eq!(game.data["baskets"], 18.0);
                    assert_eq!(game.world.entity(basket).unwrap().sprite, "sBasketSwoosh");
                    assert_eq!(faults(&game, "obj.sprite_index"), 0);
                } else {
                    assert_eq!(entity.value("obj"), -4.0);
                    assert!(entity.held);
                    assert_eq!(entity.sprite, "sDribble");
                    assert_eq!(game.player.held, Some(ball));
                    assert_eq!(game.data["baskets"], 17.0);
                    assert_eq!(game.world.entity(basket).unwrap().sprite, "sBasket");
                    assert_eq!(game.data["source_runtime_errors"], 1.0);
                    assert_eq!(faults(&game, "obj.sprite_index"), 1);
                }
                assert_eq!(game.rng.state_bits(), rng);
            }
        }
    }
}
