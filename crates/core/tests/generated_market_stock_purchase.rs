//! A generated Black Market sale item can be bought through ordinary Player input.
//!
//! Classic anchors: scrRoomGenMarket.gml:118-141,358-360 creates shop stock via
//! scrShopItemsGen.gml:174-189; oPlayer1 Step.xml:1245-1256 picks it up and
//! 1401-1426 handles Pay; scrStealItem.gml:69-79 grants three bombs and retires it.
use spelunky_core::{Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2};

fn source_shop_cell(pos: Vec2) -> (usize, usize) {
    let x = (((pos.x - 16.0).max(0.0) / 160.0).floor() as usize).min(3);
    let y = (((pos.y - 16.0).max(0.0) / 128.0).floor() as usize).min(4);
    (x, y)
}

fn generated_market_with_sale_bomb_bag() -> (Game, u64) {
    // The force flag selects Classic's ordinary Jungle Market generation
    // branch. The stock itself, its price, and its room path all come from
    // campaign generation; the fixture does not spawn or edit a shop item.
    for seed in 0..256 {
        let mut game = Game::shell(seed, RulesPreset::Classic);
        game.start_from_hub(false, 4);
        game.flags.insert("genBlackMarket".into());
        game.next_level();
        assert_eq!(game.world.level, 5);
        assert_eq!(game.world.name, "Black Market");

        let stock_id = game
            .world
            .entities
            .iter()
            .find(|entity| {
                if !entity.alive
                    || entity.object != "oBombBag"
                    || entity.value("forSale") != 1.0
                    || entity.value("cost") <= 0.0
                {
                    return false;
                }
                let Some(path) = game.world.source_room_path else {
                    return false;
                };
                let (x, y) = source_shop_cell(entity.pos);
                path[y][x] == 4
            })
            .map(|stock| stock.id);
        if let Some(stock_id) = stock_id {
            return (game, stock_id);
        }
    }
    panic!("bounded generated Black Market seeds must include a sale Bomb Bag");
}

fn position_player_and_camera(game: &mut Game, pos: Vec2) {
    game.player.pos = pos;
    game.player.previous_pos = pos;
    game.player.vel = Vec2::default();
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;

    // Follow the teleported fixture position with the normal camera update so
    // the generated keeper and stock enter the live dispatcher view.
    for _ in 0..128 {
        let previous = game.camera;
        game.update_camera();
        if game.camera == previous {
            break;
        }
    }
    game.previous_camera = game.camera;
}

#[test]
fn generated_market_bomb_bag_is_paid_for_and_removed_from_stock() {
    let (mut game, stock_id) = generated_market_with_sale_bomb_bag();
    let stock = game.world.entity(stock_id).expect("generated sale stock");
    let stock_pos = stock.pos;
    let original_price = stock.value("cost");
    let price = original_price as u32;
    let bombs_before = game.player.bombs;
    let other_stock: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive
                && entity.id != stock_id
                && entity.value("forSale") == 1.0
                && entity.value("cost") > 0.0
        })
        .map(|entity| (entity.id, entity.value("cost")))
        .collect();

    assert!(price > 0);
    assert_eq!(original_price, f64::from(price));
    assert_eq!(
        original_price,
        2500.0 + (2500.0 / 100.0) * 10.0 * f64::from(game.world.level - 2),
        "the generated stock carries the Classic level-scaled Bomb Bag price"
    );
    assert!(
        !other_stock.is_empty(),
        "the Market has other generated stock"
    );
    let path = game.world.source_room_path.expect("generated room path");
    let (shop_x, shop_y) = source_shop_cell(stock_pos);
    assert_eq!(path[shop_y][shop_x], 4, "the stock belongs to a shop room");

    position_player_and_camera(&mut game, stock_pos);
    game.player.money = price + 10_000;
    let money_before = game.player.money;
    game.step(&InputFrame::default());
    assert!(game.world.entities.iter().any(|entity| {
        entity.alive
            && entity.engine_active
            && spelunky_core::catalog::is_a(&entity.object, "oShopkeeper")
    }));

    // Down+Attack picks up the live generated item without spending money or
    // closing its sale markers. Pay is a separate Player input edge.
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..InputFrame::default()
    });
    assert_eq!(game.player.held, Some(stock_id));
    assert_eq!(game.player.money, money_before);
    let held_stock = game
        .world
        .entity(stock_id)
        .expect("held stock remains live");
    assert_eq!(held_stock.value("cost"), original_price);
    assert_eq!(held_stock.value("forSale"), 1.0);

    game.step(&InputFrame {
        pay_pressed: true,
        ..InputFrame::default()
    });

    assert_eq!(game.player.money, money_before - price);
    assert_eq!(game.player.bombs, bombs_before + 3.0);
    assert_eq!(game.player.held, None);
    assert!(
        game.world.entity(stock_id).is_none(),
        "the purchased stock is retired"
    );
    assert!(game.events.iter().any(|event| matches!(
        event,
        GameEvent::Collected { object, value: 0 } if object == "oBombBag"
    )));
    assert!(!game.flags.contains("wanted"));
    assert!(!game.flags.contains("murderer"));
    assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);

    // Buying one item does not anger the keeper or clear the rest of the
    // generated shop's stock.
    for (id, cost) in other_stock {
        let item = game.world.entity(id).expect("other stock remains live");
        assert_eq!(item.value("cost"), cost);
        assert_eq!(item.value("forSale"), 1.0);
    }
}
