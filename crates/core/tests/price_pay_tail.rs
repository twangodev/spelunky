//! Initial shop actors/stock only, then ordinary pickup and Pay inputs.
//! Source Player Step1400–1498 and2143–2188. No later price/held/HP writes.
use spelunky_core::{Area, Game, GameEvent, InputFrame, RulesPreset, Vec2, World};

fn shop(rules: RulesPreset, facing: i8, style: &str, money: u32) -> (Game, u64) {
    let mut game = Game::shell(23, rules);
    game.world = World::new(30, 24, Area::Mines, 1);
    game.world.name = "rLevel".into();
    assert!(
        game.is_level() && game.is_real_level(),
        "real level gates statistics"
    );
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(104.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.facing = facing;
    game.player.money = money;
    game.world.entrance = game.player.pos;
    let keeper = game.spawn("oShopkeeper", Vec2::new(144.0, 144.0));
    // Source room generation chooses the keeper style before ordinary play.
    game.world
        .entity_mut(keeper)
        .unwrap()
        .text
        .insert("style".into(), style.into());
    (game, keeper)
}

fn stock(game: &mut Game, object: &str, sale: bool) -> u64 {
    let id = game.spawn(object, game.player.pos);
    let item = game.world.entity_mut(id).unwrap();
    // q/High End Set leaves native positive cost and default forSale=false;
    // ordinary sale stock differs only in its initial forSale=true marker.
    assert!(item.value("cost") > 0.0, "native own Create supplies price");
    assert_eq!(item.value("forSale"), 0.0);
    item.data.insert("inDiceHouse".into(), f64::from(!sale));
    if sale {
        item.data.insert("forSale".into(), 1.0);
    }
    id
}

fn pickup(game: &mut Game, id: u64, price: f64, sale: bool) {
    for _ in 0..3 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(id), "SETUP: ordinary stock pickup");
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    let item = game.world.entity(id).unwrap();
    assert!(item.held);
    assert_eq!(
        item.value("cost"),
        price,
        "shop retains native price through pickup"
    );
    assert_eq!(item.value("forSale"), f64::from(sale));
    assert_eq!(game.player.health, 4);
    assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
}

fn pay(game: &mut Game) {
    game.step(&InputFrame {
        pay_pressed: true,
        ..Default::default()
    });
}

fn assert_post_draw_ttl(game: &Game, written: f64) {
    assert!(!game.world.dark, "fixture exercises bright-level Draw");
    let remaining = written - 1.0;
    assert_eq!(game.data.get("message_timer"), Some(&remaining));
    let draw = game.draw_view.as_ref().expect("source Draw snapshot");
    assert_eq!(draw.data.get("source_message_draw_timer"), Some(&written));
}

fn last_dialogue(game: &Game) -> (&str, &str) {
    game.events
        .iter()
        .rev()
        .find_map(|event| match event {
            GameEvent::Dialogue(message) => Some((message.first.as_str(), message.second.as_str())),
            _ => None,
        })
        .expect("source Pay dialogue")
}

#[test]
fn resource_contact_requires_zero_cost_not_absent_sale_marker() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for awarded in [false, true] {
                let (mut game, keeper) = shop(rules, facing, "Craps", 7000);
                let prize = stock(&mut game, "oBombBox", false);
                assert_eq!(game.world.entity(prize).unwrap().value("cost"), 10000.0);
                if awarded {
                    // Initial authored awarded-prize snapshot: source settlement
                    // clears cost/forSale and inDiceHouse before moving old stock.
                    let item = game.world.entity_mut(prize).unwrap();
                    item.data.insert("cost".into(), 0.0);
                    item.data.insert("inDiceHouse".into(), 0.0);
                }
                game.step(&InputFrame::default());
                assert_eq!(game.player.bombs, if awarded { 16 } else { 4 });
                assert_eq!(
                    game.world.entity(prize).is_some(),
                    !awarded,
                    "positive-price no-sale casino prize must not auto-collect"
                );
                assert_eq!(game.player.money, 7000);
                assert_eq!(game.player.health, 4);
                assert_eq!(game.world.entity(keeper).unwrap().health, 20);
                assert_eq!(game.player.held, None);
                if !awarded {
                    let item = game.world.entity(prize).unwrap();
                    assert_eq!(item.value("cost"), 10000.0);
                    assert_eq!(item.value("forSale"), 0.0);
                    assert_eq!(item.value("inDiceHouse"), 1.0);
                }
            }
        }
    }
}

#[test]
fn pay_positive_cost_stock_then_continues_craps_for_purchase_and_failure() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for sale in [false, true] {
                for sufficient in [false, true] {
                    let (mut game, keeper) = shop(rules, facing, "Craps", 7000);
                    let (object, price) = if sufficient {
                        ("oPistol", 5000.0)
                    } else {
                        ("oJetpack", 20000.0)
                    };
                    let item = stock(&mut game, object, sale);
                    pickup(&mut game, item, price, sale);
                    pay(&mut game);
                    assert_eq!(game.player.money, if sufficient { 500 } else { 5500 });
                    assert_eq!(
                        game.player.data.get("bet"),
                        Some(&1500.0),
                        "same Pay edge still starts Craps bet"
                    );
                    assert_eq!(game.lifetime_stats.dice_games_played, 1);
                    assert_eq!(game.lifetime_stats.items_bought, u64::from(sufficient));
                    assert_eq!(game.player.held, if sufficient { Some(item) } else { None });
                    let target = game
                        .world
                        .entity(item)
                        .expect("weapon kept or unaffordable equipment dropped");
                    assert_eq!(target.value("cost"), if sufficient { 0.0 } else { price });
                    assert_eq!(target.held, sufficient);
                    assert_eq!(
                        last_dialogue(&game),
                        ("YOU BET $1500!", "NOW ROLL THE DICE!")
                    );
                    assert_eq!(game.player.health, 4);
                    assert_eq!(game.world.entity(keeper).unwrap().health, 20);
                    assert_post_draw_ttl(&game, 200.0);
                    assert!(!game.player.has("Jetpack"));
                }
            }
        }
    }
}

#[test]
fn pay_purchase_continues_kiss_and_failed_purchase_uses_damsel_price_without_heal() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for sale in [false, true] {
                for sufficient in [false, true] {
                    let (mut game, keeper) = shop(
                        rules,
                        facing,
                        "Kissing",
                        if sufficient { 20000 } else { 10000 },
                    );
                    let damsel = game.spawn(
                        "oDamsel",
                        Vec2::new(104.0 + 20.0 * f64::from(facing), 152.0),
                    );
                    {
                        // Source Kissing-room stock has SLAVE status and sale
                        // marker; native Create supplies 3*getKissValue=15000.
                        let actor = game.world.entity_mut(damsel).unwrap();
                        actor.state = 5;
                        actor.data.insert("status".into(), 5.0);
                        actor.data.insert("forSale".into(), 1.0);
                        assert_eq!(actor.value("cost"), 15000.0);
                    }
                    let (object, price) = if sufficient {
                        ("oPistol", 5000.0)
                    } else {
                        ("oJetpack", 20000.0)
                    };
                    let item = stock(&mut game, object, sale);
                    pickup(&mut game, item, price, sale);
                    assert_eq!(game.world.entity(damsel).unwrap().state, 5);
                    assert!(!game.world.entity(damsel).unwrap().held);
                    pay(&mut game);
                    assert_eq!(game.player.money, 10000);
                    assert_eq!(game.player.health, if sufficient { 5 } else { 4 });
                    assert_eq!(game.lifetime_stats.items_bought, u64::from(sufficient));
                    assert_eq!(game.lifetime_stats.kisses_bought, u64::from(sufficient));
                    assert_eq!(game.player.held, if sufficient { Some(item) } else { None });
                    let actor = game.world.entity(damsel).unwrap();
                    assert_eq!(actor.health, 4);
                    assert_eq!(actor.value("cost"), 15000.0);
                    assert_eq!(actor.state, if sufficient { 6 } else { 5 });
                    assert_eq!(
                        last_dialogue(&game),
                        if sufficient {
                            ("NOW AIN'T SHE SWEET!", "")
                        } else {
                            ("YOU NEED $15000!", "GET OUTTA HERE, DEADBEAT!")
                        }
                    );
                    assert_eq!(game.world.entity(keeper).unwrap().health, 20);
                    assert_post_draw_ttl(&game, 200.0);
                    assert!(!game.player.has("Jetpack"));
                }
            }
        }
    }
}

#[test]
fn free_held_rock_pay_continues_craps_unchanged_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let (mut game, keeper) = shop(rules, facing, "Craps", 7000);
            let rock = game.spawn("oRock", game.player.pos);
            pickup(&mut game, rock, 0.0, false);
            pay(&mut game);
            assert_eq!(game.player.money, 5500);
            assert_eq!(game.player.held, Some(rock));
            assert_eq!(game.player.data.get("bet"), Some(&1500.0));
            assert_eq!(game.lifetime_stats.items_bought, 0);
            assert_eq!(game.lifetime_stats.dice_games_played, 1);
            assert_eq!(game.lifetime_stats.kisses_bought, 0);
            assert_eq!(
                last_dialogue(&game),
                ("YOU BET $1500!", "NOW ROLL THE DICE!")
            );
            assert_post_draw_ttl(&game, 200.0);
            assert_eq!(game.player.health, 4);
            assert_eq!(game.world.entity(keeper).unwrap().health, 20);
        }
    }
}

#[test]
fn general_shop_purchase_and_drop_without_services_unchanged_controls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for sufficient in [false, true] {
                let (mut game, keeper) = shop(rules, facing, "General", 7000);
                let (object, price) = if sufficient {
                    ("oPistol", 5000.0)
                } else {
                    ("oJetpack", 20000.0)
                };
                let item = stock(&mut game, object, true);
                pickup(&mut game, item, price, true);
                pay(&mut game);
                assert_eq!(game.player.money, if sufficient { 2000 } else { 7000 });
                assert_eq!(game.player.held, if sufficient { Some(item) } else { None });
                assert_eq!(
                    game.world.entity(item).unwrap().value("cost"),
                    if sufficient { 0.0 } else { price }
                );
                assert_eq!(game.lifetime_stats.items_bought, u64::from(sufficient));
                assert_eq!(game.lifetime_stats.dice_games_played, 0);
                assert_eq!(game.lifetime_stats.kisses_bought, 0);
                assert_eq!(game.player.data.get("bet").copied().unwrap_or(0.0), 0.0);
                assert_eq!(
                    last_dialogue(&game),
                    if sufficient {
                        ("YOU GOT A PISTOL!", "")
                    } else {
                        ("YOU HAVEN'T GOT ENOUGH MONEY!", "")
                    }
                );
                assert_post_draw_ttl(&game, 80.0);
                assert_eq!(game.player.health, 4);
                assert_eq!(game.world.entity(keeper).unwrap().health, 20);
            }
        }
    }
}
