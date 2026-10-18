//! Explicit initial active-family reentry order, followed by ordinary neutral
//! Game.step. This is a configured receiver-order test, not a campaign route.
use spelunky_core::{
    Game, GameEvent, InputFrame, RulesPreset, Vec2, catalog,
    dispatcher::{self, Instance},
    level::Level,
};

fn room(rules: RulesPreset) -> Game {
    let mut level = Level::default();
    level.set_tile(5, 6, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 7, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "supply-family-contact", "");
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.player.pos, Vec2::new(104.0, 120.0));
    assert!(game.player.grounded);
    assert_eq!(game.player.health, 4.0);
    game
}

#[test]
fn configured_registered_supply_reentry_selects_first_family_hit_before_eligibility() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, bombs, ropes, sprite, offset, message) in [
            ("oBombBag", 3, 0, "sBombsGet", 14.0, "YOU GOT 3 MORE BOMBS!"),
            (
                "oBombBox",
                12,
                0,
                "sBombsGet",
                14.0,
                "YOU GOT 12 MORE BOMBS!",
            ),
            ("oRopePile", 0, 3, "sRopeGet", 15.0, "YOU GOT 3 MORE ROPES!"),
        ] {
            for reentered in [false, true] {
                let mut game = room(rules);
                let paid = game.spawn(object, Vec2::new(106.0, 120.0));
                let free = game.spawn(object, Vec2::new(108.0, 120.0));
                // Initial authored free loot is the same caller override as
                // source room scripts. The earlier paid receiver stays paid.
                game.world
                    .entity_mut(free)
                    .unwrap()
                    .data
                    .insert("cost".into(), 0.0);
                assert!(game.world.entity(paid).unwrap().value("cost") > 0.0);
                assert_eq!(
                    dispatcher::descendant_instances(&game, object),
                    vec![Instance::Entity(paid), Instance::Entity(free)]
                );
                if reentered {
                    // Model a registered native-family reentry before inputs.
                    // Entity storage and creation order remain [paid,free].
                    let first = game
                        .dispatcher
                        .active_order
                        .iter()
                        .position(|i| *i == Instance::Entity(paid))
                        .unwrap();
                    let second = game
                        .dispatcher
                        .active_order
                        .iter()
                        .position(|i| *i == Instance::Entity(free))
                        .unwrap();
                    game.dispatcher.active_order.swap(first, second);
                    game.dispatcher.registration_order = game.dispatcher.active_order.clone();
                }
                let expected = if reentered {
                    vec![Instance::Entity(free), Instance::Entity(paid)]
                } else {
                    vec![Instance::Entity(paid), Instance::Entity(free)]
                };
                assert_eq!(dispatcher::descendant_instances(&game, object), expected);
                let storage: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == object)
                    .map(|e| e.id)
                    .collect();
                assert_eq!(storage, vec![paid, free]);
                for id in [paid, free] {
                    let e = game.world.entity(id).unwrap();
                    assert!(e.engine_active && !e.held);
                    assert!(!game.world.solid_at(e.pos.x, e.pos.y));
                    let mask = catalog::instance_mask(
                        &e.object,
                        &e.sprite,
                        e.pos,
                        e.animation,
                        Vec2::new(1.0, 1.0),
                        0.0,
                    );
                    assert!(mask.intersects_rectangle_bbox(
                        Vec2::new(96.0, 112.0),
                        Vec2::new(112.0, 128.0)
                    ));
                }
                let before = (game.player.bombs, game.player.ropes, game.player.money);
                let before_rng = game.rng.state_bits();
                game.step(&InputFrame::default());
                assert_eq!(game.player.pos, Vec2::new(104.0, 120.0));
                assert_eq!(game.player.health, 4.0);
                assert!(!game.player.is_stunned());
                assert_eq!(game.rng.state_bits(), before_rng);
                assert!(game.world.entity(paid).is_some());
                assert_eq!(
                    game.world.entity(free).is_none(),
                    reentered,
                    "SOURCE_FIRST_SUPPLY_RECEIVER {rules:?} {object}"
                );
                assert_eq!(
                    (game.player.bombs, game.player.ropes, game.player.money),
                    (
                        before.0 + if reentered { f64::from(bombs) } else { 0.0 },
                        before.1 + if reentered { f64::from(ropes) } else { 0.0 },
                        before.2
                    )
                );
                let displays: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oItemsGet")
                    .collect();
                assert_eq!(displays.len(), usize::from(reentered));
                if reentered {
                    assert_eq!(displays[0].sprite, sprite);
                    // Source ItemsGet Step applies ceil(y-0.1), so the
                    // integer source Create origin stays exact this frame.
                    assert_eq!(displays[0].pos, Vec2::new(108.0, 120.0 - offset));
                }
                let pickup = game.events.iter().position(|e| {
                    matches!(e,
                    GameEvent::Sound {name,..} if name=="sndPickup")
                });
                let dialogue = game.events.iter().position(|e| {
                    matches!(e,
                    GameEvent::Dialogue(m) if m.first==message && m.second.is_empty())
                });
                assert_eq!(pickup.is_some(), reentered);
                assert_eq!(dialogue.is_some(), reentered);
                if reentered {
                    assert!(pickup.unwrap() < dialogue.unwrap());
                }
                assert_eq!(game.events.iter().filter(|e|matches!(e,
                    GameEvent::Collected {object:collected,value} if collected==object && *value==0))
                    .count(),usize::from(reentered));
                assert_eq!(game.player.held, None);
                assert!(!game.events.iter().any(|e| matches!(e,
                    GameEvent::Sound {name,..} if name=="sndBreak")));
            }
        }
    }
}

#[test]
fn configured_inactive_supply_cannot_run_the_player_collection_query() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oBombBag", "oBombBox", "oRopePile"] {
            let mut game = room(rules);
            let id = game.spawn(object, game.player.pos);
            let item = game.world.entity_mut(id).unwrap();
            item.data.insert("cost".into(), 0.0);
            item.engine_active = false;
            let before = (game.player.bombs, game.player.ropes, game.player.money);
            game.events.clear();
            // Direct Player collection callback control. A full Game.step
            // would legitimately reactivate this visible receiver first.
            spelunky_core::items::collectibles::player_step(&mut game);
            assert!(game.world.entity(id).is_some());
            assert_eq!(
                (game.player.bombs, game.player.ropes, game.player.money),
                before
            );
            assert!(!game.events.iter().any(|e| matches!(e,
                GameEvent::Sound {name,..} if name=="sndPickup")));
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oItemsGet")
            );
        }
    }
}

#[test]
fn authored_fractional_supply_uses_cached_bbox_touch_with_positive_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (x, touches) in [(118.25, true), (118.75, false), (119.25, false)] {
            let mut game = room(rules);
            let id = game.spawn("oBombBag", Vec2::new(x, 120.0));
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("cost".into(), 0.0);
            let item = game.world.entity(id).unwrap();
            assert!(item.engine_active && !item.held);
            assert_eq!(item.sprite, "sBombBag");
            assert!(!game.world.solid_at(item.pos.x, item.pos.y));
            let mask = catalog::instance_mask(
                &item.object,
                &item.sprite,
                item.pos,
                item.animation,
                Vec2::new(1.0, 1.0),
                0.0,
            );
            // sBombBag origin8 and aggregate left2: native leftRTE(x-6).
            assert_eq!(
                mask.intersects_rectangle_bbox(Vec2::new(96.0, 112.0), Vec2::new(112.0, 128.0)),
                touches
            );
            let before = game.player.bombs;
            game.step(&InputFrame::default());
            assert_eq!(game.player.pos, Vec2::new(104.0, 120.0));
            assert_eq!(game.player.health, 4.0);
            assert_eq!(
                game.world.entity(id).is_none(),
                touches,
                "SOURCE_CACHED_SUPPLY_TOUCH {rules:?} x{x}"
            );
            assert_eq!(game.player.bombs, before + if touches { 3.0 } else { 0.0 });
            assert_eq!(
                game.events
                    .iter()
                    .filter(|e| matches!(e,
                GameEvent::Sound {name,..} if name=="sndPickup"))
                    .count(),
                usize::from(touches)
            );
        }
    }
}

#[test]
fn configured_supply_origin_uses_object_family_not_port_movement_flag() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oBombBag", "oBombBox", "oRopePile"] {
            for context in ["empty", "emerald", "configured_gem_motion_flag", "solid"] {
                let mut game = room(rules);
                let position = Vec2::new(112.0, 120.0);
                if context == "solid" {
                    // Initial source Solid tile contains the pickup origin
                    // while remaining outside the player's x99..109 body.
                    game.world.set_tile(7, 7, "oHardBlock");
                    assert!(game.world.solid_at(position.x, position.y));
                }
                let gem = if context == "emerald" || context == "configured_gem_motion_flag" {
                    let id = game.spawn("oEmerald", position);
                    let e = game.world.entity(id).unwrap();
                    // Entity::new uses catalog::is_solid = is_a(oSolid),
                    // not Original's builtin solid descriptor. A default
                    // Emerald is therefore a positive compatibility control.
                    assert!(e.engine_active && !e.solid);
                    assert!(catalog::is_a(&e.object, "oTreasure"));
                    assert!(!catalog::is_a(&e.object, "oSolid"));
                    assert_eq!(e.value("canCollect"), 0.0);
                    assert_eq!(e.value("value"), 200.0);
                    assert_eq!(e.alarms[0], 20);
                    assert!(
                        e.mask_contains(position, spelunky_core::CollisionPrecision::BoundingBox)
                    );
                    if context == "configured_gem_motion_flag" {
                        // Explicit initial port movement-flag discriminator,
                        // not default Game.spawn or a claimed source producer.
                        // Native collision_point(oSolid) filters object family;
                        // its result is independent of builtin solid property.
                        game.world.entity_mut(id).unwrap().solid = true;
                        assert!(game.world.entity(id).unwrap().solid);
                    }
                    Some(id)
                } else {
                    None
                };
                let id = game.spawn(object, position);
                // Literal initial source room caller override, not a buy action.
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("cost".into(), 0.0);
                let e = game.world.entity(id).unwrap();
                assert!(e.engine_active && !e.held);
                let mask = catalog::instance_mask(
                    &e.object,
                    &e.sprite,
                    e.pos,
                    e.animation,
                    Vec2::new(1.0, 1.0),
                    0.0,
                );
                assert!(
                    mask.intersects_rectangle_bbox(Vec2::new(96.0, 112.0), Vec2::new(112.0, 128.0))
                );
                let before = (game.player.bombs, game.player.ropes, game.player.money);
                game.step(&InputFrame::default());
                let collect = context != "solid";
                assert_eq!(game.player.pos, Vec2::new(104.0, 120.0));
                assert_eq!(game.player.health, 4.0);
                assert!(!game.player.is_stunned());
                assert_eq!(
                    game.world.entity(id).is_none(),
                    collect,
                    "SOURCE_SOLID_FAMILY_ELIGIBILITY {rules:?} {object} {context}"
                );
                assert_eq!(
                    game.player.bombs,
                    before.0
                        + if collect {
                            match object {
                                "oBombBag" => 3.0,
                                "oBombBox" => 12.0,
                                _ => 0.0,
                            }
                        } else {
                            0.0
                        }
                );
                assert_eq!(
                    game.player.ropes,
                    before.1
                        + if collect && object == "oRopePile" {
                            3.0
                        } else {
                            0.0
                        }
                );
                assert_eq!(game.player.money, before.2);
                assert_eq!(game.data.get("collect").copied().unwrap_or(0.0), 0.0);
                assert_eq!(
                    game.events
                        .iter()
                        .filter(|e| matches!(e,
                    GameEvent::Sound {name,..} if name=="sndPickup"))
                        .count(),
                    usize::from(collect)
                );
                assert!(!game.events.iter().any(|e| matches!(e,
                    GameEvent::Sound {name,..} if name=="sndCoin" || name=="sndGem")));
                if let Some(gem) = gem {
                    let gem = game
                        .world
                        .entity(gem)
                        .expect("unready gem survives supply pickup");
                    assert_eq!(gem.value("canCollect"), 0.0);
                    assert_eq!(gem.alarms[0], 19);
                    assert_eq!(game.data.get("emeralds").copied().unwrap_or(0.0), 0.0);
                }
            }
        }
    }
}
