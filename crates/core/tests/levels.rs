use spelunky_core::{
    editor::EditorState,
    level::{LEVEL_HEIGHT, LEVEL_WIDTH, Level, tile_spawns},
};

fn old_level(suffix: &str) -> String {
    format!(
        "{}\n\nCAVE\n4\n4\n4\nNONE{suffix}",
        vec!["0".repeat(40); 32].join("\n")
    )
}

#[test]
fn old_optional_formats_and_original_zero_count_writer_bug() {
    let oldest = Level::parse(&old_level("")).unwrap();
    assert_eq!(oldest.author, "");
    assert!(oldest.exit_targets.is_empty());
    for suffix in ["\n0", "\n00", "\n0\n0", "\n0\n0\n"] {
        assert_eq!(Level::parse(&old_level(suffix)).unwrap(), oldest);
    }
    let signs = Level::parse(&old_level("\n02\nHELLO\n")).unwrap();
    assert_eq!(signs.signs, ["HELLO", ""]);
    assert_eq!(Level::parse(&signs.to_text()).unwrap(), signs);
}

#[test]
fn empty_unicode_and_linked_metadata_roundtrip() {
    let mut level = Level::default();
    level.set_tile(0, 0, 'X').unwrap();
    level.set_tile(1, 0, '@').unwrap();
    level.set_tile(2, 0, 'I').unwrap();
    level.set_tile(3, 0, 'I').unwrap();
    level.author = "探検家 🦇".into();
    level.next_level = "next room".into();
    level.exit_targets = vec!["ending".into(), "previous".into()];
    level.signs = vec!["こんにちは #SECOND LINE".into(), "".into()];
    let text = level.to_text();
    assert_eq!(Level::parse(&text).unwrap(), level);
    assert_eq!(Level::parse(&text.replace('\n', "\r\n")).unwrap(), level);
    let world = level.to_world_from(Some("previous"));
    assert_eq!((world.width, world.height), (42, 34));
    assert_eq!(world.entrance.x, 40.0);
    let doors: Vec<_> = world
        .entities
        .iter()
        .filter(|e| matches!(e.object.as_str(), "oEntrance" | "oExit"))
        .collect();
    assert_eq!(doors[0].text["leadsTo"], "ending");
    assert_eq!(doors[1].text["leadsTo"], "previous");
    let signs: Vec<_> = world
        .entities
        .iter()
        .filter(|e| e.object == "oMsgSign")
        .collect();
    assert_eq!(signs[0].text["message"], "こんにちは #SECOND LINE");
    assert_eq!(signs[1].text["message"], "");
    assert_eq!(world.tile(0, 0).unwrap().object, "oHardBlock");
}

#[test]
fn source_inventory_floors_and_clamps() {
    let text = old_level("").replace("\n4\n4\n4\n", "\n0.9\n999\n-8\n");
    let level = Level::parse(&text).unwrap();
    assert_eq!(
        (
            level.starting_life,
            level.starting_bombs,
            level.starting_ropes
        ),
        (1, 99, 0)
    );
    assert!(Level::parse(&text.replace("999", "NaN")).is_err());
}

#[test]
fn partial_target_arrays_repeat_last_record_like_classic_loader() {
    let mut level = Level::default();
    for (x, tile) in ['@', 'X', 'X', 'I', 'I'].into_iter().enumerate() {
        level.set_tile(x, 0, tile).unwrap();
    }
    level.exit_targets = vec!["one".into()];
    level.signs = vec!["message".into()];
    let world = level.to_world();
    for entity in &world.entities {
        if matches!(entity.object.as_str(), "oExit" | "oEntrance") {
            assert_eq!(entity.text["leadsTo"], "one");
        }
        if entity.object == "oMsgSign" {
            assert_eq!(entity.text["message"], "message");
        }
    }
}

#[test]
fn exact_compound_offsets_and_object_overrides() {
    let bones = tile_spawns('k');
    assert_eq!(
        bones.iter().map(|s| s.object).collect::<Vec<_>>(),
        ["oBones", "oSkull"]
    );
    assert_eq!((bones[1].offset.x, bones[1].offset.y), (12.0, 12.0));
    let fish = tile_spawns('p');
    assert_eq!(
        fish.iter().map(|s| s.object).collect::<Vec<_>>(),
        ["oWaterSwim", "oPiranha"]
    );
    assert_eq!((fish[1].offset.x, fish[1].offset.y), (4.0, 4.0));
    let jaws = tile_spawns('{');
    assert_eq!(jaws.len(), 9);
    for (index, water) in jaws[..8].iter().enumerate() {
        assert_eq!(water.object, "oWaterSwim");
        assert_eq!(
            (water.offset.x, water.offset.y),
            ((index % 4 * 16) as f64, (index / 4 * 16) as f64)
        );
    }
    assert_eq!(jaws[8].object, "oJaws");
    assert_eq!(tile_spawns('x')[0].sprite, Some("sTreeTop"));
    assert_eq!(tile_spawns('!')[0].state, 4);
    assert_eq!(tile_spawns('R')[0].offset.y, 11.0);
    assert!(tile_spawns('J')[0].free_item);
    assert_eq!(tile_spawns('\\')[0].object, "oCapePickup");
}

#[test]
fn all_source_tile_creations_match_catalog_in_order() {
    // Read the actual reference fixture: detects omitted objects, compounds and offsets.
    let source =
        include_str!("../../../references/classic-1.1/Scripts/Level Editor/scrCreateTileObj.gml");
    let mut checked = 0;
    for block in source.split("if (argument0 == \"").skip(1) {
        let (symbol, body) = block.split_once("\")").unwrap();
        let symbol = symbol.chars().next().unwrap();
        let mut expected = Vec::new();
        for creation in body.split("instance_create(").skip(1) {
            let arguments: Vec<_> = creation
                .split(')')
                .next()
                .unwrap()
                .split(',')
                .map(str::trim)
                .collect();
            let offset = |value: &str, prefix: &str| {
                value
                    .strip_prefix(prefix)
                    .unwrap()
                    .trim_start_matches('+')
                    .parse::<f64>()
                    .unwrap_or(0.0)
            };
            expected.push((
                arguments[2],
                offset(arguments[0], "argument1"),
                offset(arguments[1], "argument2"),
            ));
        }
        let actual: Vec<_> = tile_spawns(symbol)
            .into_iter()
            .map(|spawn| (spawn.object, spawn.offset.x, spawn.offset.y))
            .collect();
        assert_eq!(actual, expected, "tile {symbol:?}");
        checked += 1;
    }
    assert_eq!(checked, 84);
}

#[test]
fn editor_preserves_cell_metadata_and_supports_undo_redo() {
    let mut editor = EditorState::default();
    editor.set_tile(4, 0, 'X').unwrap();
    editor.set_exit_target(4, 0, "later".into()).unwrap();
    editor.set_tile(1, 0, '@').unwrap();
    assert_eq!(editor.level.exit_targets, ["", "later"]);
    editor.set_exit_target(1, 0, "earlier".into()).unwrap();
    editor.set_tile(0, 0, 'I').unwrap();
    editor.set_sign(0, 0, "keep this".into()).unwrap();
    editor.erase(1, 0).unwrap();
    assert_eq!(editor.level.exit_targets, ["later"]);
    assert!(editor.undo());
    assert_eq!(editor.level.exit_targets, ["earlier", "later"]);
    assert!(editor.redo());
    editor.set_tile(2, 2, '1').unwrap();
    assert!(!editor.can_redo());
    assert_eq!(editor.level.signs, ["keep this"]);
    let before = editor.level.clone();
    let _world = editor.test_play();
    assert_eq!(editor.level, before);
}

#[test]
fn malformed_user_input_cannot_panic() {
    for text in [
        "",
        "\n",
        "not a level",
        &"x".repeat(4 * 1024 * 1024 + 1),
        &old_level("\n999999999999999999999999999999"),
        &old_level("\n2\nonly one"),
        &old_level("\n0\n-1"),
    ] {
        assert!(
            std::panic::catch_unwind(|| Level::parse(text))
                .unwrap()
                .is_err()
        );
    }
    let mut invalid = Level::default();
    invalid.tiles.clear();
    assert!(invalid.set_tile(0, 0, '1').is_err());
    assert!(invalid.set_tile(LEVEL_WIDTH, LEVEL_HEIGHT, '1').is_err());
    let _ = invalid.to_text();
    let _ = invalid.to_world();
}
