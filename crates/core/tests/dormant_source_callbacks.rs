//! Authored native callback fixtures for two source resources with no preserved
//! producer. These do not establish stock-game reachability or whole-game parity.
use spelunky_core::*;

fn game(rules: RulesPreset) -> Game {
    let mut game = Game::shell(7, rules);
    game.world = World::new(44, 34, Area::Mines, 1);
    game.world.name = "authored-dormant-callbacks".into();
    game.player.pos = Vec2::new(216.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game.world.set_tile(13, 10, "oBrick");
    game
}
fn checker(game: &mut Game, dir: f64) -> u64 {
    let id = game.spawn("oEntranceCheck", Vec2::new(64.0, 80.0));
    // Source has no Create; an authored caller must supply this variable.
    game.world
        .entity_mut(id)
        .unwrap()
        .data
        .insert("dir".into(), dir);
    id
}
fn count(game: &Game, object: &str) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .count()
}
fn rubble(game: &Game) -> Vec<(String, Vec2)> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && matches!(e.object.as_str(), "oRubble" | "oRubbleSmall"))
        .map(|e| (e.sprite.clone(), e.pos))
        .collect()
}

#[test]
fn entrance_step_uses_literal_zero_or_nonzero_dir() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (dir, expected) in [(0.0, 48.0), (1.0, 80.0), (-1.0, 80.0), (2.0, 80.0)] {
            let mut game = game(rules);
            let id = checker(&mut game, dir);
            let rng = game.rng.clone();
            game.step(&InputFrame::default());
            assert_eq!(
                game.world.entity(id).unwrap().pos,
                Vec2::new(expected, 80.0),
                "ENTRANCE_LITERAL_DIR"
            );
            assert_eq!(game.rng.clone().next_u64(), rng.clone().next_u64());
        }
    }
}
#[test]
fn entrance_undefined_dir_aborts_at_read_without_inventing_zero() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        let id = game.spawn("oEntranceCheck", Vec2::new(64.0, 80.0));
        assert!(!game.world.entity(id).unwrap().data.contains_key("dir"));
        game.step(&InputFrame::default());
        assert_eq!(
            game.data.get("source_runtime_errors"),
            Some(&1.0),
            "ENTRANCE_UNDEFINED_READ"
        );
        let entity = game.world.entity(id).unwrap();
        assert_eq!(entity.pos, Vec2::new(64.0, 80.0));
        assert!(!entity.data.contains_key("dir"));
        assert!(game.events.iter().any(
            |event| matches!(event,GameEvent::Message(text) if text.contains("undefined dir"))
        ));
    }
}
#[test]
fn entrance_non_arrow_solid_only_destroys_checker() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        game.world.set_tile(5, 5, "oHardBlock");
        let id = checker(&mut game, 1.0);
        game.step(&InputFrame::default());
        assert!(
            game.world.entity(id).is_none(),
            "ENTRANCE_SOLID_SELF_DESTROY"
        );
        assert_eq!(game.world.tile(5, 5).unwrap().object, "oHardBlock");
        assert!(rubble(&game).is_empty());
    }
}
#[test]
fn entrance_dynamic_arrow_type_runs_exact_target_destroy_once() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in [
            "oArrowTrapLeft",
            "oArrowTrapRight",
            "oArrowTrapLeftLit",
            "oArrowTrapRightLit",
            "oArrowRepeaterL",
            "oArrowRepeaterR",
        ] {
            let mut game = game(rules);
            let target = game.spawn(object, Vec2::new(80.0, 80.0));
            game.world
                .entity_mut(target)
                .unwrap()
                .data
                .insert("invincible".into(), 1.0);
            let id = checker(&mut game, 1.0);
            let mut source = game.rng.clone();
            let mut expected = Vec::new();
            for sprite in ["sRubbleTan", "sRubbleTanSmall", "sRubbleTanSmall"] {
                let pos = Vec2::new(
                    88.0 + f64::from(source.range(9)) - f64::from(source.range(9)),
                    88.0 + f64::from(source.range(9)) - f64::from(source.range(9)),
                );
                expected.push((sprite.to_string(), pos));
            }
            game.step(&InputFrame::default());
            assert!(
                game.world.entity(target).is_none(),
                "ENTRANCE_ARROW_TARGET_DESTROY {object}"
            );
            assert!(game.world.entity(id).is_none());
            assert_eq!(rubble(&game), expected);
            let arrows = usize::from(!object.starts_with("oArrowRepeater"));
            assert_eq!(count(&game, "oArrow"), arrows);
            assert_eq!(game.rng.clone().next_u64(), source.clone().next_u64());
            let born: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| matches!(e.object.as_str(), "oRubble" | "oRubbleSmall" | "oArrow"))
                .map(|e| e.id)
                .collect();
            game.step(&InputFrame::default());
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| matches!(e.object.as_str(), "oRubble" | "oRubbleSmall" | "oArrow"))
                    .map(|e| e.id)
                    .collect::<Vec<_>>(),
                born
            );
        }
    }
}
#[test]
fn entrance_tile_arrow_runs_own_destroy_and_recovers_unfired_arrow() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        game.world.set_tile(5, 5, "oArrowTrapRight");
        let id = checker(&mut game, 1.0);
        game.step(&InputFrame::default());
        assert!(
            game.world.tile(5, 5).unwrap().object.is_empty(),
            "ENTRANCE_TILE_TARGET_DESTROY"
        );
        assert!(game.world.entity(id).is_none());
        assert_eq!(rubble(&game).len(), 3);
        assert_eq!(count(&game, "oArrow"), 1);
        assert!(
            !game.world.fluid_dirty,
            "ArrowTrap own Destroy does not inherit Solid"
        );
    }
}
#[test]
fn entrance_explicit_target_type_overrides_arrow_object_default() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for dynamic in [true, false] {
            let mut game = game(rules);
            let target = if dynamic {
                game.spawn("oArrowTrapRight", Vec2::new(80.0, 80.0))
            } else {
                game.world.set_tile(5, 5, "oArrowTrapRight");
                game.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0))
            };
            game.world
                .entity_mut(target)
                .unwrap()
                .text
                .insert("type".into(), "None".into());
            let id = checker(&mut game, 1.0);
            game.step(&InputFrame::default());
            assert!(
                game.world.entity(id).is_none(),
                "ENTRANCE_OVERRIDE_SELF_DESTROY"
            );
            assert!(game.world.entity(target).is_some());
            if !dynamic {
                assert_eq!(game.world.tile(5, 5).unwrap().object, "oArrowTrapRight");
            }
            assert!(rubble(&game).is_empty());
            assert_eq!(count(&game, "oArrow"), 0);
        }
    }
}
#[test]
fn entrance_temp_and_explicit_arrow_type_keep_target_specific_destroy() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oTemp", "oHardBlock"] {
            let mut game = game(rules);
            let target = game.spawn(object, Vec2::new(80.0, 80.0));
            if object == "oHardBlock" {
                game.world
                    .entity_mut(target)
                    .unwrap()
                    .text
                    .insert("type".into(), "Arrow Trap".into());
            }
            game.world
                .entity_mut(target)
                .unwrap()
                .data
                .insert("invincible".into(), 1.0);
            let id = checker(&mut game, 1.0);
            game.step(&InputFrame::default());
            assert!(
                game.world.entity(target).is_none(),
                "ENTRANCE_TARGET_TYPE_NOT_OBJECT_ALIAS {object}"
            );
            assert!(game.world.entity(id).is_none());
            assert!(rubble(&game).is_empty());
            assert_eq!(count(&game, "oArrow"), 0);
            assert!(
                game.world.fluid_dirty,
                "these targets inherit Solid Destroy"
            );
        }
    }
}
#[test]
fn rope_bag_create_inherits_item_then_its_distinct_defaults() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for spectacles in [false, true] {
            let mut game = game(rules);
            if spectacles {
                game.player.equipment.insert("Spectacles".into());
            }
            let rng = game.rng.clone();
            let id = game.spawn("oRopeBag", Vec2::new(96.0, 64.0));
            let bag = game.world.entity(id).unwrap();
            assert_eq!(
                bag.text.get("type").map(String::as_str),
                Some("Bomb Bag"),
                "ROPE_BAG_OWN_CREATE"
            );
            assert_eq!(bag.sprite, "sRopeEnd");
            assert!(!bag.solid);
            assert_eq!(bag.text.get("shopDesc").map(String::as_str), Some(""));
            assert!(!bag.text.contains_key("buyMessage"));
            for (name, value) in [
                ("cost", 0.0),
                ("blinkToggle", 0.0),
                ("canPickUp", 1.0),
                ("new", 1.0),
                ("active", 1.0),
                ("myGrav", 0.6),
                ("bounceFactor", 0.5),
                ("frictionFactor", 0.3),
                ("bboxLeft", -4.0),
                ("bboxTop", -6.0),
                ("bboxRight", 4.0),
                ("bboxBottom", 6.0),
                ("depth", if spectacles { 51.0 } else { 101.0 }),
            ] {
                assert_eq!(bag.data.get(name), Some(&value), "{name}");
            }
            for name in [
                "xVel",
                "yVel",
                "xAcc",
                "yAcc",
                "safe",
                "heavy",
                "armed",
                "trigger",
                "value",
                "cost",
                "forSale",
                "stolen",
                "inDiceHouse",
                "cimg",
                "stuck",
                "sticky",
                "enemyID",
                "stickyXDiff",
                "stickyYDiff",
            ] {
                assert_eq!(bag.data.get(name), Some(&0.0), "{name}");
            }
            assert_eq!(bag.vel, Vec2::default());
            assert_eq!(game.rng.clone().next_u64(), rng.clone().next_u64());
            assert!(!catalog::is_a("oRopeBag", "oBombBag"));
            assert!(!catalog::is_a("oRopeBag", "oRopePile"));
            assert!(catalog::is_a("oRopeBag", "oItem"));
        }
    }
}
#[test]
fn rope_bag_runs_inherited_item_motion_and_alarm_without_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        let id = game.spawn("oRopeBag", Vec2::new(96.0, 64.0));
        let bag = game.world.entity_mut(id).unwrap();
        bag.data.insert("safe".into(), 1.0);
        bag.alarms[2] = 1;
        let rng = game.rng.clone();
        game.step(&InputFrame::default());
        let bag = game.world.entity(id).unwrap();
        assert_eq!(bag.vel.y, 0.6, "ROPE_BAG_PARENT_STEP");
        assert_eq!(bag.value("safe"), 0.0);
        assert_eq!(bag.alarms[2], 0);
        game.step(&InputFrame::default());
        let bag = game.world.entity(id).unwrap();
        assert_eq!(bag.vel.y, 1.2);
        assert!(bag.pos.y >= 64.0);
        assert_eq!(game.rng.clone().next_u64(), rng.clone().next_u64());
    }
}
#[test]
fn rope_bag_passive_overlap_is_not_a_bomb_bag_family_alias() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        let id = game.spawn("oRopeBag", game.player.pos);
        let (bombs, ropes) = (game.player.bombs, game.player.ropes);
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_some());
        assert_eq!((game.player.bombs, game.player.ropes), (bombs, ropes));
        assert_eq!(count(&game, "oItemsGet"), 0);
    }
}
#[test]
fn rope_bag_deliberate_pickup_grants_bombs_with_source_display() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        game.player.pos = Vec2::new(72.0, 72.0);
        game.player.previous_pos = game.player.pos;
        game.world.set_tile(4, 5, "oBrick");
        game.world.set_tile(5, 5, "oBrick");
        let id = game.spawn("oRopeBag", Vec2::new(80.0, 76.0));
        let (bombs, ropes) = (game.player.bombs, game.player.ropes);
        let rng = game.rng.clone();
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(game.player.bombs, bombs + 3, "ROPE_BAG_SELECTED_PICKUP");
        assert_eq!(game.player.ropes, ropes);
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.player.held, None);
        let displays: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oItemsGet")
            .collect();
        assert_eq!(displays.len(), 1);
        assert_eq!(displays[0].sprite, "sBombsGet");
        assert_eq!(displays[0].pos, Vec2::new(80.0, 62.0));
        assert_eq!(game.rng.clone().next_u64(), rng.clone().next_u64());
        assert!(
            game.events
                .iter()
                .any(|event| matches!(event,GameEvent::Collected{object,..}if object=="oRopeBag"))
        );
    }
}

// Adapter identity fixtures use the exact decimal serial written by V3 terrain
// producers. They do not claim that overlapping unmarked legacy descriptors
// have an unambiguous source receiver identity.
fn terrain_type_descriptor(game: &mut Game, marker: Option<&str>, kind: &str) -> u64 {
    let id = game.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
    let descriptor = game.world.entity_mut(id).unwrap();
    descriptor.text.insert("type".into(), kind.into());
    if let Some(marker) = marker {
        descriptor
            .text
            .insert("source_terrain_serial".into(), marker.into());
    }
    id
}
#[test]
fn entrance_bound_type_wins_over_earlier_unmarked_overlap() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        game.world.set_tile(5, 5, "oArrowTrapRight");
        let serial = game.world.tile(5, 5).unwrap().creation_serial.to_string();
        terrain_type_descriptor(&mut game, None, "Arrow Trap");
        let actual = terrain_type_descriptor(&mut game, Some(&serial), "None");
        let id = checker(&mut game, 1.0);
        game.step(&InputFrame::default());
        assert_eq!(
            game.world.tile(5, 5).unwrap().object,
            "oArrowTrapRight",
            "BOUND_TYPE_BEFORE_UNMARKED"
        );
        assert!(game.world.entity(id).is_none());
        assert!(game.world.entity(actual).is_some());
        assert!(rubble(&game).is_empty());
    }
}
#[test]
fn entrance_bound_type_wins_over_earlier_wrong_marked_overlap() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        game.world.set_tile(5, 5, "oArrowTrapRight");
        let serial = game.world.tile(5, 5).unwrap().creation_serial.to_string();
        terrain_type_descriptor(&mut game, Some("invalid-serial"), "Arrow Trap");
        let actual = terrain_type_descriptor(&mut game, Some(&serial), "None");
        let id = checker(&mut game, 1.0);
        game.step(&InputFrame::default());
        assert_eq!(
            game.world.tile(5, 5).unwrap().object,
            "oArrowTrapRight",
            "BOUND_TYPE_BEFORE_WRONG_MARKER"
        );
        assert!(game.world.entity(id).is_none());
        assert!(game.world.entity(actual).is_some());
        assert!(rubble(&game).is_empty());
    }
}
#[test]
fn entrance_wrong_only_marker_cannot_override_receiver_default() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        game.world.set_tile(5, 5, "oArrowTrapRight");
        terrain_type_descriptor(&mut game, Some("not-a-current-serial"), "None");
        let id = checker(&mut game, 1.0);
        game.step(&InputFrame::default());
        assert!(
            game.world.tile(5, 5).unwrap().object.is_empty(),
            "WRONG_ONLY_MARKER_REJECTED"
        );
        assert!(game.world.entity(id).is_none());
        assert_eq!(rubble(&game).len(), 3);
        assert_eq!(count(&game, "oArrow"), 1);
    }
}
#[test]
fn entrance_replacement_rejects_previous_receiver_serial() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        game.world.set_tile(5, 5, "oArrowTrapRight");
        let previous = game.world.tile(5, 5).unwrap().creation_serial;
        terrain_type_descriptor(&mut game, Some(&previous.to_string()), "None");
        game.world.set_tile(5, 5, "oArrowTrapRight");
        assert_ne!(game.world.tile(5, 5).unwrap().creation_serial, previous);
        let id = checker(&mut game, 1.0);
        game.step(&InputFrame::default());
        assert!(
            game.world.tile(5, 5).unwrap().object.is_empty(),
            "REPLACED_SERIAL_REJECTED"
        );
        assert!(game.world.entity(id).is_none());
        assert_eq!(rubble(&game).len(), 3);
        assert_eq!(count(&game, "oArrow"), 1);
    }
}
