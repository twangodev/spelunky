//! Controlled economy callbacks, without substituting for a natural playthrough.
//! Original: oPlayer1 Step1464-1500 and oShopkeeper Step206-225.
use spelunky_core::{Area, Game, GameEvent, InputFrame, RulesPreset, Vec2, World, economy};

fn shop(rules: RulesPreset, style: &str) -> Game {
    let mut game = Game::shell(19, rules);
    game.world = World::new(40, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(88., 56.);
    game.player.previous_pos = game.player.pos;
    game.player.money = 100000;
    let keeper = game.spawn("oShopkeeper", Vec2::new(128., 48.));
    game.world
        .entity_mut(keeper)
        .unwrap()
        .text
        .insert("style".into(), style.into());
    game
}

fn damsel(game: &mut Game, x: f64, sale: bool) -> u64 {
    let id = game.spawn("oDamsel", Vec2::new(x, 56.));
    let actor = game.world.entity_mut(id).unwrap();
    actor.state = 5;
    actor
        .data
        .insert("cost".into(), if sale { 15000. } else { 0. });
    actor
        .data
        .insert("forSale".into(), if sale { 1. } else { 0. });
    id
}

fn pay(game: &mut Game) {
    economy::tick(
        game,
        &InputFrame {
            pay_pressed: true,
            ..Default::default()
        },
    );
}

#[test]
fn nearer_non_sale_damsel_blocks_a_farther_sale_without_charging() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for near_first in [false, true] {
            let mut game = shop(rules, "Kissing");
            let (near, far) = if near_first {
                let near = damsel(&mut game, 90., false);
                (near, damsel(&mut game, 104., true))
            } else {
                let far = damsel(&mut game, 104., true);
                (damsel(&mut game, 90., false), far)
            };
            let health = game.player.health;
            pay(&mut game);
            assert_eq!(game.player.money, 100000);
            assert_eq!(game.player.health, health);
            assert_eq!(game.lifetime_stats.kisses_bought, 0);
            assert_eq!(game.world.entity(near).unwrap().state, 5);
            assert_eq!(game.world.entity(far).unwrap().state, 5);
        }
    }
}

#[test]
fn nearest_held_damsel_blocks_purchase_but_keeps_source_insufficient_funds_message() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for money in [100000, 0] {
            let mut game = shop(rules, "Kissing");
            let far = damsel(&mut game, 104., true);
            let near = damsel(&mut game, 90., true);
            game.world.entity_mut(near).unwrap().held = true;
            game.player.money = money;
            let health = game.player.health;
            pay(&mut game);
            assert_eq!(game.player.money, money);
            assert_eq!(game.player.health, health);
            assert_eq!(game.world.entity(far).unwrap().state, 5);
            assert_eq!(game.world.entity(near).unwrap().state, 5);
            let deadbeat = game.events.iter().any(|event| {
                matches!(event, GameEvent::Dialogue(message)
                    if message.first == "YOU NEED $5000!"
                    && message.second == "GET OUTTA HERE, DEADBEAT!")
            });
            assert_eq!(deadbeat, money == 0);
        }
    }
}

#[test]
fn kissing_ignores_inactive_nearer_damsels_and_keeps_current_source_order_on_ties() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = shop(rules, "Kissing");
        let inactive = damsel(&mut game, 88., false);
        game.world.entity_mut(inactive).unwrap().engine_active = false;
        let first = damsel(&mut game, 80., true);
        let second = damsel(&mut game, 96., true);
        // Existing dispatcher birth order must survive unrelated storage order.
        game.world.entities.reverse();
        let health = game.player.health;
        pay(&mut game);
        assert_eq!(game.player.money, 95000);
        assert_eq!(game.player.health, health + 1.0);
        assert_eq!(game.lifetime_stats.kisses_bought, 1);
        assert_eq!(game.world.entity(first).unwrap().state, 6);
        assert_eq!(game.world.entity(second).unwrap().state, 5);
        assert_eq!(game.world.entity(inactive).unwrap().state, 5);
    }
}

#[test]
fn kissing_selection_does_not_add_a_health_gate_absent_from_original_payment() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = shop(rules, "Kissing");
        let far = damsel(&mut game, 104., true);
        let near = damsel(&mut game, 90., true);
        game.world.entity_mut(near).unwrap().health = 0;
        let health = game.player.health;
        pay(&mut game);
        assert_eq!(game.player.money, 95000);
        assert_eq!(game.player.health, health + 1.0);
        assert_eq!(game.world.entity(near).unwrap().state, 6);
        assert_eq!(game.world.entity(far).unwrap().state, 5);
    }
}

fn rolled_dice(game: &mut Game, values: [f64; 2]) {
    game.player.data.insert("bet".into(), 1500.);
    for value in values {
        let id = game.spawn("oDice", Vec2::new(100., 56.));
        let die = game.world.entity_mut(id).unwrap();
        die.data.insert("value".into(), value);
        die.data.insert("rolled".into(), 1.);
    }
}

#[test]
fn craps_seven_creates_four_source_poofs_around_replacement_and_award() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for player_x in [88., 120.] {
            let mut game = shop(rules, "Craps");
            game.player.pos.x = player_x;
            let prize = game.spawn("oBombBag", Vec2::new(104., 56.));
            let item = game.world.entity_mut(prize).unwrap();
            item.data.insert("forSale".into(), 1.);
            item.data.insert("inDiceHouse".into(), 1.);
            rolled_dice(&mut game, [3., 4.]);
            let money = game.player.money;
            economy::tick(&mut game, &InputFrame::default());
            let replacement = game
                .world
                .entities
                .iter()
                .find(|item| item.alive && item.value("inDiceHouse") == 1.)
                .unwrap();
            let awarded_x = if player_x < 104. { 72. } else { 136. };
            let poofs: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|item| item.alive && item.object == "oPoof")
                .collect();
            assert_eq!(poofs.len(), 4);
            for (poof, (x, velocity)) in poofs.iter().zip([
                (100., -0.4),
                (108., 0.4),
                (awarded_x - 4., -0.4),
                (awarded_x + 4., 0.4),
            ]) {
                assert_eq!(poof.pos, Vec2::new(x, 62.));
                assert_eq!(poof.vel, Vec2::new(velocity, 0.));
                assert_eq!(poof.value("animation_speed"), 0.4);
            }
            assert!(poofs[0].id < poofs[1].id);
            assert!(poofs[1].id < replacement.id);
            assert!(replacement.id < poofs[2].id);
            assert!(poofs[2].id < poofs[3].id);
            assert_eq!(replacement.pos, Vec2::new(104., 56.));
            let awarded = game.world.entity(prize).unwrap();
            assert_eq!(awarded.pos, Vec2::new(awarded_x, 56.));
            assert_eq!(awarded.value("cost"), 0.);
            assert_eq!(awarded.value("forSale"), 0.);
            assert_eq!(awarded.value("inDiceHouse"), 0.);
            assert_eq!(game.player.money, money);
            assert_eq!(game.player.data["bet"], 0.);
            assert_eq!(game.lifetime_stats.dice_games_won, 1);
            assert!(game.events.iter().any(|event| matches!(event,
                GameEvent::Dialogue(message) if message.first == "YOU ROLLED A SEVEN!"
                && message.second == "YOU WIN A PRIZE!")));
        }
    }
}

#[test]
fn craps_cash_and_loss_controls_create_no_prize_poofs() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (values, collected) in [([4., 6.], 3000.), ([1., 1.], 0.)] {
            let mut game = shop(rules, "Craps");
            rolled_dice(&mut game, values);
            economy::tick(&mut game, &InputFrame::default());
            assert_eq!(game.data.get("collect").copied().unwrap_or(0.), collected);
            assert_eq!(game.player.data["bet"], 0.);
            assert!(
                game.world
                    .entities
                    .iter()
                    .all(|item| item.object != "oPoof")
            );
        }
    }
}
