//! oShopkeeper Step dialogue, including the source's welcome-time name draw.
use crate::{Entity, Game, GameEvent, SourceMessage, Vec2};

const NAMES: [&str; 32] = [
    "AHKMED", "TERRY", "SMITHY", "LEON", "ALI", "ELBERT", "KAO", "DUKE", "TONY", "GUERT", "PANCHO",
    "EARL", "IVAN", "OLLIE", "SPOONY", "BOB", "RUDY", "JIMBO", "TOR", "WILLY", "HAMISH", "LAZLO",
    "WANG", "HERBIE", "ANDY", "DONG", "LEMMY", "BARNEY", "LOU", "TARN", "SLASH", "BROM",
];

fn room(pos: Vec2) -> (i32, i32) {
    (
        (((pos.x - 16.0).max(0.0) / 160.0).floor() as i32).min(3),
        (((pos.y - 16.0).max(0.0) / 128.0).floor() as i32).min(4),
    )
}

/// `{PAY}` is resolved by the host from its current keyboard/controller binding.
pub(crate) fn say(
    game: &mut Game,
    first: impl Into<String>,
    second: impl Into<String>,
    ticks: u32,
) {
    game.data.insert("message_timer".into(), f64::from(ticks));
    game.events.push(GameEvent::Dialogue(SourceMessage {
        first: first.into(),
        second: second.into(),
        ticks,
    }));
}

pub(super) fn can_follow(game: &Game, keeper: &Entity) -> bool {
    keeper
        .text
        .get("style")
        .is_none_or(|style| style != "Craps")
        && room(keeper.pos) == room(game.player.pos)
        && game
            .player
            .held
            .and_then(|id| game.world.entity(id))
            .is_some_and(|item| item.value("cost") > 0.0)
}

/// Source purchase prompts run before the IDLE welcome branch, so a first
/// welcome replaces an existing held-item prompt during that source frame.
pub(super) fn tick(game: &mut Game, keeper: &mut Entity, peaceful: bool) {
    if matches!(keeper.state, 0 | 5) {
        if let Some(item) = game
            .player
            .held
            .and_then(|id| game.world.entity(id))
            .filter(|item| item.value("cost") > 0.0)
        {
            let description = purchase_description(game, item);
            say(game, description, "PRESS {PAY} TO PURCHASE.", 200);
        }
    }
    if keeper.state != 0
        || !peaceful
        || keeper.value("welcomed") != 0.0
        || room(keeper.pos) != room(game.player.pos)
    {
        return;
    }
    let style = keeper
        .text
        .get("style")
        .map(String::as_str)
        .unwrap_or("General");
    let first = if style == "Ankh" {
        "I HAVE SOMETHING SPECIAL...".into()
    } else {
        let name = NAMES[game.rng.range(32) as usize];
        let shop = match style {
            "Bomb" => "BOMB SHOP",
            "Weapon" => "ARMORY",
            "Clothing" => "CLOTHING SHOP",
            "Rare" => "SPECIALTY SHOP",
            "Craps" => "DICE HOUSE",
            "Kissing" => "KISSING PARLOR",
            _ => "SUPPLY SHOP",
        };
        format!("WELCOME TO {name}'S {shop}!")
    };
    let second = match style {
        "Craps" => format!(
            "PRESS {{PAY}} TO BET ${}.",
            1000_u32.saturating_add(game.world.level.saturating_mul(500))
        ),
        "Kissing" => format!(
            "${} A KISS. PRESS {{PAY}}.",
            game.world.level.saturating_mul(5000)
        ),
        _ => String::new(),
    };
    say(game, first, second, 200);
    keeper.data.insert("welcomed".into(), 1.0);
}

fn purchase_description(game: &Game, item: &Entity) -> String {
    if let Some(description) = item.text.get("buyMessage") {
        return description.clone();
    }
    let cost = item.value("cost").max(0.0) as u32;
    if item.object == "oDamsel" {
        let pronoun = if game.flags.contains("character_damsel") {
            "HIM"
        } else {
            "HER"
        };
        return format!("I'LL LET YOU HAVE {pronoun} FOR ${cost}!");
    }
    if item.object == "oJordans" {
        return "JORDANS FOR $50000!".into();
    }
    if matches!(
        item.object.as_str(),
        "oUdjatEye" | "oCrown" | "oSceptre" | "oKapala"
    ) {
        return "I SHOULDN'T BE SELLING THIS!".into();
    }
    // scrShopItemsGen rebuilds the description after level 2. This deliberately
    // retains source wording such as A SPECTACLES and A ROPE PILE there.
    let scaled_stock =
        item.value("forSale") > 0.0 && game.world.level > 2 && item.object != "oAnkh";
    let description = match item.object.as_str() {
        "oBombBag" => "A BAG OF 3 BOMBS",
        "oBombBox" => "A BOX OF 12 BOMBS",
        "oPaste" => "BOMB PASTE",
        "oSpringShoes" => "SPRINGY SHOES",
        "oGloves" => "CLIMBING GLOVES",
        "oMitt" => "PITCHER'S MITT",
        "oBow" => "BOW AND ARROWS",
        "oRopePile" if scaled_stock => "A ROPE PILE",
        "oRopePile" => "EXTRA ROPE",
        "oSpectacles" if scaled_stock => "A SPECTACLES",
        "oSpectacles" => "SPECTACLES",
        "oJetpack" if scaled_stock => "A JETPACK",
        "oJetpack" => "JETPACK",
        "oSpikeShoes" if scaled_stock => "A SPIKE SHOES",
        "oSpikeShoes" => "SPIKE SHOES",
        "oAnkh" => "AN ANKH",
        "oCompass" => "A COMPASS",
        "oParaPickup" => "A PARACHUTE",
        "oCapePickup" => "A CAPE",
        "oMattock" => "A MATTOCK",
        "oPistol" => "A PISTOL",
        "oShotgun" => "A SHOTGUN",
        "oWebCannon" => "A WEB CANNON",
        "oMachete" => "A MACHETE",
        "oTeleporter" => "A TELEPORTER",
        _ => "AN ITEM",
    };
    format!("{description} FOR ${cost}.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InputFrame, RulesPreset};

    fn fixture(style: &str) -> (Game, Entity) {
        let mut game = Game::shell(1337, RulesPreset::Classic);
        game.player.pos = Vec2::new(24.0, 56.0);
        let mut keeper = Entity::new(1, "oShopkeeper", Vec2::new(160.0, 48.0));
        keeper.text.insert("style".into(), style.into());
        (game, keeper)
    }

    fn last(game: &Game) -> &SourceMessage {
        game.events
            .iter()
            .rev()
            .find_map(|event| match event {
                GameEvent::Dialogue(message) => Some(message),
                _ => None,
            })
            .expect("source dialogue")
    }

    #[test]
    fn every_style_draws_one_name_at_first_same_room_welcome_except_ankh() {
        for (style, suffix) in [
            ("General", "SUPPLY SHOP"),
            ("Bomb", "BOMB SHOP"),
            ("Weapon", "ARMORY"),
            ("Clothing", "CLOTHING SHOP"),
            ("Rare", "SPECIALTY SHOP"),
            ("Craps", "DICE HOUSE"),
            ("Kissing", "KISSING PARLOR"),
            ("unknown", "SUPPLY SHOP"),
            ("Ankh", ""),
        ] {
            let (mut game, mut keeper) = fixture(style);
            let mut expected = game.rng.clone();
            let first = if style == "Ankh" {
                "I HAVE SOMETHING SPECIAL...".to_owned()
            } else {
                format!(
                    "WELCOME TO {}'S {suffix}!",
                    NAMES[expected.range(32) as usize]
                )
            };
            tick(&mut game, &mut keeper, true);
            assert_eq!(last(&game).first, first, "{style}");
            assert_eq!(last(&game).ticks, 200);
            assert_eq!(game.data["message_timer"], 200.0);
            assert_eq!(
                game.rng.next_u64(),
                expected.next_u64(),
                "name draw for {style}"
            );
            game.events.clear();
            let before = game.rng.clone();
            tick(&mut game, &mut keeper, true);
            assert!(game.events.is_empty());
            assert_eq!(game.rng.next_u64(), before.clone().next_u64());
        }
    }

    #[test]
    fn adjacent_room_and_angry_or_following_keeper_do_not_welcome_or_draw_name() {
        for (x, state, peaceful) in [(200.0, 0, true), (24.0, 0, false), (24.0, 5, true)] {
            let (mut game, mut keeper) = fixture("Bomb");
            game.player.pos.x = x;
            keeper.state = state;
            let mut expected = game.rng.clone();
            tick(&mut game, &mut keeper, peaceful);
            assert!(game.events.is_empty());
            assert_eq!(keeper.value("welcomed"), 0.0);
            assert_eq!(game.rng.next_u64(), expected.next_u64());
        }
    }

    #[test]
    fn source_room_boundaries_and_outer_clamps_are_preserved() {
        for (pos, expected) in [
            (Vec2::new(-100.0, -100.0), (0, 0)),
            (Vec2::new(175.0, 143.0), (0, 0)),
            (Vec2::new(176.0, 144.0), (1, 1)),
            (Vec2::new(336.0, 272.0), (2, 2)),
            (Vec2::new(496.0, 400.0), (3, 3)),
            (Vec2::new(9999.0, 528.0), (3, 4)),
        ] {
            assert_eq!(room(pos), expected);
        }
    }

    #[test]
    fn dice_and_kissing_welcomes_show_source_second_line() {
        for (style, second) in [
            ("Craps", "PRESS {PAY} TO BET $2500."),
            ("Kissing", "$15000 A KISS. PRESS {PAY}."),
        ] {
            let (mut game, mut keeper) = fixture(style);
            game.world.level = 3;
            tick(&mut game, &mut keeper, true);
            assert_eq!(last(&game).second, second);
        }
    }

    #[test]
    fn purchase_prompt_refreshes_source_scaled_description_and_follow_constraints() {
        let (mut game, mut keeper) = fixture("Bomb");
        keeper.data.insert("welcomed".into(), 1.0);
        let id = game.world.spawn("oRopePile", game.player.pos);
        game.player.held = Some(id);
        let item = game.world.entity_mut(id).unwrap();
        item.data.insert("cost".into(), 2750.0);
        item.data.insert("forSale".into(), 1.0);
        game.world.level = 3;
        tick(&mut game, &mut keeper, true);
        assert_eq!(last(&game).first, "A ROPE PILE FOR $2750.");
        assert_eq!(last(&game).second, "PRESS {PAY} TO PURCHASE.");
        assert!(can_follow(&game, &keeper));
        keeper.text.insert("style".into(), "Craps".into());
        assert!(!can_follow(&game, &keeper));
        keeper.text.insert("style".into(), "Bomb".into());
        game.player.pos.x = 200.0;
        assert!(!can_follow(&game, &keeper));
        game.data.insert("message_timer".into(), 1.0);
        game.events.clear();
        tick(&mut game, &mut keeper, true);
        assert_eq!(game.data["message_timer"], 200.0);
    }

    #[test]
    fn purchase_descriptions_keep_source_articles_special_items_and_price_scaling_quirks() {
        let (mut game, _) = fixture("General");
        game.world.level = 2;
        for (object, description) in [
            ("oBombBag", "A BAG OF 3 BOMBS"),
            ("oBombBox", "A BOX OF 12 BOMBS"),
            ("oRopePile", "EXTRA ROPE"),
            ("oSpectacles", "SPECTACLES"),
            ("oJetpack", "JETPACK"),
            ("oSpikeShoes", "SPIKE SHOES"),
            ("oSpringShoes", "SPRINGY SHOES"),
            ("oGloves", "CLIMBING GLOVES"),
            ("oMitt", "PITCHER'S MITT"),
            ("oPaste", "BOMB PASTE"),
            ("oBow", "BOW AND ARROWS"),
            ("oAnkh", "AN ANKH"),
        ] {
            let mut item = Entity::new(99, object, game.player.pos);
            item.data.insert("cost".into(), 1234.0);
            item.data.insert("forSale".into(), 1.0);
            assert_eq!(
                purchase_description(&game, &item),
                format!("{description} FOR $1234."),
                "{object}"
            );
        }
        game.world.level = 3;
        for (object, description) in [
            ("oSpectacles", "A SPECTACLES"),
            ("oRopePile", "A ROPE PILE"),
            ("oJetpack", "A JETPACK"),
            ("oSpikeShoes", "A SPIKE SHOES"),
            ("oAnkh", "AN ANKH"),
        ] {
            let mut item = Entity::new(99, object, game.player.pos);
            item.data.insert("cost".into(), 1357.0);
            item.data.insert("forSale".into(), 1.0);
            assert_eq!(
                purchase_description(&game, &item),
                format!("{description} FOR $1357.")
            );
            item.data.insert("forSale".into(), 0.0);
            if object == "oRopePile" {
                assert_eq!(purchase_description(&game, &item), "EXTRA ROPE FOR $1357.");
            }
        }
        for object in ["oUdjatEye", "oCrown", "oKapala", "oSceptre"] {
            assert_eq!(
                purchase_description(&game, &Entity::new(99, object, game.player.pos)),
                "I SHOULDN'T BE SELLING THIS!"
            );
        }
        let mut damsel = Entity::new(99, "oDamsel", game.player.pos);
        damsel.data.insert("cost".into(), 45000.0);
        assert_eq!(
            purchase_description(&game, &damsel),
            "I'LL LET YOU HAVE HER FOR $45000!"
        );
        game.flags.insert("character_damsel".into());
        assert_eq!(
            purchase_description(&game, &damsel),
            "I'LL LET YOU HAVE HIM FOR $45000!"
        );
        assert_eq!(
            purchase_description(&game, &Entity::new(99, "oJordans", game.player.pos)),
            "JORDANS FOR $50000!"
        );
    }

    #[test]
    fn natural_generated_shop_entrance_walk_welcomes_immediately_once() {
        // Source generation changes can move or replace a seeded shop. Select
        // an actual Bomb shop with a quiet viewport, without filtering on its
        // welcome result. GiantSpider Create also draws RNG outside the view.
        let (mut game, keeper, entry, camera) = std::iter::once(20)
            .chain((0..64).filter(|seed| *seed != 20))
            .find_map(|seed| {
                let mut game = Game::new(seed, RulesPreset::Classic);
                game.data.insert("next_level".into(), 2.0);
                game.next_level();
                let keeper = game
                    .world
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.object == "oShopkeeper"
                            && entity
                                .text
                                .get("style")
                                .is_some_and(|style| style == "Bomb")
                    })?
                    .clone();
                let entry = Vec2::new(keeper.pos.x + 120.0, keeper.pos.y + 8.0);
                let camera = Vec2::new((entry.x - 160.0).max(0.0), (entry.y - 120.0).max(0.0));
                let other_rng_actor = game.world.entities.iter().any(|entity| {
                    entity.id != keeper.id
                        && (entity.object == "oGiantSpider"
                            || crate::catalog::is_a(&entity.object, "oEnemy")
                                && entity.pos.x > camera.x - 36.0
                                && entity.pos.x < camera.x + 324.0
                                && entity.pos.y > camera.y - 36.0
                                && entity.pos.y < camera.y + 244.0)
                });
                if other_rng_actor
                    || !game.world.solid_at(entry.x, entry.y + 8.0)
                    || game.world.solid_at(entry.x, entry.y)
                {
                    return None;
                }
                Some((game, keeper, entry, camera))
            })
            .expect("bounded natural seeds must include a quiet Bomb shop entrance");
        assert_eq!(keeper.value("welcomed"), 0.0);
        game.player.pos = entry;
        game.player.previous_pos = game.player.pos;
        game.player.grounded = true;
        game.camera = camera;
        game.previous_camera = game.camera;
        let mut expected = game.rng.clone();
        let name = NAMES[expected.range(32) as usize];
        game.step(&InputFrame {
            left: true,
            ..Default::default()
        });
        assert_eq!(last(&game).first, format!("WELCOME TO {name}'S BOMB SHOP!"));
        assert_eq!(game.data["message_timer"], 200.0);
        assert_eq!(game.rng.next_u64(), expected.next_u64());
        for _ in 0..20 {
            game.step(&InputFrame {
                left: true,
                ..Default::default()
            });
            assert!(!game.events.iter().any(|event|matches!(event,GameEvent::Dialogue(message) if message.first.starts_with("WELCOME"))));
        }
        assert!(game.player.pos.x < entry.x);
        assert_eq!(game.player.health, 4);
    }

    #[test]
    fn actual_step_first_welcome_can_enter_follow_during_same_source_frame() {
        let mut game = Game::shell(1337, RulesPreset::Classic);
        for x in 0..game.world.width {
            game.world.set_tile(x, 4, "oTemple");
        }
        game.player.pos = Vec2::new(24.0, 56.0);
        game.player.previous_pos = game.player.pos;
        game.player.grounded = true;
        let keeper = game.world.spawn("oShopkeeper", Vec2::new(160.0, 48.0));
        game.world
            .entity_mut(keeper)
            .unwrap()
            .text
            .insert("style".into(), "Bomb".into());
        let item = game.world.spawn("oMattock", game.player.pos);
        game.player.held = Some(item);
        let stock = game.world.entity_mut(item).unwrap();
        stock.held = true;
        stock.data.insert("cost".into(), 8000.0);
        stock.data.insert("forSale".into(), 1.0);
        game.step(&InputFrame::default());
        assert!(last(&game).first.starts_with("WELCOME TO "));
        assert_eq!(game.world.entity(keeper).unwrap().state, 5);
        game.step(&InputFrame::default());
        assert_eq!(last(&game).first, "A MATTOCK FOR $8000.");
        assert_eq!(last(&game).second, "PRESS {PAY} TO PURCHASE.");
    }
}
