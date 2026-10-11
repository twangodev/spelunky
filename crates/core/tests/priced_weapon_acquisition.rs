//! Initial authored stock uses literal source Create prices and forSale=true.
//! Holding/purchasing is reached only through normal Down+Attack then Pay input.
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

const WEAPONS: [(&str, &str, u32, &str, &str); 7] = [
    ("oMachete", "Machete", 7000, "YOU GOT A MACHETE!", ""),
    (
        "oMattock",
        "Mattock",
        8000,
        "YOU GOT A MATTOCK!",
        "IT SEEMS A BIT RUSTY.",
    ),
    ("oPistol", "Pistol", 5000, "YOU GOT A PISTOL!", ""),
    (
        "oWebCannon",
        "Web Cannon",
        2000,
        "YOU GOT A WEB CANNON!",
        "",
    ),
    (
        "oTeleporter",
        "Teleporter",
        10000,
        "YOU GOT A TELEPORTER!",
        "",
    ),
    ("oShotgun", "Shotgun", 15000, "YOU GOT A SHOTGUN!", ""),
    ("oBow", "Bow", 1000, "YOU GOT THE BOW AND ARROWS!", ""),
];

fn room(rules: RulesPreset, object: &str, cost: u32, money: u32) -> (Game, u64, u64) {
    let mut game = Game::legacy_shell(19, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(104.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.money = money;
    game.world.entrance = game.player.pos;
    let keeper = game.spawn("oShopkeeper", Vec2::new(144.0, 144.0));
    game.world
        .entity_mut(keeper)
        .unwrap()
        .text
        .insert("style".into(), "Weapon".into());
    let stock = game.spawn(object, game.player.pos);
    // Initial authored shop metadata; no runtime state/price/hand injection.
    let item = game.world.entity_mut(stock).unwrap();
    item.data.insert("cost".into(), f64::from(cost));
    item.data.insert("forSale".into(), 1.0);
    for _ in 0..8 {
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 4);
        assert_eq!(game.world.entity(keeper).unwrap().health, 20);
        assert_eq!(
            game.world.entity(stock).unwrap().value("cost"),
            f64::from(cost),
            "fixture prerequisite: peaceful shop preserves price"
        );
    }
    assert!(game.player.held.is_none());
    assert_eq!(game.player.money, money);
    assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
    (game, stock, keeper)
}

fn pickup(game: &mut Game, stock: u64, source_type: &str) {
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(stock),
        "fixture prerequisite: natural weapon pickup"
    );
    assert_eq!(game.player.pickup_item_type, source_type);
    assert!(game.world.entity(stock).unwrap().held);
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.attack_timer, 0);
    assert!(!game.events.iter().any(|event| matches!(event, GameEvent::Dialogue(message) if message.first.starts_with("YOU GOT"))), "picking up a priced weapon is not payment");
}

#[test]
fn natural_positive_price_weapon_purchases_emit_literal_acquisition_and_keep_actor() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, source_type, price, first, second) in WEAPONS {
            let (mut game, stock, keeper) = room(rules, object, price, 100000);
            pickup(&mut game, stock, source_type);
            assert_eq!(
                game.world.entity(stock).unwrap().value("cost"),
                f64::from(price)
            );
            let arrows = game.data.get("arrows").copied().unwrap_or(0.0);
            if object == "oBow" {
                assert_eq!(arrows, 6.0, "new Bow ammo belongs to pickup only");
            }
            let equipment = game.player.equipment.clone();
            let sound_origin = game.player.pos;
            game.step(&InputFrame {
                pay_pressed: true,
                ..Default::default()
            });
            assert_eq!(
                game.player.money,
                100000 - price,
                "actual positive payment before message assertions"
            );
            assert_eq!(
                game.player.held,
                Some(stock),
                "weapon purchase retains original actor identity"
            );
            assert_eq!(game.player.pickup_item_type, source_type);
            let weapon = game.world.entity(stock).unwrap();
            assert!(weapon.held);
            assert_eq!(weapon.object, object);
            assert_eq!(weapon.value("cost"), 0.0);
            assert_eq!(weapon.value("forSale"), 0.0);
            assert_eq!(game.lifetime_stats.items_bought, 1);
            assert_eq!(game.player.health, 4);
            assert_eq!(game.world.entity(keeper).unwrap().health, 20);
            assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
            assert_eq!(game.player.equipment, equipment);
            assert_eq!(
                game.data.get("arrows").copied().unwrap_or(0.0),
                arrows,
                "payment does not regrant Bow ammo"
            );
            let acquisitions: Vec<_> = game
                .events
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Dialogue(message) if message.first == first => Some(message),
                    _ => None,
                })
                .collect();
            assert_eq!(
                acquisitions.len(),
                1,
                "source positive-cost weapon branch emits exactly one acquisition dialogue: {object}"
            );
            assert_eq!(acquisitions[0].second, second);
            assert_eq!(
                acquisitions[0].ticks, 120,
                "script event precedes Pay timer override"
            );
            // Pay assigns80 in NormalStep; bright LevelDraw consumes one afterward.
            assert_eq!(game.presentation().data["source_message_draw_timer"], 80.0);
            assert_eq!(game.data.get("message_timer").copied(), Some(79.0));
            let pickup_sounds: Vec<_> = game
                .events
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Sound { name, pos } if name == "sndPickup" => Some(*pos),
                    _ => None,
                })
                .collect();
            assert_eq!(pickup_sounds, vec![sound_origin]);
            game.step(&InputFrame::default());
            assert_eq!(game.player.held, Some(stock));
            assert_eq!(game.player.money, 100000 - price);
            assert_eq!(game.lifetime_stats.items_bought, 1);
            assert!(!game.events.iter().any(
                |event| matches!(event, GameEvent::Dialogue(message) if message.first == first)
            ));
            assert!(!game.events.iter().any(
                |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndPickup")
            ));
        }
    }
}

#[test]
fn natural_zero_price_weapon_pickup_and_pay_remain_silent_without_consuming_actor() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, source_type, _, first, _) in WEAPONS {
            let (mut game, stock, _) = room(rules, object, 0, 100000);
            pickup(&mut game, stock, source_type);
            assert!(!game.events.iter().any(
                |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndPickup")
            ));
            game.step(&InputFrame {
                pay_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.money, 100000);
            assert_eq!(game.player.held, Some(stock));
            assert!(game.world.entity(stock).unwrap().held);
            assert_eq!(game.lifetime_stats.items_bought, 0);
            assert!(!game.events.iter().any(
                |event| matches!(event, GameEvent::Dialogue(message) if message.first == first)
            ));
            assert!(!game.events.iter().any(
                |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndPickup")
            ));
        }
    }
}

#[test]
fn natural_unaffordable_weapon_pay_drops_stock_without_acquisition_or_money_change() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, source_type, price, first, _) in WEAPONS {
            let (mut game, stock, _) = room(rules, object, price, price - 1);
            pickup(&mut game, stock, source_type);
            game.step(&InputFrame {
                pay_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.money, price - 1);
            assert!(game.player.held.is_none());
            assert!(game.player.pickup_item_type.is_empty());
            let weapon = game.world.entity(stock).unwrap();
            assert!(!weapon.held);
            assert_eq!(weapon.value("cost"), f64::from(price));
            assert_eq!(weapon.value("forSale"), 1.0);
            assert_eq!(game.lifetime_stats.items_bought, 0);
            assert!(game.events.iter().any(|event| matches!(event, GameEvent::Dialogue(message) if message.first == "YOU HAVEN'T GOT ENOUGH MONEY!" && message.second.is_empty() && message.ticks == 80)));
            assert!(!game.events.iter().any(
                |event| matches!(event, GameEvent::Dialogue(message) if message.first == first)
            ));
            assert!(!game.events.iter().any(
                |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndPickup")
            ));
        }
    }
}
