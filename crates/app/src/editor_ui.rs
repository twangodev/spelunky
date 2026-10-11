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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Confirmation {
    Exit,
    New,
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
    confirmation: Option<Confirmation>,
    new_pressed: bool,
    pointer_left: bool,
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
            confirmation: None,
            new_pressed: false,
            pointer_left: false,
            status: "PAINT: LEFT   ERASE: RIGHT   ARROWS: PAN".into(),
        }
    }

    pub fn load(&mut self, name: &str, source: &str) -> Result<(), String> {
        let level = Level::parse(source)?;
        let name = file_name(name)?;
        self.state = EditorState::new(level);
        self.name = name;
        self.entry = None;
        self.confirmation = None;
        self.new_pressed = false;
        self.pointer_left = false;
        self.cursor = None;
        self.stroke = None;
        self.status = format!("LOADED {}", self.name);
        Ok(())
    }

    pub fn message(&mut self, value: String) {
        self.status = value;
    }

    pub fn level_name(&self) -> &str {
        &self.name
    }

    /// Focus loss and leaving the editor terminate a stroke even when the
    /// operating system delivers no matching mouse-button release event.
    pub fn cancel_pointer(&mut self) {
        self.stroke = None;
        self.new_pressed = false;
        self.pointer_left = false;
    }

    pub fn key(&mut self, key: KeyCode, typed: Option<&str>, control: bool) -> EditorCommand {
        if let Some(confirmation) = self.confirmation {
            return match key {
                KeyCode::KeyY => EditorCommand::Save {
                    name: self.name.clone(),
                    text: self.state.level.to_text(),
                },
                KeyCode::KeyN => self.complete_confirmation(),
                KeyCode::Escape if confirmation == Confirmation::Exit => {
                    self.confirmation = None;
                    EditorCommand::None
                }
                // Source Escape cancels EXIT, but leaves the editor from NEW.
                KeyCode::Escape => {
                    self.restart_document();
                    EditorCommand::Back
                }
                _ => EditorCommand::None,
            };
        }
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
                KeyCode::Backspace | KeyCode::Delete => {
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
                KeyCode::KeyN => self.confirm(Confirmation::New),
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
                KeyCode::Escape => self.confirm(Confirmation::Exit),
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

    fn confirm(&mut self, confirmation: Confirmation) {
        self.cancel_pointer();
        self.confirmation = Some(confirmation);
        self.status.clear();
    }

    fn restart_document(&mut self) {
        *self = Self::new();
        self.state = EditorState::new(Level::default());
        self.begin(Field::Name, String::new());
    }

    fn complete_confirmation(&mut self) -> EditorCommand {
        match self.confirmation {
            Some(Confirmation::Exit) => {
                self.restart_document();
                EditorCommand::Back
            }
            Some(Confirmation::New) => {
                self.restart_document();
                EditorCommand::None
            }
            None => EditorCommand::None,
        }
    }

    /// Only the host's successful persistence result may finish a Y response.
    pub fn saved(&mut self) -> EditorCommand {
        self.complete_confirmation()
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
        let pressed = left && !self.pointer_left;
        let released = !left && self.pointer_left;
        self.pointer_left = left;
        if self.confirmation.is_some() || self.entry.is_some() || !x.is_finite() || !y.is_finite() {
            return EditorCommand::None;
        }
        let over_new = (280.0..317.0).contains(&x) && (10.0..23.0).contains(&y);
        if pressed && over_new {
            self.new_pressed = true;
        } else if !over_new {
            self.new_pressed = false;
        }
        if released && self.new_pressed {
            self.confirm(Confirmation::New);
            return EditorCommand::None;
        }
        if over_new {
            self.stroke = None;
            return EditorCommand::None;
        }
        if (0.0..240.0).contains(&x) && (MAP_Y..204.0).contains(&y) {
            let cell = (
                (x / CELL) as usize + self.pan[0],
                ((y - MAP_Y) / CELL) as usize + self.pan[1],
            );
            self.cursor = Some(cell);
            let result = if left || right {
                let erase = right && !left;
                let start = self
                    .stroke
                    .filter(|&(_, _, was_erase)| was_erase == erase)
                    .map_or(cell, |(x, y, _)| (x, y));
                self.stroke = Some((cell.0, cell.1, erase));
                let steps = start
                    .0
                    .abs_diff(cell.0)
                    .max(start.1.abs_diff(cell.1))
                    .max(1);
                let mut result = Ok(());
                for step in 0..=steps {
                    let alpha = step as f64 / steps as f64;
                    let px = (start.0 as f64 + (cell.0 as f64 - start.0 as f64) * alpha).round()
                        as usize;
                    let py = (start.1 as f64 + (cell.1 as f64 - start.1 as f64) * alpha).round()
                        as usize;
                    let edit = if erase {
                        self.state.erase(px, py)
                    } else {
                        self.state.paint(px, py)
                    };
                    if let Err(error) = edit {
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
        text(quads, atlas, "CTRL N NEW", 224.0, 13.0, 0.5, WHITE);
        rect(quads, [280.0, 10.0, 37.0, 13.0], [0.15, 0.15, 0.16, 1.0]);
        text(quads, atlas, "NEW", 289.0, 13.0, 0.75, GOLD);
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
                if let Some(frame) = editor_icon(self.state.selected_tile)
                    .and_then(|sprite| atlas.sprite(sprite, 0.0))
                {
                    quads.push(DrawQuad {
                        rect: [x, y, CELL, CELL],
                        uv: frame.uv,
                        color: WHITE,
                        ..DrawQuad::default()
                    });
                }
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
            "CTRL Z/Y UNDO/REDO  PGUP/DN TILE  ESC EXIT",
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
        if let Some(confirmation) = self.confirmation {
            rect(quads, [6.0, 88.0, 308.0, 88.0], [0.02, 0.02, 0.03, 0.98]);
            text(quads, atlas, "SAVE LEVEL? (Y/N)", 92.0, 103.0, 1.0, WHITE);
            let escape = if confirmation == Confirmation::Exit {
                "ESC CANCEL"
            } else {
                "ESC RETURN TO TITLE"
            };
            text(quads, atlas, escape, 72.0, 119.0, 0.75, WHITE);
            let mut y = 134.0;
            for (tile, warning) in [('@', "WARNING: NO ENTRANCE!"), ('X', "WARNING: NO EXIT!")] {
                if !self.state.level.tiles.contains(&tile) {
                    text(quads, atlas, warning, 72.0, y, 0.75, GOLD);
                    y += 10.0;
                }
            }
        }
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

// scrSetCursorTile assigns authored editor artwork, independently of gameplay spawns.
fn editor_icon(tile: char) -> Option<&'static str> {
    match tile {
        '@' => Some("sEntrance"),
        'X' => Some("sExit"),
        'I' => Some("sMsgSign"),
        '1' => Some("sBrick"),
        '2' => Some("sLush"),
        'w' => Some("sWater"),
        '3' => Some("sDark"),
        'i' => Some("sIce"),
        'd' => Some("sDarkFall"),
        '4' => Some("sTemple"),
        'l' => Some("sLava"),
        'L' => Some("sLadder"),
        'P' => Some("sLadderTop"),
        'v' => Some("sVine"),
        't' => Some("sVineTop"),
        '|' => Some("sTreeTrunk"),
        'x' => Some("sTreeTop"),
        ')' => Some("sLeaves"),
        'q' => Some("sTreeBranchLeft"),
        'B' => Some("sBlock"),
        '&' => Some("sWeb"),
        'r' => Some("sRockTile"),
        'j' => Some("sJarTile"),
        'k' => Some("sFakeBonesLeft"),
        'b' => Some("sBatLeft"),
        'n' => Some("sSnakeLeft"),
        's' => Some("sSpider"),
        'S' => Some("sGiantSpiderHang"),
        'K' => Some("sSkeletonLeft"),
        'h' => Some("sCavemanLeft"),
        '!' => Some("sShopLeftIco"),
        'f' => Some("sFrogLeft"),
        'F' => Some("sFireFrogLeft"),
        'z' => Some("sZombieLeft"),
        'A' => Some("sVampireLeft"),
        'p' => Some("sPiranhaTile"),
        'M' => Some("sManTrapLeft"),
        'm' => Some("sMonkeyLeft"),
        'y' => Some("sYetiLeft"),
        'a' => Some("sAlien"),
        'U' => Some("sUFO"),
        'E' => Some("sAlienBoss"),
        'H' => Some("sHawkLeft"),
        'Y' => Some("sYetiKingLeft"),
        '{' => Some("sMegaMouthTile"),
        'T' => Some("sTombLordLeft"),
        '^' => Some("sSpikes"),
        '<' => Some("sArrowTrapLeft"),
        '>' => Some("sArrowTrapRight"),
        ']' => Some("sSpearTrap1"),
        '[' => Some("sSpearTrap2"),
        '_' => Some("sSpringTrap"),
        '+' => Some("sSmashTrap"),
        '$' => Some("sGoldBarTile"),
        '*' => Some("sGoldBarsTile"),
        '#' => Some("sGoldIdolTile"),
        'O' => Some("sCrystalSkullTile"),
        '5' => Some("sEmeraldBigTile"),
        '6' => Some("sSapphireBigTile"),
        '7' => Some("sRubyBigTile"),
        '8' => Some("sDiamondTile"),
        'c' => Some("sChestTile"),
        'C' => Some("sCrateTile"),
        'D' => Some("sDamselLeftIco"),
        '.' => Some("sBombBagTile"),
        ':' => Some("sBombBoxTile"),
        'u' => Some("sBombPasteTile"),
        'R' => Some("sRopePileTile"),
        '`' => Some("sParachuteTile"),
        'o' => Some("sCompassTile"),
        '/' => Some("sMacheteTile"),
        '~' => Some("sSpringShoesTile"),
        'V' => Some("sSpikeShoesTile"),
        '}' => Some("sBowTile"),
        '-' => Some("sPistolTile"),
        '=' => Some("sShotgunTile"),
        'W' => Some("sWebCannonTile"),
        '%' => Some("sSpectaclesTile"),
        'G' => Some("sGlovesTile"),
        'g' => Some("sMittTile"),
        '?' => Some("sTeleporterTile"),
        '(' => Some("sMattockTile"),
        '\\' => Some("sCapeTile"),
        'J' => Some("sJetpackTile"),
        _ => None,
    }
}

fn editor_preview_icon(tile: char) -> Option<&'static str> {
    // scrCreateTile uses hanging enemy/web previews and the boss editor icon.
    match tile {
        '&' => Some("sWebTile"),
        'b' => Some("sBatHang"),
        's' => Some("sSpiderHang"),
        'E' => Some("sAlienBossTile"),
        _ => editor_icon(tile),
    }
}

fn draw_tile(quads: &mut Vec<DrawQuad>, atlas: &Atlas, tile: char, x: f32, y: f32, size: f32) {
    if tile == '0' {
        return;
    }
    let spawns = spelunky_core::level::tile_spawns(tile);
    if let Some(frame) = editor_preview_icon(tile)
        .or_else(|| {
            spawns
                .first()
                .and_then(|spawn| spawn.sprite.or_else(|| catalog::sprite(spawn.object)))
        })
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
    fn simultaneous_mouse_buttons_paint_before_right_only_erase() {
        let mut ui = EditorUi::new();
        ui.pointer(4.0, 40.0, true, true);
        assert_eq!(ui.state.level.get_tile(0, 11), Some('1'));
        ui.pointer(20.0, 40.0, true, true);
        for x in 0..=2 {
            assert_eq!(ui.state.level.get_tile(x, 11), Some('1'));
        }
        // Switching to right-only starts a new erasing stroke at that cell.
        ui.pointer(28.0, 40.0, false, true);
        assert_eq!(ui.state.level.get_tile(2, 11), Some('1'));
        assert_eq!(ui.state.level.get_tile(3, 11), Some('0'));
        ui.pointer(20.0, 40.0, false, true);
        assert_eq!(ui.state.level.get_tile(2, 11), Some('0'));
        assert_eq!(ui.state.level.get_tile(1, 11), Some('1'));
    }

    #[test]
    fn repainting_doors_and_signs_creates_fresh_text_and_undo_restores_it() {
        for (tile, replacement) in [('@', '@'), ('X', 'X'), ('I', 'I'), ('@', 'X'), ('X', '@')] {
            let mut ui = EditorUi::new();
            ui.state.select(tile).unwrap();
            ui.pointer(4.0, 40.0, true, false);
            ui.pointer(4.0, 40.0, false, false);
            // Customize through the ordinary editor text-entry keys.
            ui.key(
                if tile == 'I' {
                    KeyCode::KeyS
                } else {
                    KeyCode::KeyL
                },
                None,
                false,
            );
            ui.key(KeyCode::KeyA, Some("custom text"), false);
            ui.key(KeyCode::Enter, None, false);
            let customized = ui.state.level.clone();
            ui.state.select(replacement).unwrap();
            ui.pointer(4.0, 40.0, true, false);
            let index = 0;
            assert_eq!(
                if tile == 'I' {
                    ui.state.level.sign_message(index)
                } else {
                    ui.state.level.exit_target(index)
                },
                ""
            );
            ui.key(KeyCode::KeyZ, None, true);
            assert_eq!(
                ui.state.level, customized,
                "undo must restore the customized {tile}"
            );
            ui.key(KeyCode::KeyY, None, true);
            assert_eq!(
                if tile == 'I' {
                    ui.state.level.sign_message(index)
                } else {
                    ui.state.level.exit_target(index)
                },
                ""
            );
        }
    }

    #[test]
    fn repainting_one_imported_record_preserves_other_cells_and_extra_records() {
        let mut authored = Level::default();
        authored.set_tile(0, 11, '@').unwrap();
        authored.set_tile(1, 11, 'X').unwrap();
        authored.set_tile(2, 11, 'I').unwrap();
        authored.set_tile(3, 11, 'I').unwrap();
        authored.exit_targets = vec!["first.lvl".into(), "other.lvl".into(), "extra-door".into()];
        authored.signs = vec![
            "first sign".into(),
            "other sign".into(),
            "extra-sign".into(),
        ];
        let mut ui = EditorUi::new();
        ui.load("records.lvl", &authored.to_text()).unwrap();
        ui.state.select('@').unwrap();
        ui.pointer(4.0, 40.0, true, false);
        assert_eq!(ui.state.level.exit_targets, ["", "other.lvl", "extra-door"]);
        assert_eq!(ui.state.level.signs, authored.signs);
        ui.pointer(12.0, 40.0, false, false);
        ui.state.select('I').unwrap();
        ui.pointer(12.0, 40.0, true, false);
        assert_eq!(ui.state.level.exit_targets, ["", "extra-door"]);
        assert_eq!(
            ui.state.level.signs,
            ["", "first sign", "other sign", "extra-sign"]
        );
        ui.key(KeyCode::KeyZ, None, true);
        assert_eq!(ui.state.level.exit_targets, ["", "other.lvl", "extra-door"]);
        assert_eq!(ui.state.level.signs, authored.signs);
    }

    #[test]
    fn ordinary_palette_selection_uses_source_cursor_and_preview_artwork() {
        let source = include_str!(
            "../../../references/classic-1.1/Scripts/Level Editor/scrSetCursorTile.gml"
        );
        let atlas = Atlas::load().unwrap();
        let mut checked = 0;
        for line in source
            .lines()
            .filter(|line| line.contains("oCursObj.sprite_index ="))
        {
            let tile = line.split('"').nth(1).unwrap().chars().next().unwrap();
            let sprite = line
                .split("sprite_index = ")
                .nth(1)
                .unwrap()
                .trim_end_matches(';');
            let expected_cursor = atlas.sprite(sprite, 0.0).unwrap();
            let preview = match tile {
                '&' => "sWebTile",
                'b' => "sBatHang",
                's' => "sSpiderHang",
                'E' => "sAlienBossTile",
                _ => sprite,
            };
            let expected_preview = atlas.sprite(preview, 0.0).unwrap();
            let mut ui = EditorUi::new();
            let index = ui
                .state
                .palette
                .iter()
                .position(|&value| value == tile)
                .unwrap();
            let palette = [
                244.0 + (index % 7) as f32 * 10.0,
                MAP_Y + (index / 7) as f32 * 10.0,
                8.0,
                8.0,
            ];
            // Use palette clicks, paint, and hover through the normal editor path.
            ui.pointer(palette[0] + 4.0, palette[1] + 4.0, true, false);
            ui.pointer(4.0, 40.0, false, false);
            ui.pointer(4.0, 40.0, true, false);
            ui.pointer(12.0, 40.0, false, false);
            let mut quads = Vec::new();
            ui.draw(&atlas, &mut quads);
            for rect in [[0.0, MAP_Y, CELL, CELL], palette] {
                assert!(
                    quads
                        .iter()
                        .any(|quad| quad.rect == rect && quad.uv == expected_preview.uv),
                    "source preview for {tile} must be {preview} at {rect:?}"
                );
            }
            assert!(
                quads
                    .iter()
                    .any(|quad| quad.rect == [CELL, MAP_Y, CELL, CELL]
                        && quad.uv == expected_cursor.uv),
                "source cursor for {tile} must be {sprite}"
            );
            checked += 1;
        }
        assert_eq!(
            checked, 84,
            "every authored selectable tile must be exercised"
        );
    }

    #[test]
    fn authored_fish_icons_render_in_map_palette_and_selected_cursor() {
        let atlas = Atlas::load().unwrap();
        let water = atlas.sprite("sWater", 0.0).unwrap();
        for (tile, sprite) in [('p', "sPiranhaTile"), ('{', "sMegaMouthTile")] {
            let icon = atlas.sprite(sprite, 0.0).unwrap();
            assert_ne!(icon.uv, water.uv);
            let mut ui = EditorUi::new();
            ui.state.select(tile).unwrap();
            // Paint through the normal pointer path, then hover an empty cell.
            ui.pointer(4.0, 40.0, true, false);
            ui.pointer(12.0, 40.0, false, false);
            let mut quads = Vec::new();
            ui.draw(&atlas, &mut quads);
            let index = ui
                .state
                .palette
                .iter()
                .position(|&value| value == tile)
                .unwrap();
            for rect in [
                [0.0, MAP_Y, CELL, CELL],
                [CELL, MAP_Y, CELL, CELL],
                [
                    244.0 + (index % 7) as f32 * 10.0,
                    MAP_Y + (index / 7) as f32 * 10.0,
                    8.0,
                    8.0,
                ],
            ] {
                assert!(
                    quads
                        .iter()
                        .any(|quad| quad.rect == rect && quad.uv == icon.uv),
                    "{tile} must use its authored icon at {rect:?}"
                );
                assert!(
                    !quads
                        .iter()
                        .any(|quad| quad.rect == rect && quad.uv == water.uv)
                );
            }
            let EditorCommand::Play(level) = ui.key(KeyCode::F5, None, false) else {
                panic!("F5 must preserve the edited document");
            };
            let world = level.to_world();
            let count = |object| {
                world
                    .entities
                    .iter()
                    .filter(|entity| entity.object == object)
                    .count()
                    + world
                        .tiles
                        .iter()
                        .filter(|tile| tile.object == object)
                        .count()
            };
            assert_eq!(count("oWaterSwim"), if tile == 'p' { 1 } else { 8 });
            assert_eq!(count(if tile == 'p' { "oPiranha" } else { "oJaws" }), 1);
            assert_eq!(
                count("oPiranhaTile") + count("oMegaMouthTile"),
                0,
                "editor artwork must not replace gameplay actors"
            );
        }
    }

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
    fn interrupted_pointer_stroke_does_not_connect_the_next_click() {
        let mut ui = EditorUi::new();
        ui.pointer(4., 40., true, false);
        ui.cancel_pointer();
        ui.pointer(44., 40., true, false);
        assert_eq!(ui.state.level.get_tile(0, 11), Some('1'));
        assert_eq!(ui.state.level.get_tile(5, 11), Some('1'));
        for x in 1..5 {
            assert_eq!(ui.state.level.get_tile(x, 11), Some('0'));
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
    #[test]
    fn source_exit_confirmation_blocks_edits_cancels_and_discards_with_ordinary_keys() {
        let mut ui = EditorUi::new();
        let original = ui.state.level.clone();
        assert!(matches!(
            ui.key(KeyCode::Escape, None, false),
            EditorCommand::None
        ));
        ui.pointer(4., 40., true, false);
        ui.pointer(4., 40., false, false);
        ui.key(KeyCode::ArrowRight, None, false);
        assert!(matches!(
            ui.key(KeyCode::F5, None, false),
            EditorCommand::None
        ));
        assert!(matches!(
            ui.key(KeyCode::KeyS, None, true),
            EditorCommand::None
        ));
        assert_eq!(ui.state.level, original);
        assert_eq!(ui.pan[0], 0);
        ui.key(KeyCode::Escape, None, false);
        ui.pointer(4., 40., true, false);
        ui.pointer(4., 40., false, false);
        assert_eq!(ui.state.level.get_tile(0, 11), Some('1'));
        ui.key(KeyCode::Escape, None, false);
        assert!(matches!(
            ui.key(KeyCode::KeyN, None, false),
            EditorCommand::Back
        ));
        assert_eq!(
            ui.state.level,
            Level::default(),
            "discarded room must not reappear on editor reopen"
        );
    }

    #[test]
    fn source_new_button_release_confirmation_and_escape_policy_are_preserved() {
        let mut ui = EditorUi::new();
        let original = ui.state.level.clone();
        ui.pointer(290., 15., true, false);
        // Releasing outside cancels the button, without opening a prompt.
        ui.pointer(270., 15., false, false);
        assert!(matches!(
            ui.key(KeyCode::KeyS, None, true),
            EditorCommand::Save { .. }
        ));
        ui.pointer(290., 15., true, false);
        ui.pointer(290., 15., false, false);
        ui.pointer(4., 40., true, false);
        assert_eq!(
            ui.state.level, original,
            "modal must block pointer painting"
        );
        assert!(matches!(
            ui.key(KeyCode::KeyN, None, false),
            EditorCommand::None
        ));
        assert_eq!(ui.state.level, Level::default());
        ui.key(KeyCode::KeyF, Some("fresh"), false);
        ui.key(KeyCode::Enter, None, false);
        ui.key(KeyCode::KeyN, None, true);
        // The source Escape NEW branch leaves the editor; EXIT Escape cancels.
        assert!(matches!(
            ui.key(KeyCode::Escape, None, false),
            EditorCommand::Back
        ));
    }

    #[test]
    fn delete_matches_backspace_for_name_metadata_sign_and_link_text() {
        let mut ui = EditorUi::new();
        ui.key(KeyCode::KeyN, None, false);
        ui.key(KeyCode::KeyC, Some("Café🦇"), false);
        ui.key(KeyCode::Delete, None, false);
        ui.key(KeyCode::Backspace, None, false);
        ui.key(KeyCode::Enter, None, false);
        assert_eq!(ui.level_name(), "Caf.lvl");
        ui.key(KeyCode::KeyM, None, false);
        ui.key(KeyCode::KeyA, Some("Ada!"), false);
        ui.key(KeyCode::Delete, None, false);
        ui.key(KeyCode::Enter, None, false);
        assert_eq!(ui.state.level.author, "Ada");
        ui.state.set_tile(0, 11, 'I').unwrap();
        ui.pointer(4., 40., false, false);
        ui.key(KeyCode::KeyS, None, false);
        ui.key(KeyCode::KeyH, Some("hello!"), false);
        ui.key(KeyCode::Delete, None, false);
        ui.key(KeyCode::Enter, None, false);
        assert_eq!(ui.state.level.sign_message(0), "hello");
        ui.state.set_tile(0, 11, 'X').unwrap();
        ui.key(KeyCode::KeyL, None, false);
        ui.key(KeyCode::KeyN, Some("next.lvl!"), false);
        ui.key(KeyCode::Delete, None, false);
        ui.key(KeyCode::Enter, None, false);
        assert_eq!(ui.state.level.exit_target(0), "next.lvl");
        ui.key(KeyCode::KeyN, None, false);
        ui.key(KeyCode::KeyA, None, true);
        ui.key(KeyCode::Delete, None, false);
        ui.key(KeyCode::Delete, None, false);
        ui.key(KeyCode::KeyN, Some("new"), false);
        ui.key(KeyCode::Enter, None, false);
        assert_eq!(ui.level_name(), "new.lvl");
    }
}
