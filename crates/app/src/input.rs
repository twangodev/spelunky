//! Keyboard/controller state and tick-consumed button edges.

use spelunky_core::InputFrame;
use std::collections::{BTreeMap, HashSet};
use winit::keyboard::KeyCode;

const DEAD_ZONE: f32 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiButton {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Cancel,
    Pause,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum Action {
    Left,
    Right,
    Up,
    Down,
    Run,
    Jump,
    Attack,
    Bomb,
    Rope,
    Flare,
    Cycle,
    Pay,
    Exit,
}

const ACTIONS: [(Action, &str, KeyCode); 13] = [
    (Action::Left, "left", KeyCode::ArrowLeft),
    (Action::Right, "right", KeyCode::ArrowRight),
    (Action::Up, "up", KeyCode::ArrowUp),
    (Action::Down, "down", KeyCode::ArrowDown),
    (Action::Run, "run", KeyCode::ShiftLeft),
    (Action::Jump, "jump", KeyCode::KeyZ),
    (Action::Attack, "attack", KeyCode::KeyX),
    (Action::Bomb, "bomb", KeyCode::KeyB),
    (Action::Rope, "rope", KeyCode::KeyR),
    (Action::Flare, "flare", KeyCode::KeyF),
    (Action::Cycle, "cycle", KeyCode::KeyC),
    (Action::Pay, "pay", KeyCode::KeyP),
    (Action::Exit, "exit", KeyCode::Enter),
];

#[derive(Default, Clone, Copy)]
struct Actions(u16);

impl Actions {
    fn set(&mut self, action: Action, pressed: bool) {
        if pressed {
            self.0 |= 1 << action as u8;
        }
    }
    fn has(self, action: Action) -> bool {
        self.0 & (1 << action as u8) != 0
    }
}

/// Binding values are comma-separated physical `KeyCode` names.
pub fn default_bindings() -> BTreeMap<String, String> {
    ACTIONS
        .iter()
        .map(|(_, name, key)| (name.to_string(), format!("{key:?}")))
        .collect()
}

/// Whether a captured physical key can round-trip through saved bindings.
pub fn supported_key(key: KeyCode) -> bool {
    parse_key(&format!("{key:?}")).is_some()
}

pub struct InputState {
    keys: HashSet<KeyCode>,
    bindings: BTreeMap<String, Vec<KeyCode>>,
    gamepad_actions: Actions,
    held: Actions,
    frame: InputFrame,
    gamepad_ui: u8,
    ui_edges: Vec<UiButton>,
    #[cfg(not(target_arch = "wasm32"))]
    gilrs: Option<gilrs::Gilrs>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            keys: HashSet::new(),
            bindings: ACTIONS
                .iter()
                .map(|(_, name, key)| (name.to_string(), vec![*key]))
                .collect(),
            gamepad_actions: Actions::default(),
            held: Actions::default(),
            frame: InputFrame::default(),
            gamepad_ui: 0,
            ui_edges: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            gilrs: gilrs::Gilrs::new().ok(),
        }
    }
}

impl InputState {
    pub fn key(&mut self, key: KeyCode, pressed: bool, repeat: bool) {
        if pressed && repeat {
            return;
        }
        if pressed {
            self.keys.insert(key);
        } else {
            self.keys.remove(&key);
        }
        self.refresh();
    }

    /// Comma-separated keys are alternatives for the same action. An empty
    /// value unbinds an action. Invalid keys are ignored, with the default kept
    /// when a nonempty value contains no valid keys. Omitted actions default.
    /// Rebinding clears held input so changing bindings cannot fire an action.
    pub fn set_bindings(&mut self, bindings: &BTreeMap<String, String>) {
        self.bindings = ACTIONS
            .iter()
            .map(|(_, name, key)| (name.to_string(), vec![*key]))
            .collect();
        for (name, value) in bindings {
            if self.bindings.contains_key(name) {
                let mut keys = Vec::new();
                for key in value.split(',').filter_map(|key| parse_key(key.trim())) {
                    if !keys.contains(&key) {
                        keys.push(key);
                    }
                }
                if value.trim().is_empty() || !keys.is_empty() {
                    self.bindings.insert(name.clone(), keys);
                }
            }
        }
        self.clear();
    }

    pub fn take_tick(&mut self) -> InputFrame {
        let frame = self.frame;
        self.frame.clear_edges();
        frame
    }

    pub fn clear(&mut self) {
        self.clear_gameplay();
        self.gamepad_ui = 0;
    }

    /// Clear gameplay at screen transitions while keeping controller menu
    /// buttons latched until release, so holding Start cannot toggle repeatedly.
    pub fn clear_gameplay(&mut self) {
        self.keys.clear();
        self.gamepad_actions = Actions::default();
        self.held = Actions::default();
        self.frame = InputFrame::default();
        self.ui_edges.clear();
    }

    /// Controller navigation edges only; the host handles keyboard menu keys.
    pub fn ui_pressed(&mut self) -> Vec<UiButton> {
        std::mem::take(&mut self.ui_edges)
    }

    fn refresh(&mut self) {
        let mut next = self.gamepad_actions;
        for (action, name, _) in ACTIONS {
            next.set(
                action,
                self.bindings
                    .get(name)
                    .is_some_and(|keys| keys.iter().any(|key| self.keys.contains(key))),
            );
        }
        let pressed = |action| next.has(action) && !self.held.has(action);
        self.frame.jump_pressed |= pressed(Action::Jump);
        self.frame.jump_released |= !next.has(Action::Jump) && self.held.has(Action::Jump);
        self.frame.attack_pressed |= pressed(Action::Attack);
        self.frame.bomb_pressed |= pressed(Action::Bomb);
        self.frame.rope_pressed |= pressed(Action::Rope);
        self.frame.flare_pressed |= pressed(Action::Flare);
        self.frame.cycle_pressed |= pressed(Action::Cycle);
        self.frame.pay_pressed |= pressed(Action::Pay);
        self.frame.exit_pressed |= pressed(Action::Exit);
        self.frame.left = next.has(Action::Left);
        self.frame.right = next.has(Action::Right);
        self.frame.up = next.has(Action::Up);
        self.frame.down = next.has(Action::Down);
        self.frame.run = next.has(Action::Run);
        self.frame.jump = next.has(Action::Jump);
        self.frame.attack = next.has(Action::Attack);
        self.held = next;
    }

    fn accept_gamepads(&mut self, actions: Actions, ui: u8) {
        for (index, button) in [
            UiButton::Up,
            UiButton::Down,
            UiButton::Left,
            UiButton::Right,
            UiButton::Confirm,
            UiButton::Cancel,
            UiButton::Pause,
        ]
        .into_iter()
        .enumerate()
        {
            if ui & (1 << index) != 0 && self.gamepad_ui & (1 << index) == 0 {
                self.ui_edges.push(button);
            }
        }
        self.gamepad_actions = actions;
        self.gamepad_ui = ui;
        self.refresh();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn update_gamepad(&mut self) {
        let Some(mut gilrs) = self.gilrs.take() else {
            self.accept_gamepads(Actions::default(), 0);
            return;
        };
        // Sampling after every queued event preserves a press/release pair that
        // arrives between two presentation frames.
        while gilrs.next_event().is_some() {
            let (actions, ui) = Self::native_gamepads(&gilrs);
            self.accept_gamepads(actions, ui);
        }
        let (actions, ui) = Self::native_gamepads(&gilrs);
        self.accept_gamepads(actions, ui);
        self.gilrs = Some(gilrs);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn native_gamepads(gilrs: &gilrs::Gilrs) -> (Actions, u8) {
        use gilrs::{Axis, Button};
        let mut actions = Actions::default();
        let mut ui = 0;
        for (_, pad) in gilrs.gamepads() {
            if !pad.is_connected() {
                continue;
            }
            let x = pad.value(Axis::LeftStickX);
            let y = pad.value(Axis::LeftStickY);
            let left = x < -DEAD_ZONE || pad.is_pressed(Button::DPadLeft);
            let right = x > DEAD_ZONE || pad.is_pressed(Button::DPadRight);
            let up = y > DEAD_ZONE || pad.is_pressed(Button::DPadUp);
            let down = y < -DEAD_ZONE || pad.is_pressed(Button::DPadDown);
            let jump = pad.is_pressed(Button::South);
            actions.set(Action::Left, left);
            actions.set(Action::Right, right);
            actions.set(Action::Up, up);
            actions.set(Action::Down, down);
            actions.set(Action::Jump, jump);
            actions.set(Action::Attack, pad.is_pressed(Button::West));
            actions.set(Action::Bomb, pad.is_pressed(Button::LeftTrigger2));
            actions.set(Action::Rope, pad.is_pressed(Button::RightTrigger2));
            actions.set(Action::Run, pad.is_pressed(Button::LeftTrigger));
            actions.set(Action::Cycle, pad.is_pressed(Button::RightTrigger));
            actions.set(Action::Flare, pad.is_pressed(Button::North));
            actions.set(Action::Pay, pad.is_pressed(Button::Select));
            for (index, pressed) in [
                up,
                down,
                left,
                right,
                jump,
                pad.is_pressed(Button::East),
                pad.is_pressed(Button::Start),
            ]
            .into_iter()
            .enumerate()
            {
                if pressed {
                    ui |= 1 << index;
                }
            }
        }
        (actions, ui)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn update_gamepad(&mut self) {
        use wasm_bindgen::JsCast;
        let mut actions = Actions::default();
        let mut ui = 0;
        if let Some(pads) =
            web_sys::window().and_then(|window| window.navigator().get_gamepads().ok())
        {
            for value in pads.iter() {
                let Ok(pad) = value.dyn_into::<web_sys::Gamepad>() else {
                    continue;
                };
                // Browser indices below are defined only for standard mappings.
                if !pad.connected() || pad.mapping() != web_sys::GamepadMappingType::Standard {
                    continue;
                }
                let buttons = pad.buttons();
                let button = |index| {
                    buttons
                        .get(index)
                        .dyn_into::<web_sys::GamepadButton>()
                        .is_ok_and(|button| button.pressed() || button.value() > 0.5)
                };
                let axes = pad.axes();
                let x = axes.get(0).as_f64().unwrap_or(0.0) as f32;
                let y = axes.get(1).as_f64().unwrap_or(0.0) as f32;
                let left = x < -DEAD_ZONE || button(14);
                let right = x > DEAD_ZONE || button(15);
                let up = y < -DEAD_ZONE || button(12);
                let down = y > DEAD_ZONE || button(13);
                let jump = button(0);
                actions.set(Action::Left, left);
                actions.set(Action::Right, right);
                actions.set(Action::Up, up);
                actions.set(Action::Down, down);
                actions.set(Action::Jump, jump);
                actions.set(Action::Attack, button(2));
                actions.set(Action::Bomb, button(6));
                actions.set(Action::Rope, button(7));
                actions.set(Action::Run, button(4));
                actions.set(Action::Cycle, button(5));
                actions.set(Action::Flare, button(3));
                actions.set(Action::Pay, button(8));
                for (index, pressed) in [up, down, left, right, jump, button(1), button(9)]
                    .into_iter()
                    .enumerate()
                {
                    if pressed {
                        ui |= 1 << index;
                    }
                }
            }
        }
        self.accept_gamepads(actions, ui);
    }
}

fn parse_key(name: &str) -> Option<KeyCode> {
    use KeyCode::*;
    let keys = [
        KeyA,
        KeyB,
        KeyC,
        KeyD,
        KeyE,
        KeyF,
        KeyG,
        KeyH,
        KeyI,
        KeyJ,
        KeyK,
        KeyL,
        KeyM,
        KeyN,
        KeyO,
        KeyP,
        KeyQ,
        KeyR,
        KeyS,
        KeyT,
        KeyU,
        KeyV,
        KeyW,
        KeyX,
        KeyY,
        KeyZ,
        Digit0,
        Digit1,
        Digit2,
        Digit3,
        Digit4,
        Digit5,
        Digit6,
        Digit7,
        Digit8,
        Digit9,
        ArrowLeft,
        ArrowRight,
        ArrowUp,
        ArrowDown,
        Space,
        Enter,
        Escape,
        ShiftLeft,
        ShiftRight,
        ControlLeft,
        ControlRight,
        AltLeft,
        AltRight,
        Tab,
        Backspace,
        CapsLock,
        Minus,
        Equal,
        BracketLeft,
        BracketRight,
        Backslash,
        Semicolon,
        Quote,
        Backquote,
        Comma,
        Period,
        Slash,
        Insert,
        Delete,
        Home,
        End,
        PageUp,
        PageDown,
        Numpad0,
        Numpad1,
        Numpad2,
        Numpad3,
        Numpad4,
        Numpad5,
        Numpad6,
        Numpad7,
        Numpad8,
        Numpad9,
        NumpadEnter,
        NumpadAdd,
        NumpadSubtract,
        NumpadMultiply,
        NumpadDivide,
        NumpadDecimal,
        F1,
        F2,
        F3,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        F10,
        F11,
        F12,
    ];
    keys.into_iter().find(|key| format!("{key:?}") == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_press_survives_until_tick_and_edges_are_consumed_once() {
        let mut input = InputState::default();
        input.key(KeyCode::KeyZ, true, false);
        input.key(KeyCode::KeyZ, false, false);
        let tick = input.take_tick();
        assert!(tick.jump_pressed && tick.jump_released);
        assert!(!tick.jump);
        assert_eq!(input.take_tick(), InputFrame::default());
    }

    #[test]
    fn repeat_does_not_repeat_edges_and_clear_releases_all_input() {
        let mut input = InputState::default();
        input.key(KeyCode::KeyZ, true, false);
        assert!(input.take_tick().jump_pressed);
        input.key(KeyCode::KeyZ, true, true);
        let tick = input.take_tick();
        assert!(tick.jump && !tick.jump_pressed);
        input.clear();
        assert_eq!(input.take_tick(), InputFrame::default());
    }

    #[test]
    fn rebinding_changes_physical_keys_and_ignores_invalid_values() {
        let mut input = InputState::default();
        input.set_bindings(&BTreeMap::from([
            ("jump".into(), "Space".into()),
            ("attack".into(), "Invalid".into()),
        ]));
        input.key(KeyCode::KeyZ, true, false);
        assert!(!input.take_tick().jump);
        input.key(KeyCode::Space, true, false);
        assert!(input.take_tick().jump_pressed);
        input.key(KeyCode::KeyX, true, false);
        assert!(input.take_tick().attack_pressed);
    }

    #[test]
    fn storage_default_bindings_generate_movement_and_jump() {
        let mut input = InputState::default();
        input.set_bindings(&crate::storage::Settings::default().bindings);
        for (key, direction) in [
            (KeyCode::ArrowLeft, Action::Left),
            (KeyCode::KeyA, Action::Left),
            (KeyCode::ArrowRight, Action::Right),
            (KeyCode::KeyD, Action::Right),
            (KeyCode::ArrowUp, Action::Up),
            (KeyCode::KeyW, Action::Up),
            (KeyCode::ArrowDown, Action::Down),
            (KeyCode::KeyS, Action::Down),
        ] {
            input.key(key, true, false);
            let tick = input.take_tick();
            assert!(
                match direction {
                    Action::Left => tick.left,
                    Action::Right => tick.right,
                    Action::Up => tick.up,
                    Action::Down => tick.down,
                    _ => unreachable!(),
                },
                "default key {key:?}"
            );
            input.key(key, false, false);
            input.take_tick();
        }
        for key in [KeyCode::KeyZ, KeyCode::Space] {
            input.key(key, true, false);
            assert!(input.take_tick().jump_pressed, "default key {key:?}");
            input.key(key, false, false);
            input.take_tick();
        }
    }

    #[test]
    fn alternate_binding_release_does_not_release_held_action() {
        let mut input = InputState::default();
        input.set_bindings(&BTreeMap::from([("jump".into(), "KeyZ, Space".into())]));
        input.key(KeyCode::KeyZ, true, false);
        assert!(input.take_tick().jump_pressed);
        input.key(KeyCode::Space, true, false);
        assert!(!input.take_tick().jump_pressed);
        input.key(KeyCode::KeyZ, false, false);
        let tick = input.take_tick();
        assert!(tick.jump && !tick.jump_released);
        input.key(KeyCode::Space, false, false);
        let tick = input.take_tick();
        assert!(!tick.jump && tick.jump_released);
    }

    #[test]
    fn empty_binding_unbinds_and_partial_invalid_list_keeps_valid_keys() {
        let mut input = InputState::default();
        input.set_bindings(&BTreeMap::from([
            ("jump".into(), "  ".into()),
            ("attack".into(), "Invalid, KeyJ, KeyJ".into()),
        ]));
        input.key(KeyCode::KeyZ, true, false);
        input.key(KeyCode::KeyX, true, false);
        let tick = input.take_tick();
        assert!(!tick.jump && !tick.attack);
        input.key(KeyCode::KeyJ, true, false);
        assert!(input.take_tick().attack_pressed);
    }

    #[test]
    fn controller_disconnect_preserves_keyboard_and_releases_controller() {
        let mut input = InputState::default();
        input.key(KeyCode::ArrowLeft, true, false);
        let mut actions = Actions::default();
        actions.set(Action::Jump, true);
        input.accept_gamepads(actions, 1 << 4);
        assert_eq!(input.ui_pressed(), vec![UiButton::Confirm]);
        assert!(input.take_tick().jump);
        input.accept_gamepads(Actions::default(), 0);
        let tick = input.take_tick();
        assert!(tick.left && tick.jump_released && !tick.jump);
        assert!(input.ui_pressed().is_empty());
    }

    #[test]
    fn screen_transition_keeps_controller_ui_buttons_latched_until_release() {
        let mut input = InputState::default();
        for (mask, button) in [(1 << 6, UiButton::Pause), (1 << 4, UiButton::Confirm)] {
            input.accept_gamepads(Actions::default(), mask);
            assert_eq!(input.ui_pressed(), vec![button]);
            input.clear_gameplay();
            input.accept_gamepads(Actions::default(), mask);
            assert!(input.ui_pressed().is_empty());
            input.accept_gamepads(Actions::default(), 0);
            input.accept_gamepads(Actions::default(), mask);
            assert_eq!(input.ui_pressed(), vec![button]);
            input.accept_gamepads(Actions::default(), 0);
        }
    }

    #[test]
    fn shared_action_is_held_until_both_devices_release() {
        let mut input = InputState::default();
        let mut actions = Actions::default();
        actions.set(Action::Jump, true);
        input.accept_gamepads(actions, 0);
        input.take_tick();
        input.key(KeyCode::KeyZ, true, false);
        input.accept_gamepads(Actions::default(), 0);
        let tick = input.take_tick();
        assert!(tick.jump && !tick.jump_pressed && !tick.jump_released);
    }
}
