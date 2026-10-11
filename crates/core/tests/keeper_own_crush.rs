//! Proposed public Step fixtures. UNCOMPILED/UNRUN. Only setup writes precede
//! the first Step. The Dice/PushBlock route uses fresh Create state throughout;
//! authored DEAD fixtures below are explicitly controlled branch tests.
use spelunky_core::*;

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::legacy_shell(42, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(40.0, 120.0);
    game.player.previous_pos = game.player.pos;
    game
}
fn births<'a>(game: &'a Game, object: &str) -> Vec<&'a Entity> {
    let mut result: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .collect();
    result.sort_by_key(|e| e.creation_serial);
    result
}

#[test]
fn proposed_natural_dice_theft_then_falling_block_keeps_first_attack_prices() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for hit in [false, true] {
            let mut game = room(rules);
            // Keeper's floor ends before a separate Dice shaft. Shop row0
            // ends at y144; gravity carries the initially in-shop die across it.
            for x in 0..=7 {
                game.world.set_tile(x, 8, "oBlock");
            }
            let keeper = game.spawn("oShopkeeper", Vec2::new(96.0, 112.0));
            let block_x = if hit { 96.0 } else { 192.0 };
            let block = game.spawn("oPushBlock", Vec2::new(block_x, 93.0));
            let die = game.spawn("oDice", Vec2::new(144.0, 141.0));
            let stock = game.spawn("oJetpack", Vec2::new(400.0, 120.0));
            {
                let item = game.world.entity_mut(stock).unwrap();
                item.data.insert("cost".into(), 20000.0);
                item.data.insert("forSale".into(), 1.0);
            }
            let actor = game.world.entity(keeper).unwrap();
            assert_eq!(
                (actor.health, actor.state, actor.value("angered")),
                (20, 0, 0.0)
            );
            assert_eq!(actor.vel, Vec2::default());
            assert!(!game.world.solid_at(104.0, 120.0), "NO_INITIAL_OWN_CRUSH");
            assert!(
                game.world.entity(block).unwrap().pos.y + 15.0 < actor.pos.y,
                "NO_INITIAL_BLOCK_BODY_OVERLAP"
            );
            let geometry: Vec<_> = game.world.tiles.iter().map(|t| t.object.clone()).collect();
            let mut armed = false;
            for _ in 0..20 {
                game.step(&InputFrame::default());
                assert!(game.player.health > 0, "CONTROLLER_SURVIVES_TO_THEFT");
                let actor = game
                    .world
                    .entity(keeper)
                    .expect("NO_EARLY_CRUSH_BEFORE_THEFT");
                if actor.state == 2 {
                    assert_eq!(
                        actor.value("angered"),
                        0.0,
                        "THEFT_AFTER_KEEPER_NORMAL_STEP"
                    );
                    assert_eq!(actor.health, 20, "NO_DAMAGE_OR_STATE_SETUP");
                    assert!(
                        game.world.entity(die).unwrap().pos.y >= 144.0,
                        "DICE_NATURALLY_CROSSED_ROOM_BOUNDARY"
                    );
                    assert_eq!(game.data.get("thiefLevel"), Some(&2.0));
                    assert!(
                        game.world.entity(block).unwrap().pos.y > 93.0,
                        "BLOCK_MOVED_BY_NORMAL_GLOBAL_GRAVITY"
                    );
                    assert_eq!(
                        game.world.entity(stock).unwrap().value("cost"),
                        20000.0,
                        "FIRST_ATTACK_LATCH_HAS_NOT_RUN"
                    );
                    assert!(game.world.entity(stock).unwrap().engine_active);
                    armed = true;
                    break;
                }
                assert_eq!(actor.state, 0, "ROUTE_REMAINS_FRESH_IDLE_UNTIL_DICE_THEFT");
            }
            assert!(armed, "NATURAL_THEFT_PRECONDITION_MUST_QUALIFY");
            // The proposed timing puts the next pre-tick block descent through
            // the keeper's center before its first ATTACK body. No repair or
            // midpoint position/state/health/RNG writes are permitted.
            game.step(&InputFrame::default());
            if hit {
                assert!(
                    game.world.entity(keeper).is_none(),
                    "NATURAL_OWN_CRUSH_MUST_QUALIFY"
                );
                assert!(
                    game.world.solid_at(104.0, 120.0),
                    "FALLING_BLOCK_CROSSED_OWN_CENTER"
                );
                assert_eq!(game.data.get("kills"), Some(&1.0));
                assert_eq!(game.lifetime_stats.enemy_kills[19], 1);
            } else {
                let actor = game.world.entity(keeper).unwrap();
                assert_eq!(
                    (actor.health, actor.state, actor.value("angered")),
                    (20, 2, 1.0)
                );
                assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
            }
            // Differential target comes last: early route failures are fixture
            // failures, never an accepted old-arm continuation result.
            assert_eq!(
                game.world.entity(stock).unwrap().value("cost"),
                0.0,
                "POST_CRUSH_FIRST_ATTACK_MUST_CLEAR_ACTIVE_STOCK"
            );
            assert_eq!(game.world.entity(stock).unwrap().value("forSale"), 0.0);
            assert_eq!(
                game.world
                    .tiles
                    .iter()
                    .map(|t| t.object.clone())
                    .collect::<Vec<_>>(),
                geometry
            );
        }
    }
}

#[test]
fn authored_dead_crush_keeps_later_dead_credit_gold_and_stars_rules() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for stars in [false, true] {
            let mut game = room(rules);
            if stars {
                game.world.name = "rStars".into();
                game.flags.insert("hub_stars".into());
            }
            for y in 5..=6 {
                for x in 6..=7 {
                    game.world.set_tile(x, y, "oBlock");
                }
            }
            let keeper = game.spawn("oShopkeeper", Vec2::new(96.0, 80.0));
            // Controlled initial DEAD branch, not a natural damage journey.
            let actor = game.world.entity_mut(keeper).unwrap();
            actor.health = 0;
            actor.state = 99;
            game.step(&InputFrame::default());
            assert!(game.world.entity(keeper).is_none());
            assert_eq!(births(&game, "oBlood").len(), 3);
            assert!(
                births(&game, "oBlood")
                    .iter()
                    .all(|e| e.pos == Vec2::new(104.0, 88.0) && e.value("life") == 60.0)
            );
            let gold = births(&game, "oGoldNugget");
            if stars {
                assert!(gold.is_empty());
                assert_eq!(
                    game.data.get("stars_kills"),
                    Some(&1.0),
                    "CRUSH_DID_NOT_PRELATCH_DEAD"
                );
                assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
                assert_eq!(game.lifetime_stats.enemy_kills[19], 0);
            } else {
                assert!(
                    (1..=4).contains(&gold.len()),
                    "CRUSH_CONTINUES_INTO_DEAD_LOOT"
                );
                assert!(
                    gold.iter()
                        .all(|e| e.pos == Vec2::new(104.0, 88.0) && e.vel.y == -1.0),
                    "GOLD461_CREATE_NOW_STEP_NEXT_FRAME"
                );
                assert_eq!(game.data.get("kills"), Some(&1.0));
                assert_eq!(game.data.get("shopkeepers"), Some(&1.0));
                assert_eq!(game.lifetime_stats.enemy_kills[19], 1);
            }
            assert_eq!(
                game.events
                    .iter()
                    .filter(|event| matches!(event,
                GameEvent::Sound { name, .. } if name == "sndCavemanDie"))
                    .count(),
                2,
                "CRUSH_SOUND_THEN_FIRST_DEAD_SOUND"
            );
        }
    }
}

#[test]
fn uncrushed_idle_and_dead_controls_preserve_live_corpse_lifecycle() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for dead in [false, true] {
            let mut game = room(rules);
            for x in 0..30 {
                game.world.set_tile(x, 8, "oBlock");
            }
            let keeper = game.spawn("oShopkeeper", Vec2::new(96.0, 112.0));
            if dead {
                let actor = game.world.entity_mut(keeper).unwrap();
                actor.health = 0;
                actor.state = 99;
            }
            game.step(&InputFrame::default());
            let actor = game.world.entity(keeper).unwrap();
            assert!(actor.alive, "ORDINARY_DEAD_BRANCH_RETAINS_CORPSE");
            assert_eq!(actor.value("enemy_destroyed"), 0.0);
            assert_eq!(actor.value("dead"), f64::from(dead));
            assert_eq!(actor.state, if dead { 99 } else { 0 });
            assert_eq!(
                game.data.get("kills").copied().unwrap_or(0.0),
                f64::from(dead)
            );
            assert_eq!(births(&game, "oBlood").len(), 0);
            assert_eq!(births(&game, "oShotgun").len(), usize::from(dead));
            if dead {
                assert!((1..=4).contains(&births(&game, "oGoldNugget").len()));
            } else {
                assert!(births(&game, "oGoldNugget").is_empty());
            }
        }
    }
}
