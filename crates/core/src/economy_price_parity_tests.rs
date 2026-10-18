//! Source-price and payment regressions for scrShopItemsGen / oPlayer1 Step.
use super::*;
use crate::RulesPreset;

#[test]
fn level_scaled_shop_prices_keep_classic_integer_costs() {
    for (base, expected) in [
        (3_000.0, 6_900.0),   // Compass
        (7_000.0, 16_100.0),  // Machete
        (12_000.0, 27_600.0), // Cape
    ] {
        assert_eq!(classic_shop_price(base, 15), expected);
    }
    assert_eq!(classic_shop_price(3_000.0, 1), 3_000.0);
    assert_eq!(classic_shop_price(3_000.0, 2), 3_000.0);
    assert_eq!(classic_shop_price(3_000.0, 3), 3_300.0);
}

#[test]
fn actual_step_charges_the_full_source_price_for_level_fifteen_stock() {
    let mut game = Game::shell(23, RulesPreset::Classic);
    game.world.level = 15;
    for x in 0..game.world.width {
        game.world.set_tile(x, 4, "oTemple");
    }
    game.player.pos = Vec2::new(88.0, 56.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.money = 10_000;
    let level = game.world.level;

    let keeper = game.spawn("oShopkeeper", Vec2::new(128.0, 48.0));
    game.world
        .entity_mut(keeper)
        .unwrap()
        .text
        .insert("style".into(), "General".into());
    let item = game.spawn("oCompass", game.player.pos);
    let stock = game.world.entity_mut(item).unwrap();
    stock
        .data
        .insert("cost".into(), classic_shop_price(3_000.0, level));
    stock.data.insert("forSale".into(), 1.0);
    stock.held = true;
    game.player.held = Some(item);

    game.step(&InputFrame {
        pay_pressed: true,
        ..Default::default()
    });

    assert_eq!(game.player.money, 3_100);
    assert!(game.player.equipment.contains("Compass"));
    assert!(game.world.entity(item).is_none());
    assert_eq!(game.data["itemsBought"], 1.0);
    assert_eq!(game.lifetime_stats.items_bought, 1);
}
