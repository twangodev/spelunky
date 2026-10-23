//! Settings menu state; the host persists changes and applies platform options.

use spelunky_core::RulesPreset;
use winit::keyboard::KeyCode;

use crate::{atlas::Atlas, renderer::DrawQuad, scene, storage::Settings};

const ACTIONS: [(&str, &str); 14] = [
    ("left", "MOVE LEFT"),
    ("right", "MOVE RIGHT"),
    ("up", "CLIMB UP"),
    ("down", "CLIMB DOWN"),
    ("jump", "JUMP"),
    ("attack", "ATTACK / THROW"),
    ("cycle", "CYCLE ITEM"),
    ("bomb", "BOMB"),
    ("rope", "ROPE"),
    ("flare", "FLARE"),
    ("pay", "PAY"),
    ("run", "RUN"),
    ("exit", "ENTER DOOR"),
    ("pause", "PAUSE"),
];
const CLASSIC_KEY_ORDER: [&str; 12] = [
    "up", "down", "left", "right", "jump", "attack", "cycle", "run", "bomb", "rope", "flare", "pay",
];
const CONTROLLER_INPUT: usize = 8;
const GRAPHICS_DETAIL: usize = CONTROLLER_INPUT + 1;
const FIRST_BINDING: usize = GRAPHICS_DETAIL + 1;
const CONFIGURE_CONTROLLER: usize = FIRST_BINDING + ACTIONS.len();
#[cfg(target_arch = "wasm32")]
const CONFIGURE_DIRECTIONS: usize = CONFIGURE_CONTROLLER + 1;
#[cfg(target_arch = "wasm32")]
const RESTORE: usize = CONFIGURE_DIRECTIONS + 1;
#[cfg(not(target_arch = "wasm32"))]
const RESTORE: usize = CONFIGURE_CONTROLLER + 1;
const BACK: usize = RESTORE + 1;
const ROWS: usize = BACK + 1;
const VISIBLE: usize = 11;
const CAPS: [Option<u32>; 5] = [None, Some(60), Some(120), Some(144), Some(240)];
const WHITE: [f32; 4] = [0.94, 0.95, 0.98, 1.0];
const GOLD: [f32; 4] = [1.0, 0.81, 0.32, 1.0];
const MUTED: [f32; 4] = [0.68, 0.74, 0.82, 1.0];
const ROW_X: f32 = 14.0;
const ROW_WIDTH: f32 = 292.0;
const ROW_TOP: f32 = 39.0;
const ROW_HEIGHT: f32 = 12.0;
const ROW_STEP: f32 = 14.0;
const VOLUME_TRACK_X: f32 = 210.0;
const VOLUME_TRACK_WIDTH: f32 = 86.0;
const CLASSIC_VOLUME_LEVELS: i32 = 18;
const CLASSIC_VOLUME_FLOOR: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCommand {
    None,
    Changed,
    Back,
    Fullscreen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAudioAction {
    PreviewCave,
    StopAllMusic,
}

#[derive(Default)]
pub struct SettingsUi {
    selected: usize,
    binding: Option<usize>,
    controller_binding: Option<usize>,
    controller_direction_binding: Option<usize>,
    unsupported_key: bool,
    volume_drag: Option<usize>,
    audio_action: Option<SettingsAudioAction>,
    effects_preview_focus: bool,
    effects_preview_alarm: Option<u8>,
}

impl SettingsUi {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn awaiting_binding(&self) -> bool {
        self.binding.is_some()
            || self.awaiting_controller_binding()
            || self.awaiting_direction_binding()
    }

    pub fn awaiting_controller_binding(&self) -> bool {
        self.controller_binding.is_some()
    }

    pub fn awaiting_direction_binding(&self) -> bool {
        self.controller_direction_binding.is_some()
    }

    pub fn controller_state_row_selected(&self) -> bool {
        matches!(self.selected, CONTROLLER_INPUT | RESTORE)
    }

    pub fn key_with_runtime_controller(
        &mut self,
        key: KeyCode,
        settings: &mut Settings,
        controller_enabled: bool,
    ) -> SettingsCommand {
        if self.selected == CONTROLLER_INPUT
            && matches!(
                key,
                KeyCode::ArrowLeft
                    | KeyCode::ArrowRight
                    | KeyCode::Enter
                    | KeyCode::NumpadEnter
                    | KeyCode::Space
            )
        {
            // Classic changes the session global shown by the row. Preserve
            // the saved preference until the user actually toggles it.
            settings.controller_enabled = controller_enabled;
        }
        self.key(key, settings)
    }

    /// Activate a clicked row. Source checkboxes and config buttons react to a
    /// mouse press; source sliders continue tracking the pointer while held.
    pub fn pointer_press(&mut self, x: f32, y: f32, settings: &mut Settings) -> SettingsCommand {
        if self.awaiting_binding() {
            return SettingsCommand::None;
        }
        let Some(index) = settings_row_at(self.selected, x, y) else {
            return SettingsCommand::None;
        };
        let slot = index - visible_start(self.selected);
        self.selected = index;
        if matches!(index, 2 | 3) {
            let track_y = ROW_TOP + slot as f32 * ROW_STEP + 9.0;
            if !(VOLUME_TRACK_X..=VOLUME_TRACK_X + VOLUME_TRACK_WIDTH).contains(&x)
                || !(track_y..track_y + 2.0).contains(&y)
            {
                return SettingsCommand::None;
            }
            self.volume_drag = Some(index);
            if index == 2 {
                self.audio_action = Some(SettingsAudioAction::PreviewCave);
            } else if index == 3 {
                // Classic oSlider2 starts its sndJump preview one simulation
                // step after the effects slider is clicked.
                self.effects_preview_focus = true;
                self.effects_preview_alarm = Some(1);
            }
            return self.set_volume_from_pointer(index, x, settings);
        }
        self.adjust(settings, 1, true)
    }

    pub fn pointer_press_with_runtime_controller(
        &mut self,
        x: f32,
        y: f32,
        settings: &mut Settings,
        controller_enabled: bool,
    ) -> SettingsCommand {
        if self.awaiting_binding() {
            return SettingsCommand::None;
        }
        if settings_row_at(self.selected, x, y) == Some(CONTROLLER_INPUT) {
            settings.controller_enabled = controller_enabled;
        }
        self.pointer_press(x, y, settings)
    }

    /// Update a source-style volume slider while the left mouse button is held.
    pub fn pointer_move(
        &mut self,
        x: f32,
        left_down: bool,
        settings: &mut Settings,
    ) -> SettingsCommand {
        let Some(index) = self.volume_drag else {
            return SettingsCommand::None;
        };
        if !left_down {
            self.volume_drag = None;
            if index == 2 {
                self.audio_action = Some(SettingsAudioAction::StopAllMusic);
            }
            return SettingsCommand::None;
        }
        self.set_volume_from_pointer(index, x, settings)
    }

    pub fn cancel_pointer(&mut self) {
        if self.volume_drag == Some(2) {
            self.audio_action = Some(SettingsAudioAction::StopAllMusic);
        }
        self.volume_drag = None;
        self.effects_preview_focus = false;
        self.effects_preview_alarm = None;
    }

    pub fn take_audio_action(&mut self) -> Option<SettingsAudioAction> {
        self.audio_action.take()
    }

    /// Advance the Classic effects-slider preview alarm by one game step.
    /// Alarm events run before the slider Step event, so a release observed
    /// since the preceding tick still permits a due preview before focus clears.
    pub fn tick_effects_preview(&mut self) -> bool {
        let play = match self.effects_preview_alarm {
            Some(remaining) if remaining > 1 => {
                self.effects_preview_alarm = Some(remaining - 1);
                false
            }
            Some(_) if self.effects_preview_focus => {
                self.effects_preview_alarm = Some(20);
                true
            }
            Some(_) => {
                self.effects_preview_alarm = None;
                false
            }
            None => false,
        };
        self.effects_preview_focus = self.volume_drag == Some(3);
        play
    }

    fn set_volume_from_pointer(
        &self,
        index: usize,
        x: f32,
        settings: &mut Settings,
    ) -> SettingsCommand {
        let progress = ((x - VOLUME_TRACK_X) / VOLUME_TRACK_WIDTH).clamp(0.0, 1.0);
        // The source handle spans x=8..144 and Step uses floor(x / 8), giving
        // levels 1..18 during a drag. Keep the redesigned track's endpoints
        // mapped to those same source steps.
        let level = 1 + (progress * (CLASSIC_VOLUME_LEVELS - 1) as f32).floor() as i32;
        let target = volume_from_classic_level(level);
        let value = if index == 2 {
            &mut settings.music_volume
        } else {
            &mut settings.effects_volume
        };
        if (*value - target).abs() < f32::EPSILON {
            return SettingsCommand::None;
        }
        *value = target;
        SettingsCommand::Changed
    }

    pub fn controller(
        &mut self,
        button: crate::controller::Button,
        settings: &mut Settings,
    ) -> SettingsCommand {
        let Some(index) = self.controller_binding else {
            return SettingsCommand::None;
        };
        let (action, _, _) = crate::controller::ACTIONS[index];
        settings
            .controller_bindings
            .insert(action.into(), button.name().into());
        self.advance_controller_binding(index);
        SettingsCommand::Changed
    }

    pub fn controller_direction(
        &mut self,
        control: crate::controller::BrowserGamepadControl,
        settings: &mut Settings,
    ) -> SettingsCommand {
        let Some(index) = self.controller_direction_binding else {
            return SettingsCommand::None;
        };
        settings.browser_direction_bindings.set(index, control);
        self.advance_controller_direction_binding(index);
        SettingsCommand::Changed
    }

    #[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
    pub fn begin_browser_direction_calibration(&mut self) {
        self.controller_direction_binding = Some(0);
    }

    fn advance_controller_binding(&mut self, index: usize) {
        self.controller_binding =
            (index + 1 < crate::controller::ACTIONS.len()).then_some(index + 1);
    }

    fn advance_controller_direction_binding(&mut self, index: usize) {
        self.controller_direction_binding = (index + 1 < 4).then_some(index + 1);
    }

    pub fn key(&mut self, key: KeyCode, settings: &mut Settings) -> SettingsCommand {
        if let Some(index) = self.controller_binding {
            if key == KeyCode::Escape {
                self.advance_controller_binding(index);
            }
            return SettingsCommand::None;
        }
        if let Some(index) = self.controller_direction_binding {
            if key == KeyCode::Escape {
                self.advance_controller_direction_binding(index);
            }
            return SettingsCommand::None;
        }
        if let Some(index) = self.binding.take() {
            self.unsupported_key = false;
            if key == KeyCode::Escape {
                // Classic oKeyConfig increments its capture status even when
                // Escape leaves the current key unchanged and advances through
                // Classic's authored capture sequence, which differs from the
                // visible row order in this host settings menu.
                let action = ACTIONS[index].0;
                if let Some(order_index) = CLASSIC_KEY_ORDER.iter().position(|key| *key == action) {
                    if let Some(next_action) = CLASSIC_KEY_ORDER.get(order_index + 1) {
                        let next_index = ACTIONS
                            .iter()
                            .position(|(key, _)| key == next_action)
                            .expect("Classic key actions have visible settings rows");
                        self.selected = FIRST_BINDING + next_index;
                        self.binding = Some(next_index);
                    }
                }
                return SettingsCommand::None;
            }
            // Classic oKeyConfig accepts every physical key except Escape.
            // In particular, Backspace and Delete are valid bindings rather
            // than implicit clear commands.
            if !crate::input::supported_key(key) {
                self.binding = Some(index);
                self.unsupported_key = true;
                return SettingsCommand::None;
            }
            let value = format!("{key:?}");
            let action = ACTIONS[index].0;
            settings.bindings.insert(action.into(), value.clone());
            if action == "up" {
                settings.bindings.insert("exit".into(), value);
            }
            return SettingsCommand::Changed;
        }
        match key {
            KeyCode::Escape => SettingsCommand::Back,
            KeyCode::ArrowUp => {
                self.selected = (self.selected + ROWS - 1) % ROWS;
                SettingsCommand::None
            }
            KeyCode::ArrowDown => {
                self.selected = (self.selected + 1) % ROWS;
                SettingsCommand::None
            }
            KeyCode::ArrowLeft => self.adjust(settings, -1, false),
            KeyCode::ArrowRight => self.adjust(settings, 1, false),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                self.adjust(settings, 1, true)
            }
            _ => SettingsCommand::None,
        }
    }

    fn adjust(
        &mut self,
        settings: &mut Settings,
        direction: i32,
        confirm: bool,
    ) -> SettingsCommand {
        match self.selected {
            0 => {
                settings.rules = match settings.rules {
                    RulesPreset::Classic => RulesPreset::Assisted,
                    RulesPreset::Assisted => RulesPreset::Classic,
                };
            }
            1 => settings.pixel_snap = !settings.pixel_snap,
            2 => settings.music_volume = volume_step(settings.music_volume, direction),
            3 => settings.effects_volume = volume_step(settings.effects_volume, direction),
            #[cfg(target_arch = "wasm32")]
            4 => return SettingsCommand::None,
            #[cfg(not(target_arch = "wasm32"))]
            4 => settings.vsync = !settings.vsync,
            5 => {
                let index = CAPS.iter().position(|cap| *cap == settings.frame_cap);
                let next = index.map_or(0, |index| {
                    (index as i32 + direction).rem_euclid(CAPS.len() as i32) as usize
                });
                settings.frame_cap = CAPS[next];
            }
            6 if confirm => return SettingsCommand::Fullscreen,
            7 => settings.down_to_run = !settings.down_to_run,
            CONTROLLER_INPUT => settings.controller_enabled = !settings.controller_enabled,
            GRAPHICS_DETAIL => settings.graphics_high = !settings.graphics_high,
            FIRST_BINDING..CONFIGURE_CONTROLLER if confirm => {
                self.binding = Some(self.selected - FIRST_BINDING);
                self.unsupported_key = false;
                return SettingsCommand::None;
            }
            CONFIGURE_CONTROLLER if confirm => {
                self.controller_binding = Some(0);
                return SettingsCommand::None;
            }
            #[cfg(target_arch = "wasm32")]
            CONFIGURE_DIRECTIONS if confirm => {
                self.begin_browser_direction_calibration();
                return SettingsCommand::None;
            }
            RESTORE if confirm => *settings = Settings::default(),
            BACK if confirm => return SettingsCommand::Back,
            _ => return SettingsCommand::None,
        }
        SettingsCommand::Changed
    }

    pub fn draw(&self, atlas: &Atlas, quads: &mut Vec<DrawQuad>, settings: &Settings) {
        self.draw_with_gamepad(atlas, quads, settings, None);
    }

    pub fn draw_with_gamepad(
        &self,
        atlas: &Atlas,
        quads: &mut Vec<DrawQuad>,
        settings: &Settings,
        primary_gamepad_connected: Option<bool>,
    ) {
        scene::rect(quads, [0.0, 0.0, 320.0, 240.0], [0.01, 0.02, 0.04, 0.95]);
        scene::rect(quads, [8.0, 8.0, 304.0, 224.0], [0.05, 0.07, 0.11, 1.0]);
        scene::text(quads, atlas, "SETTINGS", 18.0, 18.0, 1.0, GOLD);
        scene::text(
            quads,
            atlas,
            &format!("{} / {}", self.selected + 1, ROWS),
            242.0,
            20.0,
            0.75,
            MUTED,
        );
        let start = visible_start(self.selected);
        for (slot, index) in (start..start + VISIBLE).enumerate() {
            let y = 42.0 + slot as f32 * ROW_STEP;
            let selected = index == self.selected;
            if selected {
                scene::rect(quads, [14.0, y - 3.0, 292.0, 12.0], [0.2, 0.22, 0.29, 1.0]);
            }
            let (label, value) = row(index, settings);
            let value = if selected && self.binding.is_some() {
                "PRESS A KEY...".into()
            } else {
                value
            };
            let color = if selected { GOLD } else { WHITE };
            scene::text(quads, atlas, &label, 20.0, y, 0.75, color);
            scene::text(quads, atlas, &short(&value, 25), 150.0, y, 0.75, color);
            if matches!(index, 2 | 3) {
                let volume = if index == 2 {
                    settings.music_volume
                } else {
                    settings.effects_volume
                };
                scene::rect(
                    quads,
                    [VOLUME_TRACK_X, y + 6.0, VOLUME_TRACK_WIDTH, 2.0],
                    [0.20, 0.24, 0.31, 1.0],
                );
                scene::rect(
                    quads,
                    [
                        VOLUME_TRACK_X,
                        y + 6.0,
                        VOLUME_TRACK_WIDTH * classic_volume_progress(volume),
                        2.0,
                    ],
                    if selected { GOLD } else { MUTED },
                );
            }
        }
        let description = match self.selected {
            0 => "RULES APPLY WHEN STARTING A NEW RUN.",
            1 => "OFF: SMOOTH MOTION. ON: PIXEL SNAPPING.",
            2 | 3 => "CLICK / DRAG TRACK OR USE LEFT / RIGHT.",
            4 => {
                #[cfg(target_arch = "wasm32")]
                {
                    "BROWSER PRESENTATION CONTROLS FRAME PACING."
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    "SYNC TO DISPLAY REFRESH WHEN AVAILABLE."
                }
            }
            5 => "BROWSER FRAME TIMING FOLLOWS THE DISPLAY.",
            6 => "ENTER: TOGGLE FULLSCREEN.",
            7 => "HOLD DOWN WHILE MOVING TO RUN.",
            CONTROLLER_INPUT => "ENABLES CONTROLLER GAMEPLAY AND MENU INPUT.",
            GRAPHICS_DETAIL => "LOW REDUCES SOURCE-GATED DECORATION AND EFFECTS.",
            FIRST_BINDING..CONFIGURE_CONTROLLER => "ENTER: REPLACE KEYS FOR THIS ACTION.",
            CONFIGURE_CONTROLLER => "ENTER: CONFIGURE CONTROLLER BUTTONS.",
            #[cfg(target_arch = "wasm32")]
            CONFIGURE_DIRECTIONS => "UNMAPPED WEB PADS ONLY. STANDARD PADS KEEP DEFAULTS.",
            RESTORE => "RESTORES SETTINGS. RECORDS ARE PRESERVED.",
            _ => "RETURN TO THE PREVIOUS MENU.",
        };
        scene::text(quads, atlas, description, 18.0, 199.0, 0.75, MUTED);
        if self.binding.is_some() {
            scene::text(
                quads,
                atlas,
                if self.unsupported_key {
                    "UNSUPPORTED KEY. TRY ANOTHER."
                } else {
                    "PRESS A KEY. ESC: KEEP SAME."
                },
                18.0,
                211.0,
                0.75,
                GOLD,
            );
            scene::text(
                quads,
                atlas,
                "ESC KEEPS THIS KEY AND SKIPS TO THE NEXT SLOT.",
                18.0,
                223.0,
                0.75,
                MUTED,
            );
        } else {
            scene::text(
                quads,
                atlas,
                "UP / DOWN: SELECT. LEFT / RIGHT: CHANGE.",
                18.0,
                211.0,
                0.75,
                MUTED,
            );
            scene::text(
                quads,
                atlas,
                "CLICK: ACTIVATE. ENTER: CHOOSE. ESC: BACK.",
                18.0,
                223.0,
                0.75,
                MUTED,
            );
        }
        if let Some(index) = self.controller_binding {
            let (action, label, default) = crate::controller::ACTIONS[index];
            let current = settings
                .controller_bindings
                .get(action)
                .and_then(|value| crate::controller::parse(value))
                .unwrap_or(default);
            scene::rect(quads, [20.0, 66.0, 280.0, 112.0], [0.02, 0.03, 0.06, 1.0]);
            scene::text(quads, atlas, "PRESS BUTTON FOR", 32.0, 78.0, 1.0, GOLD);
            scene::text(quads, atlas, label, 32.0, 96.0, 1.0, WHITE);
            scene::text(
                quads,
                atlas,
                &format!(
                    "{} / {}   CURRENT: {}",
                    index + 1,
                    crate::controller::ACTIONS.len(),
                    current.label()
                ),
                32.0,
                116.0,
                0.75,
                MUTED,
            );
            scene::text(quads, atlas, "ESC TO KEEP SAME", 32.0, 138.0, 0.75, WHITE);
            scene::text(
                quads,
                atlas,
                "RELEASE ALL CONTROLS BEFORE NEXT BUTTON.",
                32.0,
                158.0,
                0.75,
                MUTED,
            );
            if primary_gamepad_connected == Some(false) {
                // Classic oJoyConfig Draw shows this red warning when
                // joystick_exists(1) is false.
                scene::text(
                    quads,
                    atlas,
                    "NO GAMEPAD FOUND!",
                    32.0,
                    171.0,
                    0.75,
                    [1.0, 0.0, 0.0, 1.0],
                );
            }
        }
        if let Some(index) = self.controller_direction_binding {
            let label = ["LEFT", "RIGHT", "UP", "DOWN"][index];
            let current = settings
                .browser_direction_bindings
                .get(index)
                .map_or_else(|| "UNBOUND".into(), |control| control.label());
            scene::rect(quads, [20.0, 66.0, 280.0, 112.0], [0.02, 0.03, 0.06, 1.0]);
            scene::text(quads, atlas, "MOVE INPUT FOR", 32.0, 78.0, 1.0, GOLD);
            scene::text(quads, atlas, label, 32.0, 96.0, 1.0, WHITE);
            scene::text(
                quads,
                atlas,
                &format!("{} / 4   CURRENT: {current}", index + 1),
                32.0,
                116.0,
                0.75,
                MUTED,
            );
            scene::text(
                quads,
                atlas,
                "MOVE ONE DIRECTION. ESC TO KEEP.",
                32.0,
                138.0,
                0.75,
                WHITE,
            );
            scene::text(
                quads,
                atlas,
                "RELEASE ALL CONTROLS BEFORE NEXT DIRECTION.",
                32.0,
                158.0,
                0.75,
                MUTED,
            );
            if primary_gamepad_connected == Some(false) {
                scene::text(
                    quads,
                    atlas,
                    "NO GAMEPAD FOUND!",
                    32.0,
                    171.0,
                    0.75,
                    [1.0, 0.0, 0.0, 1.0],
                );
            }
        }
    }
}

fn visible_start(selected: usize) -> usize {
    selected.saturating_sub(VISIBLE / 2).min(ROWS - VISIBLE)
}

fn settings_row_at(selected: usize, x: f32, y: f32) -> Option<usize> {
    if !(ROW_X..ROW_X + ROW_WIDTH).contains(&x) || y < ROW_TOP {
        return None;
    }
    let slot = ((y - ROW_TOP) / ROW_STEP) as usize;
    let row_top = ROW_TOP + slot as f32 * ROW_STEP;
    if slot >= VISIBLE || y >= row_top + ROW_HEIGHT {
        return None;
    }
    Some(visible_start(selected) + slot)
}

fn volume_step(value: f32, direction: i32) -> f32 {
    if value <= 0.0 && direction < 0 {
        return 0.0;
    }
    let level = classic_volume_level(value);
    volume_from_classic_level((level + direction).clamp(0, CLASSIC_VOLUME_LEVELS))
}

fn classic_volume_level(value: f32) -> i32 {
    if value <= 0.0 {
        return 0;
    }
    (((value.clamp(CLASSIC_VOLUME_FLOOR, 1.0) - CLASSIC_VOLUME_FLOOR)
        / (1.0 - CLASSIC_VOLUME_FLOOR)
        * CLASSIC_VOLUME_LEVELS as f32)
        .round() as i32)
        .clamp(0, CLASSIC_VOLUME_LEVELS)
}

fn volume_from_classic_level(level: i32) -> f32 {
    CLASSIC_VOLUME_FLOOR
        + (1.0 - CLASSIC_VOLUME_FLOOR)
            * (level.clamp(0, CLASSIC_VOLUME_LEVELS) as f32 / CLASSIC_VOLUME_LEVELS as f32)
}

fn classic_volume_progress(value: f32) -> f32 {
    classic_volume_level(value) as f32 / CLASSIC_VOLUME_LEVELS as f32
}

fn classic_volume_percent(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 100.0).round() as u32
}

fn row(index: usize, settings: &Settings) -> (String, String) {
    let on = |value| if value { "ON" } else { "OFF" }.to_owned();
    let pair = match index {
        0 => ("RULES", format!("{:?}", settings.rules)),
        1 => ("PIXEL SNAP", on(settings.pixel_snap)),
        2 => (
            "MUSIC",
            format!("{}%", classic_volume_percent(settings.music_volume)),
        ),
        3 => (
            "EFFECTS",
            format!("{}%", classic_volume_percent(settings.effects_volume)),
        ),
        #[cfg(target_arch = "wasm32")]
        4 => ("PRESENTATION", "BROWSER".into()),
        #[cfg(not(target_arch = "wasm32"))]
        4 => ("VSYNC", on(settings.vsync)),
        5 => (
            "FRAME CAP",
            settings
                .frame_cap
                .map_or("UNLIMITED".into(), |cap| cap.to_string()),
        ),
        6 => ("FULLSCREEN", "ENTER".into()),
        7 => ("DOWN TO RUN", on(settings.down_to_run)),
        CONTROLLER_INPUT => ("GAMEPAD INPUT", on(settings.controller_enabled)),
        GRAPHICS_DETAIL => (
            "GRAPHICS DETAIL",
            if settings.graphics_high {
                "HIGH"
            } else {
                "LOW"
            }
            .into(),
        ),
        FIRST_BINDING..CONFIGURE_CONTROLLER => {
            let (action, label) = ACTIONS[index - FIRST_BINDING];
            let value = settings.bindings.get(action).cloned().unwrap_or_default();
            (
                label,
                if value.is_empty() {
                    "UNBOUND".into()
                } else {
                    value
                },
            )
        }
        CONFIGURE_CONTROLLER => ("CONFIGURE CONTROLLER", "ENTER".into()),
        #[cfg(target_arch = "wasm32")]
        CONFIGURE_DIRECTIONS => ("CALIBRATE GAMEPAD DIRECTIONS", "ENTER".into()),
        RESTORE => ("RESTORE DEFAULTS", "ENTER".into()),
        _ => ("BACK", "ENTER".into()),
    };
    (pair.0.into(), pair.1)
}

fn short(value: &str, length: usize) -> String {
    if value.chars().count() <= length {
        value.into()
    } else {
        format!("{}...", value.chars().take(length - 3).collect::<String>())
    }
}

// Share real row positions with host navigation tests without exposing a
// production API or skipping ordinary menu key handling.
#[cfg(test)]
pub(crate) mod test_rows {
    pub(crate) const FIRST_BINDING: usize = super::FIRST_BINDING;
    pub(crate) const CONFIGURE_CONTROLLER: usize = super::CONFIGURE_CONTROLLER;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enter_controller_wizard(menu: &mut SettingsUi, settings: &mut Settings) {
        for _ in 0..CONFIGURE_CONTROLLER {
            assert_eq!(
                menu.key(KeyCode::ArrowDown, settings),
                SettingsCommand::None
            );
        }
        assert_eq!(menu.key(KeyCode::Enter, settings), SettingsCommand::None);
        assert!(menu.awaiting_controller_binding());
        assert!(menu.awaiting_binding());
    }

    #[test]
    fn controller_wizard_captures_defaults_in_source_order_and_returns_to_menu() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        let keyboard = settings.bindings.clone();
        let defaults = settings.controller_bindings.clone();
        enter_controller_wizard(&mut menu, &mut settings);
        for (index, (action, _, button)) in crate::controller::ACTIONS.iter().enumerate() {
            assert_eq!(
                menu.controller(*button, &mut settings),
                SettingsCommand::Changed
            );
            assert_eq!(settings.controller_bindings[*action], button.name());
            assert_eq!(
                menu.awaiting_controller_binding(),
                index + 1 < crate::controller::ACTIONS.len()
            );
        }
        assert!(!menu.awaiting_binding());
        assert_eq!(settings.controller_bindings, defaults);
        assert_eq!(settings.bindings, keyboard);
        settings.controller_bindings.clear();
        menu.key(KeyCode::ArrowDown, &mut settings);
        assert_eq!(
            menu.key(KeyCode::Enter, &mut settings),
            SettingsCommand::Changed
        );
        assert_eq!(settings.controller_bindings, defaults);
        menu.key(KeyCode::ArrowDown, &mut settings);
        assert_eq!(
            menu.key(KeyCode::Enter, &mut settings),
            SettingsCommand::Back
        );
    }

    #[test]
    fn controller_wizard_shows_source_warning_only_without_a_connected_gamepad() {
        let atlas = Atlas::load().unwrap();
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        enter_controller_wizard(&mut menu, &mut settings);
        let warning = atlas
            .sprite("sFontSmall", ('N' as u32 - 32) as f64)
            .unwrap();

        let mut without_gamepad = Vec::new();
        menu.draw_with_gamepad(&atlas, &mut without_gamepad, &settings, Some(false));
        let warning_glyphs: Vec<_> = without_gamepad
            .iter()
            .filter(|quad| quad.rect[1] == 171.0 && quad.color == [1.0, 0.0, 0.0, 1.0])
            .collect();
        assert_eq!(warning_glyphs.len(), "NO GAMEPAD FOUND!".len());
        assert!(warning_glyphs.iter().any(|quad| quad.uv == warning.uv));

        let mut with_gamepad = Vec::new();
        menu.draw_with_gamepad(&atlas, &mut with_gamepad, &settings, Some(true));
        assert!(
            !with_gamepad
                .iter()
                .any(|quad| quad.rect[1] == 171.0 && quad.color == [1.0, 0.0, 0.0, 1.0])
        );
    }

    #[test]
    fn browser_direction_wizard_captures_four_raw_controls_and_escape_keeps_current() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        menu.begin_browser_direction_calibration();
        let controls = [
            crate::controller::BrowserGamepadControl::Axis {
                index: 2,
                positive: false,
            },
            crate::controller::BrowserGamepadControl::Axis {
                index: 2,
                positive: true,
            },
            crate::controller::BrowserGamepadControl::Button { index: 17 },
            crate::controller::BrowserGamepadControl::Button { index: 18 },
        ];
        for (index, control) in controls.into_iter().enumerate() {
            assert!(menu.awaiting_binding() && menu.awaiting_direction_binding());
            assert_eq!(
                menu.controller_direction(control, &mut settings),
                SettingsCommand::Changed
            );
            assert_eq!(
                menu.awaiting_direction_binding(),
                index + 1 < controls.len()
            );
        }
        assert_eq!(
            settings.browser_direction_bindings.get(0),
            Some(controls[0])
        );
        assert_eq!(
            settings.browser_direction_bindings.get(1),
            Some(controls[1])
        );
        assert_eq!(
            settings.browser_direction_bindings.get(2),
            Some(controls[2])
        );
        assert_eq!(
            settings.browser_direction_bindings.get(3),
            Some(controls[3])
        );

        menu.controller_direction_binding = Some(0);
        assert_eq!(
            menu.key(KeyCode::Escape, &mut settings),
            SettingsCommand::None
        );
        assert_eq!(menu.controller_direction_binding, Some(1));
        assert_eq!(
            settings.browser_direction_bindings.get(0),
            Some(controls[0])
        );
    }

    #[test]
    fn controller_input_toggle_starts_off_and_changes_the_saved_setting() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        assert!(!settings.controller_enabled);
        assert_eq!(
            row(CONTROLLER_INPUT, &settings),
            ("GAMEPAD INPUT".into(), "OFF".into())
        );

        for _ in 0..CONTROLLER_INPUT {
            assert_eq!(
                menu.key(KeyCode::ArrowDown, &mut settings),
                SettingsCommand::None
            );
        }
        assert_eq!(
            menu.key(KeyCode::Enter, &mut settings),
            SettingsCommand::Changed
        );
        assert!(settings.controller_enabled);
        assert_eq!(row(CONTROLLER_INPUT, &settings).1, "ON");
        assert_eq!(
            menu.key(KeyCode::ArrowLeft, &mut settings),
            SettingsCommand::Changed
        );
        assert!(!settings.controller_enabled);
        assert_eq!(row(CONTROLLER_INPUT, &settings).1, "OFF");
    }

    #[test]
    fn runtime_controller_startup_state_preserves_saved_preference_until_changed() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        settings.controller_enabled = true;
        let session_enabled = false;

        menu.key_with_runtime_controller(KeyCode::ArrowRight, &mut settings, session_enabled);
        assert!(
            settings.controller_enabled,
            "ordinary settings edits preserve the saved preference"
        );

        for _ in 0..CONTROLLER_INPUT {
            menu.key_with_runtime_controller(KeyCode::ArrowDown, &mut settings, session_enabled);
        }
        assert!(menu.controller_state_row_selected());
        assert_eq!(
            menu.key_with_runtime_controller(KeyCode::Enter, &mut settings, session_enabled),
            SettingsCommand::Changed
        );
        assert!(
            settings.controller_enabled,
            "the row toggles from the displayed session state"
        );

        assert_eq!(
            menu.key_with_runtime_controller(KeyCode::ArrowRight, &mut settings, true),
            SettingsCommand::Changed
        );
        assert!(!settings.controller_enabled);
        assert!(menu.controller_state_row_selected());
    }

    #[test]
    fn controller_wizard_ignores_keyboard_and_escape_keeps_each_saved_mapping() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        let original = settings.controller_bindings.clone();
        let keyboard = settings.bindings.clone();
        enter_controller_wizard(&mut menu, &mut settings);
        for index in 0..crate::controller::ACTIONS.len() {
            for key in [
                KeyCode::Enter,
                KeyCode::NumpadEnter,
                KeyCode::Space,
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
                KeyCode::ArrowLeft,
                KeyCode::ArrowRight,
                KeyCode::Delete,
                KeyCode::Backspace,
                KeyCode::KeyA,
                KeyCode::F4,
            ] {
                assert_eq!(menu.key(key, &mut settings), SettingsCommand::None);
                assert_eq!(menu.controller_binding, Some(index));
            }
            assert_eq!(
                menu.key(KeyCode::Escape, &mut settings),
                SettingsCommand::None
            );
            assert_eq!(
                menu.awaiting_controller_binding(),
                index + 1 < crate::controller::ACTIONS.len()
            );
            assert_eq!(settings.controller_bindings, original);
            assert_eq!(settings.bindings, keyboard);
        }
        assert!(!menu.awaiting_binding());
        assert_eq!(
            menu.key(KeyCode::Escape, &mut settings),
            SettingsCommand::Back
        );
    }

    #[test]
    fn controller_wizard_accepts_duplicates_and_ignores_buttons_outside_capture() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        let button = crate::controller::ACTIONS[0].2;
        let original = settings.controller_bindings.clone();
        assert_eq!(
            menu.controller(button, &mut settings),
            SettingsCommand::None
        );
        assert_eq!(settings.controller_bindings, original);
        enter_controller_wizard(&mut menu, &mut settings);
        for (action, _, _) in crate::controller::ACTIONS.iter().take(2) {
            assert_eq!(
                menu.controller(button, &mut settings),
                SettingsCommand::Changed
            );
            assert_eq!(settings.controller_bindings[*action], button.name());
        }
        for _ in 2..crate::controller::ACTIONS.len() {
            menu.key(KeyCode::Escape, &mut settings);
        }
        assert!(!menu.awaiting_binding());
        let saved = settings.controller_bindings.clone();
        assert_eq!(
            menu.controller(button, &mut settings),
            SettingsCommand::None
        );
        assert_eq!(settings.controller_bindings, saved);
        assert_eq!(
            row(CONFIGURE_CONTROLLER, &settings).0,
            "CONFIGURE CONTROLLER"
        );
        assert_eq!(row(RESTORE, &settings).0, "RESTORE DEFAULTS");
        assert_eq!(row(BACK, &settings).0, "BACK");
    }

    #[test]
    // Classic's oKeyConfig Key any key event excludes only vk_escape; its
    // control capture can therefore bind Backspace and Delete as gameplay keys.
    fn bindings_match_classic_capture_for_backspace_and_delete() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        for (index, (action, _)) in ACTIONS.iter().enumerate() {
            menu.selected = FIRST_BINDING + index;
            assert_eq!(
                menu.key(KeyCode::Enter, &mut settings),
                SettingsCommand::None
            );
            assert!(menu.awaiting_binding());
            assert_eq!(
                menu.key(KeyCode::F35, &mut settings),
                SettingsCommand::Changed
            );
            assert!(!menu.awaiting_binding());
            assert_eq!(settings.bindings[*action], "F35");
            menu.key(KeyCode::Enter, &mut settings);
            assert_eq!(
                menu.key(KeyCode::ArrowUp, &mut settings),
                SettingsCommand::Changed
            );
            assert_eq!(settings.bindings[*action], "ArrowUp");
            assert!(!menu.awaiting_binding());
            menu.key(KeyCode::Enter, &mut settings);
            assert_eq!(
                menu.key(KeyCode::Backspace, &mut settings),
                SettingsCommand::Changed
            );
            assert_eq!(settings.bindings[*action], "Backspace");
            let mut input = crate::input::InputState::default();
            input.set_bindings(&settings.bindings);
            assert!(crate::input::supported_key(KeyCode::Backspace));
            if *action != "pause" {
                assert!(input.action_key(action, KeyCode::Backspace));
            }
            menu.key(KeyCode::Enter, &mut settings);
            assert_eq!(
                menu.key(KeyCode::Delete, &mut settings),
                SettingsCommand::Changed
            );
            assert_eq!(settings.bindings[*action], "Delete");
            let mut input = crate::input::InputState::default();
            input.set_bindings(&settings.bindings);
            assert!(crate::input::supported_key(KeyCode::Delete));
            if *action != "pause" {
                assert!(input.action_key(action, KeyCode::Delete));
            }
        }
        let mut save = crate::storage::SaveData::default();
        save.settings = settings;
        let reopened = crate::storage::SaveData::from_json(&save.to_json().unwrap()).unwrap();
        for (action, _) in ACTIONS {
            assert_eq!(reopened.settings.bindings[action], "Delete");
        }
        save.settings
            .bindings
            .insert("attack".into(), "Backspace".into());
        let reopened = crate::storage::SaveData::from_json(&save.to_json().unwrap()).unwrap();
        assert_eq!(reopened.settings.bindings["attack"], "Backspace");
    }

    #[test]
    // Classic oKeyConfig increments `status` after every Key any key event,
    // including Escape; Escape skips the current assignment without changing it.
    fn escape_skips_unchanged_classic_binding_and_advances_capture_status() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();

        for (index, action) in CLASSIC_KEY_ORDER.iter().enumerate() {
            let visible_index = ACTIONS
                .iter()
                .position(|(key, _)| key == action)
                .expect("Classic key actions have visible settings rows");
            if index == 0 {
                menu.selected = FIRST_BINDING + visible_index;
                menu.key(KeyCode::Enter, &mut settings);
            }
            let old = settings.bindings[*action].clone();
            assert_eq!(
                menu.key(KeyCode::Escape, &mut settings),
                SettingsCommand::None
            );
            assert_eq!(settings.bindings[*action], old);
            if let Some(next_action) = CLASSIC_KEY_ORDER.get(index + 1) {
                let next_index = ACTIONS
                    .iter()
                    .position(|(key, _)| key == next_action)
                    .expect("Classic key actions have visible settings rows");
                assert_eq!(menu.selected, FIRST_BINDING + next_index);
                assert_eq!(menu.binding, Some(next_index));
            } else {
                assert_eq!(menu.binding, None);
                assert!(!menu.awaiting_binding());
            }
        }
    }

    #[test]
    fn rebinding_up_also_rebinds_the_start_door_alias() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        let up = ACTIONS
            .iter()
            .position(|(action, _)| *action == "up")
            .unwrap();
        menu.selected = FIRST_BINDING + up;

        assert_eq!(
            menu.key(KeyCode::Enter, &mut settings),
            SettingsCommand::None
        );
        assert_eq!(
            menu.key(KeyCode::KeyI, &mut settings),
            SettingsCommand::Changed
        );
        assert_eq!(settings.bindings["up"], "KeyI");
        assert_eq!(settings.bindings["exit"], "KeyI");

        let mut input = crate::input::InputState::default();
        input.set_bindings(&settings.bindings);
        input.key(KeyCode::ArrowUp, true, false);
        let stale_key = input.take_tick();
        assert!(!stale_key.up && !stale_key.exit_pressed);
        input.key(KeyCode::ArrowUp, false, false);
        input.key(KeyCode::KeyI, true, false);
        let rebound_key = input.take_tick();
        assert!(rebound_key.up && rebound_key.up_pressed && rebound_key.exit_pressed);
    }

    #[test]
    fn presentation_options_and_volume_limits_are_selectable() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        menu.key(KeyCode::ArrowRight, &mut settings);
        assert_eq!(settings.rules, RulesPreset::Assisted);
        menu.selected = 1;
        menu.key(KeyCode::Enter, &mut settings);
        assert!(settings.pixel_snap);
        menu.selected = 2;
        for _ in 0..20 {
            menu.key(KeyCode::ArrowRight, &mut settings);
        }
        assert_eq!(settings.music_volume, 1.0);
        for _ in 0..20 {
            menu.key(KeyCode::ArrowLeft, &mut settings);
        }
        assert_eq!(classic_volume_level(settings.music_volume), 0);
        assert_eq!(settings.music_volume, CLASSIC_VOLUME_FLOOR);
        menu.selected = 5;
        for cap in CAPS.into_iter().cycle().skip(1).take(5) {
            menu.key(KeyCode::ArrowRight, &mut settings);
            assert_eq!(settings.frame_cap, cap);
        }
        menu.selected = 6;
        assert_eq!(
            menu.key(KeyCode::Enter, &mut settings),
            SettingsCommand::Fullscreen
        );
    }

    #[test]
    fn source_volume_levels_round_trip_and_report_slider_progress() {
        assert_eq!(classic_volume_level(Settings::default().music_volume), 15);
        assert_eq!(classic_volume_percent(Settings::default().music_volume), 87);
        assert_eq!(
            classic_volume_progress(Settings::default().music_volume),
            15.0 / 18.0
        );
        assert_eq!(row(2, &Settings::default()).1, "87%");
        for level in 0..=CLASSIC_VOLUME_LEVELS {
            let value = volume_from_classic_level(level);
            assert_eq!(classic_volume_level(value), level);
            assert_eq!(classic_volume_progress(value), level as f32 / 18.0);
        }

        let mut settings = Settings::default();
        let mut menu = SettingsUi::new();
        menu.selected = 2;
        let music_track_y = ROW_TOP + 2.0 * ROW_STEP + 9.0;
        menu.pointer_press(VOLUME_TRACK_X, music_track_y, &mut settings);
        assert_eq!(classic_volume_level(settings.music_volume), 1);
        menu.pointer_move(VOLUME_TRACK_X, false, &mut settings);
        menu.take_audio_action();

        menu.key(KeyCode::ArrowRight, &mut settings);
        assert_eq!(classic_volume_level(settings.music_volume), 2);
        menu.key(KeyCode::ArrowLeft, &mut settings);
        assert_eq!(classic_volume_level(settings.music_volume), 1);
    }

    #[test]
    fn pointer_clicks_hit_visible_rows_and_match_the_settings_actions() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();

        assert_eq!(
            menu.pointer_press(100.0, 55.0, &mut settings),
            SettingsCommand::Changed
        );
        assert!(settings.pixel_snap);
        assert_eq!(
            menu.pointer_press(10.0, 55.0, &mut settings),
            SettingsCommand::None
        );
        assert_eq!(
            menu.pointer_press(100.0, 52.0, &mut settings),
            SettingsCommand::None
        );

        assert_eq!(
            menu.pointer_press(100.0, 123.0 + 3.0, &mut settings),
            SettingsCommand::Fullscreen
        );
        menu.selected = BACK;
        let back_y = ROW_TOP + (VISIBLE - 1) as f32 * ROW_STEP + 3.0;
        assert_eq!(
            menu.pointer_press(100.0, back_y, &mut settings),
            SettingsCommand::Back
        );
    }

    #[test]
    fn pointer_volume_tracks_only_the_source_style_slider_and_stops_on_release() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        let music_label_y = ROW_TOP + 2.0 * ROW_STEP + 3.0;
        let music_track_y = ROW_TOP + 2.0 * ROW_STEP + 9.0;

        assert_eq!(
            menu.pointer_press(100.0, music_label_y, &mut settings),
            SettingsCommand::None
        );
        assert_eq!(
            menu.pointer_press(253.0, music_label_y, &mut settings),
            SettingsCommand::None
        );
        assert_eq!(settings.music_volume, Settings::default().music_volume);
        assert_eq!(
            menu.pointer_press(253.0, music_track_y, &mut settings),
            SettingsCommand::Changed
        );
        assert_eq!(classic_volume_level(settings.music_volume), 9);
        assert_eq!(classic_volume_percent(settings.music_volume), 60);
        assert_eq!(
            menu.take_audio_action(),
            Some(SettingsAudioAction::PreviewCave)
        );
        assert_eq!(
            menu.pointer_move(296.0, true, &mut settings),
            SettingsCommand::Changed
        );
        assert_eq!(settings.music_volume, 1.0);
        assert_eq!(
            menu.pointer_move(210.0, false, &mut settings),
            SettingsCommand::None
        );
        assert_eq!(
            menu.take_audio_action(),
            Some(SettingsAudioAction::StopAllMusic)
        );
        assert_eq!(
            menu.pointer_move(210.0, true, &mut settings),
            SettingsCommand::None
        );
        assert_eq!(settings.music_volume, 1.0);
    }

    #[test]
    fn effects_slider_plays_jump_preview_on_source_alarm_cadence_until_release() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        let effects_track_y = ROW_TOP + 3.0 * ROW_STEP + 9.0;

        assert_eq!(
            menu.pointer_press(253.0, effects_track_y, &mut settings),
            SettingsCommand::Changed
        );
        assert_eq!(menu.take_audio_action(), None);
        assert!(
            menu.tick_effects_preview(),
            "source alarm starts at one step"
        );
        for _ in 0..19 {
            assert!(!menu.tick_effects_preview());
        }
        assert!(
            menu.tick_effects_preview(),
            "source alarm rearms for 20 steps"
        );

        assert_eq!(
            menu.pointer_move(253.0, false, &mut settings),
            SettingsCommand::None
        );
        for _ in 0..20 {
            assert!(!menu.tick_effects_preview());
        }
        assert!(!menu.tick_effects_preview());
        assert!(!menu.tick_effects_preview(), "released slider stays quiet");
    }

    #[test]
    fn pointer_does_not_interrupt_keyboard_or_controller_binding_capture() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        menu.selected = FIRST_BINDING;
        assert_eq!(
            menu.key(KeyCode::Enter, &mut settings),
            SettingsCommand::None
        );
        assert!(menu.awaiting_binding());
        assert_eq!(
            menu.pointer_press(100.0, 55.0, &mut settings),
            SettingsCommand::None
        );
        assert!(menu.awaiting_binding());

        menu.binding = None;
        menu.selected = CONFIGURE_CONTROLLER;
        assert_eq!(
            menu.key(KeyCode::Enter, &mut settings),
            SettingsCommand::None
        );
        assert!(menu.awaiting_controller_binding());
        assert_eq!(
            menu.pointer_press(100.0, 55.0, &mut settings),
            SettingsCommand::None
        );
        assert!(menu.awaiting_controller_binding());
    }

    #[test]
    fn navigation_wraps_and_restore_only_replaces_settings() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        menu.key(KeyCode::ArrowUp, &mut settings);
        assert_eq!(menu.selected, BACK);
        assert_eq!(
            menu.key(KeyCode::Enter, &mut settings),
            SettingsCommand::Back
        );
        menu.key(KeyCode::ArrowDown, &mut settings);
        assert_eq!(menu.selected, 0);
        settings.music_volume = 0.0;
        settings.bindings.clear();
        menu.selected = RESTORE;
        assert_eq!(
            menu.key(KeyCode::Enter, &mut settings),
            SettingsCommand::Changed
        );
        assert_eq!(settings.music_volume, Settings::default().music_volume);
        assert_eq!(settings.bindings, Settings::default().bindings);
    }

    #[test]
    fn down_to_run_toggle_and_restore_preserve_action_binding_rows() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        menu.selected = 7;
        assert_eq!(row(7, &settings), ("DOWN TO RUN".into(), "ON".into()));
        assert_eq!(
            menu.key(KeyCode::ArrowRight, &mut settings),
            SettingsCommand::Changed
        );
        assert!(!settings.down_to_run);
        assert_eq!(row(7, &settings), ("DOWN TO RUN".into(), "OFF".into()));
        assert_eq!(
            menu.key(KeyCode::Enter, &mut settings),
            SettingsCommand::Changed
        );
        assert!(settings.down_to_run);
        assert_eq!(row(FIRST_BINDING, &settings).0, "MOVE LEFT");
        settings.down_to_run = false;
        menu.selected = RESTORE;
        menu.key(KeyCode::Enter, &mut settings);
        assert!(settings.down_to_run);
    }

    #[test]
    fn classic_graphics_detail_toggle_exposes_high_and_low_source_modes() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        assert!(settings.graphics_high);
        assert_eq!(
            row(GRAPHICS_DETAIL, &settings),
            ("GRAPHICS DETAIL".into(), "HIGH".into())
        );

        menu.selected = GRAPHICS_DETAIL;
        assert_eq!(
            menu.key(KeyCode::Enter, &mut settings),
            SettingsCommand::Changed
        );
        assert!(!settings.graphics_high);
        assert_eq!(row(GRAPHICS_DETAIL, &settings).1, "LOW");
        assert_eq!(
            menu.key(KeyCode::ArrowLeft, &mut settings),
            SettingsCommand::Changed
        );
        assert!(settings.graphics_high);
        assert_eq!(row(GRAPHICS_DETAIL, &settings).1, "HIGH");
    }
}
