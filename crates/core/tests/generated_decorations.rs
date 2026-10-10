use spelunky_core::{
    Game, InputFrame, Rect, Rng, RulesPreset, VIEW_HEIGHT, VIEW_WIDTH, Vec2, World, catalog,
    generation, hub::HubKind, level::Level,
};
use std::collections::BTreeSet;

fn flags(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).into()).collect()
}

// Synchronous source Create callbacks consume RNG during generation. Select a
// real generated forest by geometry, so unrelated corrected Create draws do
// not turn the pre-Step tree/canopy assertions into empty fixtures.
fn generated_forest_with_trunk_and_canopy(selected: &BTreeSet<String>) -> World {
    for seed in 0..64 {
        let world = generation::generate(&mut Rng::new(seed), 5, selected);
        if world
            .tiles
            .iter()
            .filter(|tile| tile.object == "oTree")
            .count()
            > 2
            && world
                .entities
                .iter()
                .filter(|entity| entity.object == "oLeaves")
                .count()
                >= 2
        {
            return world;
        }
    }
    panic!("no real generated trunk and canopy in seeds 0..64 for {selected:?}");
}

fn natural_offscreen_casino() -> Game {
    // Corrected synchronous Create draws can change which seed has a casino.
    // Select by generated geometry, never by whether the behavior under test passes.
    // Try the currently suitable seed first to avoid regenerating 52 worlds
    // in every debug run; the bounded fallback keeps the layout precondition explicit.
    for seed in std::iter::once(51).chain((0..64).filter(|seed| *seed != 51)) {
        let mut game = Game::new(seed, RulesPreset::Classic);
        game.next_level();
        let dice: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oDice")
            .collect();
        let Some(prize) = game
            .world
            .entities
            .iter()
            .find(|e| e.value("inDiceHouse") != 0.0)
        else {
            continue;
        };
        if dice.len() != 2 {
            continue;
        }
        // oLevel Step: camera rectangle, literal player x+16/y+16 sizes,
        // then the first Boulder/Olmec regions. Dice are not always-active.
        let p = game.player.pos;
        let mut regions = vec![
            Rect::new(
                game.camera.x - 96.0,
                game.camera.y - 96.0,
                VIEW_WIDTH + 192.0,
                VIEW_HEIGHT + 192.0,
            ),
            Rect::new(p.x - 16.0, p.y - 16.0, p.x + 16.0, p.y + 16.0),
        ];
        for (object, offset, size) in [("oBoulder", 32.0, 64.0), ("oOlmec", 16.0, 96.0)] {
            if let Some(e) = game
                .world
                .entities
                .iter()
                .find(|e| e.alive && catalog::is_a(&e.object, object))
            {
                regions.push(Rect::new(e.pos.x - offset, e.pos.y - offset, size, size));
            }
        }
        let outside = |bounds: Rect| {
            regions.iter().all(|region| {
                // Source region activation uses inclusive mask bounds.
                !(bounds.x <= region.x + region.w
                    && bounds.x + bounds.w - 1.0 >= region.x
                    && bounds.y <= region.y + region.h
                    && bounds.y + bounds.h - 1.0 >= region.y)
            })
        };
        if dice.iter().all(|die| outside(die.sprite_bounds())) && outside(prize.sprite_bounds()) {
            return game;
        }
    }
    panic!("no real offscreen casino in natural level 2 for seeds 0..64");
}

#[test]
fn generated_tiki_has_source_body_depth_and_both_random_arm_crops() {
    let world = generation::generate(&mut Rng::new(0), 1, &flags(&[]));
    let head = world
        .entities
        .iter()
        .find(|e| e.object == "oGiantTikiHead")
        .unwrap();
    let origin = Vec2::new(head.pos.x - 16.0, head.pos.y - 12.0);
    let body = world
        .entities
        .iter()
        .find(|e| e.sprite == "bgTiki")
        .unwrap();
    assert_eq!(body.pos, Vec2::new(origin.x, origin.y + 32.0));
    assert_eq!(body.value("depth"), 10001.0);
    let arms: Vec<_> = world
        .entities
        .iter()
        .filter(|e| e.sprite == "bgTikiArms")
        .collect();
    assert_eq!(arms.len(), 2);
    for (arm, dx, source_y) in [(arms[0], 32.0, 0.0), (arms[1], -16.0, 16.0)] {
        assert_eq!(arm.pos, Vec2::new(origin.x + dx, origin.y + 32.0));
        assert!([0.0, 16.0, 32.0].contains(&arm.value("source_x")));
        assert_eq!(arm.value("source_y"), source_y);
        assert_eq!(arm.value("source_w"), 16.0);
        assert_eq!(arm.value("source_h"), 16.0);
        assert_eq!(arm.value("depth"), 10001.0);
    }
}

#[test]
fn generated_city_of_gold_mural_retains_each_source_crop_and_block_origin() {
    let world = generation::generate(&mut Rng::new(0), 13, &flags(&["city_of_gold"]));
    let fragments: Vec<_> = world
        .entities
        .iter()
        .filter(|e| e.sprite == "bgLadyXoc")
        .collect();
    assert_eq!(fragments.len(), 30);
    let mut crops = BTreeSet::new();
    for fragment in fragments {
        assert_eq!(fragment.value("depth"), 99.0);
        assert_eq!(fragment.value("source_w"), 16.0);
        assert_eq!(fragment.value("source_h"), 16.0);
        crops.insert((
            fragment.value("source_x") as i32,
            fragment.value("source_y") as i32,
        ));
        assert_eq!(
            world
                .tile(
                    (fragment.pos.x / 16.0) as i32,
                    (fragment.pos.y / 16.0) as i32
                )
                .unwrap()
                .object,
            "oXocBlock"
        );
    }
    assert_eq!(
        crops,
        (0..5)
            .flat_map(|x| (0..6).map(move |y| (x * 16, y * 16)))
            .collect()
    );
}

#[test]
fn generated_shop_wanted_posters_use_source_gate_and_character_crops() {
    for (character, source_x) in [
        (None, 0.0),
        (Some("character_damsel"), 32.0),
        (Some("character_tunnelman"), 64.0),
    ] {
        for wanted in ["murderer", "wanted"] {
            let mut selected = flags(&["city_of_gold", wanted]);
            if let Some(character) = character {
                selected.insert(character.into());
            }
            let world = generation::generate(&mut Rng::new(1), 13, &selected);
            let posters: Vec<_> = world
                .entities
                .iter()
                .filter(|e| e.sprite == "bgWanted")
                .collect();
            assert!(!posters.is_empty(), "authored shop fixture must exercise W");
            for poster in posters {
                assert_eq!(poster.value("source_x"), source_x);
                assert_eq!(poster.value("source_y"), 0.0);
                assert_eq!(poster.value("source_w"), 32.0);
                assert_eq!(poster.value("source_h"), 32.0);
                assert_eq!(poster.value("depth"), 9004.0);
            }
        }
    }
    let clean = generation::generate(&mut Rng::new(1), 13, &flags(&["city_of_gold"]));
    assert!(!clean.entities.iter().any(|e| e.sprite == "bgWanted"));
}

#[test]
fn generated_ice_underside_reaches_real_drip_alarm_behavior() {
    let world = generation::generate(&mut Rng::new(0), 9, &flags(&[]));
    let bottom = world
        .entities
        .iter()
        .find(|e| e.object == "oIceBottom")
        .unwrap()
        .clone();
    assert_eq!(
        world
            .tile(
                (bottom.pos.x / 16.0) as i32,
                (bottom.pos.y / 16.0) as i32 - 1
            )
            .unwrap()
            .object,
        "oIce"
    );
    assert!((20.0..=400.0).contains(&bottom.value("drip_alarm")));
    let mut game = Game::shell(0, RulesPreset::Classic);
    game.player.pos = Vec2::new(bottom.pos.x + 8.0, bottom.pos.y + 24.0);
    game.player.previous_pos = game.player.pos;
    game.camera = Vec2::new(
        (bottom.pos.x - 160.0).max(0.0),
        (bottom.pos.y - 100.0).max(0.0),
    );
    game.player.invincible = 100;
    game.world = world;
    game.world
        .entity_mut(bottom.id)
        .unwrap()
        .data
        .insert("drip_alarm".into(), 1.0);
    game.step(&InputFrame::default());
    assert!(game.world.entity(bottom.id).unwrap().alive);
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| e.object == "oDrip"
                && e.pos == Vec2::new(bottom.pos.x + 8.0, bottom.pos.y + 4.0))
    );
}

#[test]
fn generated_fake_temple_doors_receive_setup_sprites_without_solidifying() {
    let world = generation::generate(&mut Rng::new(1), 13, &flags(&[]));
    let panels: Vec<_> = world
        .entities
        .iter()
        .filter(|e| e.object == "oTempleFake")
        .collect();
    assert_eq!(panels.len(), 4);
    assert!(
        panels
            .iter()
            .all(|e| e.value("fake_setup_initialized") == 1.0)
    );
    assert!(panels.iter().any(|e| e.sprite != "sTemple"));
    assert!(
        panels
            .iter()
            .all(|e| !spelunky_core::catalog::is_solid(&e.object))
    );
}

#[test]
fn generated_default_high_graphics_emits_source_fringes_and_twenty_extras() {
    for (level, background) in [
        (1, "bgCaveTop"),
        (5, "bgCaveTop2"),
        (9, "bgCaveTop3"),
        (13, "bgCaveTop4"),
    ] {
        let world = generation::generate(&mut Rng::new(0), level, &flags(&[]));
        let fringes: Vec<_> = world
            .entities
            .iter()
            .filter(|e| e.sprite == background)
            .collect();
        assert!(!fringes.is_empty(), "level {level}");
        for fringe in fringes {
            assert_eq!(fringe.value("depth"), 3.0);
            assert_eq!(fringe.value("source_w"), 16.0);
            assert_eq!(fringe.value("source_h"), 16.0);
            assert_eq!(fringe.value("source_y"), 0.0);
        }
        let extras: Vec<_> = world
            .entities
            .iter()
            .filter(|e| e.sprite.starts_with("bgExtras"))
            .collect();
        assert_eq!(extras.len(), 20, "level {level}");
        for extra in extras {
            assert_eq!(extra.value("depth"), 10002.0);
            assert_eq!(extra.value("source_w"), 32.0);
            assert_eq!(extra.value("source_h"), 32.0);
            assert!([0.0, 32.0].contains(&extra.value("source_x")));
            assert!((16.0..=672.0).contains(&extra.pos.x));
            assert!((16.0..=528.0).contains(&extra.pos.y));
        }
        let low = generation::generate(&mut Rng::new(0), level, &flags(&["low_graphics"]));
        assert!(
            !low.entities
                .iter()
                .any(|e| e.sprite.starts_with("bgCaveTop") || e.sprite.starts_with("bgExtras"))
        );
    }
}

#[test]
fn authored_hubs_emit_default_high_fringes_and_custom_rooms_skip_setup() {
    for kind in [
        HubKind::Title,
        HubKind::Scores,
        HubKind::Sun,
        HubKind::Moon,
        HubKind::Stars,
    ] {
        let mut game = Game::shell(0, RulesPreset::Classic);
        game.load_hub(kind);
        assert!(
            game.world.entities.iter().any(|e| e.sprite == "bgCaveTop"),
            "{kind:?}"
        );
        game.flags.insert("low_graphics".into());
        game.load_hub(kind);
        assert!(
            !game.world.entities.iter().any(|e| e.sprite == "bgCaveTop"),
            "{kind:?}"
        );
    }
    let mut game = Game::shell(0, RulesPreset::Classic);
    let mut level = Level::default();
    level.set_tile(4, 4, '1').unwrap();
    game.load_custom(&level, "no-setup.lvl", "");
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.sprite.starts_with("bgCaveTop") || e.sprite.starts_with("bgExtras"))
    );
}

#[test]
fn generated_canopies_run_source_create_before_first_step_including_cemetery() {
    for cemetery in [false, true] {
        let selected = if cemetery {
            flags(&["cemetery"])
        } else {
            flags(&[])
        };
        let world = generated_forest_with_trunk_and_canopy(&selected);
        let leaves: Vec<_> = world
            .entities
            .iter()
            .filter(|e| e.object == "oLeaves")
            .collect();
        assert!(
            !leaves.is_empty(),
            "actual source tree fixture must contain canopy leaves"
        );
        let left = if cemetery { "sLeavesDead" } else { "sLeaves" };
        let right = if cemetery {
            "sLeavesDeadR"
        } else {
            "sLeavesRight"
        };
        assert!(
            leaves.iter().any(|e| e.sprite == left),
            "left Create sprite must exist before any step"
        );
        assert!(
            leaves.iter().any(|e| e.sprite == right),
            "right Create sprite must exist before any step"
        );
        for leaf in leaves {
            assert_eq!(leaf.value("leaves_initialized"), 1.0);
            assert_eq!(leaf.value("sprite_set"), 0.0);
            assert!([left, right].contains(&leaf.sprite.as_str()));
        }
    }
}

#[test]
fn natural_casino_initializes_offscreen_dice_and_preserves_birth_order() {
    let mut game = natural_offscreen_casino();
    assert_eq!(game.world.level, 2);
    let dice: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.object == "oDice")
        .collect();
    assert_eq!(dice.len(), 2);
    let initial_dice: Vec<_> = dice
        .iter()
        .map(|die| (die.id, die.value("value"), die.alarms, die.creation_serial))
        .collect();
    for die in &dice {
        assert_eq!(die.age, 0);
        assert_eq!(die.value("dice_create_initialized"), 1.0);
        assert!((1.0..=6.0).contains(&die.value("value")));
        assert_eq!(die.value("value").fract(), 0.0);
    }
    let prize = game
        .world
        .entities
        .iter()
        .find(|e| e.value("inDiceHouse") != 0.0)
        .unwrap();
    // The two mirrored source casino templates contain q+dd or dd+q.
    // Row stamping must preserve that placement order, including both cases.
    let mut births = vec![prize, dice[0], dice[1]];
    assert!(births.iter().all(|e| e.pos.y == prize.pos.y));
    births.sort_by(|a, b| a.pos.x.total_cmp(&b.pos.x));
    assert!(
        births
            .windows(2)
            .all(|pair| pair[0].creation_serial < pair[1].creation_serial)
    );
    assert!(
        [
            "oJetpack",
            "oCapePickup",
            "oShotgun",
            "oGloves",
            "oTeleporter",
            "oMattock",
            "oPaste",
            "oSpringShoes",
            "oSpikeShoes",
            "oCompass",
            "oPistol",
            "oMachete",
            "oBombBox",
        ]
        .contains(&prize.object.as_str())
    );
    let initial_prize = (
        prize.id,
        prize.object.clone(),
        prize.pos,
        prize.creation_serial,
    );

    for frame in 1..=3 {
        game.step(&InputFrame::default());
        let dice: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oDice")
            .collect();
        assert_eq!(dice.len(), 2);
        for (id, face, alarms, birth) in &initial_dice {
            let die = dice.iter().find(|die| die.id == *id).unwrap();
            assert_eq!(
                die.value("value"),
                *face,
                "no deferred Create/reroll on frame {frame}"
            );
            assert_eq!(
                die.alarms, *alarms,
                "offscreen alarms freeze on frame {frame}"
            );
            assert_eq!(die.value("dice_create_initialized"), 1.0);
            assert_eq!(die.creation_serial, *birth);
            assert_eq!(
                die.age, 1,
                "fresh instances age once before Level553 deactivates Dice773; later frames freeze"
            );
            assert!(
                !die.engine_active,
                "source offscreen precondition on frame {frame}"
            );
        }
        let prize = game
            .world
            .entities
            .iter()
            .find(|e| e.id == initial_prize.0)
            .unwrap();
        assert_eq!(
            (&prize.object, prize.pos),
            (&initial_prize.1, initial_prize.2)
        );
        assert_eq!(prize.value("inDiceHouse"), 1.0);
        assert_eq!(prize.creation_serial, initial_prize.3);
    }
}

#[test]
fn actual_generated_trees_have_source_order_metadata_before_first_step() {
    let world = generated_forest_with_trunk_and_canopy(&flags(&[]));
    let mut count = 0;
    for (index, _) in world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| tile.object == "oTree")
    {
        let pos = Vec2::new(
            (index % world.width) as f64 * 16.0 + 8.0,
            (index / world.width) as f64 * 16.0 + 8.0,
        );
        let metadata: Vec<_> = world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.object == "oTerrainMetadata" && e.pos == pos)
            .collect();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].1.value("tree_initialized"), 1.0);
        assert_eq!(metadata[0].1.age, 0);
        if let Some((lower_index, _)) = world.entities.iter().enumerate().find(|(_, e)| {
            e.object == "oTerrainMetadata"
                && e.value("tree_initialized") != 0.0
                && e.pos == Vec2::new(pos.x, pos.y + 16.0)
        }) {
            assert!(
                lower_index < metadata[0].0,
                "actual generated Tree creation is bottom to top"
            );
        }
        count += 1;
    }
    assert!(count > 2, "fixture must exercise a real generated trunk");
}
