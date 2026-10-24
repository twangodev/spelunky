//! Classic 1.1 custom levels. Coordinates and spawn offsets follow scrCreateTileObj.
use crate::model::{Area, Vec2, World};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

pub const LEVEL_WIDTH: usize = 40;
pub const LEVEL_HEIGHT: usize = 32;
/// GM8 limits each one-dimensional array index to 32,000. Classic stores
/// declared exit and sign records at indices 0..count, so 32,001 records fit.
const MAX_LEVEL_RECORDS: usize = 32_001;
const CLASSIC_CP1252_TRAILER: &[u8] = b"\n#SPelunky-Rust-Encoding=Windows-1252";

/// Decode unmarked level files using Classic's Windows-1252 text format.
/// Explicitly marked UTF-8 files with a BOM are accepted as a port extension.
pub fn decode_classic_text_bytes(bytes: &[u8]) -> Cow<'_, str> {
    if let Some(classic_bytes) = bytes.strip_suffix(CLASSIC_CP1252_TRAILER) {
        // The marker is outside the Classic format. Only treat it as our
        // encoding trailer when removing it leaves one complete level. A
        // sign may legitimately contain this exact text as its final record;
        // blindly stripping the suffix then corrupts the level.
        if classic_bytes.iter().any(|byte| *byte >= 0x80)
            && let Ok(text) = std::str::from_utf8(classic_bytes)
        {
            let text = text.strip_prefix('\u{feff}').unwrap_or(text);
            if Level::parse_for_editor(text).is_ok() && complete_declared_records(text) {
                return encoding_rs::WINDOWS_1252
                    .decode_without_bom_handling(classic_bytes)
                    .0;
            }
        }
    }
    if let Some(utf8) = bytes.strip_prefix(b"\xef\xbb\xbf") {
        if let Ok(text) = std::str::from_utf8(utf8) {
            return Cow::Borrowed(text);
        }
    }
    encoding_rs::WINDOWS_1252
        .decode_without_bom_handling(bytes)
        .0
}

/// Decode an imported `.lvl` with Classic's Windows-1252 encoding, accepting
/// the port's explicit UTF-8 BOM and CP1252 disambiguation marker extensions.
pub fn decode_classic_level_file_bytes(bytes: &[u8]) -> Cow<'_, str> {
    decode_classic_text_bytes(bytes)
}

/// Split decoded level text at line boundaries, dropping CR bytes as the
/// Windows text reader does. Reads beyond EOF are represented as empty lines.
pub fn classic_level_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|line| {
            line.chars()
                .filter(|character| *character != '\r')
                .collect()
        })
        .collect();
    if lines.len() < LEVEL_HEIGHT + 6 {
        lines.resize_with(LEVEL_HEIGHT + 6, String::new);
    }
    lines
}

fn complete_declared_records(text: &str) -> bool {
    let lines = classic_level_lines(text);
    let mut cursor = LEVEL_HEIGHT + 6;
    if !lines[cursor..].iter().any(|line| !line.is_empty()) {
        return true;
    }
    let Ok(count) = parse_count(&lines[cursor], "exit") else {
        return false;
    };
    cursor += 1;
    let Some(end) = cursor.checked_add(count) else {
        return false;
    };
    if end > lines.len() {
        return false;
    }
    cursor = end;
    if !lines[cursor..].iter().any(|line| !line.is_empty()) {
        return true;
    }
    let Ok(count) = parse_count(&lines[cursor], "sign") else {
        return false;
    };
    cursor += 1;
    cursor
        .checked_add(count)
        .is_some_and(|end| end <= lines.len())
}

/// Encode `.lvl` text in Classic's Windows-1252 format. The source editor
/// writes text files without an encoding marker, so silently replacing a
/// character would make an exported level lose metadata when reopened.
pub fn encode_classic_text_bytes(text: &str) -> Result<Vec<u8>, String> {
    let (bytes, _, had_errors) = encoding_rs::WINDOWS_1252.encode(text);
    if had_errors {
        return Err(
            "Level text contains characters that Classic's Windows-1252 format cannot represent"
                .into(),
        );
    }
    Ok(bytes.into_owned())
}

/// Encode a level file in Classic's Windows-1252 format and mark ambiguous
/// byte sequences that also happen to be valid UTF-8. The trailing marker is
/// after all declared level records, which Classic ignores when loading.
pub fn encode_classic_level_bytes(text: &str) -> Result<Vec<u8>, String> {
    let mut bytes = encode_classic_text_bytes(text)?;
    if bytes.iter().any(|byte| *byte >= 0x80) && std::str::from_utf8(&bytes).is_ok() {
        bytes.extend_from_slice(CLASSIC_CP1252_TRAILER);
    }
    Ok(bytes)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub tiles: Vec<char>,
    pub author: String,
    pub music: String,
    pub starting_life: i32,
    pub starting_bombs: i32,
    pub starting_ropes: i32,
    pub next_level: String,
    /// Shared by BOTH @ and X cells in row-major order, as in Classic.
    pub exit_targets: Vec<String>,
    pub signs: Vec<String>,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            tiles: vec!['0'; LEVEL_WIDTH * LEVEL_HEIGHT],
            author: "ANONYMOUS".into(),
            music: "CAVE".into(),
            starting_life: 4,
            starting_bombs: 4,
            starting_ropes: 4,
            next_level: "NONE".into(),
            exit_targets: vec![],
            signs: vec![],
        }
    }
}

impl Level {
    pub fn parse(text: &str) -> Result<Self, String> {
        Self::parse_with_editor_inventory(text, false)
    }

    /// Parse a level for editing, where the source editor keeps starting
    /// inventory fields raw until test play or ordinary gameplay loading.
    /// `oLoadLevel` calls `scrLoadLevel(-1)` when opening the level editor, so
    /// invalid `real()` conversions do not run on this path.
    pub fn parse_for_editor(text: &str) -> Result<Self, String> {
        Self::parse_with_editor_inventory(text, true)
    }

    /// Parse a linked level using Classic's `scrLoadLevel(-1)` semantics.
    /// Continuations retain the current run inventory, so malformed starting
    /// inventory fields are not converted or rejected on this path.
    pub fn parse_for_linked_load(text: &str) -> Result<Self, String> {
        Self::parse_with_editor_inventory(text, true)
    }

    fn parse_with_editor_inventory(
        text: &str,
        allow_invalid_inventory: bool,
    ) -> Result<Self, String> {
        let lines = classic_level_lines(text);
        let mut level = Self::default();
        level.tiles.clear();
        for line in &lines[..LEVEL_HEIGHT] {
            // Classic always asks string_char_at for columns 1 through 40.
            // Short lines therefore leave trailing cells empty, while columns
            // after 40 are never read. Model empty cells as '0', the level
            // format's no-tile marker.
            let cells: Vec<char> = line.chars().take(LEVEL_WIDTH).collect();
            // Unknown symbols are preserved and ignored by the world builder, just as Classic.
            level.tiles.extend(cells.iter().copied());
            level
                .tiles
                .extend(std::iter::repeat_n('0', LEVEL_WIDTH - cells.len()));
        }
        level.author = lines[32].clone();
        level.music = lines[33].clone();
        level.starting_life =
            parse_inventory_for_mode(&lines[34], 1, level.starting_life, allow_invalid_inventory)?;
        level.starting_bombs =
            parse_inventory_for_mode(&lines[35], 0, level.starting_bombs, allow_invalid_inventory)?;
        level.starting_ropes =
            parse_inventory_for_mode(&lines[36], 0, level.starting_ropes, allow_invalid_inventory)?;
        level.next_level = lines[37].clone();
        let mut cursor = 38;
        if lines
            .get(cursor..)
            .is_some_and(|tail| tail.iter().any(|line| !line.is_empty()))
        {
            let first = lines[cursor].as_str();
            cursor += 1;
            let count = parse_count(first, "exit")?;
            level.exit_targets = read_records(&lines, &mut cursor, count, "exit")?;
            if lines
                .get(cursor..)
                .is_some_and(|tail| tail.iter().any(|line| !line.is_empty()))
            {
                let count = parse_count(&lines[cursor], "sign")?;
                cursor += 1;
                level.signs = read_records(&lines, &mut cursor, count, "sign")?;
            }
        }
        // Classic closes the file after the declared exit/sign records and
        // never validates any trailing lines. Keep this parser permissive at
        // the same boundary so harmless appended text does not reject a level.
        Ok(level)
    }

    /// Emit editor save text using Classic's record count serialization.
    /// Embedded newlines in programmatically supplied metadata become spaces.
    pub fn to_text(&self) -> String {
        let mut lines = Vec::new();
        for y in 0..LEVEL_HEIGHT {
            lines.push(
                (0..LEVEL_WIDTH)
                    .map(|x| {
                        self.get_tile(x, y)
                            // The Classic editor writes each levelArray cell
                            // verbatim. Unknown control symbols (for example,
                            // a tab imported from a hand-edited row) create no
                            // object, but remain in the file after save. Only
                            // line separators would corrupt the fixed 40x32
                            // layout if supplied through the public tile vec.
                            .filter(|c| !matches!(c, '\n' | '\r'))
                            .unwrap_or('0')
                    })
                    .collect(),
            );
        }
        lines.extend([
            one_line(&self.author),
            one_line(&self.music),
            self.starting_life.clamp(1, 99).to_string(),
            self.starting_bombs.clamp(0, 99).to_string(),
            self.starting_ropes.clamp(0, 99).to_string(),
            one_line(&self.next_level),
        ]);
        if self.exit_targets.is_empty() {
            // Both Classic's Y-save and test.tmp writer omit the newline after
            // a zero exit count. The following sign count is therefore parsed
            // as part of the exit count when the level is loaded again.
            lines.push(format!("0{}", self.signs.len()));
        } else {
            lines.push(self.exit_targets.len().to_string());
            lines.extend(self.exit_targets.iter().map(|v| one_line(v)));
            lines.push(self.signs.len().to_string());
        }
        lines.extend(self.signs.iter().map(|v| one_line(v)));
        // No trailing newline: this preserves an empty final record unambiguously.
        lines.join("\n")
    }

    pub fn get_tile(&self, x: usize, y: usize) -> Option<char> {
        if x >= LEVEL_WIDTH || y >= LEVEL_HEIGHT {
            None
        } else {
            self.tiles.get(y * LEVEL_WIDTH + x).copied()
        }
    }

    pub fn set_tile(&mut self, x: usize, y: usize, tile: char) -> Result<(), String> {
        if x >= LEVEL_WIDTH || y >= LEVEL_HEIGHT {
            return Err("Tile coordinate is outside the 40x32 map".into());
        }
        if tile.is_control() {
            return Err("Tile cannot be a control character".into());
        }
        let slot = self
            .tiles
            .get_mut(y * LEVEL_WIDTH + x)
            .ok_or("Level has a malformed tile array")?;
        *slot = tile;
        Ok(())
    }

    pub fn exit_target(&self, index: usize) -> &str {
        repeated_record(&self.exit_targets, index)
    }
    pub fn sign_message(&self, index: usize) -> &str {
        repeated_record(&self.signs, index)
    }

    pub fn to_world(&self) -> World {
        self.to_world_from(None)
    }

    /// A linked entrance leading back to the preceding level takes precedence.
    pub fn to_world_from(&self, previous_level: Option<&str>) -> World {
        // rLoadLevel starts at levelType 0. Its music field selects a track
        // in startMusic; it never selects campaign area rules or backgrounds.
        let mut world = World::new(42, 34, Area::Mines, 1);
        world.entrance = Vec2::new(24.0, 24.0);
        world.name = "Custom level".into();
        for y in 0..world.height {
            for x in 0..world.width {
                if x == 0 || y == 0 || x + 1 == world.width || y + 1 == world.height {
                    world.set_tile(x, y, "oHardBlock");
                }
            }
        }
        let mut exit_index = 0;
        let mut sign_index = 0;
        let mut entrances = Vec::new();
        let mut first_exit = true;
        for y in 0..LEVEL_HEIGHT {
            for x in 0..LEVEL_WIDTH {
                let symbol = self.get_tile(x, y).unwrap_or('0');
                let base = Vec2::new(16.0 + x as f64 * 16.0, 16.0 + y as f64 * 16.0);
                for spawn in tile_spawns(symbol) {
                    let pos = Vec2::new(base.x + spawn.offset.x, base.y + spawn.offset.y);
                    let terrain = is_static_terrain(spawn.object);
                    if terrain {
                        let tx = (pos.x / 16.0) as usize;
                        let ty = (pos.y / 16.0) as usize;
                        // Compound symbols create real overlapping instances:
                        // a Jaws pool must coexist with a later brick, branch,
                        // ladder, or the room's authored hard border. A storage
                        // cell can hold only one instance, so retain subsequent
                        // instances in the entity list instead of replacing or
                        // silently discarding either collision family.
                        if tx > 0
                            && ty > 0
                            && tx + 1 < world.width
                            && ty + 1 < world.height
                            && world
                                .tile(tx as i32, ty as i32)
                                .is_some_and(|t| t.object.is_empty())
                        {
                            world.set_tile(tx, ty, spawn.object);
                            if let Some(sprite) = spawn.sprite {
                                let id = world.spawn("oTerrainMetadata", pos);
                                if let Some(metadata) = world.entity_mut(id) {
                                    metadata.text.insert("terrain_sprite".into(), sprite.into());
                                }
                            }
                            if spawn.object == "oTree" {
                                crate::world_events::initialize_tree(&mut world, tx, ty);
                            }
                            continue;
                        }
                    }
                    // instance_create runs Create events before scrCreateTileObj's
                    // state/sprite/free-item overrides. Use the same initializer path
                    // as generated levels, so the first tick cannot undo those overrides.
                    let id = world.spawn(spawn.object, pos);
                    if spawn.object == "oSkeleton" {
                        // scrLoadLevel creates each tile object before the later
                        // Player birth. Complete Skeleton's closed query body at
                        // this producer point, against only already-born targets.
                        // Other deferred custom Create components remain separate.
                        let player = world
                            .entities
                            .iter()
                            .filter(|entity| {
                                entity.alive
                                    && entity.engine_active
                                    && crate::catalog::is_a(&entity.object, "oPlayer1")
                            })
                            .min_by_key(|entity| entity.creation_serial)
                            .map(|entity| entity.pos);
                        crate::enemies::create_skeleton_player_facing(
                            world.entity_mut(id).unwrap(),
                            player,
                        );
                    }
                    if spawn.object == "oLeaves" {
                        crate::world_events::initialize_leaves(&mut world, id, false);
                    }
                    if symbol == 'X' && first_exit {
                        world.exit = Vec2::new(pos.x + 8.0, pos.y + 8.0);
                        first_exit = false;
                    }
                    let Some(entity) = world.entity_mut(id) else {
                        continue;
                    };
                    if let Some(sprite) = spawn.sprite {
                        entity.sprite = sprite.into();
                    }
                    if symbol == '!' {
                        entity.state = spawn.state;
                    }
                    if spawn.free_item {
                        entity.data.insert("cost".into(), 0.0);
                        entity.data.insert("forSale".into(), 0.0);
                    }
                    if spawn.object == "oPushBlock" {
                        entity.solid = true;
                    }
                    if matches!(symbol, '@' | 'X') {
                        entity
                            .text
                            .insert("leadsTo".into(), self.exit_target(exit_index).into());
                        entity.data.insert("link_index".into(), exit_index as f64);
                        if symbol == '@' {
                            entrances.push((
                                Vec2::new(pos.x + 8.0, pos.y + 8.0),
                                self.exit_target(exit_index),
                            ));
                        }
                    }
                    if symbol == 'I' {
                        entity
                            .text
                            .insert("message".into(), self.sign_message(sign_index).into());
                        entity.data.insert("sign_index".into(), sign_index as f64);
                    }
                }
                if matches!(symbol, '@' | 'X') {
                    exit_index += 1;
                }
                if symbol == 'I' {
                    sign_index += 1;
                }
            }
        }
        if let Some((pos, _)) = entrances.iter().min_by(|a, b| {
            (a.0.x - 8.0)
                .hypot(a.0.y - 8.0)
                .total_cmp(&(b.0.x - 8.0).hypot(b.0.y - 8.0))
        }) {
            world.entrance = *pos;
        }
        if let Some(previous) = previous_level {
            if let Some((pos, _)) = entrances.iter().find(|(_, target)| *target == previous) {
                world.entrance = *pos;
            }
        }
        world
    }
}

fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}
/// Converts editor metadata using the behavior of the bundled GM8 runtime.
/// Empty and whitespace-only input becomes zero; malformed nonempty text raises
/// the runtime's `real()` conversion error. The bundled GM8.0 RC executable's
/// `real()` wrapper at VA `0x551200` establishes this behavior.
pub fn source_real(text: &str) -> Result<f64, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(0.0);
    }
    let number = text
        .parse::<f64>()
        .map_err(|_| "Error in function real().".to_owned())?;
    if !number.is_finite() {
        return Err("Error in function real().".to_owned());
    }
    Ok(number)
}

fn parse_inventory(text: &str, min: i32) -> Result<i32, String> {
    let number = source_real(text)?;
    Ok(number.floor().clamp(f64::from(min), 99.0) as i32)
}

fn parse_inventory_for_mode(
    text: &str,
    min: i32,
    editor_fallback: i32,
    allow_invalid_inventory: bool,
) -> Result<i32, String> {
    match parse_inventory(text, min) {
        Ok(value) => Ok(value),
        // The editor separately preserves and displays this raw text; its
        // numeric Level fields are only placeholders until F5 or game load.
        Err(_) if allow_invalid_inventory => Ok(editor_fallback),
        Err(error) => Err(error),
    }
}
fn parse_count(text: &str, name: &str) -> Result<usize, String> {
    let count = source_real(text).map_err(|_| format!("Invalid {name} count"))?;
    // Classic uses `real()` and loops while the integer index is less than
    // this value, so an empty count means zero and a positive fraction reads
    // the next whole number of records.
    let count = if count > 0.0 {
        count.ceil() as usize
    } else {
        0
    };
    if count > MAX_LEVEL_RECORDS {
        return Err(format!("Too many {name} records"));
    }
    // Classic permits unused records. Their count can exceed the number of
    // cells, especially after editing an imported file while preserving its
    // extra records. The source-derived array index bound keeps the allocation
    // bounded even when the file itself has no imposed size limit.
    Ok(count)
}
fn read_records(
    lines: &[String],
    cursor: &mut usize,
    count: usize,
    _name: &str,
) -> Result<Vec<String>, String> {
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        records.push(lines.get(*cursor).cloned().unwrap_or_default());
        *cursor = cursor.saturating_add(1);
    }
    Ok(records)
}
fn repeated_record(records: &[String], index: usize) -> &str {
    records
        .get(index)
        .or_else(|| records.last())
        .map(String::as_str)
        .unwrap_or("")
}
fn is_static_terrain(object: &str) -> bool {
    matches!(
        object,
        "oBrick"
            | "oLush"
            | "oWaterSwim"
            | "oDark"
            | "oIce"
            | "oTemple"
            | "oLava"
            | "oLadderOrange"
            | "oLadderTop"
            | "oVine"
            | "oVineTop"
            | "oTree"
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileSpawn {
    pub object: &'static str,
    pub offset: Vec2,
    pub sprite: Option<&'static str>,
    pub state: i32,
    pub free_item: bool,
}

/// Exact direct instance creation and overrides in Classic 1.1 scrCreateTileObj.
pub fn tile_spawns(tile: char) -> Vec<TileSpawn> {
    match tile {
        '@' => vec![TileSpawn {
            object: "oEntrance",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'X' => vec![TileSpawn {
            object: "oExit",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'I' => vec![TileSpawn {
            object: "oMsgSign",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '1' => vec![TileSpawn {
            object: "oBrick",
            offset: Vec2::new(0.0, 0.0),
            sprite: Some("sBrick"),
            state: 0,
            free_item: false,
        }],
        '2' => vec![TileSpawn {
            object: "oLush",
            offset: Vec2::new(0.0, 0.0),
            sprite: Some("sLush"),
            state: 0,
            free_item: false,
        }],
        'w' => vec![TileSpawn {
            object: "oWaterSwim",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '3' => vec![TileSpawn {
            object: "oDark",
            offset: Vec2::new(0.0, 0.0),
            sprite: Some("sDark"),
            state: 0,
            free_item: false,
        }],
        'i' => vec![TileSpawn {
            object: "oIce",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'd' => vec![TileSpawn {
            object: "oDarkFall",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '4' => vec![TileSpawn {
            object: "oTemple",
            offset: Vec2::new(0.0, 0.0),
            sprite: Some("sTemple"),
            state: 0,
            free_item: false,
        }],
        'l' => vec![TileSpawn {
            object: "oLava",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'L' => vec![TileSpawn {
            object: "oLadderOrange",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'P' => vec![TileSpawn {
            object: "oLadderTop",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'v' => vec![TileSpawn {
            object: "oVine",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        't' => vec![TileSpawn {
            object: "oVineTop",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '|' => vec![TileSpawn {
            object: "oTree",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'x' => vec![TileSpawn {
            object: "oTree",
            offset: Vec2::new(0.0, 0.0),
            sprite: Some("sTreeTop"),
            state: 0,
            free_item: false,
        }],
        ')' => vec![TileSpawn {
            object: "oLeaves",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'q' => vec![TileSpawn {
            object: "oTreeBranch",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'B' => vec![TileSpawn {
            object: "oPushBlock",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '&' => vec![TileSpawn {
            object: "oWeb",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'r' => vec![TileSpawn {
            object: "oRock",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'j' => vec![TileSpawn {
            object: "oJar",
            offset: Vec2::new(8.0, 10.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'k' => vec![
            TileSpawn {
                object: "oBones",
                offset: Vec2::new(0.0, 0.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
            TileSpawn {
                object: "oSkull",
                offset: Vec2::new(12.0, 12.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
        ],
        'b' => vec![TileSpawn {
            object: "oBat",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'n' => vec![TileSpawn {
            object: "oSnake",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        's' => vec![TileSpawn {
            object: "oSpiderHang",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'S' => vec![TileSpawn {
            object: "oGiantSpiderHang",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'K' => vec![TileSpawn {
            object: "oSkeleton",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'h' => vec![TileSpawn {
            object: "oCaveman",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '!' => vec![TileSpawn {
            object: "oShopkeeper",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 4,
            free_item: false,
        }],
        'f' => vec![TileSpawn {
            object: "oFrog",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'F' => vec![TileSpawn {
            object: "oFireFrog",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'z' => vec![TileSpawn {
            object: "oZombie",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'A' => vec![TileSpawn {
            object: "oVampire",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'p' => vec![
            TileSpawn {
                object: "oWaterSwim",
                offset: Vec2::new(0.0, 0.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
            TileSpawn {
                object: "oPiranha",
                offset: Vec2::new(4.0, 4.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
        ],
        '{' => vec![
            TileSpawn {
                object: "oWaterSwim",
                offset: Vec2::new(0.0, 0.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
            TileSpawn {
                object: "oWaterSwim",
                offset: Vec2::new(16.0, 0.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
            TileSpawn {
                object: "oWaterSwim",
                offset: Vec2::new(32.0, 0.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
            TileSpawn {
                object: "oWaterSwim",
                offset: Vec2::new(48.0, 0.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
            TileSpawn {
                object: "oWaterSwim",
                offset: Vec2::new(0.0, 16.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
            TileSpawn {
                object: "oWaterSwim",
                offset: Vec2::new(16.0, 16.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
            TileSpawn {
                object: "oWaterSwim",
                offset: Vec2::new(32.0, 16.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
            TileSpawn {
                object: "oWaterSwim",
                offset: Vec2::new(48.0, 16.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
            TileSpawn {
                object: "oJaws",
                offset: Vec2::new(0.0, 0.0),
                sprite: None,
                state: 0,
                free_item: false,
            },
        ],
        'M' => vec![TileSpawn {
            object: "oManTrap",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'm' => vec![TileSpawn {
            object: "oMonkey",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'y' => vec![TileSpawn {
            object: "oYeti",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'Y' => vec![TileSpawn {
            object: "oYetiKing",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'a' => vec![TileSpawn {
            object: "oAlien",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'U' => vec![TileSpawn {
            object: "oUFO",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'E' => vec![TileSpawn {
            object: "oAlienBoss",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'H' => vec![TileSpawn {
            object: "oHawkman",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'T' => vec![TileSpawn {
            object: "oTombLord",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '^' => vec![TileSpawn {
            object: "oSpikes",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '<' => vec![TileSpawn {
            object: "oArrowTrapLeft",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '>' => vec![TileSpawn {
            object: "oArrowTrapRight",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        ']' => vec![TileSpawn {
            object: "oSpearTrapTop",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '[' => vec![TileSpawn {
            object: "oSpearTrapBottom",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '_' => vec![TileSpawn {
            object: "oSpringTrap",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '+' => vec![TileSpawn {
            object: "oSmashTrap",
            offset: Vec2::new(0.0, 0.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '$' => vec![TileSpawn {
            object: "oGoldBar",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '*' => vec![TileSpawn {
            object: "oGoldBars",
            offset: Vec2::new(8.0, 8.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '#' => vec![TileSpawn {
            object: "oGoldIdol",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'O' => vec![TileSpawn {
            object: "oCrystalSkull",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '5' => vec![TileSpawn {
            object: "oEmeraldBig",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '6' => vec![TileSpawn {
            object: "oSapphireBig",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '7' => vec![TileSpawn {
            object: "oRubyBig",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '8' => vec![TileSpawn {
            object: "oDiamond",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'c' => vec![TileSpawn {
            object: "oChest",
            offset: Vec2::new(8.0, 8.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'C' => vec![TileSpawn {
            object: "oCrate",
            offset: Vec2::new(8.0, 8.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        'D' => vec![TileSpawn {
            object: "oDamsel",
            offset: Vec2::new(8.0, 8.0),
            sprite: None,
            state: 0,
            free_item: false,
        }],
        '.' => vec![TileSpawn {
            object: "oBombBag",
            offset: Vec2::new(8.0, 10.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        ':' => vec![TileSpawn {
            object: "oBombBox",
            offset: Vec2::new(8.0, 8.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        'u' => vec![TileSpawn {
            object: "oPaste",
            offset: Vec2::new(8.0, 10.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        'R' => vec![TileSpawn {
            object: "oRopePile",
            offset: Vec2::new(8.0, 11.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        '`' => vec![TileSpawn {
            object: "oParaPickup",
            offset: Vec2::new(8.0, 10.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        'o' => vec![TileSpawn {
            object: "oCompass",
            offset: Vec2::new(8.0, 10.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        '/' => vec![TileSpawn {
            object: "oMachete",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        '~' => vec![TileSpawn {
            object: "oSpringShoes",
            offset: Vec2::new(8.0, 10.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        'V' => vec![TileSpawn {
            object: "oSpikeShoes",
            offset: Vec2::new(8.0, 10.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        '}' => vec![TileSpawn {
            object: "oBow",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        '-' => vec![TileSpawn {
            object: "oPistol",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        '=' => vec![TileSpawn {
            object: "oShotgun",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        'W' => vec![TileSpawn {
            object: "oWebCannon",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        '%' => vec![TileSpawn {
            object: "oSpectacles",
            offset: Vec2::new(8.0, 10.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        'G' => vec![TileSpawn {
            object: "oGloves",
            offset: Vec2::new(8.0, 8.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        'g' => vec![TileSpawn {
            object: "oMitt",
            offset: Vec2::new(8.0, 8.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        '?' => vec![TileSpawn {
            object: "oTeleporter",
            offset: Vec2::new(8.0, 12.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        '(' => vec![TileSpawn {
            object: "oMattock",
            offset: Vec2::new(8.0, 10.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        '\\' => vec![TileSpawn {
            object: "oCapePickup",
            offset: Vec2::new(8.0, 10.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        'J' => vec![TileSpawn {
            object: "oJetpack",
            offset: Vec2::new(8.0, 8.0),
            sprite: None,
            state: 0,
            free_item: true,
        }],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LEVEL_HEIGHT, Level, decode_classic_level_file_bytes, decode_classic_text_bytes,
        encode_classic_level_bytes, encode_classic_text_bytes, parse_count,
    };

    #[test]
    fn classic_text_encoding_roundtrips_windows_1252_and_rejects_lossy_text() {
        let text = "Café – €";
        let encoded = encode_classic_text_bytes(text).unwrap();
        assert_eq!(encoded, [b'C', b'a', b'f', 0xe9, b' ', 0x96, b' ', 0x80]);
        assert_eq!(decode_classic_text_bytes(&encoded), text);
        assert!(encode_classic_text_bytes("emoji 🧗").is_err());
    }

    #[test]
    fn long_classic_record_text_roundtrips_without_truncation() {
        let mut level = Level::default();
        level.author = "é".repeat(512);
        level
            .exit_targets
            .push("exit-".to_owned() + &"x".repeat(512));
        level.signs.push("sign-".to_owned() + &"x".repeat(512));
        let text = level.to_text();
        let bytes = encode_classic_level_bytes(&text).unwrap();
        let decoded = decode_classic_level_file_bytes(&bytes);
        assert_eq!(decoded, text);
        assert_eq!(Level::parse(&decoded).unwrap(), level);
    }

    #[test]
    fn large_trailing_data_after_declared_records_is_ignored() {
        let mut level = Level::default();
        level.exit_targets.push("destination".into());
        level.signs.push("message".into());
        let mut text = level.to_text();
        text.push('\n');
        text.push_str(&"ignored".repeat(600_000));
        assert!(text.len() > 4 * 1024 * 1024);

        assert_eq!(Level::parse(&text).unwrap(), level);
    }

    #[test]
    fn classic_level_marker_preserves_cp1252_that_looks_like_utf8() {
        let mut level = Level::default();
        level.author = "MÃ©".into();
        let text = level.to_text();
        let encoded = encode_classic_level_bytes(&text).unwrap();
        assert!(encoded.ends_with(b"\n#SPelunky-Rust-Encoding=Windows-1252"));
        assert_eq!(decode_classic_text_bytes(&encoded), text);
        assert_eq!(
            Level::parse(&decode_classic_text_bytes(&encoded)).unwrap(),
            level
        );
    }

    #[test]
    fn marker_looking_final_sign_roundtrips_without_being_stripped() {
        let marker_sign = "#SPelunky-Rust-Encoding=Windows-1252";
        for author in ["Café", "ASCII", "MÃ©"] {
            let mut level = Level::default();
            level.author = author.into();
            level.set_tile(4, 4, 'I').unwrap();
            // Keep the source's zero-exit count concatenation from consuming
            // this sign as an exit target; this test isolates trailer parsing.
            level.exit_targets.push("DESTINATION".into());
            level.signs.push(marker_sign.into());

            let encoded = encode_classic_level_bytes(&level.to_text()).unwrap();
            let decoded = decode_classic_text_bytes(&encoded);
            let loaded = Level::parse(&decoded).unwrap();

            assert_eq!(loaded, level, "author encoding {author:?}");
        }
    }

    #[test]
    fn marker_decoding_accepts_editor_raw_inventory_fields() {
        let mut level = Level::default();
        level.author = "MÃ©".into();
        level
            .signs
            .push("#SPelunky-Rust-Encoding=Windows-1252".into());
        let mut lines: Vec<_> = level.to_text().lines().map(str::to_owned).collect();
        lines[34] = "12A".into();
        let text = lines.join("\n");

        let encoded = encode_classic_level_bytes(&text).unwrap();
        assert!(encoded.ends_with(
            b"\n#SPelunky-Rust-Encoding=Windows-1252\n#SPelunky-Rust-Encoding=Windows-1252"
        ));
        assert_eq!(decode_classic_text_bytes(&encoded), text);
        assert!(Level::parse_for_editor(&decode_classic_text_bytes(&encoded)).is_ok());
        assert!(Level::parse(&decode_classic_text_bytes(&encoded)).is_err());
    }

    #[test]
    fn classic_text_decoding_uses_source_cp1252_and_explicit_utf8_bom() {
        let mut level = Level::default();
        level.author = "CAFÉ".into();
        let text = level.to_text();

        let (windows_1252, _, had_errors) = encoding_rs::WINDOWS_1252.encode(&text);
        assert!(!had_errors);
        let decoded = decode_classic_text_bytes(&windows_1252);
        assert_eq!(decoded, text);
        assert_eq!(Level::parse(&decoded).unwrap().author, "CAFÉ");

        // `Ã©` is encoded as C3 A9 in Windows-1252, which is also valid UTF-8.
        // Classic has no encoding marker and reads those bytes as `Ã©`.
        level.author = "Ã©".into();
        let ambiguous = level.to_text();
        let (legacy, _, had_errors) = encoding_rs::WINDOWS_1252.encode(&ambiguous);
        assert!(!had_errors);
        assert!(std::str::from_utf8(&legacy).is_ok());
        let decoded = decode_classic_text_bytes(&legacy);
        assert_eq!(decoded, ambiguous);
        assert_eq!(Level::parse(&decoded).unwrap().author, "Ã©");

        let mut utf8_bom = b"\xef\xbb\xbf".to_vec();
        utf8_bom.extend_from_slice(text.as_bytes());
        assert_eq!(decode_classic_text_bytes(&utf8_bom), text);
        assert_eq!(decode_classic_text_bytes(&[0xff, 0xfe]), "ÿþ");
        assert_eq!(decode_classic_text_bytes(b"\xef\xbb\xbfLEVEL"), "LEVEL");
    }

    #[test]
    fn linked_level_load_skips_start_inventory_conversion() {
        let mut lines: Vec<String> = Level::default()
            .to_text()
            .lines()
            .map(str::to_owned)
            .collect();
        lines[34] = "not-a-number".into();
        lines[35] = "not-a-number".into();
        lines[36] = "not-a-number".into();
        let source = lines.join("\n");

        assert!(Level::parse(&source).is_err());
        let linked = Level::parse_for_linked_load(&source).unwrap();
        assert_eq!(
            (
                linked.starting_life,
                linked.starting_bombs,
                linked.starting_ropes,
            ),
            (4, 4, 4)
        );
    }

    #[test]
    fn short_classic_levels_default_unread_rows_and_metadata_from_eof() {
        let level = Level::parse("X").unwrap();

        assert_eq!(level.get_tile(0, 0), Some('X'));
        assert_eq!(level.get_tile(1, 0), Some('0'));
        assert_eq!(level.get_tile(0, 31), Some('0'));
        assert_eq!(level.author, "");
        assert_eq!(level.music, "");
        assert_eq!(
            (
                level.starting_life,
                level.starting_bombs,
                level.starting_ropes,
            ),
            (1, 0, 0)
        );
        assert_eq!(level.next_level, "");
        assert!(level.exit_targets.is_empty());
        assert!(level.signs.is_empty());
    }

    #[test]
    fn classic_text_reads_drop_carriage_returns_and_preserve_long_lines() {
        let mut lines: Vec<_> = Level::default()
            .to_text()
            .lines()
            .map(str::to_owned)
            .collect();
        lines[32] = format!("{}\rTAIL", "A".repeat(400));

        let parsed = Level::parse(&lines.join("\n")).unwrap();
        assert_eq!(parsed.author, format!("{}TAIL", "A".repeat(400)));

        lines[32] = "AUTHOR\rNAME".into();
        assert_eq!(
            Level::parse(&lines.join("\n")).unwrap().author,
            "AUTHORNAME"
        );
    }

    #[test]
    fn declared_records_beyond_eof_are_read_as_empty_strings() {
        let mut lines: Vec<_> = Level::default()
            .to_text()
            .lines()
            .take(LEVEL_HEIGHT + 6)
            .map(str::to_owned)
            .collect();
        lines.push("3".into());
        lines.push("first-target".into());

        let parsed = Level::parse(&lines.join("\n")).unwrap();
        assert_eq!(
            parsed.exit_targets,
            vec!["first-target".to_owned(), String::new(), String::new()]
        );
        assert!(parsed.signs.is_empty());
    }

    #[test]
    fn declared_record_count_respects_classic_array_index_limit() {
        assert_eq!(parse_count("32001", "exit").unwrap(), 32_001);
        assert_eq!(parse_count("32001", "sign").unwrap(), 32_001);
        assert_eq!(
            parse_count("32002", "exit").unwrap_err(),
            "Too many exit records"
        );
        assert_eq!(
            parse_count("32002", "sign").unwrap_err(),
            "Too many sign records"
        );
    }
}
