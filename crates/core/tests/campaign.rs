//! Positioned-interaction acceptance for generated campaign content.
//! These checks teleport the player and grant survivability; they do not prove
//! that the generated routes are physically traversable or source-equivalent.
mod common;

use common::complete_source_exit;
use spelunky_core::{Area, Game, InputFrame, Rect, RulesPreset, RunState, Vec2};

fn position_player(game: &mut Game, pos: Vec2) {
    game.player.pos = pos;
    game.player.previous_pos = pos;
    game.player.vel = Vec2::default();
    game.player.health = 99;
    game.player.invincible = 999;
    game.player.grounded = true;
    // This fixture teleports the owner between distant generated interactions.
    // Move its actual held instance too: in normal play oItem Step has already
    // kept it attached before the next room activation partition runs.
    if let Some(id) = game.player.held {
        let held = game.world.entity_mut(id).expect("held fixture instance");
        held.pos = pos;
        held.previous_pos = pos;
        held.vel = Vec2::default();
    }
    game.update_camera();
    game.previous_camera = game.camera;
}

fn positioned_exit(game: &mut Game) {
    let door = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oExit" && e.bounds().contains(game.world.exit))
        .expect("generated normal exit at the recorded destination");
    assert!(!game.world.solid_at(door.pos.x, door.pos.y));
    let bounds = door.bounds();
    position_player(
        game,
        Vec2::new(bounds.x + bounds.w / 2.0, bounds.y + bounds.h / 2.0),
    );
    // Teleporting does not establish the source ON_GROUND controller state.
    // Let the live controller settle before requesting the actual exit.
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Playing);
    game.step(&InputFrame {
        up: true,
        ..Default::default()
    });
    complete_source_exit(game);
}

#[test]
fn positioned_generated_doors_transition_through_all_four_campaign_areas() {
    for seed in [1, 42, 1337, 9999] {
        let mut game = Game::new(seed, RulesPreset::Classic);
        for level in 1..=15 {
            assert_eq!(game.world.level, level);
            assert_eq!(game.world.area, Area::for_level(level));
            positioned_exit(&mut game);
            game.next_level();
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game.world.level, level + 1);
            assert_eq!(game.world.area, Area::for_level(level + 1));
        }
    }
}

#[test]
fn generated_key_opens_generated_locked_chest_through_game_step() {
    let mut game = Game::new(42, RulesPreset::Classic);
    while !game
        .world
        .entities
        .iter()
        .any(|e| e.object == "oLockedChest")
    {
        assert!(
            game.world.level < 4,
            "scheduled Udjat chest must appear by level four"
        );
        positioned_exit(&mut game);
        game.next_level();
    }
    let key = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oKey")
        .unwrap()
        .id;
    let chest = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oLockedChest")
        .unwrap();
    let chest_id = chest.id;
    let chest_pos = chest.pos;
    // Use the actual generated key; isolate the chest interaction from pickup.
    game.player.held = Some(key);
    game.world.entity_mut(key).unwrap().held = true;
    position_player(&mut game, chest_pos);
    game.step(&InputFrame::default());
    assert!(game.world.entity(key).is_none());
    assert!(game.world.entity(chest_id).is_none());
    let eye_pos = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oUdjatEye")
        .unwrap()
        .pos;
    position_player(&mut game, eye_pos);
    game.step(&InputFrame {
        down: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(game.player.has("UdjatEye"));
}

#[test]
fn actual_pickups_and_walking_open_gold_door_without_entering_its_solid() {
    for crown in [false, true] {
        let mut level = spelunky_core::level::Level::default();
        level.set_tile(6, 7, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 8, '1').unwrap();
        }
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.load_custom(&level, "gold-door-approach", "");
        for _ in 0..8 {
            game.step(&InputFrame::default());
        }
        for object in if crown {
            vec!["oCrown", "oSceptre"]
        } else {
            vec!["oSceptre"]
        } {
            let id = game.spawn(object, Vec2::new(game.player.pos.x, game.player.pos.y + 4.));
            let item = game.world.entity_mut(id).unwrap();
            item.data.insert("cost".into(), 0.);
            item.data.insert("forSale".into(), 0.);
            game.step(&InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            game.step(&InputFrame::default());
        }
        assert_eq!(game.player.has("Crown"), crown);
        let sceptre = game.player.held.expect("actual pickup holds sceptre");
        assert_eq!(game.world.entity(sceptre).unwrap().object, "oSceptre");
        let p = game.player.pos;
        let door = game.spawn("oGoldDoor", Vec2::new(p.x + 32., p.y - 8.));
        for _ in 0..20 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game.player.health, 4);
            if game.world.entity(door).is_none() {
                break;
            }
        }
        assert_eq!(game.world.entity(door).is_none(), crown);
        assert_eq!(
            game.world.entities.iter().any(|e| e.object == "oXGold"),
            crown
        );
        assert_eq!(game.player.held.is_none(), crown);
        assert_eq!(game.world.entity(sceptre).is_none(), crown);
    }
}

fn generated_market_ankh_moai_chain_to_temple(seed: u64) -> Game {
    let mut game = Game::new(seed, RulesPreset::Classic);
    while !game.world.entities.iter().any(|e| e.object == "oXMarket") {
        assert!(
            game.world.level < 7,
            "market entrance scheduled by level seven"
        );
        positioned_exit(&mut game);
        game.next_level();
    }
    let market = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oXMarket")
        .unwrap();
    let market_pos = market.pos;
    // Isolate entrance use after discovery: remove its single covering tile,
    // preserving the generated supporting floor. This does not test excavation.
    game.world.set_tile(
        (market_pos.x / 16.0) as usize,
        (market_pos.y / 16.0) as usize,
        "",
    );
    position_player(&mut game, Vec2::new(market_pos.x + 8.0, market_pos.y + 8.0));
    game.step(&InputFrame {
        up: true,
        ..Default::default()
    });
    complete_source_exit(&mut game);
    game.next_level();
    assert_eq!(game.world.name, "Black Market");

    let ankh = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oAnkh")
        .unwrap();
    let ankh_id = ankh.id;
    let ankh_pos = ankh.pos;
    assert!(ankh.value("forSale") > 0.0);
    position_player(&mut game, ankh_pos);
    game.player.money = 1_000_000; // Purchase affordability is not this scenario's target.
    game.player.held = Some(ankh_id);
    game.world.entity_mut(ankh_id).unwrap().held = true;
    game.step(&InputFrame {
        pay_pressed: true,
        ..Default::default()
    });
    assert!(game.player.has("Ankh"));
    assert!(game.world.entity(ankh_id).is_none());

    while !game
        .world
        .entities
        .iter()
        .any(|e| e.object == "oMoaiInside")
    {
        assert!(game.world.level < 12, "Moai scheduled by level twelve");
        positioned_exit(&mut game);
        game.next_level();
    }
    let moai = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oMoaiInside")
        .unwrap();
    let moai_id = moai.id;
    let revival_pos = Vec2::new(moai.pos.x + 8.0, moai.pos.y + 8.0);
    // Source revival checks instance_exists(oMoai), which only sees active
    // instances. Stage the death beside the discovered Moai, rather than back
    // at a distant entrance whose activation region excludes the monument.
    position_player(&mut game, Vec2::new(revival_pos.x, revival_pos.y - 32.0));
    game.player.health = 0;
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.pos, revival_pos);
    assert!(!game.player.has("Ankh"));
    assert!(game.world.entity(moai_id).is_none());
    let crown_pos = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oCrown")
        .unwrap()
        .pos;
    position_player(&mut game, crown_pos);
    game.step(&InputFrame {
        down: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(game.player.has("Crown"));

    while game.world.level < 13 {
        positioned_exit(&mut game);
        game.next_level();
    }
    game
}

#[test]
fn exposed_generated_market_ankh_moai_and_gold_door_form_a_reachable_state_chain() {
    // scrEntityGen:385 enables Tomb Lord placement on thirteen, but :458–464
    // still requires eligible support and rand(1,40). Try a known natural
    // fixture first, then a bounded seed fallback if generation rolls change.
    // Select only this prerequisite after the complete asserted prefix; never
    // catch an interaction failure or filter on the downstream chain result.
    let mut game = [2, 0, 1, 3, 4, 5, 6, 7]
        .into_iter()
        .map(generated_market_ankh_moai_chain_to_temple)
        .find(|game| {
            game.world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.object == "oTombLord")
        })
        .expect("bounded natural chain fixtures must supply a level-thirteen Tomb Lord");
    let tomb = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oTombLord")
        .expect("selected chain seed must generate a Tomb Lord on level thirteen")
        .id;
    // Isolate the generated boss's death/drop transition from combat execution.
    let tomb_pos = game.world.entity(tomb).unwrap().pos;
    // Choose a nearby clear player pose instead of assuming the boss's center
    // is clear. Source crushing ignores the fixture's invincibility timer.
    let player_bounds = game.player.bounds();
    let clear_pose = |position: Vec2| {
        let bounds = Rect::new(
            position.x + player_bounds.x - game.player.pos.x,
            position.y + player_bounds.y - game.player.pos.y,
            player_bounds.w,
            player_bounds.h,
        );
        !game.world.tiles_collide(bounds, "oSolid")
            && !game.world.entities.iter().any(|entity| {
                entity.alive
                    && spelunky_core::catalog::is_a(&entity.object, "oSolid")
                    && entity.sprite_bounds().intersects(bounds)
            })
    };
    let staging = [
        Vec2::new(tomb_pos.x + 16.0, tomb_pos.y + 16.0),
        Vec2::new(tomb_pos.x - 8.0, tomb_pos.y + 16.0),
        Vec2::new(tomb_pos.x + 16.0, tomb_pos.y - 8.0),
        Vec2::new(tomb_pos.x + 40.0, tomb_pos.y + 16.0),
    ]
    .into_iter()
    .find(|position| clear_pose(*position))
    .expect("generated boss needs a nearby clear interaction pose");
    assert!(clear_pose(staging));
    position_player(&mut game, staging);
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
    game.world.entity_mut(tomb).unwrap().health = 0;
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Playing);
    assert!(game.player.health > 0);
    let sceptre_id = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oSceptre")
        .unwrap()
        .id;
    game.player.held = Some(sceptre_id);
    game.world.entity_mut(sceptre_id).unwrap().held = true;
    positioned_exit(&mut game);
    game.next_level();
    assert_eq!(game.world.level, 14);
    let door = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oGoldDoor")
        .unwrap();
    let door_id = door.id;
    let door_pos = door.pos;
    // GoldDoor is solid. Approach with the held sceptre instead of placing
    // the player inside it: Player Step crush precedes automatic Collision.
    position_player(&mut game, Vec2::new(door_pos.x - 8.0, door_pos.y + 8.0));
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
    for _ in 0..8 {
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert_eq!(game.state, RunState::Playing);
        assert!(game.player.health > 0);
        if game.world.entity(door_id).is_none() {
            break;
        }
    }
    assert!(game.world.entity(door_id).is_none());
    assert!(game.player.held.is_none());
    let gold_door = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oXGold")
        .unwrap();
    let gold_pos = gold_door.pos;
    position_player(&mut game, Vec2::new(gold_pos.x + 8.0, gold_pos.y + 8.0));
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Playing);
    game.step(&InputFrame {
        up: true,
        ..Default::default()
    });
    complete_source_exit(&mut game);
    game.next_level();
    assert_eq!(game.world.name, "City of Gold");
    assert_eq!(game.world.level, 15);
}
