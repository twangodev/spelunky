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
    gamepad_connected: bool,
    blocked_gamepad_actions: Actions,
    keyboard_held: Actions,
    gamepad_held: Actions,
    frame: InputFrame,
    gamepad_ui: u8,
    blocked_gamepad_ui: u8,
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
            gamepad_connected: false,
            blocked_gamepad_actions: Actions::default(),
            keyboard_held: Actions::default(),
            gamepad_held: Actions::default(),
            frame: InputFrame::default(),
            gamepad_ui: 0,
            blocked_gamepad_ui: 0,
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

    /// Source prompts select controller mode (`global.gamepadOn`), not the
    /// most recently used device. This port enables supported connected pads
    /// automatically, and labels their fixed mappings by physical position.
    pub fn action_label(&self, action: &str) -> String {
        if self.gamepad_connected {
            let label = match action {
                "left" => Some("LEFT"),
                "right" => Some("RIGHT"),
                "up" | "exit" => Some("UP"),
                "down" => Some("DOWN"),
                "jump" => Some("SOUTH BUTTON"),
                "attack" => Some("WEST BUTTON"),
                "bomb" => Some("LEFT TRIGGER"),
                "rope" => Some("RIGHT TRIGGER"),
                "run" => Some("LEFT SHOULDER"),
                "cycle" => Some("RIGHT SHOULDER"),
                "flare" => Some("NORTH BUTTON"),
                "pay" => Some("SELECT"),
                "pause" => Some("START"),
                _ => None,
            };
            if let Some(label) = label {
                return label.into();
            }
        }
        let Some(key) = self.bindings.get(action).and_then(|keys| keys.first()) else {
            return "UNBOUND".into();
        };
        use KeyCode::*;
        match key {
            ArrowUp => "UP ARR".into(),
            ArrowDown => "DOWN ARR".into(),
            ArrowLeft => "LEFT ARR".into(),
            ArrowRight => "RIGHT ARR".into(),
            ShiftLeft | ShiftRight => "SHIFT".into(),
            ControlLeft | ControlRight => "CTRL".into(),
            AltLeft | AltRight => "ALT".into(),
            Space => "SPACE".into(),
            Enter => "ENTER".into(),
            _ => {
                let name = format!("{key:?}");
                name.strip_prefix("Key")
                    .or_else(|| name.strip_prefix("Digit"))
                    .unwrap_or(&name)
                    .to_ascii_uppercase()
            }
        }
    }

    pub fn take_tick(&mut self) -> InputFrame {
        let frame = self.frame;
        self.frame.clear_edges();
        frame
    }

    /// Discard events at a continuous room transition while keeping physically
    /// held controls and controller menu latches. No synthetic presses result.
    pub fn clear_edges(&mut self) {
        self.frame.clear_edges();
        self.ui_edges.clear();
    }

    pub fn clear(&mut self) {
        self.clear_gameplay();
        self.gamepad_ui = 0;
        // Focus may have been lost before a controller press or release was
        // sampled. Require neutral input rather than treating the first poll
        // on return as a new menu confirmation or gameplay action.
        self.blocked_gamepad_ui = u8::MAX;
        self.blocked_gamepad_actions = Actions(u16::MAX);
    }

    /// Clear gameplay at screen transitions while keeping controller menu
    /// buttons latched until release, so holding Start cannot toggle repeatedly.
    pub fn clear_gameplay(&mut self) {
        self.keys.clear();
        // A controller is sampled continuously, unlike keyboard key-down
        // events. The button that confirmed a menu or was held while pausing
        // must be released before it can act in the new screen.
        self.blocked_gamepad_actions.0 |= self.gamepad_actions.0;
        self.gamepad_actions = Actions::default();
        self.keyboard_held = Actions::default();
        self.gamepad_held = Actions::default();
        self.frame = InputFrame::default();
        self.ui_edges.clear();
    }

    /// Controller navigation edges only; the host handles keyboard menu keys.
    pub fn ui_pressed(&mut self) -> Vec<UiButton> {
        std::mem::take(&mut self.ui_edges)
    }

    fn refresh(&mut self) {
        let mut keyboard = Actions::default();
        for (action, name, _) in ACTIONS {
            keyboard.set(
                action,
                self.bindings
                    .get(name)
                    .is_some_and(|keys| keys.iter().any(|key| self.keys.contains(key))),
            );
        }
        let next = Actions(keyboard.0 | self.gamepad_actions.0);
        // Classic's check*Pressed/Released scripts OR the keyboard and gamepad
        // edges independently. A fresh press on one device must survive even
        // when the other device already holds the same action.
        let pressed = |action| {
            (keyboard.has(action) && !self.keyboard_held.has(action))
                || (self.gamepad_actions.has(action) && !self.gamepad_held.has(action))
        };
        self.frame.jump_pressed |= pressed(Action::Jump);
        self.frame.jump_released |= (!keyboard.has(Action::Jump)
            && self.keyboard_held.has(Action::Jump))
            || (!self.gamepad_actions.has(Action::Jump) && self.gamepad_held.has(Action::Jump));
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
        self.keyboard_held = keyboard;
        self.gamepad_held = self.gamepad_actions;
    }

    fn accept_gamepads(&mut self, actions: Actions, ui: u8) {
        self.blocked_gamepad_ui &= ui;
        let ui = ui & !self.blocked_gamepad_ui;
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
        self.blocked_gamepad_actions.0 &= actions.0;
        self.gamepad_actions = Actions(actions.0 & !self.blocked_gamepad_actions.0);
        self.gamepad_ui = ui;
        self.refresh();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn update_gamepad(&mut self) {
        let Some(mut gilrs) = self.gilrs.take() else {
            self.gamepad_connected = false;
            self.accept_gamepads(Actions::default(), 0);
            return;
        };
        // Sampling after every queued event preserves a press/release pair that
        // arrives between two presentation frames.
        while gilrs.next_event().is_some() {
            let (actions, ui, connected) = Self::native_gamepads(&gilrs);
            self.gamepad_connected = connected;
            self.accept_gamepads(actions, ui);
        }
        let (actions, ui, connected) = Self::native_gamepads(&gilrs);
        self.gamepad_connected = connected;
        self.accept_gamepads(actions, ui);
        self.gilrs = Some(gilrs);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn native_gamepads(gilrs: &gilrs::Gilrs) -> (Actions, u8, bool) {
        use gilrs::{Axis, Button};
        let mut actions = Actions::default();
        let mut ui = 0;
        let mut connected = false;
        for (_, pad) in gilrs.gamepads() {
            if !pad.is_connected() {
                continue;
            }
            connected = true;
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
        (actions, ui, connected)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn update_gamepad(&mut self) {
        use wasm_bindgen::JsCast;
        let mut actions = Actions::default();
        let mut ui = 0;
        let mut connected = false;
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
                connected = true;
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
        self.gamepad_connected = connected;
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
    fn action_labels_use_validated_primary_bindings_and_show_unbound_actions() {
        let mut input = InputState::default();
        assert_eq!(input.action_label("attack"), "X");
        assert_eq!(input.action_label("pay"), "P");
        input.set_bindings(&BTreeMap::from([
            ("attack".into(), "Invalid, KeyJ, KeyX".into()),
            ("pay".into(), "".into()),
            ("jump".into(), "ShiftRight".into()),
        ]));
        assert_eq!(input.action_label("attack"), "J");
        assert_eq!(input.action_label("pay"), "UNBOUND");
        assert_eq!(input.action_label("jump"), "SHIFT");
        assert_eq!(input.action_label("up"), "UP ARR");
        assert_eq!(input.action_label("missing"), "UNBOUND");
        input.set_bindings(&BTreeMap::from([("attack".into(), "Invalid".into())]));
        assert_eq!(input.action_label("attack"), "X");
    }

    #[test]
    fn connected_controller_selects_prompts_without_a_button_or_last_device_rule() {
        let mut input = InputState::default();
        input.set_bindings(&BTreeMap::from([
            ("attack".into(), "KeyJ".into()),
            ("pay".into(), "".into()),
        ]));
        input.gamepad_connected = true;
        assert_eq!(input.take_tick(), InputFrame::default());
        assert_eq!(input.action_label("attack"), "WEST BUTTON");
        assert_eq!(input.action_label("pay"), "SELECT");
        input.key(KeyCode::KeyJ, true, false);
        assert_eq!(input.action_label("attack"), "WEST BUTTON");
        input.clear_gameplay();
        assert_eq!(input.action_label("attack"), "WEST BUTTON");
        input.gamepad_connected = false;
        assert_eq!(input.action_label("attack"), "J");
        assert_eq!(input.action_label("pay"), "UNBOUND");
    }

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
    fn controller_confirmation_cannot_jump_in_the_started_or_resumed_game() {
        use spelunky_core::{Area, Game, PlayerState, RulesPreset, Vec2, World};
        for rate in [60, 120, 144, 240] {
            let mut input = InputState::default();
            let mut actions = Actions::default();
            actions.set(Action::Jump, true);
            input.accept_gamepads(actions, 1 << 4);
            assert_eq!(input.ui_pressed(), [UiButton::Confirm]);
            // App::screen resets gameplay when Confirm starts or resumes play.
            input.clear_gameplay();
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.world = World::new(20, 20, Area::Mines, 1);
            for x in 0..20 {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.player.pos = Vec2::new(88., 152.);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Standing;
            game.player.grounded = true;
            let mut clock = crate::timing::FixedClock::new();
            for _ in 0..rate {
                input.accept_gamepads(actions, 1 << 4);
                assert!(input.ui_pressed().is_empty());
                for _ in 0..clock.advance(1. / f64::from(rate)).ticks {
                    game.step(&input.take_tick());
                    assert!(game.player.grounded, "held Confirm jumped at {rate} Hz");
                }
            }
            input.accept_gamepads(Actions::default(), 0);
            input.take_tick();
            input.accept_gamepads(actions, 1 << 4);
            game.step(&input.take_tick());
            assert!(
                game.player.vel.y < 0.,
                "a fresh press must jump at {rate} Hz"
            );
        }
    }

    #[test]
    fn paused_controller_actions_need_release_but_new_keyboard_input_remains_available() {
        let mut input = InputState::default();
        let actions = Actions(u16::MAX);
        input.accept_gamepads(actions, 0);
        input.take_tick();
        input.clear_gameplay();
        for _ in 0..10 {
            input.accept_gamepads(actions, 0);
            assert_eq!(input.take_tick(), InputFrame::default());
        }
        input.key(KeyCode::KeyZ, true, false);
        assert!(input.take_tick().jump_pressed);
        input.accept_gamepads(Actions::default(), 0);
        input.take_tick();
        input.accept_gamepads(actions, 0);
        let fresh = input.take_tick();
        assert!(fresh.jump_pressed && fresh.attack_pressed && fresh.bomb_pressed);
        assert!(fresh.rope_pressed && fresh.cycle_pressed && fresh.pay_pressed);
    }

    #[test]
    fn focus_return_does_not_resume_or_act_from_unobserved_controller_holds() {
        let mut input = InputState::default();
        input.clear(); // Focus loss; no controller state is known here.
        let mut actions = Actions::default();
        actions.set(Action::Jump, true);
        actions.set(Action::Attack, true);
        for _ in 0..5 {
            input.accept_gamepads(actions, (1 << 4) | (1 << 6));
            assert!(input.ui_pressed().is_empty());
            assert_eq!(input.take_tick(), InputFrame::default());
        }
        input.accept_gamepads(Actions::default(), 0);
        input.accept_gamepads(actions, (1 << 4) | (1 << 6));
        assert_eq!(input.ui_pressed(), [UiButton::Confirm, UiButton::Pause]);
        let tick = input.take_tick();
        assert!(tick.jump_pressed && tick.attack_pressed);
    }

    #[test]
    fn mixed_devices_preserve_independent_edges_and_combined_held_state() {
        let mut input = InputState::default();
        let mut actions = Actions::default();
        actions.set(Action::Jump, true);
        input.accept_gamepads(actions, 0);
        input.take_tick();
        input.key(KeyCode::KeyZ, true, false);
        let tick = input.take_tick();
        assert!(tick.jump && tick.jump_pressed && !tick.jump_released);
        input.accept_gamepads(Actions::default(), 0);
        let tick = input.take_tick();
        assert!(tick.jump && !tick.jump_pressed && tick.jump_released);
        input.key(KeyCode::KeyZ, false, false);
        let tick = input.take_tick();
        assert!(!tick.jump && tick.jump_released);
    }

    #[test]
    fn fast_repress_retains_fresh_edge_until_authoritative_tick_at_all_rates() {
        for rate in [60, 120, 144, 240] {
            let mut input = InputState::default();
            let mut clock = crate::timing::FixedClock::new();
            input.key(KeyCode::KeyZ, true, false);
            assert!(input.take_tick().jump_pressed);
            input.key(KeyCode::KeyZ, false, false);
            input.key(KeyCode::KeyZ, true, false);
            let mut consumed = false;
            for _ in 0..rate {
                let frame = clock.advance(1.0 / f64::from(rate));
                for _ in 0..frame.ticks {
                    let tick = input.take_tick();
                    assert!(tick.jump);
                    if !consumed {
                        assert!(tick.jump_pressed && tick.jump_released, "{rate} Hz");
                        consumed = true;
                    } else {
                        assert!(!tick.jump_pressed && !tick.jump_released);
                    }
                }
            }
            assert!(consumed);
        }
    }

    #[test]
    fn catch_up_ticks_consume_a_jump_edge_once_and_keep_held_jump() {
        let mut input = InputState::default();
        let mut clock = crate::timing::FixedClock::new();
        input.key(KeyCode::KeyZ, true, false);
        let frame = clock.advance(0.1);
        assert_eq!(frame.ticks, 3);
        for tick_index in 0..frame.ticks {
            let tick = input.take_tick();
            assert!(tick.jump);
            assert_eq!(tick.jump_pressed, tick_index == 0);
        }
    }

    #[test]
    fn room_transition_clears_edges_but_preserves_held_controls() {
        let mut input = InputState::default();
        input.key(KeyCode::ArrowRight, true, false);
        input.key(KeyCode::KeyZ, true, false);
        let mut actions = Actions::default();
        actions.set(Action::Attack, true);
        input.accept_gamepads(actions, 1 << 6);
        input.clear_edges();
        let tick = input.take_tick();
        assert!(tick.right && tick.jump && tick.attack);
        assert!(!tick.jump_pressed && !tick.attack_pressed);
        assert!(input.ui_pressed().is_empty());
        input.accept_gamepads(actions, 1 << 6);
        let tick = input.take_tick();
        assert!(tick.right && tick.jump && tick.attack);
        assert!(!tick.jump_pressed && !tick.attack_pressed);
        assert!(input.ui_pressed().is_empty());
        input.key(KeyCode::KeyZ, false, false);
        assert!(input.take_tick().jump_released);
    }

    #[test]
    fn host_fed_fast_repress_jumps_from_hanging_at_every_presentation_rate() {
        use spelunky_core::{Area, Game, PlayerState, RulesPreset, Vec2, World};
        let mut results = Vec::new();
        for rate in [60, 120, 144, 240] {
            let mut input = InputState::default();
            let mut clock = crate::timing::FixedClock::new();
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.world = World::new(20, 20, Area::Mines, 1);
            game.world.set_tile(6, 5, "oBlock");
            game.player.pos = Vec2::new(92.0, 88.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Hanging;
            input.key(KeyCode::KeyZ, true, false);
            input.take_tick(); // The preceding jump press has already been used.
            input.key(KeyCode::KeyZ, false, false);
            input.key(KeyCode::KeyZ, true, false);
            loop {
                let frame = clock.advance(1.0 / f64::from(rate));
                if frame.ticks == 0 {
                    continue;
                }
                game.step(&input.take_tick());
                break;
            }
            assert_eq!(game.player.state, PlayerState::Jumping, "{rate} Hz");
            assert!(game.player.vel.y < 0.0, "{rate} Hz");
            results.push((game.player.pos, game.player.vel));
        }
        assert!(results.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn fresh_keyboard_jump_exits_climb_while_controller_jump_is_held() {
        use spelunky_core::{Area, Game, PlayerState, RulesPreset, Vec2, World};
        let mut input = InputState::default();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(20, 20, Area::Mines, 1);
        for y in 4..=6 {
            game.world.set_tile(5, y, "oLadder");
        }
        game.player.pos = Vec2::new(88.0, 88.0);
        game.player.state = PlayerState::Climbing;
        let mut actions = Actions::default();
        actions.set(Action::Jump, true);
        input.accept_gamepads(actions, 0);
        input.take_tick();
        input.key(KeyCode::ArrowRight, true, false);
        input.key(KeyCode::KeyZ, true, false);
        game.step(&input.take_tick());
        assert_ne!(game.player.state, PlayerState::Climbing);
        assert!(game.player.vel.x > 0.0 && game.player.vel.y < 0.0);
    }
}
