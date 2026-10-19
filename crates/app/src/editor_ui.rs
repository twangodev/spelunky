//! Classic tile editor. Coordinates use the renderer's 320x240 logical viewport.
use crate::{
    atlas::Atlas,
    renderer::DrawQuad,
    scene::{rect, text},
};
use spelunky_core::{
    Vec2, catalog,
    editor::{
        BLOCK_PALETTE, ENEMY_PALETTE, EditorPaletteProgress, EditorState, ITEM_PALETTE,
        TRAP_PALETTE,
        preview::{self, PreviewContext},
    },
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
    Play(EditorPlay),
    Back,
}

pub struct EditorPlay {
    pub level: Level,
    pub starting_inventory: [f64; 3],
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Confirmation {
    Exit,
    New,
}

#[derive(Clone, Copy)]
enum PaletteGroup {
    Blocks,
    Enemies,
    Traps,
    Items,
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
    Sign(u64),
    Link(u64),
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
    palette_progress: EditorPaletteProgress,
    name: String,
    pan: [f32; 2],
    cursor: Option<(usize, usize)>,
    stroke: Option<(usize, usize, bool)>,
    entry: Option<Entry>,
    metadata_index: usize,
    confirmation: Option<Confirmation>,
    new_pressed: bool,
    pointer_left: bool,
    status: String,
    play_snapshot: Option<(Level, [String; 3])>,
    starting_inventory: [String; 3],
    pointer_sample: Option<(f32, f32, bool, bool)>,
}

impl Default for EditorUi {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorUi {
    pub fn new() -> Self {
        Self::with_context(PreviewContext::default())
    }

    pub fn with_context(context: PreviewContext) -> Self {
        Self::with_source_rng(context, spelunky_core::Rng::new_canonical(0))
    }
    pub fn with_source_rng(context: PreviewContext, rng: spelunky_core::Rng) -> Self {
        Self {
            state: EditorState::with_source_rng(Level::default(), context, rng),
            palette_progress: EditorPaletteProgress::all_unlocked(),
            name: "custom.lvl".into(),
            pan: [0.0, 0.0],
            cursor: None,
            stroke: None,
            entry: None,
            metadata_index: 0,
            confirmation: None,
            new_pressed: false,
            pointer_left: false,
            status: "PAINT: LEFT   ERASE: RIGHT   ARROWS: PAN".into(),
            play_snapshot: None,
            starting_inventory: ["4".into(), "4".into(), "4".into()],
            pointer_sample: None,
        }
    }

    pub fn load(&mut self, name: &str, source: &str) -> Result<(), String> {
        let level = Level::parse_for_editor(source)?;
        let starting_inventory = raw_starting_inventory(source)?;
        let name = file_name(name)?;
        let mut state = EditorState::with_source_rng(
            level,
            self.state.previews.context,
            self.state.previews.rng.clone(),
        );
        state.set_palette_progress(self.palette_progress);
        self.state = state;
        self.name = name;
        self.starting_inventory = starting_inventory;
        self.entry = None;
        self.confirmation = None;
        self.new_pressed = false;
        self.pointer_left = false;
        self.cursor = None;
        self.stroke = None;
        self.pan = [0.0, 0.0];
        self.play_snapshot = None;
        self.status = format!("LOADED {}", self.name);
        Ok(())
    }

    /// Load a selector/catalog key. Keys have already had the file's final
    /// `.lvl` removed, so appending the extension preserves names such as
    /// `room.lvl` that refer to a native `room.lvl.lvl` file.
    pub fn load_catalog_key(&mut self, key: &str, source: &str) -> Result<(), String> {
        self.load(&format!("{key}.lvl"), source)
    }

    pub fn message(&mut self, value: String) {
        self.status = value;
    }

    pub fn set_source_context(&mut self, context: PreviewContext) {
        self.state.previews.context = context;
    }
    pub fn set_source_rng(&mut self, rng: spelunky_core::Rng) {
        self.state.previews.rng = rng;
    }
    pub fn take_source_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.state.previews.sounds)
    }
    pub fn source_check_water(&self) -> bool {
        self.state.previews.check_water
    }
    pub fn source_error(&self) -> Option<preview::PreviewSourceError> {
        self.state.previews.source_error
    }
    pub fn source_rng(&self) -> &spelunky_core::Rng {
        &self.state.previews.rng
    }

    pub fn set_palette_progress(&mut self, progress: EditorPaletteProgress) {
        self.palette_progress = progress;
        self.state.set_palette_progress(progress);
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
        self.pointer_sample = None;
    }

    pub fn key(&mut self, key: KeyCode, typed: Option<&str>, control: bool) -> EditorCommand {
        if let Some(confirmation) = self.confirmation {
            return match key {
                KeyCode::KeyY => EditorCommand::Save {
                    name: self.name.clone(),
                    text: self.document_text(),
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
                    let entry = self.entry.take().unwrap();
                    if matches!(
                        entry.field,
                        Field::Name
                            | Field::Author
                            | Field::Music
                            | Field::Life
                            | Field::Bombs
                            | Field::Ropes
                            | Field::Next
                    ) {
                        return EditorCommand::Back;
                    }
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
                            let max_chars = match entry.field {
                                Field::Sign(_) | Field::Link(_) => 38,
                                _ => 1024,
                            };
                            if entry.value.chars().count() < max_chars {
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
                        text: self.document_text(),
                    };
                }
                KeyCode::KeyE => {
                    return EditorCommand::Export {
                        name: self.name.clone(),
                        text: self.document_text(),
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
                KeyCode::F5 => return self.playtest(),
                KeyCode::ArrowLeft => self.pan[0] = (self.pan[0] - 1.0).max(0.0),
                KeyCode::ArrowRight => {
                    self.pan[0] = (self.pan[0] + 1.0).min((LEVEL_WIDTH - COLS) as f32)
                }
                KeyCode::ArrowUp => self.pan[1] = (self.pan[1] - 1.0).max(0.0),
                KeyCode::ArrowDown => {
                    self.pan[1] = (self.pan[1] + 1.0).min((LEVEL_HEIGHT - ROWS) as f32)
                }
                KeyCode::KeyM => {
                    self.metadata_index = 0;
                    self.metadata();
                }
                KeyCode::KeyN => self.begin(Field::Name, self.name.clone()),
                KeyCode::KeyE => self.record_at_cursor(None),
                KeyCode::Digit1 => self.select_palette_group(PaletteGroup::Blocks),
                KeyCode::Digit2 => self.select_palette_group(PaletteGroup::Enemies),
                KeyCode::Digit3 => self.select_palette_group(PaletteGroup::Traps),
                KeyCode::Digit4 => self.select_palette_group(PaletteGroup::Items),
                KeyCode::KeyA => self.cycle_palette(-1),
                KeyCode::KeyS => self.cycle_palette(1),
                KeyCode::KeyL => self.record(true),
                KeyCode::PageUp | KeyCode::PageDown => {
                    self.cycle_palette(if key == KeyCode::PageUp { -1 } else { 1 });
                }
                _ => {}
            }
        }
        EditorCommand::None
    }

    /// Classic's mouse wheel steps the selected tile just like its A/S
    /// shortcuts within the selected category. Positive values follow
    /// wheel-up/PageDown; negative values follow wheel-down/PageUp.
    pub fn wheel(&mut self, vertical: f32) {
        if self.entry.is_some() || self.confirmation.is_some() || !vertical.is_finite() {
            return;
        }
        if vertical > 0.0 {
            self.cycle_palette(1);
        } else if vertical < 0.0 {
            self.cycle_palette(-1);
        }
    }

    fn cycle_palette(&mut self, delta: isize) {
        let group = self.selected_palette_group();
        let tiles = self.palette_group_tiles(group);
        let len = tiles.len();
        if len == 0 {
            return;
        }
        let index = tiles
            .iter()
            .position(|&tile| tile == self.state.selected_tile)
            .unwrap_or(0);
        self.state.selected_tile =
            tiles[(index as isize + delta).rem_euclid(len as isize) as usize];
    }

    fn selected_palette_group(&self) -> PaletteGroup {
        let selected = self.state.selected_tile;
        if BLOCK_PALETTE.contains(selected) {
            PaletteGroup::Blocks
        } else if ENEMY_PALETTE.contains(selected) {
            PaletteGroup::Enemies
        } else if TRAP_PALETTE.contains(selected) {
            PaletteGroup::Traps
        } else if ITEM_PALETTE.contains(selected) {
            PaletteGroup::Items
        } else {
            PaletteGroup::Blocks
        }
    }

    fn palette_group_tiles(&self, group: PaletteGroup) -> Vec<char> {
        let source = match group {
            PaletteGroup::Blocks => BLOCK_PALETTE,
            PaletteGroup::Enemies => ENEMY_PALETTE,
            PaletteGroup::Traps => TRAP_PALETTE,
            PaletteGroup::Items => ITEM_PALETTE,
        };
        self.state
            .palette
            .iter()
            .copied()
            .filter(|tile| source.contains(*tile))
            .collect()
    }

    fn select_palette_group(&mut self, group: PaletteGroup) {
        if let Some(tile) = self.palette_group_tiles(group).first().copied() {
            self.state.selected_tile = tile;
        }
    }

    fn playtest(&mut self) -> EditorCommand {
        let starting_inventory = match self.starting_inventory_values() {
            Ok(values) => values,
            Err(error) => {
                self.status = error;
                return EditorCommand::None;
            }
        };
        let document = self.state.document();
        self.play_snapshot = Some((document.clone(), self.starting_inventory.clone()));
        EditorCommand::Play(EditorPlay {
            level: document,
            starting_inventory,
        })
    }

    fn document_text(&self) -> String {
        let mut lines: Vec<String> = self
            .state
            .document()
            .to_text()
            .split('\n')
            .map(str::to_owned)
            .collect();
        for (index, value) in self.starting_inventory.iter().enumerate() {
            lines[34 + index] = value.clone();
        }
        lines.join("\n")
    }

    fn starting_inventory_values(&self) -> Result<[f64; 3], String> {
        Ok([
            spelunky_core::level::source_real(&self.starting_inventory[0])?,
            spelunky_core::level::source_real(&self.starting_inventory[1])?,
            spelunky_core::level::source_real(&self.starting_inventory[2])?,
        ])
    }

    pub fn can_playtest_action(&self) -> bool {
        self.entry.is_none() && self.confirmation.is_none()
    }

    fn confirm(&mut self, confirmation: Confirmation) {
        self.cancel_pointer();
        self.confirmation = Some(confirmation);
        self.state.previews.room_instance_destroyed();
        self.status.clear();
    }

    fn restart_document(&mut self) {
        let context = self.state.previews.context;
        let rng = self.state.previews.rng.clone();
        let palette_progress = self.palette_progress;
        *self = Self::with_source_rng(context, rng);
        self.set_palette_progress(palette_progress);
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
        self.state.previews.room_instance_destroyed();
        self.entry = Some(Entry {
            field,
            value,
            // Classic appends typed characters and removes one at a time in
            // existing metadata, sign, and door strings. The host filename
            // prompt keeps its select-on-first-input convenience.
            replace: matches!(field, Field::Name),
        });
    }
    fn metadata(&mut self) {
        let level = &self.state.level;
        let (field, value) = match self.metadata_index {
            0 => (Field::Author, level.author.clone()),
            1 => (Field::Music, level.music.clone()),
            2 => (Field::Life, self.starting_inventory[0].clone()),
            3 => (Field::Bombs, self.starting_inventory[1].clone()),
            4 => (Field::Ropes, self.starting_inventory[2].clone()),
            _ => (Field::Next, level.next_level.clone()),
        };
        self.begin(field, value);
    }
    fn record(&mut self, door: bool) {
        self.record_at_cursor(Some(door));
    }
    fn record_at_cursor(&mut self, door: Option<bool>) {
        let Some((x, y)) = self.cursor else {
            self.status = "POINT AT A SIGN OR DOOR FIRST".into();
            return;
        };
        // E in the original selects the first precise preview, then checks its
        // type. It must not search behind an overlapping unrelated object.
        let point = Vec2::new(24.0 + x as f64 * 16.0, 28.0 + y as f64 * 16.0);
        let Some(instance) = self
            .state
            .previews
            .at_point(point, "oDrawnSprite")
            .and_then(|id| self.state.previews.instance(id))
        else {
            return;
        };
        let field = if instance.is_door() && door != Some(false) {
            Field::Link(instance.id)
        } else if instance.is_sign() && door != Some(true) {
            Field::Sign(instance.id)
        } else {
            self.status = "POINT AT A SIGN OR DOOR FIRST".into();
            return;
        };
        self.begin(field, instance.text.clone());
    }
    /// The authored editor room is recreated from test.tmp on actual return.
    /// Focus changes only cancel input and must not recreate preview objects.
    pub fn return_from_playtest(&mut self) {
        if let Some((document, starting_inventory)) = self.play_snapshot.take() {
            let mut state = EditorState::with_source_rng(
                document,
                PreviewContext::default(),
                self.state.previews.rng.clone(),
            );
            state.set_palette_progress(self.palette_progress);
            self.state = state;
            self.starting_inventory = starting_inventory;
            self.entry = None;
            self.confirmation = None;
            self.cursor = None;
            self.cancel_pointer();
        }
    }
    fn commit_entry(&mut self) {
        let Some(entry) = self.entry.take() else {
            return;
        };
        let mut next = self.state.level.clone();
        let result: Result<(), String> = match entry.field {
            Field::Name => file_name(&entry.value).map(|name| self.name = name),
            Field::Sign(id) | Field::Link(id) => {
                self.state.set_preview_text(id, entry.value.clone())
            }
            field => {
                let result = match field {
                    Field::Author => {
                        if entry.value.is_empty() {
                            Err("AUTHOR CANNOT BE EMPTY".to_owned())
                        } else {
                            next.author = entry.value.clone();
                            Ok(())
                        }
                    }
                    Field::Music => {
                        if entry.value.is_empty() {
                            Err("MUSIC CANNOT BE EMPTY".to_owned())
                        } else {
                            next.music = entry.value.to_ascii_uppercase();
                            Ok(())
                        }
                    }
                    Field::Next => {
                        if entry.value.is_empty() {
                            Err("NEXT LEVEL CANNOT BE EMPTY".to_owned())
                        } else {
                            next.next_level = entry.value.clone();
                            Ok(())
                        }
                    }
                    Field::Life | Field::Bombs | Field::Ropes => {
                        if entry.value.is_empty() {
                            Err("STARTING VALUE CANNOT BE EMPTY".to_owned())
                        } else {
                            // Classic accepts nonempty text here and keeps it
                            // raw. F5 later calls real() directly; normal level
                            // loading also floors and clamps the result.
                            let value = spelunky_core::level::source_real(&entry.value).ok();
                            match field {
                                Field::Life => {
                                    self.starting_inventory[0] = entry.value.clone();
                                    if let Some(value) = value {
                                        next.starting_life = value.floor().clamp(1.0, 99.0) as i32;
                                    }
                                }
                                Field::Bombs => {
                                    self.starting_inventory[1] = entry.value.clone();
                                    if let Some(value) = value {
                                        next.starting_bombs = value.floor().clamp(0.0, 99.0) as i32;
                                    }
                                }
                                _ => {
                                    self.starting_inventory[2] = entry.value.clone();
                                    if let Some(value) = value {
                                        next.starting_ropes = value.floor().clamp(0.0, 99.0) as i32;
                                    }
                                }
                            }
                            Ok(())
                        }
                    }
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

    /// Sampled source Step plus cleanup/Draw-order boundary. CPU interactions
    /// call this directly; the host stages OS events and calls source_tick at30Hz.
    pub fn pointer(&mut self, x: f32, y: f32, left: bool, right: bool) -> EditorCommand {
        self.pan_to_pointer_edge(x, y);
        self.state.previews.step_effects(Vec2::new(
            16.0 + f64::from(self.pan[0]) * 16.0,
            16.0 + f64::from(self.pan[1]) * 16.0,
        ));
        let command = self.pointer_step(x, y, left, right);
        self.state.previews.finish_source_frame();
        command
    }
    pub fn stage_pointer(&mut self, x: f32, y: f32, left: bool, right: bool) -> EditorCommand {
        self.pointer_sample = Some((x, y, left, right));
        // Modern palette/New chrome retains edge-driven interaction; map edits
        // belong exclusively to the source fixed Step, including stationary holds.
        if !(0.0..240.0).contains(&x) || !(MAP_Y..204.0).contains(&y) {
            self.pointer_step(x, y, left, right)
        } else {
            EditorCommand::None
        }
    }
    pub fn source_tick(&mut self) -> EditorCommand {
        if let Some((x, y, _, _)) = self.pointer_sample {
            self.pan_to_pointer_edge(x, y);
        }
        self.state.previews.step_effects(Vec2::new(
            16.0 + f64::from(self.pan[0]) * 16.0,
            16.0 + f64::from(self.pan[1]) * 16.0,
        ));
        let command = self
            .pointer_sample
            .map_or(EditorCommand::None, |(x, y, left, right)| {
                self.pointer_step(x, y, left, right)
            });
        self.state.previews.finish_source_frame();
        if self.source_error() == Some(preview::PreviewSourceError::MissingGameClock) {
            self.status = "SOURCE ERROR: oGame.time IS UNAVAILABLE".into();
        }
        command
    }
    fn pan_to_pointer_edge(&mut self, x: f32, y: f32) {
        if self.confirmation.is_some()
            || self.entry.is_some()
            || !x.is_finite()
            || !y.is_finite()
            || !(0.0..COLS as f32 * CELL).contains(&x)
            || !(MAP_Y..MAP_Y + ROWS as f32 * CELL).contains(&y)
        {
            return;
        }
        // Classic moves the room camera four source pixels per 30 Hz editor
        // step when the cursor enters the outer 16-pixel band. The editor
        // preview is half size, so that is a quarter tile.
        const CAMERA_STEP_TILES: f32 = 0.25;
        if x > COLS as f32 * CELL - CELL {
            self.pan[0] = (self.pan[0] + CAMERA_STEP_TILES).min((LEVEL_WIDTH - COLS) as f32);
        } else if x < CELL {
            self.pan[0] = (self.pan[0] - CAMERA_STEP_TILES).max(0.0);
        }
        if y > MAP_Y + ROWS as f32 * CELL - CELL {
            self.pan[1] = (self.pan[1] + CAMERA_STEP_TILES).min((LEVEL_HEIGHT - ROWS) as f32);
        } else if y < MAP_Y + CELL {
            self.pan[1] = (self.pan[1] - CAMERA_STEP_TILES).max(0.0);
        }
    }
    fn pointer_step(&mut self, x: f32, y: f32, left: bool, right: bool) -> EditorCommand {
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
                (x / CELL + self.pan[0]).floor() as usize,
                ((y - MAP_Y) / CELL + self.pan[1]).floor() as usize,
            );
            self.cursor = Some(cell);
            let result = if left || right {
                let erase = right && !left;
                self.stroke = Some((cell.0, cell.1, erase));
                // The source Step edits the sampled cursor cell, never invented
                // intermediate cells when the pointer jumps across the room.
                if erase {
                    self.state.erase(cell.0, cell.1)
                } else {
                    self.state.paint(cell.0, cell.1)
                }
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
            "ACTION/F5 PLAY  1-4 GROUP  A/S TILE  E TEXT  L LINK",
            3.0,
            21.0,
            0.5,
            WHITE,
        );
        text(quads, atlas, "CTRL N NEW", 224.0, 13.0, 0.5, WHITE);
        rect(quads, [280.0, 10.0, 37.0, 13.0], [0.15, 0.15, 0.16, 1.0]);
        text(quads, atlas, "NEW", 289.0, 13.0, 0.75, GOLD);
        let fractional_x = self.pan[0].fract() * CELL;
        let fractional_y = self.pan[1].fract() * CELL;
        for sy in 0..=ROWS {
            for sx in 0..=COLS {
                let x = sx as f32 * CELL - fractional_x;
                let y = MAP_Y + sy as f32 * CELL - fractional_y;
                map_rect(
                    quads,
                    [x, y, CELL - 0.25, CELL - 0.25],
                    [0.12, 0.11, 0.10, 1.0],
                );
            }
        }
        // Preview Draw uses native sprite size/origin and source depth order.
        // The modern viewport is half-size; artwork keeps its original footprint.
        for instance in self.state.previews.draw_instances() {
            draw_map_sprite(
                quads,
                atlas,
                instance.sprite,
                instance.animation(),
                ((instance.pos.x - 16.0) / 16.0 - f64::from(self.pan[0])) as f32 * CELL,
                MAP_Y + ((instance.pos.y - 16.0) / 16.0 - f64::from(self.pan[1])) as f32 * CELL,
            );
            if !instance.text.is_empty() {
                let start = quads.len();
                text(
                    quads,
                    atlas,
                    &instance.text,
                    ((instance.pos.x - 16.0) / 16.0 - f64::from(self.pan[0])) as f32 * CELL,
                    MAP_Y + ((instance.pos.y - 16.0) / 16.0 - f64::from(self.pan[1])) as f32 * CELL,
                    CELL / 16.0,
                    WHITE,
                );
                let labels: Vec<_> = quads.drain(start..).collect();
                quads.extend(labels.into_iter().filter_map(clip_map_quad));
            }
        }
        if self.entry.is_none() && self.confirmation.is_none() {
            if let Some((x, y)) = self.cursor {
                let world_x = x as f32;
                let world_y = y as f32;
                if world_x + 1.0 > self.pan[0]
                    && world_x < self.pan[0] + COLS as f32
                    && world_y + 1.0 > self.pan[1]
                    && world_y < self.pan[1] + ROWS as f32
                {
                    let x = (world_x - self.pan[0]) * CELL;
                    let y = MAP_Y + (world_y - self.pan[1]) * CELL;
                    if let Some(sprite) = editor_icon(self.state.selected_tile) {
                        draw_map_sprite(quads, atlas, sprite, 0.0, x, y);
                    }
                    map_rect(quads, [x, y, CELL, 0.5], GOLD);
                    map_rect(quads, [x, y, 0.5, CELL], GOLD);
                    map_rect(quads, [x, y + CELL - 0.5, CELL, 0.5], GOLD);
                    map_rect(quads, [x + CELL - 0.5, y, 0.5, CELL], GOLD);
                }
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
            &format!("{}  PAN {:.2},{:.2}", label, self.pan[0], self.pan[1]),
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
                let object = if tile == '@' { "oEntrance" } else { "oExit" };
                if !self
                    .state
                    .previews
                    .draw_instances()
                    .any(|instance| instance.object == object)
                {
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

fn raw_starting_inventory(source: &str) -> Result<[String; 3], String> {
    let lines: Vec<&str> = source
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    let values: Vec<String> = lines
        .get(34..37)
        .ok_or_else(|| "Expected starting inventory metadata".to_owned())?
        .iter()
        .map(|value| (*value).to_owned())
        .collect();
    values
        .try_into()
        .map_err(|_| "Expected three starting inventory values".to_owned())
}

fn file_name(name: &str) -> Result<String, String> {
    let name = name.rsplit(['/', '\\']).next().unwrap_or("").trim();
    let stem = if name.to_ascii_lowercase().ends_with(".lvl") {
        &name[..name.len() - 4]
    } else {
        name
    };
    if stem.is_empty() || name == "." || name == ".." || name.chars().any(char::is_control) {
        return Err("ENTER A VALID FILE NAME".into());
    }
    Ok(if name.to_ascii_lowercase().ends_with(".lvl") {
        name.into()
    } else {
        format!("{name}.lvl")
    })
}

// Both tables are authored in the core preview model, independently of gameplay.
fn editor_icon(tile: char) -> Option<&'static str> {
    preview::cursor_sprite(tile)
}
fn editor_preview_icon(tile: char) -> Option<&'static str> {
    preview::preview_sprite(tile)
}

fn draw_map_sprite(
    quads: &mut Vec<DrawQuad>,
    atlas: &Atlas,
    sprite: &str,
    animation: f64,
    x: f32,
    y: f32,
) {
    let Some(frame) = atlas.sprite(sprite, animation) else {
        return;
    };
    let scale = CELL / 16.0;
    let original = [
        x - frame.origin[0] * scale,
        y - frame.origin[1] * scale,
        frame.size[0] * scale,
        frame.size[1] * scale,
    ];
    if let Some(quad) = clip_map_quad(DrawQuad {
        rect: original,
        uv: frame.uv,
        color: WHITE,
        ..DrawQuad::default()
    }) {
        quads.push(quad);
    }
}
fn map_rect(quads: &mut Vec<DrawQuad>, rect: [f32; 4], color: [f32; 4]) {
    if let Some(quad) = clip_map_quad(DrawQuad {
        rect,
        uv: [0.0; 4],
        color,
        ..DrawQuad::default()
    }) {
        quads.push(quad);
    }
}
fn clip_map_quad(mut quad: DrawQuad) -> Option<DrawQuad> {
    let original = quad.rect;
    let left = original[0].max(0.0);
    let top = original[1].max(MAP_Y);
    let right = (original[0] + original[2]).min(COLS as f32 * CELL);
    let bottom = (original[1] + original[3]).min(MAP_Y + ROWS as f32 * CELL);
    if left >= right || top >= bottom {
        return None;
    }
    quad.uv = [
        quad.uv[0] + (left - original[0]) / original[2] * quad.uv[2],
        quad.uv[1] + (top - original[1]) / original[3] * quad.uv[3],
        (right - left) / original[2] * quad.uv[2],
        (bottom - top) / original[3] * quad.uv[3],
    ];
    quad.rect = [left, top, right - left, bottom - top];
    Some(quad)
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

    fn editor_with_block_selected() -> EditorUi {
        let mut ui = EditorUi::new();
        // Keep pointer-cell tests on a deliberately scrolled source view.
        ui.pan[1] = 11.0;
        ui.state.select('1').unwrap();
        ui
    }

    #[test]
    fn editor_opens_at_classic_origin_and_edge_pans_quarter_tile_per_source_step() {
        let mut ui = EditorUi::new();
        assert_eq!(ui.pan, [0.0, 0.0]);
        ui.stage_pointer(236.0, 200.0, true, false);
        ui.source_tick();
        assert_eq!(ui.pan, [0.25, 0.25]);
        assert_eq!(ui.state.level.get_tile(29, 20), Some('@'));

        ui.source_tick();
        assert_eq!(ui.pan, [0.5, 0.5]);
        assert_eq!(ui.state.level.get_tile(30, 21), Some('@'));

        ui.stage_pointer(4.0, 40.0, false, false);
        ui.source_tick();
        ui.source_tick();
        assert_eq!(ui.pan, [0.0, 0.0], "top and left camera edges clamp");
    }

    #[test]
    fn simultaneous_mouse_buttons_paint_before_right_only_erase() {
        let mut ui = editor_with_block_selected();
        ui.pointer(4.0, 48.0, true, true);
        assert_eq!(ui.state.level.get_tile(0, 12), Some('1'));
        ui.pointer(20.0, 48.0, true, true);
        assert_eq!(ui.state.level.get_tile(0, 12), Some('1'));
        assert_eq!(ui.state.level.get_tile(1, 12), Some('0'));
        assert_eq!(ui.state.level.get_tile(2, 12), Some('1'));
        // Switching to right-only starts a new erasing stroke at that cell.
        ui.pointer(28.0, 48.0, false, true);
        assert_eq!(ui.state.level.get_tile(2, 12), Some('1'));
        assert_eq!(ui.state.level.get_tile(3, 12), Some('0'));
        ui.pointer(20.0, 48.0, false, true);
        assert_eq!(ui.state.level.get_tile(2, 12), Some('0'));
        assert_eq!(ui.state.level.get_tile(1, 12), Some('0'));
    }

    #[test]
    fn mouse_wheel_cycles_the_palette_in_classic_order_and_wraps() {
        let mut ui = EditorUi::new();
        assert_eq!(ui.state.selected_tile, '@');
        ui.wheel(1.0);
        assert_eq!(ui.state.selected_tile, 'X');
        ui.wheel(-1.0);
        assert_eq!(ui.state.selected_tile, '@');
        ui.wheel(f32::NEG_INFINITY);
        assert_eq!(ui.state.selected_tile, '@');

        ui.state.select('0').unwrap();
        ui.wheel(-1.0);
        assert_eq!(ui.state.selected_tile, 'l');
        ui.wheel(1.0);
        assert_eq!(ui.state.selected_tile, '@');
    }

    #[test]
    fn source_palette_keys_select_categories_and_cycle_only_that_category() {
        let mut ui = EditorUi::new();
        ui.key(KeyCode::Digit2, None, false);
        assert_eq!(ui.state.selected_tile, 'b');
        ui.key(KeyCode::KeyA, None, false);
        assert_eq!(ui.state.selected_tile, 'T');
        ui.key(KeyCode::KeyS, None, false);
        assert_eq!(ui.state.selected_tile, 'b');

        ui.key(KeyCode::Digit3, None, false);
        assert_eq!(ui.state.selected_tile, '^');
        ui.key(KeyCode::Digit4, None, false);
        assert_eq!(ui.state.selected_tile, '$');
        ui.key(KeyCode::Digit1, None, false);
        assert_eq!(ui.state.selected_tile, '@');
    }

    #[test]
    fn source_playtest_shortcut_is_f5_and_enter_remains_an_editing_key() {
        let mut ui = EditorUi::new();
        assert!(ui.can_playtest_action());
        assert!(matches!(
            ui.key(KeyCode::F5, None, false),
            EditorCommand::Play(_)
        ));
        assert!(matches!(
            ui.key(KeyCode::Enter, None, false),
            EditorCommand::None
        ));
        assert!(matches!(
            ui.key(KeyCode::NumpadEnter, None, false),
            EditorCommand::None
        ));
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
                    KeyCode::KeyE
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
        authored.set_tile(0, 0, '@').unwrap();
        authored.set_tile(1, 0, 'X').unwrap();
        authored.set_tile(2, 0, 'I').unwrap();
        authored.set_tile(3, 0, 'I').unwrap();
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
            ui.pointer(36.0, 48.0, false, false);
            ui.pointer(36.0, 48.0, true, false);
            ui.pointer(44.0, 48.0, false, false);
            let mut quads = Vec::new();
            ui.draw(&atlas, &mut quads);
            for rect in [
                [
                    32.0 - expected_preview.origin[0] * 0.5,
                    44.0 - expected_preview.origin[1] * 0.5,
                    expected_preview.size[0] * 0.5,
                    expected_preview.size[1] * 0.5,
                ],
                palette,
            ] {
                assert!(
                    quads
                        .iter()
                        .any(|quad| quad.rect == rect && quad.uv == expected_preview.uv),
                    "source preview for {tile} must be {preview} at {rect:?}"
                );
            }
            assert!(
                quads.iter().any(|quad| quad.rect
                    == [
                        40.0 - expected_cursor.origin[0] * 0.5,
                        44.0 - expected_cursor.origin[1] * 0.5,
                        expected_cursor.size[0] * 0.5,
                        expected_cursor.size[1] * 0.5
                    ]
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
            ui.pointer(36.0, 48.0, true, false);
            ui.pointer(44.0, 48.0, false, false);
            let mut quads = Vec::new();
            ui.draw(&atlas, &mut quads);
            let index = ui
                .state
                .palette
                .iter()
                .position(|&value| value == tile)
                .unwrap();
            for rect in [
                [
                    32.0 - icon.origin[0] * 0.5,
                    44.0 - icon.origin[1] * 0.5,
                    icon.size[0] * 0.5,
                    icon.size[1] * 0.5,
                ],
                [
                    40.0 - icon.origin[0] * 0.5,
                    44.0 - icon.origin[1] * 0.5,
                    icon.size[0] * 0.5,
                    icon.size[1] * 0.5,
                ],
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
            let EditorCommand::Play(test) = ui.key(KeyCode::F5, None, false) else {
                panic!("F5 must preserve the edited document");
            };
            let world = test.level.to_world();
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
        let mut ui = editor_with_block_selected();
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
        assert_eq!(ui.state.level.get_tile(0, 12), Some('1'));
        assert_eq!(ui.state.level.get_tile(5, 12), Some('1'));
        for x in 1..5 {
            assert_eq!(ui.state.level.get_tile(x, 12), Some('0'));
        }
    }
    #[test]
    fn interrupted_pointer_stroke_does_not_connect_the_next_click() {
        let mut ui = editor_with_block_selected();
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
        assert!(ui.entry.is_none());
        assert_eq!(ui.state.level.starting_life, 1);
        ui.begin(Field::Bombs, "12.9".into());
        ui.commit_entry();
        assert_eq!(ui.state.level.starting_bombs, 12);
        ui.begin(Field::Ropes, "1000".into());
        ui.commit_entry();
        assert_eq!(ui.state.level.starting_ropes, 99);
        ui.begin(Field::Author, String::new());
        ui.commit_entry();
        assert!(
            ui.entry.is_some(),
            "Classic does not advance on empty metadata"
        );
        ui.entry = None;
        ui.state.set_tile(0, 0, 'I').unwrap();
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
    fn source_inventory_fields_keep_invalid_text_and_report_real_error_on_f5() {
        let mut ui = EditorUi::new();
        for (field, initial, key, typed) in [
            (Field::Life, "4", KeyCode::KeyA, "A"),
            (Field::Bombs, "4", KeyCode::KeyB, "B"),
            (Field::Ropes, "4", KeyCode::KeyC, "C"),
        ] {
            ui.begin(field, initial.into());
            ui.key(KeyCode::Backspace, None, false);
            ui.key(key, Some(typed), false);
            ui.key(KeyCode::Enter, None, false);
        }

        let text = ui.document_text();
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(&lines[34..37], &["A", "B", "C"]);
        assert!(
            Level::parse(&text).is_err(),
            "normal gameplay loading converts and rejects malformed metadata"
        );
        assert!(matches!(
            ui.key(KeyCode::F5, None, false),
            EditorCommand::None
        ));
        assert_eq!(ui.status, "Error in function real().");

        let mut reloaded = EditorUi::new();
        reloaded.load("non-numeric.lvl", &text).unwrap();
        assert_eq!(
            &reloaded.document_text().lines().collect::<Vec<_>>()[34..37],
            &["A", "B", "C"],
            "the editor preserves source text through save/load"
        );
    }

    #[test]
    fn source_empty_inventory_strings_convert_to_zero_for_editor_testplay() {
        let mut lines: Vec<String> = Level::default()
            .to_text()
            .lines()
            .map(str::to_owned)
            .collect();
        lines[34..37].fill(String::new());
        let mut ui = EditorUi::new();
        ui.load("empty-inventory.lvl", &lines.join("\n")).unwrap();

        let EditorCommand::Play(test) = ui.key(KeyCode::F5, None, false) else {
            panic!("empty strings convert to zero in the source real() function");
        };
        assert_eq!(test.starting_inventory, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn editor_testplay_and_save_preserve_raw_inventory_values() {
        let mut ui = EditorUi::new();
        ui.begin(Field::Life, "0.75".into());
        ui.commit_entry();
        ui.begin(Field::Bombs, "12.9".into());
        ui.commit_entry();
        ui.begin(Field::Ropes, "1000.25".into());
        ui.commit_entry();

        assert_eq!(ui.state.level.starting_life, 1);
        assert_eq!(ui.state.level.starting_bombs, 12);
        assert_eq!(ui.state.level.starting_ropes, 99);
        let saved: Vec<_> = ui.document_text().lines().map(str::to_owned).collect();
        assert_eq!(&saved[34..37], &["0.75", "12.9", "1000.25"]);

        let EditorCommand::Play(test) = ui.key(KeyCode::F5, None, false) else {
            panic!("F5 must launch editor test play");
        };
        assert_eq!(test.starting_inventory, [0.75, 12.9, 1000.25]);
        assert_eq!(test.level.starting_life, 1);
        assert_eq!(test.level.starting_bombs, 12);
        assert_eq!(test.level.starting_ropes, 99);

        ui.return_from_playtest();
        assert_eq!(ui.document_text(), saved.join("\n"));
    }
    #[test]
    fn import_failure_preserves_document_and_palette_is_complete() {
        let mut ui = EditorUi::new();
        let before = ui.state.level.clone();
        ui.pan = [7.0, 9.0];
        assert!(ui.load("broken.lvl", "invalid").is_err());
        assert_eq!(ui.state.level, before);
        assert_eq!(ui.pan, [7.0, 9.0], "failed loads preserve the current view");

        ui.load("loaded.lvl", &Level::default().to_text()).unwrap();
        assert_eq!(
            ui.pan,
            [0.0, 0.0],
            "Classic opens a loaded editor room at its source camera origin"
        );
        for tile in ui.state.palette.clone() {
            ui.state.select(tile).unwrap();
        }
        assert_eq!(file_name("../../map").unwrap(), "map.lvl");
        assert!(file_name("..").is_err());
        assert!(file_name(".lvl").is_err());
    }

    #[test]
    fn keyboard_shortcuts_save_export_import_and_edit_links() {
        let mut ui = EditorUi::new();
        ui.state.set_tile(0, 0, 'X').unwrap();
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
        assert_eq!(ui.state.level.author, "ANONYMOUSAda");
        ui.key(KeyCode::KeyJ, Some("jungle"), false);
        ui.key(KeyCode::Enter, None, false);
        assert_eq!(ui.state.level.music, "CAVEJUNGLE");
    }
    #[test]
    fn source_exit_confirmation_blocks_edits_cancels_and_discards_with_ordinary_keys() {
        let mut ui = editor_with_block_selected();
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
        assert_eq!(ui.pan[0], 0.0);
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
        assert_eq!(ui.state.level.author, "ANONYMOUSAda");
        ui.state.set_tile(0, 0, 'I').unwrap();
        ui.pointer(4., 40., false, false);
        ui.key(KeyCode::KeyE, None, false);
        ui.key(KeyCode::KeyH, Some("hello!"), false);
        ui.key(KeyCode::Delete, None, false);
        ui.key(KeyCode::Enter, None, false);
        assert_eq!(ui.state.level.sign_message(0), "hello");
        ui.state.set_tile(0, 0, 'X').unwrap();
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

    #[test]
    fn existing_sign_and_door_text_append_and_erase_one_character() {
        let mut sign = EditorUi::new();
        let mut level = Level::default();
        level.set_tile(0, 0, 'I').unwrap();
        level.signs.push("hello".into());
        sign.load("sign.lvl", &level.to_text()).unwrap();
        sign.pointer(4.0, 40.0, false, false);
        sign.key(KeyCode::KeyE, None, false);
        sign.key(KeyCode::KeyX, Some("!"), false);
        sign.key(KeyCode::Backspace, None, false);
        sign.key(KeyCode::KeyX, Some("?"), false);
        sign.key(KeyCode::Enter, None, false);
        assert_eq!(sign.state.level.sign_message(0), "hello?");

        let mut door = EditorUi::new();
        let mut level = Level::default();
        level.set_tile(0, 0, 'X').unwrap();
        level.exit_targets.push("next".into());
        door.load("door.lvl", &level.to_text()).unwrap();
        door.pointer(4.0, 40.0, false, false);
        door.key(KeyCode::KeyL, None, false);
        door.key(KeyCode::KeyJ, Some(".lvl"), false);
        door.key(KeyCode::Enter, None, false);
        assert_eq!(door.state.level.exit_target(0), "next.lvl");
    }

    #[test]
    fn escape_from_metadata_leaves_for_title_but_text_edit_returns_to_editor() {
        let mut metadata = EditorUi::new();
        metadata.key(KeyCode::KeyM, None, false);
        assert!(matches!(
            metadata.key(KeyCode::Escape, None, false),
            EditorCommand::Back
        ));

        let mut sign = EditorUi::new();
        sign.state.set_tile(0, 0, 'I').unwrap();
        sign.pointer(4.0, 40.0, false, false);
        sign.key(KeyCode::KeyE, None, false);
        assert!(matches!(
            sign.key(KeyCode::Escape, None, false),
            EditorCommand::None
        ));
        assert!(sign.entry.is_none());
    }

    #[test]
    fn sign_and_door_text_match_classics_38_character_limit() {
        for (tile, edit_key) in [('I', KeyCode::KeyE), ('X', KeyCode::KeyL)] {
            let mut ui = EditorUi::new();
            ui.state.select(tile).unwrap();
            ui.pointer(4.0, 40.0, true, false);
            ui.pointer(4.0, 40.0, false, false);
            ui.key(edit_key, None, false);
            assert!(ui.entry.is_some(), "{tile} text entry opens");
            ui.key(KeyCode::KeyA, Some(&"x".repeat(50)), false);
            assert_eq!(ui.entry.as_ref().unwrap().value.chars().count(), 38);
            ui.key(KeyCode::Enter, None, false);
            let text = if tile == 'I' {
                ui.state.level.sign_message(0)
            } else {
                ui.state.level.exit_target(0)
            };
            assert_eq!(text.chars().count(), 38);
        }
    }
    fn choose_palette(ui: &mut EditorUi, tile: char) {
        let index = ui.state.palette.iter().position(|&c| c == tile).unwrap();
        ui.pointer(
            248.0 + (index % 7) as f32 * 10.0,
            40.0 + (index / 7) as f32 * 10.0,
            true,
            false,
        );
        ui.pointer(0.0, 0.0, false, false);
    }
    fn paint_at(ui: &mut EditorUi, tile: char, x: f32, y: f32) {
        choose_palette(ui, tile);
        ui.pointer(x, y, true, false);
        ui.pointer(x, y, false, false);
    }
    fn map_has(quads: &[DrawQuad], uv: [f32; 4]) -> bool {
        quads
            .iter()
            .any(|q| q.uv == uv && q.rect[0] < 240.0 && q.rect[1] >= MAP_Y && q.rect[1] < 204.0)
    }
    #[test]
    fn source_large_previews_and_cursor_keep_native_size_and_origin() {
        let atlas = Atlas::load().unwrap();
        for (tile, sprite, size) in [
            ('{', "sMegaMouthTile", [64.0, 32.0]),
            ('S', "sGiantSpiderHang", [32.0, 16.0]),
            ('Y', "sYetiKingLeft", [32.0, 32.0]),
            ('T', "sTombLordLeft", [32.0, 32.0]),
            ('E', "sAlienBossTile", [32.0, 32.0]),
        ] {
            let mut ui = EditorUi::new();
            paint_at(&mut ui, tile, 36.0, 48.0);
            ui.pointer(84.0, 48.0, false, false);
            let frame = atlas.sprite(sprite, 0.0).unwrap();
            assert_eq!(frame.size, size, "original native source dimensions");
            let mut quads = Vec::new();
            ui.draw(&atlas, &mut quads);
            assert!(
                quads.iter().any(|q| q.uv == frame.uv
                    && q.rect
                        == [
                            32.0 - frame.origin[0] * 0.5,
                            44.0 - frame.origin[1] * 0.5,
                            size[0] * 0.5,
                            size[1] * 0.5
                        ]),
                "{tile} map artwork must retain its multi-cell footprint"
            );
            let cursor = atlas.sprite(editor_icon(tile).unwrap(), 0.0).unwrap();
            assert!(
                quads.iter().any(|q| q.uv == cursor.uv
                    && q.rect
                        == [
                            80.0 - cursor.origin[0] * 0.5,
                            44.0 - cursor.origin[1] * 0.5,
                            cursor.size[0] * 0.5,
                            cursor.size[1] * 0.5
                        ]),
                "{tile} selected cursor must retain native origin/size"
            );
        }
    }
    #[test]
    fn source_off_anchor_erase_removes_preview_without_erasing_saved_anchor() {
        let atlas = Atlas::load().unwrap();
        let mega = atlas.sprite("sMegaMouthTile", 0.0).unwrap();
        let mut ui = EditorUi::new();
        paint_at(&mut ui, '{', 20.0, 48.0);
        let mut quads = Vec::new();
        ui.draw(&atlas, &mut quads);
        ui.pointer(28.0, 48.0, false, true);
        ui.pointer(100.0, 0.0, false, false);
        choose_palette(&mut ui, '0');
        quads.clear();
        ui.draw(&atlas, &mut quads);
        assert_eq!(
            ui.state.level.get_tile(2, 1),
            Some('{'),
            "saved neighbor anchor survives"
        );
        assert_eq!(ui.state.level.get_tile(3, 1), Some('0'));
        assert!(
            !map_has(&quads, mega.uv),
            "right erase destroys the overlapping Mega Mouth preview"
        );
        ui.cancel_pointer();
        quads.clear();
        ui.draw(&atlas, &mut quads);
        assert!(
            !map_has(&quads, mega.uv),
            "focus/input reset must preserve the missing preview"
        );
        let EditorCommand::Play(test) = ui.key(KeyCode::F5, None, false) else {
            panic!("F5");
        };
        assert_eq!(test.level.get_tile(2, 1), Some('{'));
        assert_eq!(
            test.level
                .to_world()
                .entities
                .iter()
                .filter(|e| e.object == "oJaws")
                .count(),
            1,
            "gameplay still loads the saved anchor independently of missing preview"
        );
    }
    #[test]
    fn source_first_rectangle_erase_can_leave_a_live_preview_over_an_empty_cell() {
        let atlas = Atlas::load().unwrap();
        let brick = atlas.sprite("sBrick", 0.0).unwrap();
        let mega = atlas.sprite("sMegaMouthTile", 0.0).unwrap();
        let mut ui = EditorUi::new();
        let mut quads = Vec::new();
        paint_at(&mut ui, '{', 20.0, 48.0);
        ui.draw(&atlas, &mut quads);
        paint_at(&mut ui, '1', 28.0, 48.0);
        quads.clear();
        ui.draw(&atlas, &mut quads);
        assert!(
            map_has(&quads, mega.uv),
            "paint protects off-anchor Mega Mouth"
        );
        ui.pointer(28.0, 48.0, false, true);
        ui.pointer(0.0, 0.0, false, false);
        choose_palette(&mut ui, '0');
        quads.clear();
        ui.draw(&atlas, &mut quads);
        assert_eq!(ui.state.level.get_tile(2, 1), Some('{'));
        assert_eq!(ui.state.level.get_tile(3, 1), Some('0'));
        assert!(!map_has(&quads, mega.uv));
        assert!(
            quads
                .iter()
                .any(|q| q.uv == brick.uv && q.rect == [24.0, 44.0, 8.0, 8.0]),
            "first rectangle hit destroys Mega Mouth, leaving the later brick preview"
        );
        ui.key(KeyCode::KeyZ, None, true);
        quads.clear();
        ui.draw(&atlas, &mut quads);
        assert!(
            map_has(&quads, mega.uv),
            "undo restores live preview room, not just tile grid"
        );
        ui.key(KeyCode::KeyY, None, true);
        quads.clear();
        ui.draw(&atlas, &mut quads);
        assert!(!map_has(&quads, mega.uv));
    }
    #[test]
    fn source_record_selection_observes_family_rebuild_after_button_destruction() {
        let atlas = Atlas::load().unwrap();
        let mut ui = EditorUi::new();
        let mut quads = Vec::new();
        paint_at(&mut ui, '@', 20.0, 48.0);
        ui.draw(&atlas, &mut quads);
        paint_at(&mut ui, 'S', 12.0, 48.0);
        quads.clear();
        ui.draw(&atlas, &mut quads);
        ui.pointer(20.0, 48.0, false, false);
        ui.key(KeyCode::KeyL, None, false);
        assert!(
            ui.entry.is_some(),
            "new instances append: first precise point selects the earlier door"
        );
        ui.key(KeyCode::Escape, None, false);
        ui.pointer(20.0, 48.0, false, false);
        quads.clear();
        ui.draw(&atlas, &mut quads);
        ui.key(KeyCode::KeyL, None, false);
        assert!(
            ui.entry.is_none(),
            "modal button destruction rebuilds query order from depth-sorted room; Spider wins"
        );
    }
    #[test]
    fn source_pointer_jump_does_not_paint_unvisited_cells() {
        let mut ui = editor_with_block_selected();
        ui.pointer(4.0, 48.0, true, false);
        ui.pointer(44.0, 48.0, true, false);
        ui.pointer(44.0, 48.0, false, false);
        assert_eq!(ui.state.level.get_tile(0, 12), Some('1'));
        assert_eq!(ui.state.level.get_tile(5, 12), Some('1'));
        for x in 1..5 {
            assert_eq!(
                ui.state.level.get_tile(x, 12),
                Some('0'),
                "source Step never visits this cell"
            );
        }
    }
    #[test]
    fn source_fresh_editor_is_blank_and_new_discards_old_preview_objects() {
        let mut ui = EditorUi::new();
        assert_eq!(
            ui.state.level,
            Level::default(),
            "source Create fills all cells with zero"
        );
        paint_at(&mut ui, '{', 20.0, 48.0);
        ui.key(KeyCode::KeyN, None, true);
        ui.key(KeyCode::KeyN, None, false);
        ui.key(KeyCode::KeyA, Some("fresh"), false);
        ui.key(KeyCode::Enter, None, false);
        let atlas = Atlas::load().unwrap();
        let mut quads = Vec::new();
        ui.draw(&atlas, &mut quads);
        assert_eq!(ui.state.level, Level::default());
        assert!(!map_has(
            &quads,
            atlas.sprite("sMegaMouthTile", 0.0).unwrap().uv
        ));
    }
    #[test]
    fn source_precise_paint_preserves_a_bow_that_bbox_erase_removes() {
        let atlas = Atlas::load().unwrap();
        let bow = atlas.sprite("sBowTile", 0.0).unwrap();
        let mut ui = EditorUi::new();
        let mut quads = Vec::new();
        paint_at(&mut ui, '}', 36.0, 48.0);
        ui.draw(&atlas, &mut quads);
        // The Bow mask misses both authored +8,+12 and +8,+4 sample points.
        // Selecting eraser and LEFT painting changes levelArray only.
        paint_at(&mut ui, '0', 36.0, 48.0);
        quads.clear();
        ui.draw(&atlas, &mut quads);
        assert_eq!(ui.state.level.get_tile(4, 1), Some('0'));
        assert!(
            map_has(&quads, bow.uv),
            "precise point misses preserve the Bow preview"
        );
        ui.pointer(36.0, 48.0, false, true);
        ui.pointer(0.0, 0.0, false, false);
        quads.clear();
        ui.draw(&atlas, &mut quads);
        assert!(
            !map_has(&quads, bow.uv),
            "right erase uses bbox and destroys the Bow"
        );
    }
    #[test]
    fn source_staged_map_input_waits_for_fixed_ticks_and_stationary_holds_repeat() {
        let atlas = Atlas::load().unwrap();
        let mut ui = editor_with_block_selected();
        ui.stage_pointer(4.0, 48.0, true, false);
        assert_eq!(ui.state.level.get_tile(0, 12), Some('0'));
        let before = ui.state.previews.clone();
        let mut quads = Vec::new();
        for _ in 0..8 {
            ui.draw(&atlas, &mut quads);
        }
        assert_eq!(
            ui.state.previews, before,
            "high-refresh Draw never mutates source state"
        );
        ui.source_tick();
        assert_eq!(ui.state.level.get_tile(0, 12), Some('1'));
        let rng = ui.source_rng().state_bits();
        ui.source_tick();
        assert_ne!(
            ui.source_rng().state_bits(),
            rng,
            "held stationary terrain is freshly created each source Step"
        );
        ui.cancel_pointer();
        let stopped = ui.source_rng().state_bits();
        ui.source_tick();
        assert_eq!(
            ui.source_rng().state_bits(),
            stopped,
            "focus reset ends held paint"
        );
        ui.stage_pointer(12.0, 48.0, true, false);
        ui.stage_pointer(44.0, 48.0, true, false);
        ui.source_tick();
        for x in 1..5 {
            assert_eq!(ui.state.level.get_tile(x, 12), Some('0'));
        }
        assert_eq!(ui.state.level.get_tile(5, 12), Some('1'));
    }
    #[test]
    fn source_editor_ticks_rng_and_live_debris_match_at_30_60_144_240_hz() {
        let mut expected = None;
        for hz in [30, 60, 144, 240] {
            let mut ui = EditorUi::with_source_rng(
                PreviewContext::default(),
                spelunky_core::Rng::new_canonical(239),
            );
            choose_palette(&mut ui, 'B');
            ui.stage_pointer(36.0, 48.0, true, false);
            let mut clock = crate::timing::FixedClock::new();
            let mut ticks = 0;
            for _ in 0..hz {
                for _ in 0..clock.advance(1.0 / hz as f64).ticks {
                    ui.source_tick();
                    ticks += 1;
                }
            }
            assert_eq!(ticks, 30);
            assert_ne!(
                ui.source_rng().state_bits(),
                239,
                "real block Destroy consumes source RNG"
            );
            if let Some(expected) = &expected {
                assert_eq!(&ui.state.previews, expected, "source state at{hz}Hz");
            } else {
                expected = Some(ui.state.previews.clone());
            }
        }
    }
    #[test]
    fn source_inherited_support_destroy_removes_spike_preview_but_preserves_saved_spike() {
        let atlas = Atlas::load().unwrap();
        let spike = atlas.sprite("sSpikes", 0.0).unwrap();
        for (support, inherited) in [
            ('1', true),
            ('2', true),
            ('3', true),
            ('4', true),
            ('B', false),
        ] {
            let mut ui = EditorUi::new();
            paint_at(&mut ui, support, 36.0, 56.0);
            paint_at(&mut ui, '^', 36.0, 48.0);
            choose_palette(&mut ui, '0');
            ui.pointer(36.0, 56.0, false, true);
            ui.pointer(0.0, 0.0, false, false);
            let mut quads = Vec::new();
            ui.draw(&atlas, &mut quads);
            assert_eq!(ui.state.level.get_tile(4, 1), Some('^'));
            assert_eq!(ui.state.level.get_tile(4, 2), Some('0'));
            assert_eq!(
                map_has(&quads, spike.uv),
                !inherited,
                "resolved{support} Destroy inheritance"
            );
            assert_eq!(ui.source_check_water(), inherited);
            let EditorCommand::Play(test) = ui.key(KeyCode::F5, None, false) else {
                panic!("F5");
            };
            assert!(
                test.level
                    .to_world()
                    .entities
                    .iter()
                    .any(|e| e.object == "oSpikes")
                    || test
                        .level
                        .to_world()
                        .tiles
                        .iter()
                        .any(|t| t.object == "oSpikes")
            );
        }
    }
    #[test]
    fn source_actual_play_snapshot_return_rebuilds_previews_but_focus_does_not() {
        let atlas = Atlas::load().unwrap();
        let mega = atlas.sprite("sMegaMouthTile", 0.0).unwrap();
        let mut ui = EditorUi::new();
        paint_at(&mut ui, '{', 20.0, 48.0);
        choose_palette(&mut ui, '0');
        ui.pointer(28.0, 48.0, false, true);
        ui.pointer(0.0, 0.0, false, false);
        let mut quads = Vec::new();
        ui.draw(&atlas, &mut quads);
        assert!(!map_has(&quads, mega.uv));
        ui.cancel_pointer();
        ui.source_tick();
        quads.clear();
        ui.draw(&atlas, &mut quads);
        assert!(!map_has(&quads, mega.uv));
        let EditorCommand::Play(test) = ui.key(KeyCode::F5, None, false) else {
            panic!("F5");
        };
        assert_eq!(test.level.get_tile(2, 1), Some('{'));
        ui.return_from_playtest();
        quads.clear();
        ui.draw(&atlas, &mut quads);
        assert!(
            map_has(&quads, mega.uv),
            "source room recreation reloads test.tmp, unlike focus loss"
        );
        assert!(!ui.state.can_undo(), "recreated room has a fresh history");
    }
    #[test]
    fn source_context_changes_preview_block_but_not_cursor_or_editor_tree_leaves() {
        let atlas = Atlas::load().unwrap();
        // An explicit inherited-global boundary; no natural true-city editor route is claimed.
        let mut ui = EditorUi::with_context(PreviewContext {
            city_of_gold: true,
            treasure_vision: false,
        });
        paint_at(&mut ui, 'B', 36.0, 48.0);
        ui.pointer(44.0, 48.0, false, false);
        let mut quads = Vec::new();
        ui.draw(&atlas, &mut quads);
        assert!(
            quads
                .iter()
                .any(|q| q.uv == atlas.sprite("sGoldBlock", 0.0).unwrap().uv
                    && q.rect == [32.0, 44.0, 8.0, 8.0])
        );
        assert!(
            quads
                .iter()
                .any(|q| q.uv == atlas.sprite("sBlock", 0.0).unwrap().uv
                    && q.rect == [40.0, 44.0, 8.0, 8.0])
        );
        paint_at(&mut ui, '|', 52.0, 48.0);
        paint_at(&mut ui, ')', 60.0, 48.0);
        choose_palette(&mut ui, '0');
        quads.clear();
        ui.draw(&atlas, &mut quads);
        assert!(
            quads
                .iter()
                .any(|q| q.uv == atlas.sprite("sLeaves", 0.0).unwrap().uv && q.rect[0] == 56.0)
        );
        assert!(
            !map_has(&quads, atlas.sprite("sLeavesRight", 0.0).unwrap().uv),
            "TreeTile is not real oTree"
        );
        ui.key(KeyCode::KeyN, None, true);
        ui.key(KeyCode::KeyN, None, false);
        assert!(
            ui.state.previews.context.city_of_gold,
            "NEW preserves globals"
        );
        ui.key(KeyCode::KeyA, Some("gold"), false);
        ui.key(KeyCode::Enter, None, false);
        ui.key(KeyCode::F5, None, false);
        ui.return_from_playtest();
        assert!(
            !ui.state.previews.context.city_of_gold,
            "actual test return clears globals"
        );
    }
    #[test]
    fn source_live_door_text_is_drawn_at_object_origin_and_survives_grid_only_erase() {
        let atlas = Atlas::load().unwrap();
        let mut ui = EditorUi::new();
        paint_at(&mut ui, 'S', 12.0, 48.0);
        paint_at(&mut ui, '@', 20.0, 48.0);
        // S is first in the query array, so use the ordinary metadata adapter
        // before arranging the overlap in a loaded fixture.
        let mut authored = Level::default();
        authored.set_tile(2, 1, '@').unwrap();
        authored.exit_targets = vec!["NEXT".into()];
        ui.load("label.lvl", &authored.to_text()).unwrap();
        paint_at(&mut ui, 'S', 12.0, 48.0);
        ui.pointer(20.0, 48.0, false, false);
        ui.key(KeyCode::KeyL, None, false);
        ui.key(KeyCode::Escape, None, false);
        ui.pointer(20.0, 48.0, false, false);
        choose_palette(&mut ui, '0');
        ui.pointer(20.0, 48.0, false, true);
        ui.pointer(0.0, 0.0, false, false);
        assert_eq!(ui.state.level.get_tile(2, 1), Some('0'));
        let mut quads = Vec::new();
        ui.draw(&atlas, &mut quads);
        let glyph = atlas
            .sprite("sFontSmall", ('N' as u32 - 32) as f64)
            .unwrap();
        assert!(
            quads
                .iter()
                .any(|q| q.uv == glyph.uv && q.rect == [16.0, 44.0, 4.0, 4.0]),
            "live target Draw follows surviving preview"
        );
        assert!(
            ui.state
                .previews
                .draw_instances()
                .any(|i| i.object == "oEntrance")
        );
        let EditorCommand::Export { text, .. } = ui.key(KeyCode::KeyE, None, true) else {
            panic!("export");
        };
        assert!(
            Level::parse(&text).unwrap().exit_targets.is_empty(),
            "saved grid0 omits surviving ghost text"
        );
    }
    #[test]
    fn source_gold_clock_error_is_visible_after_natural_editor_destroy_and_steps() {
        let mut ui = EditorUi::with_source_rng(
            PreviewContext {
                city_of_gold: true,
                ..Default::default()
            },
            spelunky_core::Rng::new_canonical(927),
        );
        paint_at(&mut ui, 'B', 36.0, 48.0);
        choose_palette(&mut ui, '0');
        ui.stage_pointer(36.0, 48.0, false, true);
        ui.source_tick();
        ui.cancel_pointer();
        assert_eq!(ui.source_error(), None, "birth frame runs no gold Step");
        ui.source_tick();
        assert_eq!(ui.source_error(), None, "integer launch needs no clock");
        ui.source_tick();
        assert_eq!(
            ui.source_error(),
            Some(preview::PreviewSourceError::MissingGameClock)
        );
        assert_eq!(ui.status, "SOURCE ERROR: oGame.time IS UNAVAILABLE");
        assert_eq!(ui.state.level.get_tile(4, 1), Some('0'));
    }
}
