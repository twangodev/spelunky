//! Player Step's payment branch requires an existing active shopkeeper;
//! authoritative roomPath membership alone does not make a purchase possible.
use super::*;
use crate::RulesPreset;

fn payment_scene(rules: RulesPreset, keeper_position: Option<Vec2>) -> (Game, u64, Option<u64>) {
    let mut game = Game::shell(23, rules);
    let mut path = [[0; 4]; 5];
    path[0][0] = 4;
    game.world.source_room_path = Some(path);
    for x in 0..game.world.width {
        game.world.set_tile(x, 4, "oTemple");
    }
    game.player.pos = Vec2::new(88., 56.);
    game.player.previous_pos = game.player.pos;
    game.player.money = 100000;
    let keeper = keeper_position.map(|pos| game.spawn("oShopkeeper", pos));
    // Level's resource553 Step runs after Player319, so establish ordinary
    // activation membership with a neutral tick before the purchase input.
    game.step(&InputFrame::default());
    let item = game.spawn("oMachete", Vec2::new(92., 58.));
    let held = game.world.entity_mut(item).unwrap();
    held.held = true;
    held.data.insert("cost".into(), 10000.);
    held.data.insert("forSale".into(), 1.);
    game.player.held = Some(item);
    (game, item, keeper)
}

#[test]
fn actual_step_payment_requires_active_keeper_after_level_activation() {
    // Source oPlayer1 Step1404: isInShop(x,y) and instance_exists(oShopkeeper).
    // The previous ordinary Level Step deactivated the distant actor before
    // this Player Step. The authoritative shop cell remains present throughout.
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for position in [None, Some(Vec2::new(600., 48.)), Some(Vec2::new(128., 48.))] {
            let (mut game, item, keeper) = payment_scene(rules, position);
            game.step(&InputFrame {
                pay_pressed: true,
                ..Default::default()
            });
            let keeper_exists = position.is_some_and(|pos| pos.x < 400.);
            if let Some(id) = keeper {
                assert_eq!(game.world.entity(id).unwrap().engine_active, keeper_exists);
            }
            assert!(in_shop(&game, game.player.pos));
            assert_eq!(
                game.player.money,
                if keeper_exists { 90000 } else { 100000 }
            );
            assert_eq!(game.lifetime_stats.items_bought, u64::from(keeper_exists));
            assert_eq!(game.player.held, Some(item));
            assert_eq!(game.world.entity(item).unwrap().value("cost"), 0.);
            assert_eq!(game.lifetime_stats.items_stolen, u64::from(!keeper_exists));
        }
    }
}

#[test]
fn payment_existence_accepts_active_keeper_corpse_but_not_retired_actor() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for retired in [false, true] {
            let (mut game, item, keeper) = payment_scene(rules, Some(Vec2::new(128., 48.)));
            let keeper = game.world.entity_mut(keeper.unwrap()).unwrap();
            keeper.health = 0;
            keeper.state = 99;
            keeper.data.insert("dead".into(), 1.);
            keeper.alive = !retired;
            // Initial corpse/retirement controls isolate instance existence;
            // no claim is made that this setup is a natural lethal-input route.
            game.step(&InputFrame {
                pay_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.money, if retired { 100000 } else { 90000 });
            assert_eq!(game.lifetime_stats.items_bought, u64::from(!retired));
            assert_eq!(game.player.held, Some(item));
        }
    }
}
