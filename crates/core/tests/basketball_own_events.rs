//! Initially configured dormant Basketball/Basket/Rim events through Game::step.
//! Stock Moon does not create these objects; no gameplay-time fixture writes.
use spelunky_core::{
    Area, Game, GenerationRevision, InputFrame, PlayerState, RulesPreset, Vec2, World, hub::HubKind,
};

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(42, rules);
    game.generation_revision = GenerationRevision::SourceClassicV2;
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rMoon".into();
    game.flags.extend(["hub".into(), "hub_moon".into()]);
    for x in 0..40 {
        game.world.set_tile(x, 13, "oHardBlock");
    }
    game.player.pos = Vec2::new(24.0, 200.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    game.data.insert("timer".into(), 1000.0);
    game.data.insert("baskets".into(), 0.0);
    game
}

#[test]
fn stock_moon_keeps_basketball_objects_dormant_while_bats_are_created() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(42, rules);
        game.load_hub(HubKind::Moon);
        for _ in 0..70 {
            game.step(&InputFrame::default());
            assert!(!game.world.entities.iter().any(|e| matches!(
                e.object.as_str(),
                "oBasketball" | "oBasket" | "oRim" | "oRimDeflect"
            )));
        }
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBatTarget")
                .count(),
            3
        );
    }
}

#[test]
fn basketball_create_inherits_context_depth_once_and_preserves_producer_overrides() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for spectacles in [false, true] {
            let mut game = room(rules);
            if spectacles {
                game.player.equipment.insert("Spectacles".into());
            }
            let before = game.rng.state_bits();
            let id = game.spawn("oBasketball", Vec2::new(160.0, 100.0));
            let ball = game.world.entity(id).unwrap();
            assert_eq!(ball.value("depth"), if spectacles { 51.0 } else { 101.0 });
            assert_eq!(ball.vel, Vec2::default());
            assert_eq!(ball.value("bounceFactor"), 0.7);
            assert_eq!(ball.value("frictionFactor"), 0.6);
            assert_eq!(ball.value("bounces"), 0.0);
            assert_eq!(ball.value("goingIn"), 0.0);
            assert_eq!(game.rng.state_bits(), before);
            let ball = game.world.entity_mut(id).unwrap();
            ball.data.insert("bounceFactor".into(), 0.25);
            ball.data.insert("frictionFactor".into(), 0.9);
            ball.data.insert("bboxRight".into(), 5.0);
            game.step(&InputFrame::default());
            let ball = game.world.entity(id).unwrap();
            assert_eq!(ball.value("bounceFactor"), 0.25);
            assert_eq!(ball.value("frictionFactor"), 0.9);
            assert_eq!(ball.value("bboxRight"), 5.0);
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn raw_inclusive_deflection_gate_and_independent_nearest_family_keep_source_order() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (nearest_elsewhere, initial_bounces) in [(false, 0.0), (true, 0.0), (false, 40.0)] {
            let mut game = room(rules);
            // This deflector touches only the query's last x/y endpoint.
            let touching = game.spawn("oRimDeflect", Vec2::new(161.0, 105.0));
            let nearest = if nearest_elsewhere {
                // Closer origin, but bbox bottom103 is outside query104..105.
                game.spawn("oRimDeflect", Vec2::new(157.0, 100.0))
            } else {
                touching
            };
            let id = game.spawn("oBasketball", Vec2::new(160.0, 100.0));
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("bounces".into(), initial_bounces);
            assert!(game.world.entity(touching).unwrap().mask_contains(
                Vec2::new(161.0, 105.0),
                spelunky_core::CollisionPrecision::BoundingBox
            ));
            game.step(&InputFrame::default());
            let ball = game.world.entity(id).unwrap();
            assert_eq!(
                ball.pos,
                Vec2::new(160.0, 100.0),
                "zero incoming velocity before parent gravity"
            );
            if initial_bounces == 40.0 {
                assert_eq!(
                    ball.value("bounces"),
                    39.0,
                    "strict threshold, then decrement"
                );
                assert_eq!(ball.vel, Vec2::new(0.0, 0.6));
                assert_eq!(ball.value("obj"), 0.0);
            } else {
                assert_eq!(
                    ball.value("bounces"),
                    9.0,
                    "add10 then decrement1 exactly once"
                );
                assert_eq!(ball.value("obj"), nearest as f64);
                if nearest_elsewhere {
                    assert_eq!(ball.vel.x, 0.0);
                    assert!((ball.vel.y + 0.48).abs() < 1e-12);
                } else {
                    assert_eq!(ball.vel, Vec2::new(-2.0, -0.5));
                }
            }
            assert_eq!(game.data["baskets"], 0.0);
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn cached_wall_contact_wins_over_floor_contact_and_scores_once() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (wall, expected) in [(false, 0.0), (true, 1.0)] {
            let mut game = room(rules);
            for x in 0..40 {
                game.world.set_tile(x, 10, "oHardBlock");
            }
            if wall {
                game.world.set_tile(10, 9, "oHardBlock");
            }
            let id = game.spawn("oBasketball", Vec2::new(180.0, 156.0));
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("wallBounce".into(), 1.0 - expected);
            game.step(&InputFrame::default());
            let ball = game.world.entity(id).unwrap();
            // oItem snapshots colLeft before its final side-contact backoff.
            // The own Step consumes that snapshot after the +1 x movement.
            assert_eq!(ball.pos, Vec2::new(180.0 + f64::from(wall), 156.0));
            assert_eq!(ball.value("colBot"), 1.0);
            assert_eq!(ball.value("colLeft"), f64::from(wall));
            assert_eq!(
                ball.value("wallBounce"),
                expected,
                "literal if left/right else-if bottom"
            );
            assert_eq!(game.data["baskets"], 0.0);
            assert_eq!(game.player.health, 4);
        }
        for (wall, expected) in [(false, 2.0), (true, 3.0)] {
            let mut game = room(rules);
            let rim = game.spawn("oRim", Vec2::new(156.0, 100.0));
            let basket = game.spawn("oBasket", Vec2::new(156.0, 104.0));
            let id = game.spawn("oBasketball", Vec2::new(160.0, 100.0));
            let ball = game.world.entity_mut(id).unwrap();
            ball.vel.y = 1.0;
            ball.data.insert("wallBounce".into(), f64::from(wall));
            game.step(&InputFrame::default());
            let ball = game.world.entity(id).unwrap();
            assert_eq!(ball.pos, Vec2::new(160.0, 101.0));
            assert!(
                game.world
                    .entity(rim)
                    .unwrap()
                    .mask_contains(ball.pos, spelunky_core::CollisionPrecision::BoundingBox)
            );
            assert_eq!(ball.value("colBot"), 0.0);
            assert_eq!(ball.value("goingIn"), 1.0);
            assert_eq!(ball.vel, Vec2::default());
            assert_eq!(game.data["baskets"], expected);
            assert_eq!(game.data["hub_alarm0"], 20.0);
            assert_eq!(game.world.entity(basket).unwrap().sprite, "sBasketSwoosh");
            assert_eq!(game.world.entity(basket).unwrap().animation, 1.0);
            game.step(&InputFrame::default());
            assert_eq!(
                game.data["baskets"], expected,
                "goingIn prevents repeated credit"
            );
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn falling_held_dunk_clears_the_hand_without_requiring_a_matching_backlink() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for mismatched_hand in [false, true] {
            let mut game = room(rules);
            game.player.pos = Vec2::new(156.0, 99.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Falling;
            game.player.grounded = false;
            game.world.entrance = game.player.pos;
            game.spawn("oRim", Vec2::new(156.0, 100.0));
            let basket = game.spawn("oBasket", Vec2::new(156.0, 104.0));
            let id = game.spawn("oBasketball", Vec2::new(240.0, 80.0));
            game.world.entity_mut(id).unwrap().held = true;
            if mismatched_hand {
                let rock = game.spawn("oRock", Vec2::new(240.0, 80.0));
                game.world.entity_mut(rock).unwrap().held = true;
                game.player.held = Some(rock);
                game.player.pickup_item_type = "Rock".into();
            } else {
                game.player.held = Some(id);
                game.player.pickup_item_type = "Basketball".into();
            }
            game.step(&InputFrame::default());
            let ball = game.world.entity(id).unwrap();
            assert_eq!(
                game.player.state,
                PlayerState::Falling,
                "source state16 dunk guard"
            );
            assert_eq!(
                ball.pos,
                Vec2::new(game.player.pos.x + 4.0, game.player.pos.y + 2.0)
            );
            assert_eq!(ball.value("goingIn"), 1.0);
            assert!(!ball.held);
            assert_eq!(game.player.held, None);
            assert_eq!(
                game.player.pickup_item_type,
                if mismatched_hand {
                    "Rock"
                } else {
                    "Basketball"
                }
            );
            assert_eq!(game.data["baskets"], 1.0);
            assert_eq!(game.data["hub_alarm0"], 20.0);
            assert_eq!(game.world.entity(basket).unwrap().sprite, "sBasketSwoosh");
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn basket_animation_end_restores_default_sprite_without_inheriting_item_body() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let id = game.spawn("oBasket", Vec2::new(156.0, 104.0));
        let basket = game.world.entity_mut(id).unwrap();
        basket.set_sprite("sBasketSwoosh");
        basket.animation = 5.5;
        basket.set_image_speed(1.0);
        game.step(&InputFrame::default());
        let basket = game.world.entity(id).unwrap();
        assert_eq!(basket.sprite, "sBasket");
        assert_eq!(
            basket.animation, 0.5,
            "native advance before callback, assignment preserves valid fraction"
        );
        assert_eq!(basket.pos, Vec2::new(156.0, 104.0));
        assert_eq!(basket.vel, Vec2::default());
        let depth = basket
            .data
            .get("depth")
            .copied()
            .unwrap_or_else(|| spelunky_core::catalog::object("oBasket").unwrap().depth as f64);
        assert_eq!(depth, 99.0, "root object's builtin catalog depth");
        assert!(!basket.data.contains_key("gravity"));
        assert!(!basket.data.contains_key("bboxLeft"));
        assert_eq!(game.player.health, 4);
    }
}

#[test]
fn rim_and_nearest_basket_queries_exclude_inactive_and_other_families() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for active_rim in [false, true] {
            let mut game = room(rules);
            let rim = game.spawn("oRim", Vec2::new(156.0, 100.0));
            game.world.entity_mut(rim).unwrap().engine_active = active_rim;
            // A same-point Basket is not a Rim, and is not query-visible to nearest.
            let inactive_basket = game.spawn("oBasket", Vec2::new(160.0, 101.0));
            game.world
                .entity_mut(inactive_basket)
                .unwrap()
                .engine_active = false;
            let basket = game.spawn("oBasket", Vec2::new(180.0, 104.0));
            let id = game.spawn("oBasketball", Vec2::new(160.0, 100.0));
            game.world.entity_mut(id).unwrap().vel.y = 1.0;
            assert_eq!(game.world.entity(rim).unwrap().engine_active, active_rim);
            assert!(!game.world.entity(inactive_basket).unwrap().engine_active);
            game.step(&InputFrame::default());
            let ball = game.world.entity(id).unwrap();
            assert_eq!(ball.pos, Vec2::new(160.0, 101.0));
            assert_eq!(ball.value("goingIn"), f64::from(active_rim));
            assert_eq!(game.data["baskets"], if active_rim { 2.0 } else { 0.0 });
            assert_eq!(
                game.world.entity(basket).unwrap().sprite,
                if active_rim {
                    "sBasketSwoosh"
                } else {
                    "sBasket"
                }
            );
            if active_rim {
                assert_eq!(ball.value("obj"), basket as f64);
            }
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn inherited_held_attachment_precedes_own_dribble_point_query() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for supported in [false, true] {
            let mut game = room(rules);
            if !supported {
                for x in 0..40 {
                    game.world.set_tile(x, 13, "");
                }
                for x in 0..40 {
                    game.world.set_tile(x, 14, "oHardBlock");
                }
            }
            let id = game.spawn("oBasketball", Vec2::new(160.0, 100.0));
            game.world.entity_mut(id).unwrap().held = true;
            game.player.held = Some(id);
            game.player.pickup_item_type = "Basketball".into();
            game.step(&InputFrame::default());
            let ball = game.world.entity(id).unwrap();
            assert!(ball.held);
            assert_eq!(game.player.held, Some(id));
            assert_eq!(
                ball.pos,
                Vec2::new(game.player.pos.x + 4.0, game.player.pos.y + 2.0)
            );
            assert_eq!(ball.vel, Vec2::default());
            assert_eq!(
                ball.sprite,
                if supported { "sDribble" } else { "sBasketball" }
            );
            assert_eq!(
                ball.value("depth"),
                0.0,
                "linked Item depth after Player EndStep"
            );
            assert_eq!(game.data["baskets"], 0.0);
            assert_eq!(game.player.health, 4);
        }
    }
}
