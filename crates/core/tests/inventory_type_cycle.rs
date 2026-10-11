//! Source inventory cycles destroy the selected actor, retaining only its type.
//! Authored quiet room; all Bow/Flare acquisition and cycling below use inputs.
use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, Vec2};

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(42, rules);
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oBrickSmooth");
    }
    game.player.pos = Vec2::new(64.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    game
}

fn pickup(game: &mut Game, object: &str) -> u64 {
    let id = game.spawn(object, game.player.pos);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(id), "natural pickup of {object}");
    game.step(&InputFrame::default());
    id
}

fn draw(game: &mut Game) {
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    for _ in 0..4 {
        game.step(&InputFrame {
            attack: true,
            ..Default::default()
        });
    }
    assert!(game.player.data["bow_strength"] > 0.0);
    assert_eq!(game.player.data["bow_armed"], 1.0);
}

fn cycle(game: &mut Game, keep_drawing: bool) {
    game.step(&InputFrame {
        cycle_pressed: true,
        attack: keep_drawing,
        ..Default::default()
    });
}

#[test]
fn natural_drawn_bow_cycle_fires_before_replacing_the_bow_in_both_presets() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for reserve_bomb in [true, false] {
            let mut game = room(rules);
            if !reserve_bomb {
                game.player.bombs = 0;
            }
            let old = pickup(&mut game, "oBow");
            assert_eq!(game.data["arrows"], 6.0);
            draw(&mut game);
            let strength = game.player.data["bow_strength"];
            cycle(&mut game, true);
            assert_eq!(game.data["arrows"], 5.0);
            let arrows: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oArrow")
                .collect();
            assert_eq!(arrows.len(), 1);
            assert!(
                (arrows[0].vel.x - (1.0 + strength)).abs() < 1e-9,
                "cycle precedes the later +0.2 charge"
            );
            assert_eq!(arrows[0].value("safe"), 1.0);
            assert_eq!(game.player.data["bow_armed"], 0.0);
            assert!(game.world.entity(old).is_none());
            assert!(!game.player.data.contains_key("stash_id"));
            assert_eq!(
                game.world.entity(game.player.held.unwrap()).unwrap().object,
                if reserve_bomb { "oBomb" } else { "oRopeThrow" }
            );
        }
    }
}

#[test]
fn natural_cycle_round_trip_creates_a_fresh_bow_without_regranting_arrows() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let old = pickup(&mut game, "oBow");
        draw(&mut game);
        cycle(&mut game, true); // Bow -> Bomb; source fires first.
        cycle(&mut game, false); // refund Bomb -> reserve Rope.
        cycle(&mut game, false); // refund Rope -> scrHoldItem("Bow").
        let fresh = game.player.held.unwrap();
        assert_ne!(fresh, old);
        let bow = game.world.entity(fresh).unwrap();
        assert_eq!(bow.object, "oBow");
        assert_eq!(bow.value("new"), 0.0);
        assert_eq!(bow.value("cost"), 0.0);
        assert_eq!(game.data["arrows"], 5.0);
        assert_eq!((game.player.bombs, game.player.ropes), (4, 4));
        assert!(
            game.world
                .entities
                .iter()
                .all(|e| !e.alive || e.value("stashed") == 0.0)
        );
        // Real crouch/drop then pickup; fresh scrHoldItem Bow is not new.
        game.step(&InputFrame {
            down: true,
            ..Default::default()
        });
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(game.player.held.is_none());
        game.step(&InputFrame {
            down: true,
            ..Default::default()
        });
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(fresh));
        assert_eq!(game.data["arrows"], 5.0);
    }
}

#[test]
fn cycle_without_reserves_keeps_the_drawn_bow_and_continues_charging() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let bow = pickup(&mut game, "oBow");
        game.player.bombs = 0;
        game.player.ropes = 0;
        draw(&mut game);
        let strength = game.player.data["bow_strength"];
        cycle(&mut game, true);
        assert_eq!(game.player.held, Some(bow));
        assert_eq!(game.data["arrows"], 6.0);
        assert!((game.player.data["bow_strength"] - strength - 0.2).abs() < 1e-9);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oArrow")
        );
    }
}

#[test]
fn natural_bomb_arrow_cycle_transfers_the_running_fuse_once() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        pickup(&mut game, "oBow");
        draw(&mut game);
        game.step(&InputFrame {
            attack: true,
            bomb_pressed: true,
            ..Default::default()
        });
        let fuse = game.player.data["bow_bomb_fuse"];
        cycle(&mut game, true);
        let arrows: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oArrow")
            .collect();
        assert_eq!(arrows.len(), 1);
        assert_eq!(arrows[0].value("bomb_arrow"), 1.0);
        assert_eq!(
            arrows[0].alarms[1],
            fuse as i32 - 1,
            "Player's early bomb-arrow counter precedes cycle; newborn Arrow Alarm1 waits until next frame"
        );
        assert_eq!(game.data["arrows"], 5.0);
        assert_eq!(
            game.player.bombs, 3,
            "bomb-arrow attachment does not consume a bomb; reservation does"
        );
        assert_eq!(game.player.data["bow_bomb_armed"], 0.0);
    }
}

#[test]
fn natural_flare_cycle_destroys_the_alarm_owner_and_restores_fresh_create() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let old = pickup(&mut game, "oFlare");
        cycle(&mut game, false);
        assert!(game.world.entity(old).is_none());
        let mut expected_rng = game.rng.clone();
        for _ in 0..4 {
            game.step(&InputFrame::default());
        }
        assert_eq!(
            game.rng.next_u64(),
            expected_rng.next_u64(),
            "no hidden Flare Alarm0 RNG during reservation"
        );
        cycle(&mut game, false);
        cycle(&mut game, false);
        let fresh = game.player.held.unwrap();
        assert_ne!(fresh, old);
        let flare = game.world.entity(fresh).unwrap();
        assert_eq!(flare.object, "oFlare");
        assert_eq!(
            flare.alarms[0], 1,
            "fresh Create after this frame's alarm phase"
        );
        let mut expected_rng = game.rng.clone();
        for _ in 0..4 {
            expected_rng.range(4);
        }
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(fresh).unwrap().alarms[0], 2);
        assert_eq!(
            game.rng.next_u64(),
            expected_rng.next_u64(),
            "fresh Flare Alarm0 consumes exactly four offset draws"
        );
    }
}

#[test]
fn serialized_legacy_flare_stash_is_migrated_before_its_due_alarm() {
    // Explicit compatibility fixture for a save produced by the previous port.
    // This is not a naturally reachable state in the corrected implementation.
    let mut original = room(RulesPreset::Classic);
    let flare = original.spawn("oFlare", Vec2::new(-100_000.0, -100_000.0));
    let item = original.world.entity_mut(flare).unwrap();
    item.held = true;
    item.data.insert("stashed".into(), 1.0);
    original.player.data.insert("stash_id".into(), flare as f64);
    let bomb = original.spawn("oBomb", original.player.pos);
    original.world.entity_mut(bomb).unwrap().held = true;
    original.player.held = Some(bomb);
    original.player.bombs -= 1;
    let mut saved = serde_json::to_value(&original).unwrap();
    saved["player"]
        .as_object_mut()
        .unwrap()
        .remove("pickup_item_type");
    let mut game: Game = serde_json::from_value(saved).unwrap();
    let mut expected_rng = game.rng.clone();
    game.step(&InputFrame::default());
    assert!(game.world.entity(flare).is_none());
    assert!(!game.player.data.contains_key("stash_id"));
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oFlareSpark")
    );
    assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
    cycle(&mut game, false);
    cycle(&mut game, false);
    let fresh = game.world.entity(game.player.held.unwrap()).unwrap();
    assert_eq!(fresh.object, "oFlare");
    assert_ne!(fresh.id, flare);
    assert_eq!(fresh.alarms[0], 1);
}

#[test]
fn invalid_legacy_stash_reference_does_not_delete_an_unmarked_actor() {
    let mut game = room(RulesPreset::Classic);
    let rock = game.spawn("oRock", Vec2::new(100.0, 152.0));
    game.player.data.insert("stash_id".into(), rock as f64);
    game.step(&InputFrame::default());
    assert!(game.world.entity(rock).is_some());
    assert!(!game.player.data.contains_key("stash_id"));
    cycle(&mut game, false);
    cycle(&mut game, false);
    cycle(&mut game, false);
    assert!(game.player.held.is_none());
    assert!(game.world.entity(rock).is_some());
}

#[test]
fn natural_cycle_recreates_every_literal_scr_hold_item_type() {
    // Exact scrHoldItem whitelist; no former per-instance timers or sale state
    // should survive. Source Create runs again, then new/cost are overridden.
    for object in [
        "oRock",
        "oJar",
        "oSkull",
        "oFishBone",
        "oArrow",
        "oMachete",
        "oMattock",
        "oMattockHead",
        "oPistol",
        "oWebCannon",
        "oTeleporter",
        "oShotgun",
        "oBow",
        "oFlare",
        "oSceptre",
        "oKey",
    ] {
        let mut game = room(RulesPreset::Classic);
        let old = pickup(&mut game, object);
        cycle(&mut game, false);
        cycle(&mut game, false);
        cycle(&mut game, false);
        let fresh = game.world.entity(game.player.held.unwrap()).unwrap();
        assert_eq!(fresh.object, object);
        assert_ne!(fresh.id, old);
        assert!(game.world.entity(old).is_none());
        assert_eq!(fresh.value("new"), 0.0);
        assert_eq!(fresh.value("cost"), 0.0);
        assert_eq!((game.player.bombs, game.player.ropes), (4, 4));
    }
}

#[test]
fn natural_armed_reserved_bomb_cannot_be_cycled_away() {
    let mut game = room(RulesPreset::Classic);
    let old = pickup(&mut game, "oRock");
    cycle(&mut game, false);
    let bomb = game.player.held.unwrap();
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.world.entity(bomb).unwrap().sprite, "sBombArmed");
    cycle(&mut game, false);
    assert_eq!(game.player.held, Some(bomb));
    assert_eq!((game.player.bombs, game.player.ropes), (3, 4));
    assert!(game.world.entity(old).is_none());
    // Normal attack throws it and scrHoldItem recreates the original type.
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    let fresh = game.world.entity(game.player.held.unwrap()).unwrap();
    assert_eq!(fresh.object, "oRock");
    assert_ne!(fresh.id, old);
    assert!(!game.world.entity(bomb).unwrap().held);
}

#[test]
fn unsupported_light_basketball_type_is_destroyed_then_scr_hold_item_clears_the_selection() {
    let mut game = room(RulesPreset::Classic);
    let old = pickup(&mut game, "oBasketball");
    cycle(&mut game, false);
    assert!(game.world.entity(old).is_none());
    cycle(&mut game, false);
    cycle(&mut game, false);
    assert!(
        game.player.held.is_none(),
        "Basketball is absent from scrHoldItem's literal whitelist"
    );
    assert_eq!((game.player.bombs, game.player.ropes), (4, 4));
}

#[test]
fn natural_throw_of_the_remembered_rock_clears_selection_without_recreation() {
    let mut game = room(RulesPreset::Classic);
    let rock = pickup(&mut game, "oRock");
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(game.player.held.is_none());
    assert!(!game.world.entity(rock).unwrap().held);
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oRock")
            .count(),
        1
    );
    cycle(&mut game, false);
    cycle(&mut game, false);
    cycle(&mut game, false);
    assert!(game.player.held.is_none());
}

#[test]
fn naturally_picked_up_key_consumed_by_chest_cannot_reappear_after_a_cycle() {
    let mut game = room(RulesPreset::Classic);
    let key = pickup(&mut game, "oKey");
    let pos = game.world.entity(key).unwrap().pos;
    let chest = game.spawn("oLockedChest", pos);
    game.step(&InputFrame::default());
    assert!(game.world.entity(key).is_none());
    assert!(game.player.held.is_none());
    // The original event selects the open sprite, emits Udjat, then destroys itself.
    assert!(game.world.entity(chest).is_none());
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oUdjatEye")
    );
    cycle(&mut game, false);
    cycle(&mut game, false);
    cycle(&mut game, false);
    assert!(
        game.player.held.is_none(),
        "source chest event clears pickupItemType with the consumed Key"
    );
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oKey")
    );
}

#[test]
fn naturally_picked_up_flare_extinguished_in_water_cannot_be_restored() {
    let mut game = room(RulesPreset::Classic);
    let flare = pickup(&mut game, "oFlare");
    let pos = game.world.entity(flare).unwrap().pos;
    game.world.set_tile(
        (pos.x / 16.0).floor() as usize,
        (pos.y / 16.0).floor() as usize,
        "oWaterSwim",
    );
    game.step(&InputFrame::default());
    assert!(game.world.entity(flare).is_none());
    assert!(game.player.held.is_none());
    cycle(&mut game, false);
    cycle(&mut game, false);
    cycle(&mut game, false);
    assert!(
        game.player.held.is_none(),
        "source Flare water branch clears pickupItemType"
    );
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oFlare")
    );
}

#[test]
fn natural_reserved_item_drop_at_a_ledge_recreates_the_saved_type() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        // An authored right ledge; no player state/held-state injection.
        for x in 6..game.world.width {
            game.world.set_tile(x, 10, "");
        }
        let old = pickup(&mut game, "oRock");
        cycle(&mut game, false);
        let bomb = game.player.held.unwrap();
        for _ in 0..120 {
            game.step(&InputFrame {
                down: true,
                right: true,
                ..Default::default()
            });
            if game.player.data.get("duck_to_hang").copied().unwrap_or(0.0) != 0.0 {
                break;
            }
        }
        assert_eq!(game.player.data.get("duck_to_hang").copied(), Some(1.0));
        let fresh = game.world.entity(game.player.held.unwrap()).unwrap();
        assert_eq!(fresh.object, "oRock");
        assert_ne!(fresh.id, old);
        let released = game.world.entity(bomb).unwrap();
        assert!(!released.held);
        assert_eq!(released.vel.x, 1.0);
        assert!(
            (released.vel.y + 3.4).abs() < 1e-12,
            "later Bomb Step applies source gravity once"
        );
    }
}

#[test]
fn stunned_yeti_recovers_while_carried_or_supported_but_not_airborne_unheld() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let yeti = game.spawn("oYeti", Vec2::new(56.0, 144.0));
        // Authored surviving STUNNED actor, then actual input acquisition/jump.
        let actor = game.world.entity_mut(yeti).unwrap();
        actor.state = 98;
        actor.timer = 4;
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(yeti));
        game.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        for _ in 0..6 {
            if game.player.held.is_none() {
                break;
            }
            game.step(&InputFrame {
                jump: true,
                ..Default::default()
            });
        }
        assert!(
            game.player.held.is_none(),
            "held Yeti recovery must not require standing on terrain"
        );
        assert!(!game.world.entity(yeti).unwrap().held);
        assert_eq!(
            serde_json::to_value(&game.player)
                .unwrap()
                .get("pickup_item_type")
                .and_then(|v| v.as_str()),
            Some("")
        );
        for supported in [true, false] {
            let mut game = room(rules);
            let yeti = game.spawn(
                "oYeti",
                Vec2::new(200.0, if supported { 144.0 } else { 100.0 }),
            );
            let actor = game.world.entity_mut(yeti).unwrap();
            actor.state = 98;
            actor.timer = 0;
            actor.vel.x = 1.0;
            game.step(&InputFrame::default());
            assert_eq!(
                game.world.entity(yeti).unwrap().state,
                if supported { 0 } else { 98 }
            );
        }
    }
}

#[test]
fn serialized_legacy_ordinary_bow_infers_type_without_recreate_or_ammo_grant() {
    let mut original = room(RulesPreset::Classic);
    let bow = pickup(&mut original, "oBow");
    let mut saved = serde_json::to_value(&original).unwrap();
    saved["player"]
        .as_object_mut()
        .unwrap()
        .remove("pickup_item_type");
    let mut game: Game = serde_json::from_value(saved).unwrap();
    let mut expected_rng = game.rng.clone();
    game.step(&InputFrame::default());
    assert_eq!(game.player.held, Some(bow));
    assert_eq!(game.data["arrows"], 6.0);
    assert_eq!(
        serde_json::to_value(&game.player)
            .unwrap()
            .get("pickup_item_type")
            .and_then(|v| v.as_str()),
        Some("Bow")
    );
    assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
}

#[test]
fn legacy_empty_selection_never_infers_a_weapon_from_reserved_bomb_or_rope() {
    for reserve_bomb in [true, false] {
        let mut original = room(RulesPreset::Classic);
        if !reserve_bomb {
            original.player.bombs = 0;
        }
        cycle(&mut original, false);
        let reserved = original.player.held.unwrap();
        let mut saved = serde_json::to_value(&original).unwrap();
        saved["player"]
            .as_object_mut()
            .unwrap()
            .remove("pickup_item_type");
        let mut game: Game = serde_json::from_value(saved).unwrap();
        game.step(&InputFrame::default());
        assert_eq!(game.player.held, Some(reserved));
        assert_eq!(
            serde_json::to_value(&game.player)
                .unwrap()
                .get("pickup_item_type")
                .and_then(|v| v.as_str()),
            Some("")
        );
        if reserve_bomb {
            cycle(&mut game, false);
        }
        cycle(&mut game, false);
        assert!(game.player.held.is_none());
    }
}

#[test]
fn heavy_dice_cycle_is_a_source_noop_control() {
    let mut game = room(RulesPreset::Classic);
    let die = pickup(&mut game, "oDice");
    let reserves = (game.player.bombs, game.player.ropes);
    assert_eq!(game.world.entity(die).unwrap().value("heavy"), 1.0);
    cycle(&mut game, false);
    assert_eq!(game.player.held, Some(die));
    assert_eq!((game.player.bombs, game.player.ropes), reserves);
    assert!(game.world.entity(die).unwrap().held);
}
