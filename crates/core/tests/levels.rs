use spelunky_core::{
    catalog,
    editor::{
        EditorState,
        preview::{PreviewContext, PreviewRoom, cursor_sprite, preview_sprite},
    },
    level::{LEVEL_HEIGHT, LEVEL_WIDTH, Level, tile_spawns},
};

fn old_level(suffix: &str) -> String {
    format!(
        "{}\n\nCAVE\n4\n4\n4\nNONE{suffix}",
        vec!["0".repeat(40); 32].join("\n")
    )
}

#[test]
fn old_optional_formats_and_source_sign_count_parsing() {
    let oldest = Level::parse(&old_level("")).unwrap();
    assert_eq!(oldest.author, "");
    assert!(oldest.exit_targets.is_empty());
    for suffix in ["\n0", "\n00", "\n0\n0", "\n0\n0\n"] {
        assert_eq!(Level::parse(&old_level(suffix)).unwrap(), oldest);
    }
    let signs = Level::parse(&old_level("\n0\n2\nHELLO\n")).unwrap();
    assert_eq!(signs.signs, ["HELLO", ""]);
}

#[test]
fn blank_exit_count_means_zero_and_keeps_sign_metadata_aligned() {
    let level = Level::parse(&old_level("\n\n1\nHELLO")).unwrap();
    assert!(level.exit_targets.is_empty());
    assert_eq!(level.signs, ["HELLO"]);

    let fractional = Level::parse(&old_level("\n1.5\nFIRST\nSECOND\n0")).unwrap();
    assert_eq!(fractional.exit_targets, ["FIRST", "SECOND"]);
    assert!(fractional.signs.is_empty());

    let negative = Level::parse(&old_level("\n-1\n-1")).unwrap();
    assert!(negative.exit_targets.is_empty());
    assert!(negative.signs.is_empty());
}

#[test]
fn classic_zero_exit_sign_count_serializes_and_reloads_as_an_exit_target() {
    // Both Classic writers omit the newline after a zero exit count. With a
    // single sign, this writes "01"; scrLoadLevel then consumes the sign text
    // as an exit target, leaving the sign with its default empty message.
    let mut authored = Level::default();
    authored.set_tile(1, 0, 'I').unwrap();
    authored.signs = vec!["LEGACY TARGET".into()];

    let text = authored.to_text();
    assert_eq!(
        text.lines().skip(LEVEL_HEIGHT + 6).collect::<Vec<_>>(),
        ["01", "LEGACY TARGET"]
    );

    let level = Level::parse(&text).unwrap();
    assert_eq!(level.exit_targets, ["LEGACY TARGET"]);
    assert!(level.signs.is_empty());

    let world = level.to_world();
    let sign = world
        .entities
        .iter()
        .find(|entity| entity.object == "oMsgSign")
        .unwrap();
    assert_eq!(sign.text["message"], "");
}

#[test]
fn source_loader_ignores_lines_after_declared_records() {
    // With no exit records Classic joins the exit and sign counts, so provide
    // an exit to make the declared-record boundary explicit for this test.
    let mut level = Level::default();
    level.exit_targets.push("destination".into());
    let text = format!("{}\nCOMMENT", level.to_text());

    assert_eq!(Level::parse(&text).unwrap(), level);
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

    let empty_inventory = old_level("").replace("\n4\n4\n4\n", "\n\n\n\n");
    let level = Level::parse(&empty_inventory).unwrap();
    assert_eq!(
        (
            level.starting_life,
            level.starting_bombs,
            level.starting_ropes
        ),
        (1, 0, 0),
        "GM8 empty real conversion returns zero before ordinary load applies its clamps"
    );

    let malformed_inventory = old_level("").replace("\n4\n4\n4\n", "\n12A\n4\n4\n");
    assert_eq!(
        Level::parse(&malformed_inventory).unwrap_err(),
        "Error in function real()."
    );
    assert!(Level::parse_for_editor(&malformed_inventory).is_ok());
}

#[test]
fn source_level_loader_preserves_short_rows_and_ignores_columns_after_forty() {
    let mut lines: Vec<String> = Level::default()
        .to_text()
        .lines()
        .map(str::to_owned)
        .collect();
    lines[0] = "1".repeat(39);
    lines[1] = format!("{}X", "2".repeat(40));

    let parsed = Level::parse(&lines.join("\n")).unwrap();
    assert_eq!(&parsed.tiles[..39], &vec!['1'; 39]);
    assert_eq!(parsed.tiles[39], '\0', "the absent source cell stays empty");
    assert_eq!(parsed.to_text().lines().next().unwrap(), "1".repeat(39));
    assert_eq!(&parsed.tiles[40..80], &vec!['2'; 40]);
    assert_eq!(parsed.tiles.len(), LEVEL_WIDTH * LEVEL_HEIGHT);
}

#[test]
fn source_editor_save_concatenates_missing_cells_as_empty_strings() {
    let mut lines: Vec<String> = Level::default()
        .to_text()
        .lines()
        .map(str::to_owned)
        .collect();
    lines[0] = "1".repeat(38);

    let mut parsed = Level::parse(&lines.join("\n")).unwrap();
    parsed.set_tile(39, 0, 'X').unwrap();

    assert_eq!(
        parsed.to_text().lines().next().unwrap(),
        format!("{}X", "1".repeat(38)),
        "Classic writes each cell string, so untouched missing cells emit no character"
    );
}

#[test]
fn source_level_loader_ignores_unknown_control_symbols_in_map_cells() {
    let mut lines: Vec<String> = Level::default()
        .to_text()
        .lines()
        .map(str::to_owned)
        .collect();
    lines[0] = format!("\t\u{0001}\u{007f}{}", "0".repeat(LEVEL_WIDTH - 3));

    let parsed = Level::parse(&lines.join("\n")).unwrap();
    assert_eq!(parsed.get_tile(0, 0), Some('\t'));
    assert_eq!(parsed.get_tile(1, 0), Some('\u{0001}'));
    assert_eq!(parsed.get_tile(2, 0), Some('\u{007f}'));
    assert!(tile_spawns('\t').is_empty());
    assert!(tile_spawns('\u{0001}').is_empty());
    assert!(tile_spawns('\u{007f}').is_empty());

    // scrTestLevel and the editor's Y-save both write levelArray cells
    // directly. A no-op editor save therefore preserves an ignored symbol.
    let editor = EditorState::new(parsed);
    let saved = editor.document().to_text();
    let reloaded = Level::parse(&saved).unwrap();
    assert_eq!(
        &reloaded.tiles[..LEVEL_WIDTH],
        &editor.document().tiles[..LEVEL_WIDTH],
        "all 40 source columns survive an import, editor save, and reload"
    );
    assert_eq!(reloaded.get_tile(0, 0), Some('\t'));
    assert_eq!(reloaded.get_tile(1, 0), Some('\u{0001}'));
    assert_eq!(reloaded.get_tile(2, 0), Some('\u{007f}'));

    // Keep the public in-memory tile vector from injecting extra map rows.
    let mut malformed = Level::default();
    malformed.tiles[0] = '\n';
    // Classic omits the newline between zero exits and zero signs, writing
    // one combined "00" count line.
    assert_eq!(malformed.to_text().lines().count(), LEVEL_HEIGHT + 7);
    assert_eq!(malformed.to_text().lines().last(), Some("00"));
    assert_eq!(
        malformed.to_text().lines().next().unwrap().chars().next(),
        Some('0')
    );
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
        let spawns = tile_spawns(symbol);
        for (index, spawn) in spawns.iter().enumerate() {
            // In this script `obj` always names the most recently created
            // instance. Compare its post-Create overrides as well as the
            // direct creations; offsets alone miss broken free pickups.
            let is_last = index + 1 == spawns.len();
            let sprite = body
                .split_once("obj.sprite_index = ")
                .map(|(_, tail)| tail.split(';').next().unwrap().trim());
            assert_eq!(
                spawn.sprite,
                sprite.filter(|_| is_last),
                "sprite {symbol:?}"
            );
            let state = body
                .split_once("obj.status = ")
                .map(|(_, tail)| {
                    tail.split(';')
                        .next()
                        .unwrap()
                        .trim()
                        .parse::<i32>()
                        .unwrap()
                })
                .unwrap_or(0);
            assert_eq!(
                spawn.state,
                if is_last { state } else { 0 },
                "status {symbol:?}"
            );
            assert_eq!(
                spawn.free_item,
                is_last && body.contains("obj.cost = 0;") && body.contains("obj.forSale = false;"),
                "free pickup {symbol:?}"
            );
        }
        assert!(
            EditorState::default().palette.contains(&symbol),
            "palette {symbol:?}"
        );
        checked += 1;
    }
    assert_eq!(checked, 84);
}

#[test]
fn all_editor_preview_symbols_match_classic_objects_sprites_and_cursor_sprites() {
    let create_source =
        include_str!("../../../references/classic-1.1/Scripts/Level Editor/scrCreateTile.gml");
    let cursor_source =
        include_str!("../../../references/classic-1.1/Scripts/Level Editor/scrSetCursorTile.gml");

    let mut source_cursors = std::collections::BTreeMap::new();
    for line in cursor_source.lines() {
        let Some((left, right)) = line.split_once("oCursObj.sprite_index = ") else {
            continue;
        };
        let symbol = left
            .split_once("== \"")
            .expect("cursor source symbol")
            .1
            .chars()
            .next()
            .expect("one-character palette symbol");
        let sprite = right.trim().trim_end_matches(';').trim();
        source_cursors.insert(symbol, sprite);
    }
    assert_eq!(source_cursors.len(), 84);

    let source_lines: Vec<_> = create_source.lines().collect();
    let branch_starts: Vec<_> = source_lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.contains("argument0 == \"").then_some(index))
        .collect();
    let mut checked = 0;
    for (branch_index, &start) in branch_starts.iter().enumerate() {
        let line = source_lines[start];
        let tile = line
            .split_once("argument0 == \"")
            .unwrap()
            .1
            .chars()
            .next()
            .unwrap();
        let end = branch_starts
            .get(branch_index + 1)
            .copied()
            .unwrap_or(source_lines.len());
        let body = &source_lines[start + 1..end];
        let Some(create) = body.iter().find(|line| line.contains("instance_create(")) else {
            continue;
        };
        let object = create
            .split_once("instance_create(")
            .unwrap()
            .1
            .split(')')
            .next()
            .unwrap()
            .rsplit(',')
            .next()
            .unwrap()
            .trim();
        let expected_sprite = body
            .iter()
            .find_map(|line| line.trim().strip_prefix("obj.sprite_index = "))
            .map(|sprite| sprite.trim_end_matches(';').trim())
            .unwrap_or_else(|| catalog::sprite(object).expect("source object default sprite"));
        let expected_cursor = source_cursors
            .get(&tile)
            .unwrap_or_else(|| panic!("missing cursor sprite for {tile:?}"));

        let mut room = PreviewRoom::new(PreviewContext::default());
        let id = room
            .create_tile(tile, 0, 0)
            .unwrap_or_else(|| panic!("preview does not recognize {tile:?}"));
        let preview = room.instance(id).unwrap();
        assert_eq!(preview.object, object, "preview object {tile:?}");
        assert_eq!(preview.sprite, expected_sprite, "preview sprite {tile:?}");
        assert_eq!(
            preview_sprite(tile),
            Some(expected_sprite),
            "palette sprite {tile:?}"
        );
        assert_eq!(
            cursor_sprite(tile),
            Some(*expected_cursor),
            "cursor sprite {tile:?}"
        );
        assert_eq!(
            preview.depth,
            if matches!(tile, '@' | 'X' | 'I') {
                59
            } else {
                100
            }
        );
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
    let huge_count = old_level("\n999999999999999999999999999999");
    for text in [
        "",
        "\n",
        "not a level",
        huge_count.as_str(),
        &old_level("\n2\nonly one"),
        &old_level("\n0\ninvalid"),
    ] {
        assert!(
            std::panic::catch_unwind(|| Level::parse(text)).is_ok(),
            "parsing never panics for {text:?}"
        );
    }
    assert!(Level::parse(&huge_count).is_err());
    let truncated_records = Level::parse(&old_level("\n2\nonly one")).unwrap();
    assert_eq!(
        truncated_records.exit_targets,
        ["only one", ""],
        "Classic's text reader supplies an empty second record at EOF"
    );
    assert!(Level::parse(&old_level("\n0\ninvalid")).is_err());
    let mut invalid = Level::default();
    invalid.tiles.clear();
    assert!(invalid.set_tile(0, 0, '1').is_err());
    assert!(invalid.set_tile(LEVEL_WIDTH, LEVEL_HEIGHT, '1').is_err());
    let _ = invalid.to_text();
    let _ = invalid.to_world();
}

#[test]
fn legacy_sections_accept_the_original_final_line_terminator() {
    let oldest = Level::parse(&old_level("\n")).unwrap();
    assert!(oldest.exit_targets.is_empty());
    assert!(oldest.signs.is_empty());
    let exit_only = Level::parse(&old_level("\n1\nlast-door\n")).unwrap();
    assert_eq!(exit_only.exit_targets, ["last-door"]);
    assert!(exit_only.signs.is_empty());
    // The empty target is data, rather than a discarded line terminator.
    let empty_exit = Level::parse(&old_level("\n1\n")).unwrap();
    assert_eq!(empty_exit.exit_targets, [""]);
}

#[test]
fn nearest_custom_entrance_uses_source_instance_origins() {
    let mut level = Level::default();
    // Source nearest(0, 0) selects (192, 80). Measuring the player centers
    // would incorrectly select (208, 16) and visibly jump to the other door.
    level.set_tile(11, 4, '@').unwrap();
    level.set_tile(12, 0, '@').unwrap();
    let world = level.to_world();
    assert_eq!(world.entrance, spelunky_core::Vec2::new(200.0, 88.0));
    level.exit_targets = vec!["back".into(), "other".into()];
    assert_eq!(
        level.to_world_from(Some("back")).entrance,
        spelunky_core::Vec2::new(216.0, 24.0)
    );
}

#[test]
fn custom_creates_run_before_source_state_and_pickup_overrides() {
    let mut level = Level::default();
    for (x, tile) in ['!', 'h', 'S', 'R', 'J', '<', 'd', '+']
        .into_iter()
        .enumerate()
    {
        level.set_tile(x, 3, tile).unwrap();
    }
    let mut world = level.to_world();
    let shop = world
        .entities
        .iter_mut()
        .find(|e| e.object == "oShopkeeper")
        .unwrap();
    assert_eq!(shop.state, 4);
    assert_eq!(shop.value("enemy_initialized"), 1.0);
    spelunky_core::enemies::init_entity(shop);
    assert_eq!(shop.state, 4);
    let giant = world
        .entities
        .iter()
        .find(|e| e.object == "oGiantSpiderHang")
        .unwrap();
    assert_eq!(giant.value("enemy_initialized"), 1.0);
    assert_eq!(giant.bounds().w, 32.0);
    let rope = world
        .entities
        .iter()
        .find(|e| e.object == "oRopePile")
        .unwrap();
    assert_eq!(rope.value("items_initialized"), 1.0);
    assert_eq!(rope.value("cost"), 0.0);
    assert_eq!(rope.value("forSale"), 0.0);
    assert_eq!(rope.pos, spelunky_core::Vec2::new(72.0, 75.0));
    for name in ["oArrowTrapLeft", "oDarkFall", "oSmashTrap"] {
        let entity = world.entities.iter().find(|e| e.object == name).unwrap();
        assert!(entity.solid, "{name}");
        assert_eq!(entity.pos.y, 64.0);
    }
}

#[test]
fn custom_tiles_retain_source_solid_platform_and_ladder_families() {
    use spelunky_core::{Vec2, catalog};
    let mut level = Level::default();
    level.music = "LUSH".into();
    for (x, tile) in ['|', 'x', 'q', ')', 'L', 'P', 'v', 't']
        .into_iter()
        .enumerate()
    {
        level.set_tile(x, 3, tile).unwrap();
    }
    let world = level.to_world();
    assert_eq!(world.area, spelunky_core::Area::Mines);
    for (x, object, group) in [
        (1, "oTree", "oSolid"),
        (2, "oTree", "oSolid"),
        (5, "oLadderOrange", "oLadder"),
        (6, "oLadderTop", "oPlatform"),
        (7, "oVine", "oLadder"),
        (8, "oVineTop", "oLadder"),
    ] {
        assert_eq!(world.tile(x, 4).unwrap().object, object);
        assert!(catalog::is_a(object, group));
        assert_eq!(world.tile(x, 4).unwrap().is_solid(), group == "oSolid");
    }
    for (x, object) in [(3, "oTreeBranch"), (4, "oLeaves")] {
        assert!(world.tile(x, 4).unwrap().object.is_empty());
        let platform = world
            .entities
            .iter()
            .find(|e| e.object == object && e.pos == Vec2::new(f64::from(x) * 16.0, 64.0))
            .unwrap();
        assert!(catalog::is_a(&platform.object, "oPlatform"));
        assert!(!platform.solid);
    }
    assert_eq!(
        catalog::bounds("oTreeBranch", Vec2::new(48.0, 64.0)).h,
        11.0
    );
    let top = world
        .entities
        .iter()
        .find(|e| {
            e.object == "oTerrainMetadata"
                && (e.pos.x / 16.0).floor() == 2.0
                && (e.pos.y / 16.0).floor() == 4.0
        })
        .unwrap();
    assert_eq!(top.text["terrain_sprite"], "sTreeTop");
    assert!(!top.solid);
    assert!(!world.entities.iter().any(|e| e.object == "oTree"));
}

#[test]
fn editing_keeps_unrelated_and_unused_custom_text_records() {
    let mut level = Level::default();
    level.set_tile(5, 0, 'X').unwrap();
    level.set_tile(8, 0, 'I').unwrap();
    level.exit_targets = vec!["LEVEL ONE".into(), "unused door 🦇".into()];
    level.signs = vec!["EXISTING SIGN".into(), "unused sign #second line".into()];
    let mut editor = EditorState::new(level.clone());
    editor.set_tile(2, 2, '1').unwrap();
    assert_eq!(editor.level.exit_targets, level.exit_targets);
    assert_eq!(editor.level.signs, level.signs);
    editor.set_tile(5, 0, '@').unwrap();
    assert_eq!(editor.level.exit_targets, level.exit_targets);
    editor.set_exit_target(5, 0, "changed".into()).unwrap();
    assert_eq!(editor.level.exit_targets, ["changed", "unused door 🦇"]);
    editor.set_sign(8, 0, "changed sign".into()).unwrap();
    assert_eq!(
        editor.level.signs,
        ["changed sign", "unused sign #second line"]
    );
    editor.set_tile(1, 0, 'I').unwrap();
    assert_eq!(
        editor.level.signs,
        ["", "changed sign", "unused sign #second line"]
    );
    let exported = Level::parse(&editor.level.to_text()).unwrap();
    assert_eq!(exported.exit_targets, editor.level.exit_targets);
    assert_eq!(exported.signs, editor.level.signs);
    let mut absent = Level::default();
    absent.set_tile(1, 0, '@').unwrap();
    let mut editor = EditorState::new(absent);
    editor.set_tile(3, 3, '1').unwrap();
    assert!(editor.level.exit_targets.is_empty());
    assert!(editor.level.signs.is_empty());
}

#[test]
fn classic_editor_document_drops_custom_text_records_without_live_cells() {
    let mut level = Level::default();
    level.set_tile(5, 0, '@').unwrap();
    level.set_tile(8, 0, 'I').unwrap();
    level.exit_targets = vec!["linked.lvl".into(), "orphaned door".into()];
    level.signs = vec!["live sign".into(), "orphaned sign".into()];

    let editor = EditorState::new(level);
    let written = editor.document();
    assert_eq!(written.exit_targets, ["linked.lvl"]);
    assert_eq!(written.signs, ["live sign"]);
}

#[test]
fn compound_pool_keeps_overlapping_platform_and_water_in_actual_game_step() {
    use spelunky_core::{Game, InputFrame, RulesPreset, Vec2};
    let mut level = Level::default();
    level.set_tile(4, 4, '{').unwrap();
    level.set_tile(6, 4, 'q').unwrap();
    let mut game = Game::new(1, RulesPreset::Classic);
    game.load_custom(&level, "pool", "");
    // The authored pool and branch are both present at (112,80). Removing
    // the fish isolates the environment interaction from enemy damage.
    game.world.entities.retain(|e| e.object != "oJaws");
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| { e.object == "oTreeBranch" && e.pos == Vec2::new(112.0, 80.0) })
    );
    game.player.pos = Vec2::new(120.0, 84.0);
    game.player.previous_pos = game.player.pos;
    game.step(&InputFrame::default());
    assert_eq!(game.player.data.get("swimming"), Some(&1.0));
}

#[test]
fn compound_pool_preserves_hard_border_and_instances_beyond_it() {
    use spelunky_core::Vec2;
    let mut level = Level::default();
    level.set_tile(39, 31, '{').unwrap();
    let world = level.to_world();
    assert_eq!(world.tile(41, 32).unwrap().object, "oHardBlock");
    assert_eq!(world.tile(40, 33).unwrap().object, "oHardBlock");
    assert!(
        world
            .entities
            .iter()
            .any(|e| { e.object == "oWaterSwim" && e.pos == Vec2::new(656.0, 512.0) })
    );
    assert!(
        world
            .entities
            .iter()
            .any(|e| { e.object == "oWaterSwim" && e.pos == Vec2::new(688.0, 528.0) })
    );
    assert!(world.solid_point_except(Vec2::new(660.0, 516.0), 0));
}

#[test]
fn custom_border_matches_every_authored_source_room_instance() {
    use std::collections::BTreeSet;
    let source = include_str!("../../../references/classic-1.1/Rooms/rLoadLevel.xml");
    let expected: BTreeSet<_> = source
        .split("<instance id=")
        .skip(1)
        .filter(|instance| instance.contains("<object>oHardBlock</object>"))
        .map(|instance| {
            let position = instance.split_once("<position x=\"").unwrap().1;
            let (x, tail) = position.split_once('"').unwrap();
            let y = tail
                .split_once("y=\"")
                .unwrap()
                .1
                .split('"')
                .next()
                .unwrap();
            (
                x.parse::<usize>().unwrap() / 16,
                y.parse::<usize>().unwrap() / 16,
            )
        })
        .collect();
    let world = Level::default().to_world();
    let actual: BTreeSet<_> = (0..world.height)
        .flat_map(|y| (0..world.width).map(move |x| (x, y)))
        .filter(|&(x, y)| world.tile(x as i32, y as i32).unwrap().object == "oHardBlock")
        .collect();
    assert_eq!(expected.len(), 148);
    assert_eq!(actual, expected);
}

#[test]
fn custom_music_does_not_select_campaign_area_rules() {
    use spelunky_core::{Area, Game, InputFrame, RulesPreset};
    for music in ["CAVE", "LUSH", "ICE", "TEMPLE", "BOSS"] {
        let level = Level {
            music: music.into(),
            ..Level::default()
        };
        let mut game = Game::new(1, RulesPreset::Classic);
        game.load_custom(&level, "music", "");
        assert_eq!(game.world.area, Area::Mines, "{music}");
        assert_eq!(game.world.level, 1);
        assert_eq!(level.music, music);
        // Boss music previously selected Area::Boss and Ankh revival put
        // the player at Olmec's (24,552), below this 544-pixel custom room.
        let entrance = game.world.entrance;
        game.player.equipment.insert("Ankh".into());
        game.player.health = 0.0;
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 4.0, "revive with {music}");
        assert_eq!(game.player.pos, entrance, "revive position with {music}");
    }
}

#[test]
fn editing_an_import_with_unused_records_still_exports_a_readable_level() {
    let level = Level {
        exit_targets: (0..1280).map(|i| format!("unused door {i}")).collect(),
        signs: (0..1280).map(|i| format!("unused sign {i}")).collect(),
        ..Level::default()
    };
    let imported = Level::parse(&level.to_text()).unwrap();
    let mut editor = EditorState::new(imported);
    editor.set_tile(2, 2, 'X').unwrap();
    editor.set_tile(3, 2, 'I').unwrap();
    assert_eq!(editor.level.exit_targets.len(), 1281);
    assert_eq!(editor.level.signs.len(), 1281);
    assert_eq!(Level::parse(&editor.level.to_text()).unwrap(), editor.level);
    assert!(editor.undo());
    assert_eq!(editor.level.signs, level.signs);
    assert!(editor.redo());
    assert_eq!(Level::parse(&editor.level.to_text()).unwrap(), editor.level);
}

#[test]
fn imported_canopies_collapse_after_their_supporting_tree_is_removed() {
    use spelunky_core::{Game, InputFrame, RulesPreset, Vec2};
    let mut level = Level::default();
    // Row-major creation: left leaves are created before their trunk; the
    // right branch is created after it. The source Create event observes that
    // order, and Step removes either platform when the tree disappears.
    level.set_tile(3, 4, ')').unwrap();
    level.set_tile(4, 4, '|').unwrap();
    level.set_tile(5, 4, 'q').unwrap();
    level.set_tile(4, 5, '1').unwrap();
    let mut game = Game::new(1, RulesPreset::Classic);
    game.load_custom(&level, "canopy", "");
    let leaves = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oLeaves")
        .unwrap()
        .id;
    let branch = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oTreeBranch")
        .unwrap()
        .id;
    assert_eq!(game.world.entity(leaves).unwrap().sprite, "sLeaves");
    game.step(&InputFrame::default());
    assert!(game.world.entity(leaves).is_some());
    assert!(game.world.entity(branch).is_some());
    game.world.set_tile(5, 5, "");
    game.step(&InputFrame::default());
    assert!(game.world.entity(leaves).is_none());
    assert!(game.world.entity(branch).is_none());
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.object == "oLeaf")
            .count(),
        3
    );
    // Gone platforms no longer catch the player.
    game.player.pos = Vec2::new(72.0, 64.0);
    game.player.vel = Vec2::new(0.0, 2.0);
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.pos.y > 80.0);
}

#[test]
fn imported_tree_column_steps_in_authored_top_to_bottom_order() {
    use spelunky_core::{Game, InputFrame, RulesPreset};
    let mut level = Level::default();
    for y in 3..=5 {
        level.set_tile(4, y, '|').unwrap();
    }
    level.set_tile(4, 6, '1').unwrap();
    let mut game = Game::new(1, RulesPreset::Classic);
    game.load_custom(&level, "tree-order", "");
    let trees: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.object == "oTerrainMetadata" && e.value("tree_initialized") != 0.0)
        .collect();
    assert_eq!(trees.len(), 3);
    assert!(trees.windows(2).all(|pair| pair[0].pos.y < pair[1].pos.y));
    game.step(&InputFrame::default());
    game.world.set_tile(5, 7, "");
    // Earlier instances see the lower segment before its own Step destroys
    // it. Reversing grid iteration incorrectly removes the whole column in
    // one tick and differs visibly from Classic custom-loader creation order.
    for bottom in (4..=6).rev() {
        game.step(&InputFrame::default());
        for y in 4..=6 {
            assert_eq!(game.world.tile(5, y).unwrap().object == "oTree", y < bottom);
        }
    }
}
