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
const FIRST_BINDING: usize = 8;
const CONFIGURE_CONTROLLER: usize = FIRST_BINDING + ACTIONS.len();
const RESTORE: usize = CONFIGURE_CONTROLLER + 1;
const BACK: usize = RESTORE + 1;
const ROWS: usize = BACK + 1;
const VISIBLE: usize = 11;
const CAPS: [Option<u32>; 5] = [None, Some(60), Some(120), Some(144), Some(240)];
const WHITE: [f32; 4] = [0.94, 0.95, 0.98, 1.0];
const GOLD: [f32; 4] = [1.0, 0.81, 0.32, 1.0];
const MUTED: [f32; 4] = [0.68, 0.74, 0.82, 1.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCommand {
    None,
    Changed,
    Back,
    Fullscreen,
}

#[derive(Default)]
pub struct SettingsUi {
    selected: usize,
    binding: Option<usize>,
    controller_binding: Option<usize>,
    unsupported_key: bool,
}

impl SettingsUi {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn awaiting_binding(&self) -> bool {
        self.binding.is_some() || self.awaiting_controller_binding()
    }

    pub fn awaiting_controller_binding(&self) -> bool {
        self.controller_binding.is_some()
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

    fn advance_controller_binding(&mut self, index: usize) {
        self.controller_binding =
            (index + 1 < crate::controller::ACTIONS.len()).then_some(index + 1);
    }

    pub fn key(&mut self, key: KeyCode, settings: &mut Settings) -> SettingsCommand {
        if let Some(index) = self.controller_binding {
            if key == KeyCode::Escape {
                self.advance_controller_binding(index);
            }
            return SettingsCommand::None;
        }
        if let Some(index) = self.binding.take() {
            self.unsupported_key = false;
            if key == KeyCode::Escape {
                return SettingsCommand::None;
            }
            if !matches!(key, KeyCode::Backspace | KeyCode::Delete)
                && !crate::input::supported_key(key)
            {
                self.binding = Some(index);
                self.unsupported_key = true;
                return SettingsCommand::None;
            }
            let value = if matches!(key, KeyCode::Backspace | KeyCode::Delete) {
                String::new()
            } else {
                format!("{key:?}")
            };
            settings.bindings.insert(ACTIONS[index].0.into(), value);
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
            FIRST_BINDING..CONFIGURE_CONTROLLER if confirm => {
                self.binding = Some(self.selected - FIRST_BINDING);
                self.unsupported_key = false;
                return SettingsCommand::None;
            }
            CONFIGURE_CONTROLLER if confirm => {
                self.controller_binding = Some(0);
                return SettingsCommand::None;
            }
            RESTORE if confirm => *settings = Settings::default(),
            BACK if confirm => return SettingsCommand::Back,
            _ => return SettingsCommand::None,
        }
        SettingsCommand::Changed
    }

    pub fn draw(&self, atlas: &Atlas, quads: &mut Vec<DrawQuad>, settings: &Settings) {
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
        let start = self
            .selected
            .saturating_sub(VISIBLE / 2)
            .min(ROWS - VISIBLE);
        for (slot, index) in (start..start + VISIBLE).enumerate() {
            let y = 42.0 + slot as f32 * 14.0;
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
        }
        let description = match self.selected {
            0 => "RULES APPLY WHEN STARTING A NEW RUN.",
            1 => "OFF: SMOOTH MOTION. ON: PIXEL SNAPPING.",
            2 | 3 => "LEFT / RIGHT: ADJUST VOLUME.",
            4 => "SYNC TO DISPLAY REFRESH WHEN AVAILABLE.",
            5 => "BROWSER FRAME TIMING FOLLOWS THE DISPLAY.",
            6 => "ENTER: TOGGLE FULLSCREEN.",
            7 => "HOLD DOWN WHILE MOVING TO RUN.",
            FIRST_BINDING..CONFIGURE_CONTROLLER => "ENTER: REPLACE KEYS FOR THIS ACTION.",
            CONFIGURE_CONTROLLER => "ENTER: CONFIGURE CONTROLLER BUTTONS.",
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
                    "PRESS A KEY. ESC: CANCEL."
                },
                18.0,
                211.0,
                0.75,
                GOLD,
            );
            scene::text(
                quads,
                atlas,
                "ESC: CANCEL. DELETE / BACKSPACE: UNBIND.",
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
                "ENTER: CHOOSE. ESC: BACK.",
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
        }
    }
}

fn volume_step(value: f32, direction: i32) -> f32 {
    ((value * 10.0).round() + direction as f32).clamp(0.0, 10.0) / 10.0
}

fn row(index: usize, settings: &Settings) -> (String, String) {
    let on = |value| if value { "ON" } else { "OFF" }.to_owned();
    let pair = match index {
        0 => ("RULES", format!("{:?}", settings.rules)),
        1 => ("PIXEL SNAP", on(settings.pixel_snap)),
        2 => (
            "MUSIC",
            format!("{}%", (settings.music_volume * 100.0).round()),
        ),
        3 => (
            "EFFECTS",
            format!("{}%", (settings.effects_volume * 100.0).round()),
        ),
        4 => ("VSYNC", on(settings.vsync)),
        5 => (
            "FRAME CAP",
            settings
                .frame_cap
                .map_or("UNLIMITED".into(), |cap| cap.to_string()),
        ),
        6 => ("FULLSCREEN", "ENTER".into()),
        7 => ("DOWN TO RUN", on(settings.down_to_run)),
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
    fn bindings_can_capture_navigation_keys_cancel_and_unbind() {
        let mut menu = SettingsUi::new();
        let mut settings = Settings::default();
        for (index, (action, _)) in ACTIONS.iter().enumerate() {
            menu.selected = FIRST_BINDING + index;
            assert_eq!(
                menu.key(KeyCode::Enter, &mut settings),
                SettingsCommand::None
            );
            assert!(menu.awaiting_binding());
            assert_eq!(menu.key(KeyCode::F35, &mut settings), SettingsCommand::None);
            assert!(menu.awaiting_binding());
            assert!(menu.unsupported_key);
            assert_eq!(
                menu.key(KeyCode::ArrowUp, &mut settings),
                SettingsCommand::Changed
            );
            assert_eq!(settings.bindings[*action], "ArrowUp");
            assert!(!menu.awaiting_binding());
            menu.key(KeyCode::Enter, &mut settings);
            menu.key(KeyCode::Escape, &mut settings);
            assert_eq!(settings.bindings[*action], "ArrowUp");
            menu.key(KeyCode::Enter, &mut settings);
            menu.key(KeyCode::Delete, &mut settings);
            assert!(settings.bindings[*action].is_empty());
        }
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
        assert_eq!(settings.music_volume, 0.0);
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
        assert_eq!(menu.key(KeyCode::ArrowRight, &mut settings), SettingsCommand::Changed);
        assert!(!settings.down_to_run);
        assert_eq!(row(7, &settings), ("DOWN TO RUN".into(), "OFF".into()));
        assert_eq!(menu.key(KeyCode::Enter, &mut settings), SettingsCommand::Changed);
        assert!(settings.down_to_run);
        assert_eq!(row(FIRST_BINDING, &settings).0, "MOVE LEFT");
        settings.down_to_run = false;
        menu.selected = RESTORE;
        menu.key(KeyCode::Enter, &mut settings);
        assert!(settings.down_to_run);
    }
}
