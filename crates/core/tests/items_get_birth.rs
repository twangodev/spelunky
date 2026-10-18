//! Source pickup displays from initial actors, followed only by ordinary input.
//! Native default oItemsGet lifecycle; no imported velocity/yAcc alias claim.
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

const EQUIPMENT: [(&str, &str); 15] = [
    ("oUdjatEye", "UdjatEye"),
    ("oAnkh", "Ankh"),
    ("oCrown", "Crown"),
    ("oKapala", "Kapala"),
    ("oPaste", "Paste"),
    ("oParaPickup", "Parachute"),
    ("oSpectacles", "Spectacles"),
    ("oGloves", "Gloves"),
    ("oMitt", "Mitt"),
    ("oCompass", "Compass"),
    ("oSpringShoes", "SpringShoes"),
    ("oSpikeShoes", "SpikeShoes"),
    // Jordans is an authored dormant class: no ordinary XML/GML Create caller.
    ("oJordans", "Jordans"),
    ("oCapePickup", "Cape"),
    ("oJetpack", "Jetpack"),
];

fn room(rules: RulesPreset, facing: i8, x: f64) -> Game {
    let mut game = Game::shell(23, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for column in 0..game.world.width {
        game.world.set_tile(column, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(x, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = game.player.pos;
    game
}

fn free_actor(game: &mut Game, object: &str, pos: Vec2) -> u64 {
    let id = game.spawn(object, pos);
    // Initial authored free-loot metadata mirrors explicit source free-item
    // producers; no actor fields are written after the first ordinary input.
    let item = game.world.entity_mut(id).unwrap();
    item.data.insert("cost".into(), 0.0);
    item.data.insert("forSale".into(), 0.0);
    id
}

fn pickup(game: &mut Game) {
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
}

fn popup_ids(game: &Game) -> Vec<u64> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oItemsGet")
        .map(|e| e.id)
        .collect()
}

fn pickup_sound_count(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndPickup"))
        .count()
}

fn check_popup(game: &Game, id: u64, stock_pos: Vec2, offset: f64, sprite: &str) {
    let popup = game.world.entity(id).expect("source pickup display");
    let birth = Vec2::new(stock_pos.x, stock_pos.y - offset);
    assert_eq!(
        popup.previous_pos, birth,
        "caller creates display at stock position before retirement"
    );
    // ItemsGet595 follows Player319, so even a newborn runs its own ceil Step.
    assert_eq!(popup.pos, Vec2::new(birth.x.ceil(), (birth.y - 0.1).ceil()));
    assert_eq!(popup.sprite, sprite);
    assert_eq!(popup.age, 0);
    assert_eq!(popup.alarms[0], 40, "born after this frame's Alarm0 phase");
    assert_eq!(popup.image_speed(), 0.8);
    assert_eq!(popup.animation, 0.8);
    assert_eq!(popup.vel.y, 0.1);
}

#[test]
fn free_resource_contact_creates_one_literal_icon_and_default_lifetime() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for (object, sprite, offset, bombs, ropes) in [
                ("oBombBag", "sBombsGet", 14.0, 7, 4),
                ("oBombBox", "sBombsGet", 14.0, 16, 4),
                ("oRopePile", "sRopeGet", 15.0, 4, 7),
            ] {
                let mut game = room(rules, facing, 104.0);
                let pos = Vec2::new(104.25, 152.25);
                let stock = free_actor(&mut game, object, pos);
                let rng = game.rng.state_bits();
                game.step(&InputFrame::default());
                assert!(game.world.entity(stock).is_none());
                assert_eq!(
                    (game.player.bombs, game.player.ropes),
                    (f64::from(bombs), f64::from(ropes))
                );
                assert_eq!(game.player.held, None);
                assert_eq!(pickup_sound_count(&game), 1);
                let ids = popup_ids(&game);
                assert_eq!(
                    ids.len(),
                    1,
                    "one source resource grant creates exactly one display"
                );
                check_popup(&game, ids[0], pos, offset, sprite);
                assert_eq!(
                    game.rng.state_bits(),
                    rng,
                    "fixed native display Create consumes no RNG"
                );
                for remaining in (1..40).rev() {
                    game.step(&InputFrame::default());
                    assert_eq!(game.world.entity(ids[0]).unwrap().alarms[0], remaining);
                    assert_eq!(popup_ids(&game), ids);
                }
                game.step(&InputFrame::default());
                assert!(
                    game.world.entity(ids[0]).is_none(),
                    "fortieth subsequent Alarm0 destroys display"
                );
                assert!(popup_ids(&game).is_empty());
                assert_eq!(game.player.health, 4.0);
            }
        }
    }
}

#[test]
fn free_equipment_ordinary_pickup_uses_default_items_get_after_grant() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for (object, equipment) in EQUIPMENT {
                let mut game = room(rules, facing, 104.0);
                let pos = Vec2::new(104.25, 152.25);
                let stock = free_actor(&mut game, object, pos);
                let rng = game.rng.state_bits();
                pickup(&mut game);
                assert!(
                    game.player.has(equipment),
                    "ordinary {object} pickup grants source flag"
                );
                assert!(game.world.entity(stock).is_none());
                assert_eq!(game.player.held, None);
                assert_eq!(pickup_sound_count(&game), 1);
                let ids = popup_ids(&game);
                assert_eq!(ids.len(), 1);
                check_popup(&game, ids[0], pos, 14.0, "sItemGet");
                assert_eq!(game.rng.state_bits(), rng);
                assert_eq!(game.player.health, 4.0);
            }
        }
    }
}

#[test]
fn dormant_rope_bag_alias_keeps_exactly_one_bomb_icon_control() {
    // Explicit authored dormant class, not a claim of regular level-generation
    // reachability. Its source own type is Bomb Bag, not Rope Pile.
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let mut game = room(rules, facing, 104.0);
            let pos = Vec2::new(104.25, 152.25);
            let stock = free_actor(&mut game, "oRopeBag", pos);
            assert_eq!(
                game.world
                    .entity(stock)
                    .unwrap()
                    .text
                    .get("type")
                    .map(String::as_str),
                Some("Bomb Bag")
            );
            pickup(&mut game);
            assert!(game.world.entity(stock).is_none());
            assert_eq!((game.player.bombs, game.player.ropes), (7.0, 4.0));
            assert_eq!(game.player.held, None);
            let ids = popup_ids(&game);
            assert_eq!(
                ids.len(),
                1,
                "shared display path cannot duplicate existing alias display"
            );
            check_popup(&game, ids[0], pos, 14.0, "sBombsGet");
            assert_eq!(pickup_sound_count(&game), 1);
        }
    }
}

fn shop_keeper(game: &mut Game, style: &str) -> u64 {
    let keeper = game.spawn("oShopkeeper", Vec2::new(144.0, 144.0));
    game.world
        .entity_mut(keeper)
        .unwrap()
        .text
        .insert("style".into(), style.into());
    keeper
}

#[test]
fn paid_equipment_creates_display_at_held_stock_before_consumption() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let mut game = room(rules, facing, 104.0);
            game.player.money = 5000;
            let keeper = shop_keeper(&mut game, "General");
            let stock = game.spawn("oCompass", game.player.pos);
            game.world
                .entity_mut(stock)
                .unwrap()
                .data
                .insert("forSale".into(), 1.0);
            assert_eq!(game.world.entity(stock).unwrap().value("cost"), 3000.0);
            pickup(&mut game);
            assert_eq!(game.player.held, Some(stock));
            assert!(!game.player.has("Compass"));
            assert!(popup_ids(&game).is_empty());
            game.step(&InputFrame {
                attack_released: true,
                ..Default::default()
            });
            let pos = game.world.entity(stock).unwrap().pos;
            game.step(&InputFrame {
                pay_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.money, 2000);
            assert!(game.player.has("Compass"));
            assert_eq!(game.player.held, None);
            assert!(game.world.entity(stock).is_none());
            assert_eq!(game.lifetime_stats.items_bought, 1);
            let ids = popup_ids(&game);
            assert_eq!(ids.len(), 1);
            check_popup(&game, ids[0], pos, 14.0, "sItemGet");
            assert_eq!(pickup_sound_count(&game), 1);
            assert_eq!(game.world.entity(keeper).unwrap().health, 20);
        }
    }
}

#[test]
fn native_price_no_sale_prize_theft_creates_display_when_player_leaves_shop() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let mut game = room(rules, facing, 168.0);
            let keeper = shop_keeper(&mut game, "Craps");
            let stock = game.spawn("oCompass", game.player.pos);
            // q/High End prize retains positive native price and no sale
            // marker. Only initial prize metadata is authored here.
            game.world
                .entity_mut(stock)
                .unwrap()
                .data
                .insert("inDiceHouse".into(), 1.0);
            assert_eq!(game.world.entity(stock).unwrap().value("cost"), 3000.0);
            assert_eq!(game.world.entity(stock).unwrap().value("forSale"), 0.0);
            pickup(&mut game);
            assert_eq!(game.player.held, Some(stock));
            game.step(&InputFrame {
                attack_released: true,
                ..Default::default()
            });
            let mut witnessed = false;
            for _ in 0..24 {
                let pos = game.world.entity(stock).unwrap().pos;
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
                if game.player.has("Compass") {
                    assert!(
                        game.player.pos.x >= 176.0,
                        "ordinary movement leaves keeper's shop room"
                    );
                    assert_eq!(game.player.held, None);
                    assert!(game.world.entity(stock).is_none());
                    let ids = popup_ids(&game);
                    assert_eq!(ids.len(), 1);
                    check_popup(&game, ids[0], pos, 14.0, "sItemGet");
                    assert_eq!(pickup_sound_count(&game), 1);
                    witnessed = true;
                    break;
                }
                assert!(popup_ids(&game).is_empty());
            }
            assert!(
                witnessed,
                "bounded ordinary walk must trigger existing held-price theft branch"
            );
            assert_eq!(game.player.money, 0);
            assert_eq!(game.player.health, 4.0);
            assert_eq!(game.world.entity(keeper).unwrap().health, 20);
        }
    }
}

#[test]
fn natural_back_slot_swaps_create_display_before_replacement_gear() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for (first, first_flag, second, second_flag) in [
                ("oCapePickup", "Cape", "oJetpack", "Jetpack"),
                ("oJetpack", "Jetpack", "oCapePickup", "Cape"),
            ] {
                let mut game = room(rules, facing, 104.0);
                let pos = Vec2::new(104.25, 152.25);
                let old_stock = free_actor(&mut game, first, pos);
                let next_stock = free_actor(&mut game, second, pos);
                // Exact same initial position: source strict-nearest tie selects
                // first registration; the source prerequisite below is mandatory.
                pickup(&mut game);
                assert!(
                    game.player.has(first_flag),
                    "SETUP: first real back-slot pickup"
                );
                assert!(!game.player.has(second_flag));
                assert!(game.world.entity(old_stock).is_none());
                assert!(game.world.entity(next_stock).is_some());
                assert_eq!(popup_ids(&game).len(), 1);
                game.step(&InputFrame {
                    attack_released: true,
                    ..Default::default()
                });
                let birth_stock_pos = game.world.entity(next_stock).unwrap().pos;
                pickup(&mut game);
                assert!(game.player.has(second_flag));
                assert!(!game.player.has(first_flag));
                assert_eq!(game.player.held, None);
                assert!(game.world.entity(next_stock).is_none());
                let popups = popup_ids(&game);
                assert_eq!(popups.len(), 2);
                let new_popup = *popups.last().unwrap();
                check_popup(&game, new_popup, birth_stock_pos, 14.0, "sItemGet");
                let dropped: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == first)
                    .collect();
                assert_eq!(dropped.len(), 1);
                assert!(
                    dropped[0].id > new_popup,
                    "source display Create precedes opposite back-slot Create"
                );
                assert_eq!(
                    dropped[0].previous_pos, game.player.pos,
                    "replacement originates at Player, not consumed stock"
                );
                assert_eq!(dropped[0].value("cost"), 0.0);
                assert_eq!(dropped[0].value("forSale"), 0.0);
                assert!(!dropped[0].held);
                assert_eq!(pickup_sound_count(&game), 1);
                assert_eq!(game.player.health, 4.0);
            }
        }
    }
}

#[test]
fn paid_weapon_and_damsel_branches_retain_hand_without_display_controls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for (object, price) in [("oPistol", 5000.0), ("oDamsel", 15000.0)] {
                let mut game = room(rules, facing, 104.0);
                game.player.money = 20000;
                let keeper = shop_keeper(&mut game, "General");
                let stock = game.spawn(object, game.player.pos);
                let item = game.world.entity_mut(stock).unwrap();
                item.data.insert("forSale".into(), 1.0);
                if object == "oDamsel" {
                    item.state = 5;
                    item.data.insert("status".into(), 5.0);
                }
                assert_eq!(item.value("cost"), price);
                pickup(&mut game);
                assert_eq!(game.player.held, Some(stock));
                game.step(&InputFrame {
                    attack_released: true,
                    ..Default::default()
                });
                game.step(&InputFrame {
                    pay_pressed: true,
                    ..Default::default()
                });
                assert_eq!(game.player.money, 20000 - price as u32);
                assert_eq!(game.player.held, Some(stock));
                assert_eq!(game.world.entity(stock).unwrap().value("cost"), 0.0);
                assert!(
                    popup_ids(&game).is_empty(),
                    "source weapon/Damsel popup lines are commented out"
                );
                assert_eq!(pickup_sound_count(&game), 1);
                assert_eq!(game.player.health, 4.0);
                assert_eq!(game.world.entity(keeper).unwrap().health, 20);
            }
        }
    }
}

#[test]
fn free_resource_auto_collection_preserves_a_naturally_held_rock() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let mut game = room(rules, facing, 104.0);
            let rock = game.spawn("oRock", game.player.pos);
            let bag = free_actor(&mut game, "oBombBag", Vec2::new(120.0, 152.0));
            pickup(&mut game);
            assert_eq!(
                game.player.held,
                Some(rock),
                "SETUP: real Rock pickup before bag contact"
            );
            assert_eq!(game.player.pickup_item_type, "Rock");
            assert_eq!(game.player.bombs, 4.0);
            assert!(game.world.entity(bag).is_some());
            assert!(popup_ids(&game).is_empty());
            game.step(&InputFrame {
                attack_released: true,
                ..Default::default()
            });
            let mut witnessed = false;
            for _ in 0..16 {
                let pos = game.world.entity(bag).unwrap().pos;
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
                if game.world.entity(bag).is_none() {
                    assert_eq!(game.player.bombs, 7.0);
                    assert_eq!(
                        game.player.held,
                        Some(rock),
                        "automatic resource grant preserves unrelated hand"
                    );
                    assert_eq!(game.player.pickup_item_type, "Rock");
                    assert!(game.world.entity(rock).unwrap().held);
                    let ids = popup_ids(&game);
                    assert_eq!(ids.len(), 1);
                    check_popup(&game, ids[0], pos, 14.0, "sBombsGet");
                    assert_eq!(pickup_sound_count(&game), 1);
                    witnessed = true;
                    break;
                }
                assert_eq!(game.player.held, Some(rock));
                assert!(popup_ids(&game).is_empty());
            }
            assert!(
                witnessed,
                "ordinary movement must enter adjacent free Bag contact"
            );
            assert_eq!(game.player.health, 4.0);
        }
    }
}
