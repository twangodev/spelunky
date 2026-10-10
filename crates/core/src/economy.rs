//! Shop policy from scrShopkeeperAnger, oItem Step and oPlayer1 Step.
//! Original shop membership is a 160×128 room, not distance to the keeper.
use crate::model::{Game, GameEvent, InputFrame, Rect, RunState, Vec2};

fn room(pos: Vec2) -> (i32, i32) {
    (
        ((pos.x - 16.).max(0.) / 160.).floor() as i32,
        ((pos.y - 16.).max(0.) / 128.).floor() as i32,
    )
}
fn in_shop(game: &Game, pos: Vec2) -> bool {
    game.world.entities.iter().any(|e| {
        e.alive
            && e.object == "oShopkeeper"
            && (
                e.data
                    .get("shop_room_x")
                    .copied()
                    .unwrap_or(f64::from(room(e.pos).0)) as i32,
                e.data
                    .get("shop_room_y")
                    .copied()
                    .unwrap_or(f64::from(room(e.pos).1)) as i32,
            ) == room(pos)
    })
}
fn rect_distance(a: Rect, b: Rect) -> f64 {
    let x = (a.x - b.x - b.w).max(b.x - a.x - a.w).max(0.);
    let y = (a.y - b.y - b.h).max(b.y - a.y - a.h).max(0.);
    x.hypot(y)
}
fn count(game: &mut Game, key: &str) {
    *game.data.entry(key.into()).or_default() += 1.;
}

/// Make the closest living keeper hostile; repeated reports do not add bounty.
pub fn anger(game: &mut Game, pos: Vec2, reason: &str) {
    let nearest = game
        .world
        .entities
        .iter()
        .enumerate()
        .filter(|(_, e)| e.alive && e.health > 0 && e.object == "oShopkeeper")
        .min_by(|(_, a), (_, b)| a.pos.distance(pos).total_cmp(&b.pos.distance(pos)))
        .map(|(i, _)| i);
    let Some(index) = nearest else {
        return;
    };
    if game.world.entities[index].value("angered") > 0. {
        return;
    }
    let keeper = &mut game.world.entities[index];
    keeper.data.insert("angered".into(), 1.);
    keeper.data.insert("angry".into(), 1.);
    keeper.state = 2;
    let thief = game.data.entry("thiefLevel".into()).or_default();
    *thief += if *thief > 0. { 3. } else { 2. };
    game.flags.insert("wanted".into());
    game.events
        .push(GameEvent::Message(if game.flags.contains("murderer") {
            "YOU'LL PAY FOR YOUR CRIMES!".into()
        } else {
            reason.into()
        }));
}

fn clear_sale(game: &mut Game, id: u64) {
    if let Some(entity) = game.world.entity_mut(id) {
        entity.data.insert("cost".into(), 0.);
        entity.data.insert("forSale".into(), 0.);
    }
}

fn craps_keeper(game: &Game) -> bool {
    game.world.entities.iter().any(|e| {
        e.alive
            && e.health > 0
            && e.object == "oShopkeeper"
            && e.text.get("style").is_some_and(|style| style == "Craps")
            && (
                e.data
                    .get("shop_room_x")
                    .copied()
                    .unwrap_or(f64::from(room(e.pos).0)) as i32,
                e.data
                    .get("shop_room_y")
                    .copied()
                    .unwrap_or(f64::from(room(e.pos).1)) as i32,
            ) == room(game.player.pos)
    })
}

fn high_end_prize(game: &mut Game) -> &'static str {
    // scrGenerateItem set 1 uses sequential conditional draws.
    for (object, denominator) in [
        ("oJetpack", 40),
        ("oCapePickup", 25),
        ("oShotgun", 20),
        ("oGloves", 10),
        ("oTeleporter", 10),
        ("oMattock", 8),
        ("oPaste", 8),
        ("oSpringShoes", 8),
        ("oSpikeShoes", 8),
        ("oCompass", 8),
        ("oPistol", 8),
        ("oMachete", 8),
    ] {
        if game.rng.chance(1, denominator) {
            return object;
        }
    }
    "oBombBox"
}

fn settle_dice(game: &mut Game) {
    let bet = game.player.data.get("bet").copied().unwrap_or(0.);
    if bet <= 0.
        || !game.world.entities.iter().any(|e| {
            e.alive
                && e.health > 0
                && e.object == "oShopkeeper"
                && e.text.get("style").is_some_and(|style| style == "Craps")
        })
    {
        return;
    }
    let dice: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oDice")
        .map(|e| (e.id, e.value("rolled"), e.value("value")))
        .collect();
    if dice.len() != 2 || dice.iter().any(|(_, rolled, _)| *rolled == 0.) {
        return;
    }
    let total = dice.iter().map(|(_, _, value)| *value as u32).sum::<u32>();
    if total == 7 {
        count(game, "diceGamesWon");
        let prizes: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.value("inDiceHouse") > 0.)
            .map(|e| (e.id, e.pos))
            .collect();
        for (id, pos) in prizes {
            clear_sale(game, id);
            if let Some(prize) = game.world.entity_mut(id) {
                prize.data.insert("inDiceHouse".into(), 0.);
                prize.pos.x += if game.player.pos.x < pos.x { -32. } else { 32. };
            }
            let object = high_end_prize(game);
            let replacement = game.world.spawn(object, pos);
            game.world
                .entity_mut(replacement)
                .unwrap()
                .data
                .insert("inDiceHouse".into(), 1.);
        }
        game.events.push(GameEvent::Message(
            "YOU ROLLED A SEVEN! YOU WIN A PRIZE!".into(),
        ));
    } else if total > 7 {
        count(game, "diceGamesWon");
        *game.data.entry("collect".into()).or_default() += bet * 2.;
        let counter = game.data.entry("collectCounter".into()).or_default();
        *counter = (*counter + 20.).min(100.);
        game.events.push(GameEvent::Message(format!(
            "YOU ROLLED A {total}! CONGRATULATIONS! YOU WIN!"
        )));
    } else {
        count(game, "diceGamesLost");
        game.events.push(GameEvent::Message(format!(
            "YOU ROLLED A {total}! I'M SORRY, BUT YOU LOSE!"
        )));
    }
    game.player.data.insert("bet".into(), 0.);
    for (id, _, _) in dice {
        if let Some(die) = game.world.entity_mut(id) {
            die.data.insert("rolled".into(), 0.);
        }
    }
}

pub fn tick(game: &mut Game, input: &InputFrame) {
    if game.state != RunState::Playing || game.flags.contains("hub") {
        return;
    }
    // Source membership uses immutable roomPath entries, not the keeper's
    // following/patrol position. Retain each keeper's original shop room.
    for keeper in &mut game.world.entities {
        if keeper.object == "oShopkeeper" {
            let (x, y) = room(keeper.pos);
            keeper
                .data
                .entry("shop_room_x".into())
                .or_insert(f64::from(x));
            keeper
                .data
                .entry("shop_room_y".into())
                .or_insert(f64::from(y));
        }
    }
    settle_dice(game);
    // A keeper struck by another subsystem reports anger through its state.
    let assaults: Vec<Vec2> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            e.object == "oShopkeeper"
                && e.alive
                && e.value("angered") == 0.
                && (e.value("angry") > 0. || e.value("assaulted") > 0.)
        })
        .map(|e| e.pos)
        .collect();
    for pos in assaults {
        anger(game, pos, "NOW I'M REALLY STEAMED!");
    }
    if game.flags.contains("shopkeepers_angry") && !game.flags.contains("wanted") {
        anger(game, game.player.pos, "HEY, ONLY I CAN DO THAT!");
    }
    if game
        .world
        .entities
        .iter()
        .any(|e| e.object == "oShopkeeper" && e.value("killed_by_player") > 0.)
    {
        game.flags.insert("murderer".into());
        game.flags.insert("wanted".into());
    }
    let wanted =
        game.flags.contains("murderer") || game.data.get("thiefLevel").copied().unwrap_or(0.) > 0.;
    let keepers_exist = game
        .world
        .entities
        .iter()
        .any(|e| e.alive && e.health > 0 && e.object == "oShopkeeper");
    let stock: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.value("forSale") > 0. && e.value("cost") > 0.)
        .map(|e| (e.id, e.pos))
        .collect();
    for (id, pos) in stock {
        if wanted || !keepers_exist {
            clear_sale(game, id);
            count(game, "itemsStolen");
        } else if !in_shop(game, pos) {
            anger(game, pos, "COME BACK HERE, THIEF!");
            clear_sale(game, id);
            count(game, "itemsStolen");
        }
    }
    if let Some(id) = game.player.held {
        if game
            .world
            .entity(id)
            .is_some_and(|item| item.value("forSale") == 0. && item.value("cost") == 0.)
        {
            crate::items::collectibles::grant(game, id);
        }
    }
    if !input.pay_pressed
        || game.player.health <= 0
        || game.player.stun > 0
        || !in_shop(game, game.player.pos)
    {
        return;
    }
    // Purchases require the item in hand, matching Classic's pay button.
    if let Some(id) = game.player.held {
        let price = game
            .world
            .entity(id)
            .filter(|e| e.value("forSale") > 0.)
            .map(|e| e.value("cost").max(0.) as u32)
            .unwrap_or(0);
        if price > game.player.money {
            if let Some(item) = game.world.entity_mut(id) {
                item.held = false;
            }
            game.player.held = None;
            game.events
                .push(GameEvent::Message("YOU HAVEN'T GOT ENOUGH MONEY!".into()));
            return;
        }
        if price > 0 {
            game.player.money -= price;
            clear_sale(game, id);
            count(game, "itemsBought");
            crate::items::collectibles::grant(game, id);
            return;
        }
    }
    if craps_keeper(game) {
        if wanted {
            return;
        }
        let bet = game.player.data.get("bet").copied().unwrap_or(0.);
        let cost = 1000_u32.saturating_add(game.world.level.saturating_mul(500));
        if bet > 0. {
            game.events.push(GameEvent::Message(
                "ONE BET AT A TIME! PLEASE ROLL THE DICE!".into(),
            ));
        } else if game.player.money >= cost {
            game.player.money -= cost;
            game.player.data.insert("bet".into(), cost as f64);
            count(game, "diceGamesPlayed");
            game.events.push(GameEvent::Message(format!(
                "YOU BET ${cost}! NOW ROLL THE DICE!"
            )));
        } else {
            game.events
                .push(GameEvent::Message(format!("YOU NEED ${cost} TO BET!")));
        }
        return;
    }
    let kiss = game
        .world
        .entities
        .iter()
        .find(|e| {
            e.alive
                && e.health > 0
                && e.object == "oDamsel"
                && e.value("forSale") > 0.
                && !e.held
                && rect_distance(e.sprite_bounds(), game.player.sprite_bounds()) < 16.
        })
        .map(|e| e.pos);
    if let Some(pos) = kiss {
        if wanted {
            return;
        }
        let cost = 5000_u32.saturating_mul(game.world.level);
        if game.player.money >= cost {
            game.player.money -= cost;
            game.player.health += 1;
            count(game, "kissesBought");
            game.sound("sndKiss", pos);
            game.events
                .push(GameEvent::Message("NOW AIN'T SHE SWEET!".into()));
        } else {
            game.events
                .push(GameEvent::Message(format!("YOU NEED ${cost}!")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RulesPreset, Vec2};
    fn shop() -> (Game, u64) {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(40., 40.);
        game.world.spawn("oShopkeeper", Vec2::new(96., 40.));
        let id = game.world.spawn("oBombBag", game.player.pos);
        let item = game.world.entity_mut(id).unwrap();
        item.data.insert("forSale".into(), 1.);
        item.data.insert("cost".into(), 2500.);
        item.held = true;
        game.player.held = Some(id);
        (game, id)
    }
    #[test]
    fn purchase_consumes_exact_cost_and_grants_supply() {
        let (mut game, id) = shop();
        game.player.money = 3000;
        tick(
            &mut game,
            &InputFrame {
                pay_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(game.player.money, 500);
        assert_eq!(game.player.bombs, 7);
        assert!(game.world.entity(id).is_none());
        assert!(!game.flags.contains("wanted"));
    }
    #[test]
    fn insufficient_funds_drop_without_buying() {
        let (mut game, id) = shop();
        game.player.money = 2499;
        tick(
            &mut game,
            &InputFrame {
                pay_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(game.player.money, 2499);
        assert_eq!(game.player.bombs, 4);
        assert_eq!(game.player.held, None);
        assert_eq!(game.world.entity(id).unwrap().value("forSale"), 1.);
    }
    #[test]
    fn theft_uses_shop_room_and_escalates_once() {
        let (mut game, id) = shop();
        game.world.entity_mut(id).unwrap().pos.x = 185.;
        tick(&mut game, &InputFrame::default());
        tick(&mut game, &InputFrame::default());
        assert_eq!(game.data["thiefLevel"], 2.);
        assert_eq!(game.data["itemsStolen"], 1.);
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.player.bombs, 7);
    }
    #[test]
    fn ordinary_pickup_price_is_not_sale() {
        let (mut game, id) = shop();
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("forSale".into(), 0.);
        game.world.entity_mut(id).unwrap().pos.x = 300.;
        tick(&mut game, &InputFrame::default());
        assert!(!game.flags.contains("wanted"));
    }
    #[test]
    fn dice_wager_and_cash_payout_preserve_source_accounting() {
        let (mut g, id) = shop();
        g.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("forSale".into(), 0.);
        g.player.held = None;
        g.player.money = 5000;
        g.world
            .entities
            .iter_mut()
            .find(|e| e.object == "oShopkeeper")
            .unwrap()
            .text
            .insert("style".into(), "Craps".into());
        tick(
            &mut g,
            &InputFrame {
                pay_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.money, 3500);
        assert_eq!(g.player.data["bet"], 1500.);
        for value in [4., 6.] {
            let die = g.world.spawn("oDice", g.player.pos);
            let e = g.world.entity_mut(die).unwrap();
            e.data.insert("rolled".into(), 1.);
            e.data.insert("value".into(), value);
        }
        tick(&mut g, &InputFrame::default());
        assert_eq!(g.data["collect"], 3000.);
        assert_eq!(g.player.data["bet"], 0.);
        assert_eq!(g.data["diceGamesWon"], 1.);
    }
    #[test]
    fn dice_seven_releases_prize_and_replenishes_display() {
        let (mut g, id) = shop();
        g.player.held = None;
        g.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("forSale".into(), 0.);
        g.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("inDiceHouse".into(), 1.);
        g.world
            .entities
            .iter_mut()
            .find(|e| e.object == "oShopkeeper")
            .unwrap()
            .text
            .insert("style".into(), "Craps".into());
        g.player.data.insert("bet".into(), 1500.);
        for value in [3., 4.] {
            let die = g.world.spawn("oDice", g.player.pos);
            let e = g.world.entity_mut(die).unwrap();
            e.data.insert("rolled".into(), 1.);
            e.data.insert("value".into(), value);
        }
        tick(&mut g, &InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().value("inDiceHouse"), 0.);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.value("inDiceHouse") > 0.)
                .count(),
            1
        );
        assert_eq!(g.player.data["bet"], 0.);
    }
    #[test]
    fn actual_step_theft_uses_original_shop_room_when_keeper_follows_outside() {
        let (mut g, id) = shop();
        for x in 0..g.world.width {
            g.world.set_tile(x, 4, "oTemple");
        }
        g.player.pos = Vec2::new(140., 56.);
        g.player.previous_pos = g.player.pos;
        g.player.invincible = 999;
        let keeper = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oShopkeeper")
            .unwrap()
            .id;
        g.world.entity_mut(keeper).unwrap().pos = Vec2::new(128., 48.);
        g.step(&InputFrame::default());
        assert!(!g.flags.contains("wanted"));
        g.player.pos = Vec2::new(185., 56.);
        g.player.previous_pos = g.player.pos;
        g.world.entity_mut(keeper).unwrap().pos = Vec2::new(188., 48.);
        g.step(&InputFrame::default());
        assert!(g.flags.contains("wanted"));
        assert_eq!(g.data["thiefLevel"], 2.);
        assert_eq!(g.player.bombs, 7);
        assert!(g.world.entity(id).is_none());
    }
    #[test]
    fn hub_shopkeeper_flags_do_not_create_campaign_bounty() {
        let (mut g, _) = shop();
        g.flags.insert("hub".into());
        g.flags.insert("hub_stars".into());
        g.flags.insert("shopkeepers_angry".into());
        tick(&mut g, &InputFrame::default());
        assert!(!g.flags.contains("wanted"));
        assert!(!g.data.contains_key("thiefLevel"));
    }
}
