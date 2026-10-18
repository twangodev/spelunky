//! Full inherited Item -> Jar/Skull Destroy controls. Inconsistent hand ledgers
//! below are initial configured states, not alleged ordinary campaign states.
use spelunky_core::{Area, Game, GameEvent, InputFrame, RulesPreset, Vec2, World, items};

#[test]
fn inherited_fragile_destroy_reads_receiver_held_not_backlink_identity() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oJar", "oSkull"] {
            for pieces in [false, true] {
                for receiver_held in [false, true] {
                    for backlink in 0..3 {
                        let mut game = Game::shell(16, rules);
                        let target = game.spawn(object, Vec2::new(100.0, 100.0));
                        let unrelated = game.spawn("oRock", Vec2::new(200.0, 100.0));
                        let receiver = game.world.entity_mut(target).unwrap();
                        receiver.held = receiver_held;
                        receiver
                            .data
                            .insert("breakPieces".into(), f64::from(pieces));
                        game.player.held = match backlink {
                            0 => None,
                            1 => Some(target),
                            2 => Some(unrelated),
                            _ => unreachable!(),
                        };
                        game.player.pickup_item_type = "remembered-source-type".into();
                        let prior_hand = game.player.held;
                        let prior_rng = game.rng.state_bits();
                        let first_birth = game.world.next_id;
                        let unrelated_before =
                            serde_json::to_value(game.world.entity(unrelated).unwrap()).unwrap();
                        assert_eq!(game.player.health, 4.0);
                        assert!(game.player.engine_active);
                        // From this point, invoke the complete real callback once;
                        // no actor/hand/HP/RNG writes are made by the fixture.
                        items::break_item(&mut game, target);
                        assert!(game.world.entity(target).is_none());
                        assert_eq!(
                            game.player.held,
                            if receiver_held { None } else { prior_hand },
                            "SOURCE_INHERITED_ITEM_HELD {object} pieces={pieces} receiverheld={receiver_held} backlink={backlink}"
                        );
                        assert_eq!(
                            game.player.pickup_item_type, "remembered-source-type",
                            "Destroy writes pickupItem, not pickupItemType"
                        );
                        let retired = game.world.entities.iter().find(|e| e.id == target).unwrap();
                        assert_eq!(
                            retired.held, receiver_held,
                            "neither inherited nor own Destroy changes receiver held"
                        );
                        assert_eq!(
                            serde_json::to_value(game.world.entity(unrelated).unwrap()).unwrap(),
                            unrelated_before
                        );
                        assert_eq!(game.player.health, 4.0);
                        assert_eq!(
                            game.events
                                .iter()
                                .filter(|event| matches!(event,
                            GameEvent::Sound { name, .. } if name == "sndBreak"))
                                .count(),
                            usize::from(pieces)
                        );
                        if !pieces {
                            assert_eq!(game.world.next_id, first_birth);
                            assert_eq!(game.rng.state_bits(), prior_rng);
                            assert!(game.events.is_empty());
                        } else {
                            assert!(game.world.next_id > first_birth);
                            assert_eq!(
                                game.world
                                    .entities
                                    .iter()
                                    .filter(|e| e.id >= first_birth && e.object == "oSmokePuff")
                                    .count(),
                                1
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn ordinary_jar_skull_pickup_and_clean_inventory_cycle_keep_the_source_type() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, item_type) in [("oJar", "Jar"), ("oSkull", "Skull")] {
            let mut game = Game::shell(16, rules);
            game.world = World::new(30, 20, Area::Mines, 1);
            game.world.name = "rLevel".into();
            for x in 0..30 {
                game.world.set_tile(x, 10, "oHardBlock");
            }
            game.player.pos = Vec2::new(64.0, 152.0);
            game.player.previous_pos = game.player.pos;
            let old = game.spawn(object, Vec2::new(64.0, 156.0));
            // Natural support/acquisition; default Falling, HP4, inv0 and
            // zero velocity retained. No scene writes after this first input.
            for _ in 0..4 {
                game.step(&InputFrame::default());
            }
            assert!(game.player.grounded);
            assert_eq!(game.player.health, 4.0);
            assert_eq!(game.player.invincible, 0);
            assert!(!game.player.is_stunned());
            assert_eq!(game.player.pos, Vec2::new(64.0, 152.0));
            game.step(&InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.held, Some(old));
            assert!(game.world.entity(old).unwrap().held);
            game.step(&InputFrame {
                attack_released: true,
                ..Default::default()
            });
            assert_eq!(game.player.held, Some(old));
            assert!(game.world.entity(old).unwrap().held);
            assert_eq!(game.world.entity(old).unwrap().value("breakPieces"), 1.0);
            let rng = game.rng.state_bits();
            let first_birth = game.world.next_id;
            game.step(&InputFrame {
                cycle_pressed: true,
                ..Default::default()
            });
            assert!(game.world.entity(old).is_none());
            // Public Step purges retired storage after End Step. The live
            // pre-cycle held receiver and literal Player cycle producer
            // qualify breakPieces=false; zero break/RNG/birth side effects
            // below remain mandatory observable evidence of clean Destroy.
            assert!(
                !game.world.entities.iter().any(|e| e.id == old),
                "clean cycle must completely retire the old receiver"
            );
            let bomb = game.player.held.unwrap();
            assert_eq!(game.world.entity(bomb).unwrap().object, "oBomb");
            assert_eq!(game.player.pickup_item_type, item_type);
            assert_eq!((game.player.bombs, game.player.ropes), (3.0, 4.0));
            assert_eq!(
                game.rng.state_bits(),
                rng,
                "clean Destroy + fixed Bomb Create has no draws"
            );
            assert_eq!(
                game.world.next_id,
                first_birth + 1,
                "only replacement Bomb born"
            );
            assert!(!game.events.iter().any(|event| matches!(event,
                GameEvent::Sound { name, .. } if name == "sndBreak")));
            for _ in 0..2 {
                game.step(&InputFrame {
                    cycle_pressed: true,
                    ..Default::default()
                });
            }
            let fresh = game.player.held.unwrap();
            assert_ne!(fresh, old);
            let item = game.world.entity(fresh).unwrap();
            assert_eq!(item.object, object);
            assert!(item.held);
            assert_eq!(item.value("breakPieces"), 1.0);
            assert_eq!(game.player.pickup_item_type, item_type);
            assert_eq!((game.player.bombs, game.player.ropes), (4.0, 4.0));
            assert_eq!(game.player.health, 4.0);
            assert!(!game.player.is_stunned());
        }
    }
}
