//! Classic tile editor. Coordinates use the renderer's 320x240 logical viewport.
use crate::{
    atlas::Atlas,
    renderer::DrawQuad,
    scene::{rect, text},
};
use spelunky_core::{
    catalog,
    editor::EditorState,
    level::{LEVEL_HEIGHT, LEVEL_WIDTH, Level},
};
use winit::keyboard::KeyCode;

const WHITE: [f32; 4] = [1.0; 4];
const GOLD: [f32; 4] = [1.0, 0.8, 0.2, 1.0];
const MAP_Y: f32 = 36.0;
const CELL: f32 = 8.0;
const COLS: usize = 30;
const ROWS: usize = 21;

pub enum EditorCommand {
    None,
    Save { name: String, text: String },
    Export { name: String, text: String },
    Import,
    Play(Level),
    Back,
}

#[derive(Clone, Copy)]
enum Field {
    Author,
    Music,
    Life,
    Bombs,
    Ropes,
    Next,
    Name,
    Sign(usize, usize),
    Link(usize, usize),
}
impl Field {
    fn label(self) -> &'static str {
        match self {
            Self::Author => "AUTHOR",
            Self::Music => "MUSIC: CAVE/JUNGLE/ICE/TEMPLE",
            Self::Life => "STARTING LIFE (1-99)",
            Self::Bombs => "STARTING BOMBS (0-99)",
            Self::Ropes => "STARTING ROPES (0-99)",
            Self::Next => "NEXT LEVEL FILE OR NONE",
            Self::Name => "LEVEL FILE NAME",
            Self::Sign(..) => "SIGN MESSAGE",
            Self::Link(..) => "DOOR TARGET FILE",
        }
    }
}
struct Entry {
    field: Field,
    value: String,
    replace: bool,
}

pub struct EditorUi {
    state: EditorState,
    name: String,
    pan: [usize; 2],
    cursor: Option<(usize, usize)>,
    stroke: Option<(usize, usize, bool)>,
    entry: Option<Entry>,
    metadata_index: usize,
    status: String,
}

impl Default for EditorUi {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorUi {
    pub fn new() -> Self {
        let mut level = Level::default();
        // A testable blank room, with its entrance, exit and a floor already present.
        for x in 0..LEVEL_WIDTH {
            level.set_tile(x, LEVEL_HEIGHT - 1, '1').unwrap();
        }
        level.set_tile(2, LEVEL_HEIGHT - 2, '@').unwrap();
        level
            .set_tile(LEVEL_WIDTH - 3, LEVEL_HEIGHT - 2, 'X')
            .unwrap();
        Self {
            state: EditorState::new(level),
            name: "custom.lvl".into(),
            pan: [0, LEVEL_HEIGHT - ROWS],
            cursor: None,
            stroke: None,
            entry: None,
            metadata_index: 0,
            status: "PAINT: LEFT   ERASE: RIGHT   ARROWS: PAN".into(),
        }
    }

    pub fn load(&mut self, name: &str, source: &str) -> Result<(), String> {
        let level = Level::parse(source)?;
        let name = file_name(name)?;
        self.state = EditorState::new(level);
        self.name = name;
        self.entry = None;
        self.cursor = None;
        self.stroke = None;
        self.status = format!("LOADED {}", self.name);
        Ok(())
    }

    pub fn message(&mut self, value: String) {
        self.status = value;
    }

    pub fn key(&mut self, key: KeyCode, typed: Option<&str>, control: bool) -> EditorCommand {
        if self.entry.is_some() {
            match key {
                KeyCode::Escape => {
                    self.entry = None;
                }
                KeyCode::Enter | KeyCode::NumpadEnter => self.commit_entry(),
                KeyCode::Tab => {
                    self.commit_entry();
                    if self.entry.is_none() {
                        self.metadata_index = (self.metadata_index + 1) % 6;
                        self.metadata();
                    }
                }
                KeyCode::Backspace => {
                    let entry = self.entry.as_mut().unwrap();
                    if entry.replace {
                        entry.value.clear();
                        entry.replace = false;
                    } else {
                        entry.value.pop();
                    }
                }
                KeyCode::KeyA if control => {
                    self.entry.as_mut().unwrap().replace = true;
                }
                _ if !control => {
                    if let Some(value) = typed {
                        let entry = self.entry.as_mut().unwrap();
                        if entry.replace {
                            entry.value.clear();
                            entry.replace = false;
                        }
                        for ch in value.chars().filter(|ch| !ch.is_control()) {
                            if entry.value.len() < 1024 {
                                entry.value.push(ch);
                            }
                        }
                    }
                }
                _ => {}
            }
            return EditorCommand::None;
        }
        if control {
            match key {
                KeyCode::KeyS => {
                    return EditorCommand::Save {
                        name: self.name.clone(),
                        text: self.state.level.to_text(),
                    };
                }
                KeyCode::KeyE => {
                    return EditorCommand::Export {
                        name: self.name.clone(),
                        text: self.state.level.to_text(),
                    };
                }
                KeyCode::KeyI | KeyCode::KeyO => return EditorCommand::Import,
                KeyCode::KeyZ => {
                    self.state.undo();
                }
                KeyCode::KeyY => {
                    self.state.redo();
                }
                _ => {}
            }
        } else {
            match key {
                KeyCode::Escape => return EditorCommand::Back,
                KeyCode::F5 => return EditorCommand::Play(self.state.level.clone()),
                KeyCode::ArrowLeft => self.pan[0] = self.pan[0].saturating_sub(1),
                KeyCode::ArrowRight => self.pan[0] = (self.pan[0] + 1).min(LEVEL_WIDTH - COLS),
                KeyCode::ArrowUp => self.pan[1] = self.pan[1].saturating_sub(1),
                KeyCode::ArrowDown => self.pan[1] = (self.pan[1] + 1).min(LEVEL_HEIGHT - ROWS),
                KeyCode::KeyM => {
                    self.metadata_index = 0;
                    self.metadata();
                }
                KeyCode::KeyN => self.begin(Field::Name, self.name.clone()),
                KeyCode::KeyS => self.record(false),
                KeyCode::KeyL => self.record(true),
                KeyCode::PageUp | KeyCode::PageDown => {
                    let len = self.state.palette.len();
                    let index = self
                        .state
                        .palette
                        .iter()
                        .position(|&c| c == self.state.selected_tile)
                        .unwrap_or(0);
                    self.state.selected_tile = self.state.palette[if key == KeyCode::PageUp {
                        (index + len - 1) % len
                    } else {
                        (index + 1) % len
                    }];
                }
                _ => {}
            }
        }
        EditorCommand::None
    }

    fn begin(&mut self, field: Field, value: String) {
        self.stroke = None;
        self.entry = Some(Entry {
            field,
            value,
            replace: true,
        });
    }
    fn metadata(&mut self) {
        let level = &self.state.level;
        let (field, value) = match self.metadata_index {
            0 => (Field::Author, level.author.clone()),
            1 => (Field::Music, level.music.clone()),
            2 => (Field::Life, level.starting_life.to_string()),
            3 => (Field::Bombs, level.starting_bombs.to_string()),
            4 => (Field::Ropes, level.starting_ropes.to_string()),
            _ => (Field::Next, level.next_level.clone()),
        };
        self.begin(field, value);
    }
    fn record(&mut self, door: bool) {
        let Some((x, y)) = self.cursor else {
            self.status = "POINT AT A SIGN OR DOOR FIRST".into();
            return;
        };
        let tile = self.state.level.get_tile(x, y).unwrap_or('0');
        let matches = |c| {
            if door {
                matches!(c, '@' | 'X')
            } else {
                c == 'I'
            }
        };
        if !matches(tile) {
            self.status = "POINT AT A SIGN OR DOOR FIRST".into();
            return;
        }
        let index = self
            .state
            .level
            .tiles
            .iter()
            .take(y * LEVEL_WIDTH + x)
            .filter(|&&c| matches(c))
            .count();
        let value = if door {
            self.state.level.exit_target(index)
        } else {
            self.state.level.sign_message(index)
        }
        .to_owned();
        self.begin(
            if door {
                Field::Link(x, y)
            } else {
                Field::Sign(x, y)
            },
            value,
        );
    }
    fn commit_entry(&mut self) {
        let Some(entry) = self.entry.take() else {
            return;
        };
        let mut next = self.state.level.clone();
        let result: Result<(), String> = match entry.field {
            Field::Name => file_name(&entry.value).map(|name| self.name = name),
            Field::Sign(x, y) => self.state.set_sign(x, y, entry.value.clone()),
            Field::Link(x, y) => self.state.set_exit_target(x, y, entry.value.clone()),
            field => {
                let result = match field {
                    Field::Author => {
                        next.author = entry.value.clone();
                        Ok(())
                    }
                    Field::Music => {
                        next.music = entry.value.to_ascii_uppercase();
                        Ok(())
                    }
                    Field::Next => {
                        next.next_level = entry.value.clone();
                        Ok(())
                    }
                    Field::Life | Field::Bombs | Field::Ropes => entry
                        .value
                        .parse::<i32>()
                        .map_err(|_| "ENTER A WHOLE NUMBER".to_owned())
                        .and_then(|value| {
                            let minimum = if matches!(field, Field::Life) { 1 } else { 0 };
                            if !(minimum..=99).contains(&value) {
                                return Err(format!("VALUE MUST BE {minimum}-99"));
                            }
                            match field {
                                Field::Life => next.starting_life = value,
                                Field::Bombs => next.starting_bombs = value,
                                _ => next.starting_ropes = value,
                            };
                            Ok(())
                        }),
                    _ => unreachable!(),
                };
                if result.is_ok() {
                    self.state.replace_level(next);
                }
                result
            }
        };
        match result {
            Ok(()) => self.status = "UPDATED. CTRL+Z TO UNDO".into(),
            Err(error) => {
                self.status = error;
                self.entry = Some(entry);
            }
        }
    }

    pub fn pointer(&mut self, x: f32, y: f32, left: bool, right: bool) -> EditorCommand {
        if self.entry.is_some() || !x.is_finite() || !y.is_finite() {
            return EditorCommand::None;
        }
        if (0.0..240.0).contains(&x) && (MAP_Y..204.0).contains(&y) {
            let cell = (
                (x / CELL) as usize + self.pan[0],
                ((y - MAP_Y) / CELL) as usize + self.pan[1],
            );
            self.cursor = Some(cell);
            let result = if left || right {
                let start = self
                    .stroke
                    .filter(|&(_, _, erase)| erase == right)
                    .map_or(cell, |(x, y, _)| (x, y));
                self.stroke = Some((cell.0, cell.1, right));
                let steps = start
                    .0
                    .abs_diff(cell.0)
                    .max(start.1.abs_diff(cell.1))
                    .max(1);
                let tile = if right { '0' } else { self.state.selected_tile };
                let mut result = Ok(());
                for step in 0..=steps {
                    let alpha = step as f64 / steps as f64;
                    let px = (start.0 as f64 + (cell.0 as f64 - start.0 as f64) * alpha).round()
                        as usize;
                    let py = (start.1 as f64 + (cell.1 as f64 - start.1 as f64) * alpha).round()
                        as usize;
                    if let Err(error) = self.state.set_tile(px, py, tile) {
                        result = Err(error);
                        break;
                    }
                }
                result
            } else {
                self.stroke = None;
                Ok(())
            };
            if let Err(error) = result {
                self.status = error;
            }
        } else {
            self.stroke = None;
            if left && (244.0..314.0).contains(&x) && (MAP_Y..204.0).contains(&y) {
                let index = ((y - MAP_Y) / 10.0) as usize * 7 + ((x - 244.0) / 10.0) as usize;
                if let Some(&tile) = self.state.palette.get(index) {
                    self.state.selected_tile = tile;
                }
            }
        }
        EditorCommand::None
    }

    pub fn draw(&self, atlas: &Atlas, quads: &mut Vec<DrawQuad>) {
        rect(quads, [0.0, 0.0, 320.0, 240.0], [0.025, 0.025, 0.035, 1.0]);
        text(
            quads,
            atlas,
            &format!("EDITOR: {}", self.name),
            3.0,
            2.0,
            0.75,
            GOLD,
        );
        text(
            quads,
            atlas,
            "CTRL S SAVE / E EXPORT / I IMPORT",
            3.0,
            12.0,
            0.5,
            WHITE,
        );
        text(
            quads,
            atlas,
            "F5 PLAY  M METADATA  N NAME  S SIGN  L LINK",
            3.0,
            21.0,
            0.5,
            WHITE,
        );
        for sy in 0..ROWS {
            for sx in 0..COLS {
                let x = sx as f32 * CELL;
                let y = MAP_Y + sy as f32 * CELL;
                rect(
                    quads,
                    [x, y, CELL - 0.25, CELL - 0.25],
                    [0.12, 0.11, 0.10, 1.0],
                );
                let tile = self
                    .state
                    .level
                    .get_tile(sx + self.pan[0], sy + self.pan[1])
                    .unwrap_or('0');
                draw_tile(quads, atlas, tile, x, y, CELL);
            }
        }
        if let Some((x, y)) = self.cursor {
            if (self.pan[0]..self.pan[0] + COLS).contains(&x)
                && (self.pan[1]..self.pan[1] + ROWS).contains(&y)
            {
                let x = (x - self.pan[0]) as f32 * CELL;
                let y = MAP_Y + (y - self.pan[1]) as f32 * CELL;
                rect(quads, [x, y, CELL, 0.5], GOLD);
                rect(quads, [x, y, 0.5, CELL], GOLD);
                rect(quads, [x, y + CELL - 0.5, CELL, 0.5], GOLD);
                rect(quads, [x + CELL - 0.5, y, 0.5, CELL], GOLD);
            }
        }
        for (index, &tile) in self.state.palette.iter().enumerate() {
            let x = 244.0 + (index % 7) as f32 * 10.0;
            let y = MAP_Y + (index / 7) as f32 * 10.0;
            if tile == self.state.selected_tile {
                rect(quads, [x - 1.0, y - 1.0, 10.0, 10.0], GOLD);
            }
            rect(quads, [x, y, 8.0, 8.0], [0.15, 0.15, 0.16, 1.0]);
            draw_tile(quads, atlas, tile, x, y, 8.0);
        }
        let label = spelunky_core::level::tile_spawns(self.state.selected_tile)
            .first()
            .map_or("ERASER", |spawn| spawn.object);
        text(
            quads,
            atlas,
            &format!("{}  PAN {},{}", label, self.pan[0], self.pan[1]),
            3.0,
            207.0,
            0.5,
            GOLD,
        );
        text(
            quads,
            atlas,
            "CTRL Z/Y UNDO/REDO  PGUP/DN TILE  ESC BACK",
            3.0,
            218.0,
            0.5,
            WHITE,
        );
        text(
            quads,
            atlas,
            &self.status.chars().take(62).collect::<String>(),
            3.0,
            229.0,
            0.5,
            WHITE,
        );
        if let Some(entry) = &self.entry {
            rect(quads, [4.0, 70.0, 312.0, 94.0], [0.02, 0.02, 0.04, 1.0]);
            text(quads, atlas, entry.field.label(), 10.0, 78.0, 0.65, GOLD);
            let value: String = entry
                .value
                .chars()
                .rev()
                .take(48)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            rect(
                quads,
                [9.0, 95.0, 301.0, 21.0],
                if entry.replace {
                    [0.12, 0.22, 0.3, 1.0]
                } else {
                    [0.1, 0.1, 0.13, 1.0]
                },
            );
            text(
                quads,
                atlas,
                &format!("{}|", value),
                11.0,
                100.0,
                0.6,
                WHITE,
            );
            text(
                quads,
                atlas,
                "ENTER APPLY  TAB NEXT FIELD  ESC CANCEL",
                10.0,
                126.0,
                0.5,
                WHITE,
            );
            text(
                quads,
                atlas,
                &self.status.chars().take(60).collect::<String>(),
                10.0,
                145.0,
                0.5,
                GOLD,
            );
        }
    }
}

fn file_name(name: &str) -> Result<String, String> {
    let name = name.rsplit(['/', '\\']).next().unwrap_or("").trim();
    if name.is_empty() || name == "." || name == ".." || name.chars().any(char::is_control) {
        return Err("ENTER A VALID FILE NAME".into());
    }
    Ok(if name.to_ascii_lowercase().ends_with(".lvl") {
        name.into()
    } else {
        format!("{name}.lvl")
    })
}

fn draw_tile(quads: &mut Vec<DrawQuad>, atlas: &Atlas, tile: char, x: f32, y: f32, size: f32) {
    if tile == '0' {
        return;
    }
    let spawns = spelunky_core::level::tile_spawns(tile);
    if let Some(frame) = spawns
        .first()
        .and_then(|spawn| spawn.sprite.or_else(|| catalog::sprite(spawn.object)))
        .and_then(|sprite| atlas.sprite(sprite, 0.0))
    {
        quads.push(DrawQuad {
            rect: [x, y, size, size],
            uv: frame.uv,
            color: WHITE,
            ..DrawQuad::default()
        });
    } else {
        text(quads, atlas, &tile.to_string(), x, y, 0.65, GOLD);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paint_erase_history_and_play() {
        let mut ui = EditorUi::new();
        ui.pointer(4.0, 40.0, true, false);
        assert_eq!(ui.state.level.get_tile(0, 11), Some('1'));
        ui.pointer(4.0, 40.0, false, true);
        assert_eq!(ui.state.level.get_tile(0, 11), Some('0'));
        ui.key(KeyCode::KeyZ, None, true);
        assert_eq!(ui.state.level.get_tile(0, 11), Some('1'));
        assert!(matches!(
            ui.key(KeyCode::F5, None, false),
            EditorCommand::Play(_)
        ));
        ui.pointer(4.0, 48.0, false, false);
        ui.pointer(4.0, 48.0, true, false);
        ui.pointer(44.0, 48.0, true, false);
        for x in 0..=5 {
            assert_eq!(ui.state.level.get_tile(x, 12), Some('1'));
        }
    }
    #[test]
    fn metadata_validation_and_sign_roundtrip() {
        let mut ui = EditorUi::new();
        ui.begin(Field::Life, "0".into());
        ui.commit_entry();
        assert!(ui.entry.is_some());
        assert_eq!(ui.state.level.starting_life, 4);
        ui.entry = None;
        ui.state.set_tile(0, 11, 'I').unwrap();
        ui.pointer(4.0, 40.0, false, false);
        ui.record(false);
        ui.key(KeyCode::KeyH, Some("hello"), false);
        ui.key(KeyCode::Enter, None, false);
        let saved = ui.state.level.to_text();
        assert_eq!(Level::parse(&saved).unwrap().sign_message(0), "hello");
        ui.key(KeyCode::KeyZ, None, true);
        assert_eq!(ui.state.level.sign_message(0), "");
    }
    #[test]
    fn import_failure_preserves_document_and_palette_is_complete() {
        let mut ui = EditorUi::new();
        let before = ui.state.level.clone();
        assert!(ui.load("broken.lvl", "invalid").is_err());
        assert_eq!(ui.state.level, before);
        for tile in ui.state.palette.clone() {
            ui.state.select(tile).unwrap();
        }
        assert_eq!(file_name("../../map").unwrap(), "map.lvl");
        assert!(file_name("..").is_err());
    }

    #[test]
    fn keyboard_shortcuts_save_export_import_and_edit_links() {
        let mut ui = EditorUi::new();
        ui.state.set_tile(0, 11, 'X').unwrap();
        ui.pointer(4.0, 40.0, false, false);
        ui.key(KeyCode::KeyL, Some("l"), false);
        ui.key(KeyCode::KeyN, Some("next.lvl"), false);
        ui.key(KeyCode::Enter, None, false);
        assert_eq!(ui.state.level.exit_target(0), "next.lvl");
        let EditorCommand::Save { name, text } = ui.key(KeyCode::KeyS, Some("s"), true) else {
            panic!("Ctrl+S must save");
        };
        assert_eq!(name, "custom.lvl");
        assert_eq!(Level::parse(&text).unwrap(), ui.state.level);
        assert!(matches!(
            ui.key(KeyCode::KeyE, None, true),
            EditorCommand::Export { .. }
        ));
        assert!(matches!(
            ui.key(KeyCode::KeyI, None, true),
            EditorCommand::Import
        ));
        ui.key(KeyCode::KeyM, None, false);
        ui.key(KeyCode::KeyA, Some("Ada"), false);
        ui.key(KeyCode::Tab, None, false);
        assert_eq!(ui.state.level.author, "Ada");
        ui.key(KeyCode::KeyJ, Some("jungle"), false);
        ui.key(KeyCode::Enter, None, false);
        assert_eq!(ui.state.level.music, "JUNGLE");
    }
}
