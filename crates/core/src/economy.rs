//! Shop policy from scrShopkeeperAnger, oItem Step and oPlayer1 Step.
//! Original shop membership is a 160×128 room, not distance to the keeper.
use crate::enemies::shop_dialogue::say;
use crate::model::{Entity, Game, InputFrame, Rect, RunState, Vec2};
use crate::statistics::{LifetimeCounter, is_real_level};

fn room(pos: Vec2) -> (i32, i32) {
    (
        (((pos.x - 16.).max(0.) / 160.).floor() as i32).min(3),
        (((pos.y - 16.).max(0.) / 128.).floor() as i32).min(4),
    )
}
pub(crate) fn in_shop(game: &Game, pos: Vec2) -> bool {
    game.world.entities.iter().any(|e| {
        e.alive
            && crate::catalog::is_a(&e.object, "oShopkeeper")
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
/// isInShop and scrGetRoomX/Y write the calling Player's persistent tx/ty.
/// These are also the Teleporter's target variables; keep query side effects
/// separate from item/enemy callers of the pure room-membership adapter.
pub(crate) fn player_in_shop(game: &mut Game, pos: Vec2) -> bool {
    game.player.data.insert("tx".into(), pos.x);
    game.player.data.insert("ty".into(), pos.y);
    in_shop(game, pos)
}

fn rect_distance(a: Rect, b: Rect) -> f64 {
    let x = (a.x - b.x - b.w + 1.).max(b.x - a.x - a.w + 1.).max(0.);
    let y = (a.y - b.y - b.h + 1.).max(b.y - a.y - a.h + 1.).max(0.);
    x.hypot(y)
}
fn count(game: &mut Game, key: &str, counter: LifetimeCounter) {
    if is_real_level(game) {
        *game.data.entry(key.into()).or_default() += 1.;
        game.lifetime_stats.increment(counter);
    }
}

/// scrShopkeeperAnger selects the nearest active keeper before checking its
/// dead/angered flags. ATTACK Step, rather than this script, latches angered.
pub fn anger(game: &mut Game, pos: Vec2, reason: &str) {
    crate::dispatcher::register_instances(game);
    let mut nearest = None;
    let mut nearest_distance = f64::INFINITY;
    for instance in crate::dispatcher::descendant_instances(game, "oShopkeeper") {
        let crate::dispatcher::Instance::Entity(id) = instance else {
            continue;
        };
        let Some(keeper) = game.world.entity(id) else {
            continue;
        };
        if !keeper.engine_active {
            continue;
        }
        let distance = keeper.pos.distance(pos);
        if distance < nearest_distance {
            nearest_distance = distance;
            nearest = Some(id);
        }
    }
    let Some(id) = nearest else {
        return;
    };
    let keeper = game.world.entity_mut(id).unwrap();
    if keeper.value("dead") != 0. || keeper.value("angered") != 0. {
        return;
    }
    keeper.state = 2;
    let thief = game.data.entry("thiefLevel".into()).or_default();
    *thief += if *thief > 0. { 3. } else { 2. };
    game.flags.insert("wanted".into());
    let message = if game.flags.contains("murderer") {
        "YOU'LL PAY FOR YOUR CRIMES!"
    } else {
        reason
    };
    say(game, message, "", 80);
}

/// The first ATTACK Step clears prices across the active oItem family before
/// marking this keeper angered (oShopkeeper Step367–374).
pub(crate) fn keeper_attack_step(game: &mut Game, keeper: &mut Entity) {
    if keeper.value("angered") != 0. {
        return;
    }
    for item in &mut game.world.entities {
        if item.alive && item.engine_active && crate::catalog::is_a(&item.object, "oItem") {
            item.data.insert("cost".into(), 0.);
            item.data.insert("forSale".into(), 0.);
        }
    }
    keeper.data.insert("angered".into(), 1.);
}

fn clear_sale(game: &mut Game, id: u64) {
    if let Some(entity) = game.world.entity_mut(id) {
        entity.data.insert("cost".into(), 0.);
        entity.data.insert("forSale".into(), 0.);
    }
}

fn acquire_sale_damsel(game: &mut Game, id: u64) -> bool {
    if !game
        .world
        .entity(id)
        .is_some_and(|e| e.object == "oDamsel" && e.value("cost") > 0.)
    {
        return false;
    }
    *game.data.entry("damselsBought".into()).or_default() += 1.;
    // scrStealItem tracks Damsel acquisition without an isRealLevel gate.
    game.lifetime_stats
        .increment(LifetimeCounter::DamselsBought);
    game.sound("sndPickup", game.player.pos);
    say(game, "YOU MUST BE IN LOVE!", "", 120);
    true
}

fn craps_keeper(game: &Game) -> bool {
    game.world.entities.iter().any(|e| {
        e.alive
            && e.health > 0
            && crate::catalog::is_a(&e.object, "oShopkeeper")
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
    if bet <= 0. {
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
        count(game, "diceGamesWon", LifetimeCounter::DiceGamesWon);
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
        say(game, "YOU ROLLED A SEVEN!", "YOU WIN A PRIZE!", 200);
    } else if total > 7 {
        count(game, "diceGamesWon", LifetimeCounter::DiceGamesWon);
        *game.data.entry("collect".into()).or_default() += bet * 2.;
        let counter = game.data.entry("collectCounter".into()).or_default();
        *counter = (*counter + 20.).min(100.);
        say(
            game,
            format!("YOU ROLLED A {total}!"),
            "CONGRATULATIONS! YOU WIN!",
            200,
        );
    } else {
        count(game, "diceGamesLost", LifetimeCounter::DiceGamesLost);
        say(
            game,
            format!("YOU ROLLED A {total}!"),
            "I'M SORRY, BUT YOU LOSE!",
            200,
        );
    }
    game.player.data.insert("bet".into(), 0.);
    for (id, _, _) in dice {
        if let Some(die) = game.world.entity_mut(id) {
            die.data.insert("rolled".into(), 0.);
        }
    }
}

pub(crate) fn keeper_step(game: &mut Game, keeper: &Entity) {
    if keeper.alive
        && keeper.health > 0
        && keeper.state == 0
        && keeper
            .text
            .get("style")
            .is_some_and(|style| style == "Craps")
    {
        settle_dice(game);
    }
}

/// oItem's inherited Step checks its pre-attachment position and camera gate.
pub(crate) fn item_step(game: &mut Game, item: &mut Entity) {
    if !(item.pos.x > game.camera.x - 16.
        && item.pos.x < game.camera.x + 336.
        && item.pos.y > game.camera.y - 16.
        && item.pos.y < game.camera.y + 256.)
        && item.object != "oRope"
        && item.object != "oRopeThrow"
    {
        return;
    }
    let keeper_exists = game
        .world
        .entities
        .iter()
        .any(|keeper| keeper.alive && crate::catalog::is_a(&keeper.object, "oShopkeeper"));
    let wanted =
        game.flags.contains("murderer") || game.data.get("thiefLevel").copied().unwrap_or(0.) > 0.;
    if (!keeper_exists || wanted) && item.value("cost") > 0. {
        count(game, "itemsStolen", LifetimeCounter::ItemsStolen);
        item.data.insert("cost".into(), 0.);
        item.data.insert("forSale".into(), 0.);
    }
    if game.is_level() {
        if item.value("cost") > 0. && item.value("forSale") > 0. && !in_shop(game, item.pos) {
            count(game, "itemsStolen", LifetimeCounter::ItemsStolen);
            anger(game, item.pos, "COME BACK HERE, THIEF!");
            item.data.insert("cost".into(), 0.);
        }
    } else {
        item.data.insert("cost".into(), 0.);
    }
}

/// Player Step580 runs before doors and its later inventory/pay actions.
pub(crate) fn player_held_step(game: &mut Game) {
    let Some(id) = game.player.held else {
        return;
    };
    if !game
        .world
        .entity(id)
        .is_some_and(|item| item.value("cost") > 0.)
    {
        return;
    }
    if game.is_level() && player_in_shop(game, game.player.pos) {
        return;
    }
    let damsel = acquire_sale_damsel(game, id);
    if !damsel {
        crate::items::collectibles::grant(game, id);
    }
    clear_sale(game, id);
    if game.is_level()
        && game
            .world
            .entities
            .iter()
            .any(|keeper| keeper.alive && crate::catalog::is_a(&keeper.object, "oShopkeeper"))
    {
        anger(game, game.player.pos, "COME BACK HERE, THIEF!");
    }
}

pub fn tick(game: &mut Game, input: &InputFrame) {
    if !matches!(game.state, RunState::Playing | RunState::Dead) || game.flags.contains("hub") {
        return;
    }
    // Source membership uses immutable roomPath entries, not the keeper's
    // following/patrol position. Retain each keeper's original shop room.
    for keeper in &mut game.world.entities {
        if crate::catalog::is_a(&keeper.object, "oShopkeeper") {
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
    if game.world.entities.iter().any(|keeper| {
        keeper.alive
            && keeper.health > 0
            && keeper.state == 0
            && keeper
                .text
                .get("style")
                .is_some_and(|style| style == "Craps")
    }) {
        settle_dice(game);
    }
    // A keeper struck by another subsystem reports anger through its state.
    let assaults: Vec<Vec2> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            crate::catalog::is_a(&e.object, "oShopkeeper")
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
        .any(|e| crate::catalog::is_a(&e.object, "oShopkeeper") && e.value("killed_by_player") > 0.)
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
        .any(|e| e.alive && e.health > 0 && crate::catalog::is_a(&e.object, "oShopkeeper"));
    let stock: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.value("forSale") > 0. && e.value("cost") > 0.)
        .map(|e| (e.id, e.pos))
        .collect();
    for (id, pos) in stock {
        if game.player.held == Some(id) && !in_shop(game, game.player.pos) {
            // oPlayer1's held-item theft check uses the player's room, before
            // the item's inherited Step has necessarily attached its offset.
            acquire_sale_damsel(game, id);
            clear_sale(game, id);
            if keepers_exist {
                anger(game, game.player.pos, "COME BACK HERE, THIEF!");
            }
            continue;
        }
        if game.player.held == Some(id) && (wanted || !keepers_exist) {
            acquire_sale_damsel(game, id);
        }
        if wanted || !keepers_exist {
            clear_sale(game, id);
            count(game, "itemsStolen", LifetimeCounter::ItemsStolen);
        } else if !in_shop(game, pos) {
            anger(game, pos, "COME BACK HERE, THIEF!");
            clear_sale(game, id);
            count(game, "itemsStolen", LifetimeCounter::ItemsStolen);
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
    player_pay_step(game, input);
}

/// Player Step1400, after inventory actions and before its hurt block.
pub(crate) fn player_pay_step(game: &mut Game, input: &InputFrame) {
    let wanted =
        game.flags.contains("murderer") || game.data.get("thiefLevel").copied().unwrap_or(0.) > 0.;
    if !input.pay_pressed
        || !game.is_level()
        || game.player.data.get("active").copied().unwrap_or(1.) == 0.
        || game.state == RunState::Dead
        || game.player.state == crate::model::PlayerState::Dead
        || game.player.is_stunned()
    {
        return;
    }
    if !player_in_shop(game, game.player.pos) {
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
            game.player.pickup_item_type.clear();
            say(game, "YOU HAVEN'T GOT ENOUGH MONEY!", "", 80);
            return;
        }
        if price > 0 {
            let damsel = acquire_sale_damsel(game, id);
            game.player.money -= price;
            count(game, "itemsBought", LifetimeCounter::ItemsBought);
            if !damsel {
                crate::items::collectibles::grant(game, id);
            }
            clear_sale(game, id);
            game.data.insert("message_timer".into(), 80.);
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
            say(game, "ONE BET AT A TIME!", "PLEASE ROLL THE DICE!", 200);
        } else if game.player.money >= cost {
            game.player.money -= cost;
            game.player.data.insert("bet".into(), cost as f64);
            count(game, "diceGamesPlayed", LifetimeCounter::DiceGamesPlayed);
            say(game, format!("YOU BET ${cost}!"), "NOW ROLL THE DICE!", 200);
        } else {
            say(game, format!("YOU NEED ${cost} TO BET!"), "", 200);
        }
        return;
    }
    if !game.world.entities.iter().any(|keeper| {
        keeper.alive
            && keeper.health > 0
            && crate::catalog::is_a(&keeper.object, "oShopkeeper")
            && keeper
                .text
                .get("style")
                .is_some_and(|style| style == "Kissing")
            && (
                keeper.value("shop_room_x") as i32,
                keeper.value("shop_room_y") as i32,
            ) == room(game.player.pos)
    }) {
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
        .map(|e| e.id);
    if let Some(id) = kiss {
        if wanted {
            return;
        }
        let cost = 5000_u32.saturating_mul(game.world.level);
        if game.player.money >= cost {
            game.player.money -= cost;
            game.player.health += 1;
            count(game, "kissesBought", LifetimeCounter::KissesBought);
            let damsel_character = game.flags.contains("character_damsel");
            let damsel = game.world.entity_mut(id).expect("kiss target");
            damsel.state = 6;
            damsel.animation = 0.;
            damsel.sprite = if damsel_character {
                "sPKissL"
            } else {
                "sDamselKissL"
            }
            .into();
            // The NPC animation emits sndKiss and the heart at frame 7.
            say(
                game,
                if damsel_character {
                    "NOW AIN'T HE SWEET!"
                } else {
                    "NOW AIN'T SHE SWEET!"
                },
                "",
                200,
            );
        } else {
            say(
                game,
                format!("YOU NEED ${cost}!"),
                "GET OUTTA HERE, DEADBEAT!",
                200,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GameEvent, RulesPreset, Vec2};
    #[test]
    fn actual_step_two_dice_defer_anger_latch_until_keeper_attack_step() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for die_count in [1, 2] {
                let mut game = Game::shell(123, preset);
                for x in 0..game.world.width {
                    game.world.set_tile(x, 10, "oBlock");
                }
                game.player.pos = Vec2::new(40., 152.);
                game.player.previous_pos = game.player.pos;
                let keeper = game.spawn("oShopkeeper", Vec2::new(96., 144.));
                for index in 0..die_count {
                    game.spawn("oDice", Vec2::new(200. + 24. * f64::from(index), 152.));
                }
                // Initial authored stock: the first is outside Item's own
                // viewport but inside Level's activation margin; the second
                // stays inactive. Neither can clear its own price here.
                let active_stock = game.spawn("oJetpack", Vec2::new(400., 120.));
                let inactive_stock = game.spawn("oJetpack", Vec2::new(600., 120.));
                for id in [active_stock, inactive_stock] {
                    let stock = game.world.entity_mut(id).unwrap();
                    stock.data.insert("cost".into(), 20000.);
                    stock.data.insert("forSale".into(), 1.);
                }

                game.step(&InputFrame::default());

                let expected_bounty = if die_count == 1 { 2. } else { 5. };
                assert_eq!(game.data["thiefLevel"], expected_bounty);
                assert_eq!(game.world.entity(keeper).unwrap().state, 2);
                assert_eq!(game.world.entity(keeper).unwrap().value("angered"), 0.);
                let messages = game
                    .events
                    .iter()
                    .filter(|event| {
                        matches!(event, GameEvent::Dialogue(message)
                        if message.first == "COME BACK HERE, THIEF!")
                    })
                    .count();
                assert_eq!(messages, die_count as usize);
                assert!(game.world.entity(active_stock).unwrap().engine_active);
                assert!(!game.world.entity(inactive_stock).unwrap().engine_active);
                for id in [active_stock, inactive_stock] {
                    assert_eq!(game.world.entity(id).unwrap().value("cost"), 20000.);
                    assert_eq!(game.world.entity(id).unwrap().value("forSale"), 1.);
                }

                game.step(&InputFrame::default());

                assert_eq!(game.world.entity(keeper).unwrap().value("angered"), 1.);
                assert_eq!(game.data["thiefLevel"], expected_bounty);
                assert_eq!(game.world.entity(active_stock).unwrap().value("cost"), 0.);
                assert_eq!(
                    game.world.entity(active_stock).unwrap().value("forSale"),
                    0.
                );
                assert_eq!(
                    game.world.entity(inactive_stock).unwrap().value("cost"),
                    20000.
                );
                assert_eq!(
                    game.world.entity(inactive_stock).unwrap().value("forSale"),
                    1.
                );
                for _ in 0..3 {
                    game.step(&InputFrame::default());
                    assert_eq!(game.data["thiefLevel"], expected_bounty);
                }
            }
        }
    }

    #[test]
    fn actual_step_natural_keeper_corpse_blocks_farther_keeper_anger_selection() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = Game::shell(123, preset);
            for x in 0..game.world.width {
                game.world.set_tile(x, 6, "oBlock");
            }
            // A separate raised shop keeps the second keeper active and out
            // of the shotgun trajectory and player attack range throughout.
            for x in 24..=26 {
                game.world.set_tile(x, 2, "oBlock");
            }
            for y in 0..2 {
                game.world.set_tile(24, y, "oBlock");
                game.world.set_tile(26, y, "oBlock");
            }
            game.player.pos = Vec2::new(64., 88.);
            game.player.previous_pos = game.player.pos;
            let gun = game.spawn("oShotgun", game.player.pos);
            let corpse = game.spawn("oShopkeeper", Vec2::new(80., 80.));
            let farther = game.spawn("oShopkeeper", Vec2::new(400., 16.));
            let die = game.spawn("oDice", Vec2::new(164., 88.));
            let action = InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            };
            game.step(&action);
            assert_eq!(game.player.held, Some(gun), "natural shotgun pickup");
            game.step(&InputFrame::default());
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            for _ in 0..200 {
                if game
                    .world
                    .entity(corpse)
                    .is_some_and(|keeper| keeper.value("dead") != 0.)
                {
                    break;
                }
                game.step(&InputFrame::default());
            }
            let dead_keeper = game.world.entity(corpse).unwrap();
            assert!(
                dead_keeper.health <= 0,
                "shotgun must actually kill nearest keeper"
            );
            assert_eq!(
                dead_keeper.value("dead"),
                1.,
                "corpse must naturally settle"
            );
            assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.), 0.);
            assert!(game.world.entity(farther).unwrap().health > 0);
            assert_eq!(game.world.entity(farther).unwrap().value("angered"), 0.);

            // All subsequent changes come from inputs: drop the gun, approach
            // the die, pick it up and carry it beyond the original shop room.
            game.step(&InputFrame::default());
            game.step(&action);
            assert_eq!(game.player.held, None);
            for _ in 0..60 {
                if game.player.pos.x >= 160. {
                    break;
                }
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            for _ in 0..12 {
                game.step(&InputFrame::default());
            }
            assert!((game.player.pos.x - game.world.entity(die).unwrap().pos.x).abs() < 8.);
            game.step(&action);
            assert_eq!(game.player.held, Some(die), "natural die pickup");
            let mut witnessed_outside_call = false;
            for _ in 0..12 {
                let caller = game.world.entity(die).unwrap().pos;
                if !in_shop(&game, caller) {
                    let nearest = game.world.entity(corpse).unwrap();
                    let other = game.world.entity(farther).unwrap();
                    assert!(nearest.engine_active && other.engine_active);
                    assert_eq!(nearest.value("dead"), 1.);
                    assert_eq!(other.value("dead"), 0.);
                    assert_eq!(other.value("angered"), 0.);
                    assert!(nearest.pos.distance(caller) < other.pos.distance(caller));
                    witnessed_outside_call = true;
                }
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
                assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.), 0.);
                assert_eq!(game.world.entity(farther).unwrap().value("angered"), 0.);
                if witnessed_outside_call {
                    break;
                }
            }
            assert!(
                witnessed_outside_call,
                "die must actually reach an outside-shop caller position"
            );
        }
    }

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
        assert_eq!(g.lifetime_stats.dice_games_won, 1);
        tick(&mut g, &InputFrame::default());
        assert_eq!(g.lifetime_stats.dice_games_won, 1);
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
    fn stepping_shop(object: &str, style: &str) -> (Game, u64) {
        let mut g = Game::shell(19, RulesPreset::Classic);
        for x in 0..g.world.width {
            g.world.set_tile(x, 4, "oTemple");
        }
        g.player.pos = Vec2::new(88., 56.);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        g.player.money = 100000;
        let keeper = g.world.spawn("oShopkeeper", Vec2::new(128., 48.));
        g.world
            .entity_mut(keeper)
            .unwrap()
            .text
            .insert("style".into(), style.into());
        let id = g.world.spawn(object, Vec2::new(96., 56.));
        let item = g.world.entity_mut(id).unwrap();
        item.held = true;
        item.data.insert("cost".into(), 15000.);
        item.data.insert("forSale".into(), 1.);
        g.player.held = Some(id);
        (g, id)
    }

    #[test]
    fn actual_step_damsel_purchase_keeps_her_in_hand_and_records_purchase() {
        let (mut g, id) = stepping_shop("oDamsel", "Kissing");
        g.step(&InputFrame {
            pay_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.money, 85000);
        assert_eq!(g.player.held, Some(id));
        assert_eq!(g.data["damselsBought"], 1.);
        assert_eq!(g.data["itemsBought"], 1.);
        assert_eq!(g.lifetime_stats.items_bought, 1);
        assert_eq!(g.lifetime_stats.damsels_bought, 1);
        assert_eq!(g.world.entity(id).unwrap().value("cost"), 0.);
        assert!(g.events.iter().any(|event| matches!(event,
            GameEvent::Dialogue(message) if message.first == "YOU MUST BE IN LOVE!" && message.second.is_empty() && message.ticks == 120)));
    }

    #[test]
    fn actual_step_kiss_requires_kissing_shop_and_starts_npc_animation() {
        for (style, expected_health) in [("Bomb", 4), ("Kissing", 5)] {
            let (mut g, id) = stepping_shop("oDamsel", style);
            g.player.held = None;
            g.world.entity_mut(id).unwrap().held = false;
            g.world.entity_mut(id).unwrap().state = 5;
            g.step(&InputFrame {
                pay_pressed: true,
                ..Default::default()
            });
            assert_eq!(g.player.health, expected_health, "shop style {style}");
            if style == "Kissing" {
                assert_eq!(g.lifetime_stats.kisses_bought, 1);
                assert_eq!(g.world.entity(id).unwrap().state, 6);
                assert_eq!(g.world.entity(id).unwrap().sprite, "sDamselKissL");
                assert!(
                    !g.events.iter().any(|event| matches!(event,
                    GameEvent::Sound { name, .. } if name == "sndKiss")),
                    "kiss sound belongs to NPC frame 7, not the payment tick"
                );
            } else {
                assert_eq!(g.player.money, 100000);
            }
        }
    }

    #[test]
    fn actual_step_custom_purchase_does_not_change_campaign_item_statistics() {
        let (mut g, _) = stepping_shop("oBombBag", "Bomb");
        g.flags.insert("custom_level".into());
        g.step(&InputFrame {
            pay_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.bombs, 7);
        assert!(!g.data.contains_key("itemsBought"));
        assert_eq!(g.lifetime_stats.items_bought, 0);
    }

    #[test]
    fn attacking_craps_keeper_does_not_settle_pending_wager() {
        let (mut g, _) = stepping_shop("oRock", "Craps");
        g.player.held = None;
        let keeper = g
            .world
            .entities
            .iter_mut()
            .find(|e| e.object == "oShopkeeper")
            .unwrap();
        keeper.state = 2;
        keeper.data.insert("angry".into(), 1.);
        keeper.data.insert("angered".into(), 1.);
        g.flags.insert("wanted".into());
        g.data.insert("thiefLevel".into(), 2.);
        g.player.data.insert("bet".into(), 1500.);
        for x in [80., 104.] {
            let id = g.world.spawn("oDice", Vec2::new(x, 56.));
            let die = g.world.entity_mut(id).unwrap();
            die.data.insert("rolled".into(), 1.);
            die.data.insert("value".into(), 5.);
        }
        tick(&mut g, &InputFrame::default());
        assert_eq!(g.player.data["bet"], 1500.);
        assert_eq!(g.data.get("collect").copied().unwrap_or(0.), 0.);
        assert!(!g.data.contains_key("diceGamesWon"));
    }

    #[test]
    fn shop_room_coordinates_match_source_outer_room_clamps() {
        assert_eq!(room(Vec2::new(-100., -100.)), (0, 0));
        assert_eq!(room(Vec2::new(176., 144.)), (1, 1));
        assert_eq!(room(Vec2::new(10000., 10000.)), (3, 4));
    }
    #[test]
    fn held_theft_uses_player_room_before_attachment_and_counts_damsel_once() {
        let (mut g, id) = stepping_shop("oDamsel", "Kissing");
        g.player.pos.x = 177.;
        // Left-facing held offset remains inside the previous shop room.
        g.world.entity_mut(id).unwrap().pos.x = 173.;
        tick(&mut g, &InputFrame::default());
        assert!(g.flags.contains("wanted"));
        assert_eq!(g.world.entity(id).unwrap().value("cost"), 0.);
        assert_eq!(g.data["damselsBought"], 1.);
        tick(&mut g, &InputFrame::default());
        assert_eq!(g.data["damselsBought"], 1.);
        assert_eq!(g.data["thiefLevel"], 2.);
    }
    #[test]
    fn zero_timer_airborne_stun_still_blocks_shop_payment() {
        let (mut g, id) = stepping_shop("oBombBag", "Bomb");
        g.player.stun = 0;
        g.player.state = crate::model::PlayerState::Stunned;
        g.player.sprite = "sDieLBounce".into();
        tick(
            &mut g,
            &InputFrame {
                pay_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.money, 100000);
        assert_eq!(g.player.bombs, 4);
        assert_eq!(g.world.entity(id).unwrap().value("forSale"), 1.);
    }
    #[test]
    fn healed_latched_dead_player_cannot_pay_for_shop_item() {
        let (mut g, id) = stepping_shop("oBombBag", "Bomb");
        g.player.health = 1;
        g.player.state = crate::model::PlayerState::Dead;
        g.state = RunState::Dead;
        tick(
            &mut g,
            &InputFrame {
                pay_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.money, 100000);
        assert_eq!(g.world.entity(id).unwrap().value("forSale"), 1.);
    }
}
