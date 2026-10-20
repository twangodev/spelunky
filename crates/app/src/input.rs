//! Keyboard/controller state and tick-consumed button edges.

use crate::controller::{self, Button as ControllerButton};
use spelunky_core::InputFrame;
use std::collections::{BTreeMap, HashMap, HashSet};
use winit::keyboard::KeyCode;

const DEAD_ZONE: f32 = 0.25;

// Classic's trigger mappings use joystick_zpos beyond +/-0.1 for both
// configuration and gameplay, independently of the provider's digital flag.
fn source_trigger_pressed(button: ControllerButton, pressed: bool, value: f32) -> bool {
    pressed
        || (matches!(
            button,
            ControllerButton::LeftTrigger | ControllerButton::RightTrigger
        ) && value > 0.1)
}

/// Gilrs exposes separate trigger axes on most controllers, normalized to
/// [-1, 1]. Some controllers expose the Classic signed Z axis as one mapping,
/// with its positive and negative halves assigned to the two triggers.
fn classic_trigger_axis_values(
    left_z_mapped: bool,
    right_z_mapped: bool,
    left_z: Option<f32>,
    right_z: Option<f32>,
) -> [f32; 2] {
    let separate_axis =
        |sample: Option<f32>| sample.map_or(0.0, |value| ((value + 1.0) * 0.5).clamp(0.0, 1.0));
    match (left_z_mapped, right_z_mapped) {
        (true, false) => {
            let value = left_z.unwrap_or(0.0);
            [value.max(0.0), (-value).max(0.0)]
        }
        (false, true) => [0.0, separate_axis(right_z)],
        (true, true) => [separate_axis(left_z), separate_axis(right_z)],
        (false, false) => [0.0, 0.0],
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, Default)]
struct BrowserButtonSample {
    pressed: bool,
    value: f32,
}

#[cfg(any(test, target_arch = "wasm32"))]
fn browser_button_pressed(button: ControllerButton, sample: BrowserButtonSample) -> bool {
    let threshold = if matches!(
        button,
        ControllerButton::LeftTrigger | ControllerButton::RightTrigger
    ) {
        0.1
    } else {
        0.5
    };
    sample.pressed || sample.value > threshold
}

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
    (Action::Bomb, "bomb", KeyCode::KeyA),
    (Action::Rope, "rope", KeyCode::KeyS),
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
    controller_enabled: bool,
    controller_initialized: bool,
    pending_startup_controller_sample: Option<(Actions, u8, u8)>,
    controller_bindings: BTreeMap<String, String>,
    gamepad_available: bool,
    primary_gamepad_connected: bool,
    controller_capture_held: bool,
    controller_capture_blocked: bool,
    controller_edges: Vec<ControllerButton>,
    blocked_gamepad_actions: Actions,
    keyboard_held: Actions,
    gamepad_held: Actions,
    frame: InputFrame,
    gamepad_ui: u8,
    // Cardinal joystick directions before Classic's diagonal holds are added.
    gamepad_direction_press_allowed: u8,
    blocked_gamepad_ui: u8,
    ui_edges: Vec<UiButton>,
    #[cfg(not(target_arch = "wasm32"))]
    gilrs: Option<gilrs::Gilrs>,
    #[cfg(not(target_arch = "wasm32"))]
    native_extra_button_codes: HashMap<gilrs::GamepadId, [Option<gilrs::ev::Code>; 3]>,
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
            controller_enabled: false,
            controller_initialized: false,
            pending_startup_controller_sample: None,
            controller_bindings: controller::default_bindings(),
            gamepad_available: false,
            primary_gamepad_connected: false,
            controller_capture_held: false,
            controller_capture_blocked: false,
            controller_edges: Vec::new(),
            blocked_gamepad_actions: Actions::default(),
            keyboard_held: Actions::default(),
            gamepad_held: Actions::default(),
            frame: InputFrame::default(),
            gamepad_ui: 0,
            gamepad_direction_press_allowed: 0x0f,
            blocked_gamepad_ui: 0,
            ui_edges: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            gilrs: gilrs::Gilrs::new().ok(),
            #[cfg(not(target_arch = "wasm32"))]
            native_extra_button_codes: HashMap::new(),
        }
    }
}

impl InputState {
    pub fn controller_enabled(&self) -> bool {
        self.controller_enabled
    }

    pub fn gamepad_available(&self) -> bool {
        self.gamepad_available
    }

    pub fn primary_gamepad_connected(&self) -> bool {
        self.primary_gamepad_connected
    }

    /// Independent physical source key; remapping does not own this edge.
    /// An inactive controller discards it rather than replaying after resume.
    pub fn music_key(
        &mut self,
        key: KeyCode,
        pressed: bool,
        repeat: bool,
        synthetic: bool,
        active: bool,
    ) {
        if key != KeyCode::KeyM {
            return;
        }
        if !active {
            self.frame.music_pressed = false;
            return;
        }
        if pressed && !repeat && !synthetic {
            self.frame.music_pressed = true;
        }
    }
    pub fn take_music_pressed(&mut self) -> bool {
        std::mem::take(&mut self.frame.music_pressed)
    }
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

    /// Query the validated keyboard alternatives, including default fallbacks.
    pub fn action_key(&self, action: &str, key: KeyCode) -> bool {
        self.bindings
            .get(action)
            .is_some_and(|keys| keys.contains(&key))
    }

    pub fn set_controller_bindings(&mut self, bindings: &BTreeMap<String, String>) {
        self.controller_bindings = controller::validated_bindings(bindings);
        self.clear();
    }

    /// Classic defaults controller input off and only polls gameplay/menu
    /// controls while its `global.gamepadOn` option is enabled.
    pub fn set_controller_enabled(&mut self, enabled: bool) {
        if self.controller_enabled == enabled {
            return;
        }
        self.controller_initialized = true;
        self.pending_startup_controller_sample = None;
        self.controller_enabled = enabled;
        if !enabled {
            self.blocked_gamepad_actions = Actions::default();
            self.blocked_gamepad_ui = 0;
            self.accept_gamepads(Actions::default(), 0);
        }
    }

    /// Applies the saved option during startup and replays the first controller
    /// sample if startup polling happened before the saved option was applied.
    /// Classic's first enabled gamepad step sees controls already held.
    pub fn initialize_controller_enabled(&mut self, enabled: bool) {
        self.controller_enabled = enabled;
        self.controller_initialized = true;
        if enabled {
            if let Some((actions, ui, press_allowed)) =
                self.pending_startup_controller_sample.take()
            {
                self.accept_gamepads_with_direction_edges(actions, ui, press_allowed);
            }
        } else {
            self.pending_startup_controller_sample = None;
        }
    }

    /// Raw physical controls are captured before menu navigation is translated
    /// to keyboard keys. Classic accepts one control per all-neutral stroke.
    pub fn controller_pressed(&mut self) -> Vec<ControllerButton> {
        std::mem::take(&mut self.controller_edges)
    }

    fn controller_binding(&self, action: &str) -> ControllerButton {
        controller::parse(&self.controller_bindings[action]).expect("validated controller binding")
    }

    fn mapped_controller_actions(&self, held: u32) -> Actions {
        let mut actions = Actions::default();
        for (action, name, _) in ACTIONS {
            if let Some(value) = self.controller_bindings.get(name) {
                if let Some(button) = controller::parse(value) {
                    actions.set(action, held & button.mask() != 0);
                }
            }
        }
        actions
    }

    fn portable_controller(&self, held: u32, directions: [bool; 4]) -> (Actions, u8) {
        let [left, right, up, down] = directions;
        let mut actions = self.mapped_controller_actions(held);
        actions.set(Action::Left, left);
        actions.set(Action::Right, right);
        actions.set(Action::Up, up);
        actions.set(Action::Down, down);
        let attack_button = self.controller_binding("attack");
        // Classic menus call checkAttackPressed(), so the controller's
        // confirmation edge follows the configured ACTION binding.
        let confirm = held & attack_button.mask() != 0;
        // East is a conventional extra cancel affordance in this port. When
        // it is rebound as ACTION, keep the source confirmation meaning only.
        let cancel =
            held & ControllerButton::East.mask() != 0 && attack_button != ControllerButton::East;
        let mut ui = 0;
        for (index, pressed) in [
            up,
            down,
            left,
            right,
            confirm,
            cancel,
            held & self.controller_binding("start").mask() != 0,
        ]
        .into_iter()
        .enumerate()
        {
            if pressed {
                ui |= 1 << index;
            }
        }
        (actions, ui)
    }

    fn portable_controller_with_direction_edges(
        &self,
        held: u32,
        directions: [bool; 4],
    ) -> (Actions, u8, u8) {
        let (actions, ui) = self.portable_controller(held, directions);
        let [left, right, up, down] = directions;
        let mut press_allowed = 0;
        if up && !left && !right {
            press_allowed |= 1 << 0;
        }
        if down && !left && !right {
            press_allowed |= 1 << 1;
        }
        if left && !up && !down {
            press_allowed |= 1 << 2;
        }
        if right && !up && !down {
            press_allowed |= 1 << 3;
        }
        (actions, ui, press_allowed)
    }

    /// Keep sampling raw browser button ordinals so users can bind them in the
    /// controller setup. Only the Standard Gamepad mapping identifies axes and
    /// d-pad button positions; an empty mapping cannot identify those controls.
    #[cfg(any(test, target_arch = "wasm32"))]
    fn accept_browser_controller_sample(
        &mut self,
        buttons: &[BrowserButtonSample],
        axes: [f32; 2],
        standard_mapping: bool,
    ) {
        let mut held = 0;
        let mut capture = 0;
        for button in controller::ALL {
            let sample = buttons.get(button as usize).copied().unwrap_or_default();
            if browser_button_pressed(button, sample) {
                held |= button.mask();
                capture |= button.mask();
            }
        }
        let directions = if standard_mapping {
            let [x, y] = axes;
            [
                x < -DEAD_ZONE || held & ControllerButton::DPadLeft.mask() != 0,
                x > DEAD_ZONE || held & ControllerButton::DPadRight.mask() != 0,
                y < -DEAD_ZONE || held & ControllerButton::DPadUp.mask() != 0,
                y > DEAD_ZONE || held & ControllerButton::DPadDown.mask() != 0,
            ]
        } else {
            [false; 4]
        };
        let (actions, ui, press_allowed) =
            self.portable_controller_with_direction_edges(held, directions);
        self.accept_controller_capture(capture);
        self.accept_controller_sample_with_direction_edges(actions, ui, press_allowed);
    }

    fn accept_controller_capture(&mut self, held: u32) {
        if held == 0 {
            self.controller_capture_blocked = false;
            self.controller_capture_held = false;
        } else if !self.controller_capture_held && !self.controller_capture_blocked {
            // Source scans physical buttons before its two trigger axes.
            let button = controller::ALL
                .into_iter()
                .filter(|button| {
                    !matches!(
                        button,
                        ControllerButton::LeftTrigger | ControllerButton::RightTrigger
                    )
                })
                .chain([
                    ControllerButton::LeftTrigger,
                    ControllerButton::RightTrigger,
                ])
                .find(|button| held & button.mask() != 0);
            if let Some(button) = button {
                self.controller_edges.push(button);
            }
            self.controller_capture_held = true;
        }
    }

    /// Source prompts select controller mode (`global.gamepadOn`), not the
    /// most recently used device.
    pub fn action_label(&self, action: &str) -> String {
        if self.controller_enabled {
            let controller_action = if action == "pause" { "start" } else { action };
            if let Some(value) = self.controller_bindings.get(controller_action) {
                if let Some(button) = controller::parse(value) {
                    return button.label().into();
                }
            }
            match action {
                "left" => return "LEFT".into(),
                "right" => return "RIGHT".into(),
                "up" | "exit" => return "UP".into(),
                "down" => return "DOWN".into(),
                _ => {}
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
        self.controller_edges.clear();
    }

    pub fn clear(&mut self) {
        self.clear_gameplay();
        self.gamepad_ui = 0;
        // Focus may have been lost before a controller press or release was
        // sampled. Require neutral input rather than treating the first poll
        // on return as a new menu confirmation or gameplay action.
        self.blocked_gamepad_ui = u8::MAX;
        self.blocked_gamepad_actions = Actions(u16::MAX);
        self.controller_capture_blocked = true;
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
        self.controller_edges.clear();
    }

    /// Pause deactivates the Classic instances without changing the physical
    /// keyboard state. Keep keyboard levels for the reactivated Step while
    /// dropping edges and requiring held controller actions to return neutral.
    pub fn clear_gameplay_preserving_keyboard(&mut self) {
        self.blocked_gamepad_actions.0 |= self.gamepad_actions.0;
        self.gamepad_actions = Actions::default();
        self.gamepad_held = Actions::default();
        self.frame = InputFrame {
            left: self.keyboard_held.has(Action::Left),
            right: self.keyboard_held.has(Action::Right),
            up: self.keyboard_held.has(Action::Up),
            down: self.keyboard_held.has(Action::Down),
            run: self.keyboard_held.has(Action::Run),
            jump: self.keyboard_held.has(Action::Jump),
            attack: self.keyboard_held.has(Action::Attack),
            ..InputFrame::default()
        };
        self.ui_edges.clear();
        self.controller_edges.clear();
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
        let direction_press_allowed = |action| match action {
            Action::Up => self.gamepad_direction_press_allowed & (1 << 0) != 0,
            Action::Down => self.gamepad_direction_press_allowed & (1 << 1) != 0,
            Action::Left => self.gamepad_direction_press_allowed & (1 << 2) != 0,
            Action::Right => self.gamepad_direction_press_allowed & (1 << 3) != 0,
            _ => true,
        };
        let pressed = |action| {
            (keyboard.has(action) && !self.keyboard_held.has(action))
                || (self.gamepad_actions.has(action)
                    && !self.gamepad_held.has(action)
                    && direction_press_allowed(action))
        };
        let source_diagonal_release = |action| {
            matches!(
                action,
                Action::Up | Action::Down | Action::Left | Action::Right
            ) && !direction_press_allowed(action)
        };
        let released = |action| {
            (!keyboard.has(action) && self.keyboard_held.has(action))
                || (self.gamepad_held.has(action)
                    && (!self.gamepad_actions.has(action) || source_diagonal_release(action)))
        };
        self.frame.left_released |= released(Action::Left);
        self.frame.right_released |= released(Action::Right);
        self.frame.up_pressed |= pressed(Action::Up);
        self.frame.down_pressed |= pressed(Action::Down);
        self.frame.attack_released |= released(Action::Attack);
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
        self.accept_gamepads_with_direction_edges(actions, ui, 0x0f);
    }

    fn accept_gamepads_with_direction_edges(
        &mut self,
        actions: Actions,
        ui: u8,
        press_allowed: u8,
    ) {
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
            if ui & (1 << index) != 0
                && self.gamepad_ui & (1 << index) == 0
                && (index >= 4 || press_allowed & (1 << index) != 0)
            {
                self.ui_edges.push(button);
            }
        }
        self.blocked_gamepad_actions.0 &= actions.0;
        self.gamepad_actions = Actions(actions.0 & !self.blocked_gamepad_actions.0);
        self.gamepad_ui = ui;
        self.gamepad_direction_press_allowed = press_allowed;
        self.refresh();
    }

    fn accept_controller_sample(&mut self, actions: Actions, ui: u8) {
        self.accept_controller_sample_with_direction_edges(actions, ui, 0x0f);
    }

    fn accept_controller_sample_with_direction_edges(
        &mut self,
        actions: Actions,
        ui: u8,
        press_allowed: u8,
    ) {
        if self.controller_enabled {
            self.accept_gamepads_with_direction_edges(actions, ui, press_allowed);
        } else {
            if !self.controller_initialized {
                self.pending_startup_controller_sample = Some((actions, ui, press_allowed));
            }
            self.accept_gamepads_with_direction_edges(Actions::default(), 0, 0x0f);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn update_gamepad(&mut self) {
        let Some(mut gilrs) = self.gilrs.take() else {
            self.gamepad_available = false;
            self.primary_gamepad_connected = false;
            self.accept_controller_capture(0);
            self.accept_controller_sample(Actions::default(), 0);
            return;
        };
        // Sampling after every queued event preserves a press/release pair that
        // arrives between two presentation frames.
        while gilrs.next_event().is_some() {
            let (actions, ui, press_allowed, capture) = self.native_gamepads(&gilrs);
            self.accept_controller_capture(capture);
            self.accept_controller_sample_with_direction_edges(actions, ui, press_allowed);
        }
        let (actions, ui, press_allowed, capture) = self.native_gamepads(&gilrs);
        self.gamepad_available = gilrs.gamepads().next().is_some();
        self.primary_gamepad_connected = self.gamepad_available;
        self.accept_controller_capture(capture);
        self.accept_controller_sample_with_direction_edges(actions, ui, press_allowed);
        self.gilrs = Some(gilrs);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn native_gamepads(&mut self, gilrs: &gilrs::Gilrs) -> (Actions, u8, u8, u32) {
        use gilrs::{Axis, Button};
        // Gilrs IDs are stable device identities, not Classic joystick slots:
        // unplugging a pad can leave a gap and later devices receive higher IDs.
        // Classic chooses the first two currently connected devices instead.
        let gamepads = first_two_connected_gamepads(gilrs.gamepads().map(|(_id, gamepad)| gamepad));
        let slots = gamepads
            .iter()
            .enumerate()
            .filter_map(|(slot, pad)| pad.is_some().then_some(slot));
        let Some(slot) = controller::classic_gamepad_slot(slots) else {
            return (Actions::default(), 0, 0x0f, 0);
        };
        let pad = gamepads[slot].expect("selected native gamepad slot is connected");
        let extra_codes = self.native_extra_button_codes(&pad);
        let mut capture = 0;
        let x = pad.value(Axis::LeftStickX);
        let y = pad.value(Axis::LeftStickY);
        let left = x < -DEAD_ZONE || pad.is_pressed(Button::DPadLeft);
        let right = x > DEAD_ZONE || pad.is_pressed(Button::DPadRight);
        let up = y > DEAD_ZONE || pad.is_pressed(Button::DPadUp);
        let down = y < -DEAD_ZONE || pad.is_pressed(Button::DPadDown);
        let [left_trigger_axis, right_trigger_axis] = classic_trigger_axis_values(
            pad.axis_code(Axis::LeftZ).is_some(),
            pad.axis_code(Axis::RightZ).is_some(),
            pad.axis_data(Axis::LeftZ).map(|axis| axis.value()),
            pad.axis_data(Axis::RightZ).map(|axis| axis.value()),
        );
        let mut held = 0;
        for button in controller::ALL {
            let native_data = native_controller_button(button)
                .and_then(|native| pad.button_data(native))
                .or_else(|| {
                    native_extra_slot(button)
                        .and_then(|slot| extra_codes[slot])
                        .and_then(|code| pad.state().button_data(code))
                });
            let native_pressed = native_controller_button(button)
                .map(|native| pad.is_pressed(native))
                .or_else(|| {
                    native_extra_slot(button)
                        .and_then(|slot| extra_codes[slot])
                        .map(|code| pad.state().is_pressed(code))
                })
                .unwrap_or(false);
            if native_data.is_none()
                && !matches!(
                    button,
                    ControllerButton::LeftTrigger | ControllerButton::RightTrigger
                )
            {
                continue;
            }
            let axis_value = match button {
                ControllerButton::LeftTrigger => left_trigger_axis,
                ControllerButton::RightTrigger => right_trigger_axis,
                _ => 0.0,
            };
            let value = native_data.map_or(0.0, |data| data.value()).max(axis_value);
            if source_trigger_pressed(button, native_pressed, value) {
                held |= button.mask();
            }
            let capturing = if matches!(
                button,
                ControllerButton::LeftTrigger | ControllerButton::RightTrigger
            ) {
                source_trigger_pressed(button, native_pressed, value)
            } else {
                native_pressed
            };
            if capturing {
                capture |= button.mask();
            }
        }
        let (actions, ui, press_allowed) =
            self.portable_controller_with_direction_edges(held, [left, right, up, down]);
        (actions, ui, press_allowed, capture)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn native_extra_button_codes(
        &mut self,
        pad: &gilrs::Gamepad<'_>,
    ) -> [Option<gilrs::ev::Code>; 3] {
        use gilrs::Button;

        let slots = self
            .native_extra_button_codes
            .entry(pad.id())
            .or_insert_with(|| [pad.button_code(Button::C), pad.button_code(Button::Z), None]);
        let mut unknown_codes: Vec<_> = pad
            .state()
            .buttons()
            .filter_map(|(code, _)| pad.axis_or_btn_name(code).is_none().then_some(code))
            .collect();
        unknown_codes.sort_by_key(gilrs::ev::Code::into_u32);
        fill_native_extra_button_slots(slots, unknown_codes);
        *slots
    }

    #[cfg(target_arch = "wasm32")]
    pub fn update_gamepad(&mut self) {
        use wasm_bindgen::JsCast;
        let actions = Actions::default();
        let ui = 0;
        let capture = 0;
        let direction_press_allowed = 0x0f;
        self.primary_gamepad_connected = false;
        self.gamepad_available = false;
        if let Some(pads) =
            web_sys::window().and_then(|window| window.navigator().get_gamepads().ok())
        {
            let slots: Vec<_> = [0_usize, 1]
                .into_iter()
                .filter(|slot| {
                    pads.get(*slot as u32)
                        .dyn_into::<web_sys::Gamepad>()
                        .is_ok_and(|pad| pad.connected())
                })
                .collect();
            self.primary_gamepad_connected =
                controller::classic_primary_gamepad_connected(slots.iter().copied());
            self.gamepad_available = slots.iter().any(|slot| *slot <= 1);
            if let Some(slot) = controller::classic_gamepad_slot(slots.iter().copied()) {
                let Ok(pad) = pads.get(slot as u32).dyn_into::<web_sys::Gamepad>() else {
                    self.accept_controller_capture(0);
                    self.accept_controller_sample_with_direction_edges(actions, ui, 0x0f);
                    return;
                };
                // Select by source slot before checking connection. Raw button
                // ordinals remain bindable, but only `standard` defines the
                // axis and d-pad indices consumed as movement below.
                if !pad.connected() {
                    self.accept_controller_capture(0);
                    self.accept_controller_sample_with_direction_edges(actions, ui, 0x0f);
                    return;
                }
                let buttons = pad.buttons();
                let samples: [BrowserButtonSample; controller::ALL.len()] =
                    std::array::from_fn(|index| {
                        buttons
                            .get(index as u32)
                            .dyn_into::<web_sys::GamepadButton>()
                            .map(|button| BrowserButtonSample {
                                pressed: button.pressed(),
                                value: button.value() as f32,
                            })
                            .unwrap_or_default()
                    });
                let axes = pad.axes();
                let x = axes.get(0).as_f64().unwrap_or(0.0) as f32;
                let y = axes.get(1).as_f64().unwrap_or(0.0) as f32;
                self.accept_browser_controller_sample(
                    &samples,
                    [x, y],
                    pad.mapping() == web_sys::GamepadMappingType::Standard,
                );
                return;
            }
        }
        self.accept_controller_capture(capture);
        self.accept_controller_sample_with_direction_edges(actions, ui, direction_press_allowed);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn first_two_connected_gamepads<T>(connected: impl IntoIterator<Item = T>) -> [Option<T>; 2] {
    let mut connected = connected.into_iter();
    [connected.next(), connected.next()]
}

#[cfg(not(target_arch = "wasm32"))]
fn native_controller_button(button: ControllerButton) -> Option<gilrs::Button> {
    use gilrs::Button;
    Some(match button {
        ControllerButton::South => Button::South,
        ControllerButton::East => Button::East,
        ControllerButton::West => Button::West,
        ControllerButton::North => Button::North,
        ControllerButton::LeftShoulder => Button::LeftTrigger,
        ControllerButton::RightShoulder => Button::RightTrigger,
        ControllerButton::LeftTrigger => Button::LeftTrigger2,
        ControllerButton::RightTrigger => Button::RightTrigger2,
        ControllerButton::Select => Button::Select,
        ControllerButton::Start => Button::Start,
        ControllerButton::LeftStick => Button::LeftThumb,
        ControllerButton::RightStick => Button::RightThumb,
        ControllerButton::DPadUp => Button::DPadUp,
        ControllerButton::DPadDown => Button::DPadDown,
        ControllerButton::DPadLeft => Button::DPadLeft,
        ControllerButton::DPadRight => Button::DPadRight,
        ControllerButton::Mode => Button::Mode,
        ControllerButton::Extra1 | ControllerButton::Extra2 | ControllerButton::Extra3 => {
            return None;
        }
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn native_extra_slot(button: ControllerButton) -> Option<usize> {
    match button {
        ControllerButton::Extra1 => Some(0),
        ControllerButton::Extra2 => Some(1),
        ControllerButton::Extra3 => Some(2),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fill_native_extra_button_slots<T: Copy + Eq>(
    slots: &mut [Option<T>; 3],
    codes: impl IntoIterator<Item = T>,
) {
    for code in codes {
        if slots.iter().flatten().any(|current| *current == code) {
            continue;
        }
        if let Some(slot) = slots.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(code);
        }
    }
}

fn parse_key(name: &str) -> Option<KeyCode> {
    serde_json::from_value(serde_json::Value::String(name.to_owned())).ok()
}

#[cfg(test)]
mod tests {
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_gamepad_slots_follow_connected_order_instead_of_sparse_ids() {
        // Gilrs retains IDs across hotplug, so connected IDs can begin at 2 or
        // have gaps. The first connected device is still Classic joystick 1.
        let candidates = first_two_connected_gamepads([2, 5, 7]);
        assert_eq!(candidates, [Some(2), Some(5)]);
        assert_eq!(
            crate::controller::classic_gamepad_slot(
                candidates
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, pad)| pad.map(|_| slot))
            ),
            Some(0)
        );
        let candidates = first_two_connected_gamepads([4]);
        assert_eq!(candidates, [Some(4), None]);
        assert_eq!(
            crate::controller::classic_gamepad_slot(
                candidates
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, pad)| pad.map(|_| slot))
            ),
            Some(0)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_extra_controller_slots_map_to_gilrs_extra_buttons() {
        assert_eq!(native_controller_button(ControllerButton::Extra1), None);
        assert_eq!(native_controller_button(ControllerButton::Extra2), None);
        assert_eq!(native_controller_button(ControllerButton::Extra3), None);
        assert_eq!(native_extra_slot(ControllerButton::Extra1), Some(0));
        assert_eq!(native_extra_slot(ControllerButton::Extra2), Some(1));
        assert_eq!(native_extra_slot(ControllerButton::Extra3), Some(2));

        let mut slots = [Some(10), Some(20), None];
        fill_native_extra_button_slots(&mut slots, [20, 30, 40]);
        assert_eq!(slots, [Some(10), Some(20), Some(30)]);

        let mut raw_slots = [None; 3];
        fill_native_extra_button_slots(&mut raw_slots, [30, 10]);
        assert_eq!(raw_slots, [Some(30), Some(10), None]);
    }

    #[test]
    fn host_input_audit_partial_triggers_preserve_mapped_edges_and_neutral_capture() {
        for trigger in [
            ControllerButton::LeftTrigger,
            ControllerButton::RightTrigger,
        ] {
            let saved = crate::storage::SaveData::from_json(&format!(
                r#"{{"settings":{{"controller_bindings":{{"jump":"{}","start":"{}"}}}}}}"#,
                trigger.name(),
                trigger.name()
            ))
            .unwrap();
            let mut input = InputState::default();
            input.set_controller_bindings(&saved.settings.controller_bindings);
            for (value, down, pressed, released) in [
                (0.0, false, false, false),
                (0.1, false, false, false),
                (0.2, true, true, false),
                (0.2, true, false, false),
                (0.1, false, false, true),
                (0.2, true, true, false),
            ] {
                let held = if source_trigger_pressed(trigger, false, value) {
                    trigger.mask()
                } else {
                    0
                };
                input.accept_controller_capture(held);
                let (actions, ui) = input.portable_controller(held, [false; 4]);
                input.accept_gamepads(actions, ui);
                let frame = input.take_tick();
                assert_eq!(frame.jump, down, "{trigger:?} at {value}");
                assert_eq!(frame.jump_pressed, pressed);
                assert_eq!(frame.jump_released, released);
                assert_eq!(input.ui_pressed().contains(&UiButton::Pause), pressed);
                assert_eq!(
                    input.controller_pressed(),
                    if pressed { vec![trigger] } else { vec![] }
                );
                assert!(!input.take_tick().jump_pressed);
            }
        }
    }

    #[test]
    fn host_input_audit_trigger_threshold_keeps_digital_controls_independent() {
        for trigger in [
            ControllerButton::LeftTrigger,
            ControllerButton::RightTrigger,
        ] {
            assert!(!source_trigger_pressed(trigger, false, 0.1));
            assert!(source_trigger_pressed(trigger, false, 0.2));
            assert!(source_trigger_pressed(trigger, true, 0.0));
        }
        for button in controller::ALL {
            if !matches!(
                button,
                ControllerButton::LeftTrigger | ControllerButton::RightTrigger
            ) {
                assert!(!source_trigger_pressed(button, false, 1.0));
                assert!(source_trigger_pressed(button, true, 0.0));
            }
        }
    }

    #[test]
    fn native_trigger_axes_accept_separate_and_signed_classic_z_layouts() {
        assert_eq!(
            classic_trigger_axis_values(false, false, None, None),
            [0.0, 0.0]
        );
        assert_eq!(
            classic_trigger_axis_values(true, false, Some(0.6), None),
            [0.6, 0.0]
        );
        assert_eq!(
            classic_trigger_axis_values(true, false, Some(-0.7), None),
            [0.0, 0.7]
        );
        assert_eq!(
            classic_trigger_axis_values(true, true, Some(-0.2), Some(0.6)),
            [0.4, 0.8]
        );
        assert_eq!(
            classic_trigger_axis_values(false, true, None, Some(0.8)),
            [0.0, 0.9]
        );
        // An axis mapping remains present before it has produced a sample.
        // A released unipolar trigger normalizes to zero rather than being
        // misidentified as the negative half of a signed shared axis.
        assert_eq!(
            classic_trigger_axis_values(true, true, Some(-1.0), None),
            [0.0, 0.0]
        );
        let [_, right] = classic_trigger_axis_values(true, false, Some(-0.1), None);
        assert!(!source_trigger_pressed(
            ControllerButton::RightTrigger,
            false,
            right
        ));
        let [left, right] = classic_trigger_axis_values(true, false, Some(-0.11), None);
        assert!(!source_trigger_pressed(
            ControllerButton::LeftTrigger,
            false,
            left
        ));
        assert!(source_trigger_pressed(
            ControllerButton::RightTrigger,
            false,
            right
        ));
    }

    #[test]
    fn startup_gamepad_availability_is_distinct_from_the_saved_preference() {
        assert!(!startup_controller_enabled(false, false));
        assert!(!startup_controller_enabled(false, true));
        assert!(!startup_controller_enabled(true, false));
        assert!(startup_controller_enabled(true, true));
    }

    #[test]
    fn startup_controller_mode_accepts_controls_already_held_at_launch() {
        let mut input = InputState::default();
        let mut actions = Actions::default();
        actions.set(Action::Jump, true);
        // App startup samples controller availability before applying the
        // saved option. Preserve that held sample for Classic's first step.
        input.accept_controller_sample(actions, 0);
        input.initialize_controller_enabled(true);
        let frame = input.take_tick();
        assert!(frame.jump);
        assert!(frame.jump_pressed);
    }

    #[test]
    fn browser_gamepad_unmapped_buttons_work_but_unknown_directions_are_not_guessed() {
        // The browser may expose a connected joystick with mapping="".
        // GameMaker still queries numbered buttons (Jump defaults to button 2),
        // so preserve button ordinals for the configurable action bindings.
        // The browser does not identify the stick or d-pad positions when its
        // mapping is empty, so those positions must not be treated as standard.
        let mut input = InputState::default();
        input.set_controller_enabled(true);
        input.accept_browser_controller_sample(&[], [0.0, 0.0], false);
        input.take_tick();
        let mut buttons = vec![BrowserButtonSample::default(); 2];
        buttons[1] = BrowserButtonSample {
            pressed: true,
            value: 1.0,
        };
        input.accept_browser_controller_sample(&buttons, [0.0, 0.0], false);
        let tick = input.take_tick();
        assert!(tick.jump && tick.jump_pressed);
        assert!(!tick.attack);
        assert_eq!(input.controller_pressed(), vec![ControllerButton::East]);

        input.accept_browser_controller_sample(&[], [0.0, 0.0], false);
        input.take_tick();
        let mut extra = vec![BrowserButtonSample::default(); 20];
        extra[19] = BrowserButtonSample {
            pressed: true,
            value: 1.0,
        };
        input.accept_browser_controller_sample(&extra, [0.0, 0.0], false);
        assert_eq!(input.controller_pressed(), vec![ControllerButton::Extra3]);

        input.accept_browser_controller_sample(&[], [0.0, -1.0], false);
        let tick = input.take_tick();
        assert!(!tick.jump);
        assert!(!tick.up);
        assert!(!tick.up_pressed);

        let mut unknown_dpad = vec![BrowserButtonSample::default(); 13];
        unknown_dpad[ControllerButton::DPadUp as usize] = BrowserButtonSample {
            pressed: true,
            value: 1.0,
        };
        input.accept_browser_controller_sample(&unknown_dpad, [0.0, 0.0], false);
        assert!(!input.take_tick().up);
        assert_eq!(input.controller_pressed(), vec![ControllerButton::DPadUp]);
        input.accept_browser_controller_sample(&[], [0.0, 0.0], false);
        input.take_tick();

        buttons[1] = BrowserButtonSample {
            pressed: false,
            value: 0.6,
        };
        input.accept_browser_controller_sample(&buttons, [0.0, 0.0], false);
        let tick = input.take_tick();
        assert!(tick.jump && tick.jump_pressed);
        assert_eq!(input.controller_pressed(), vec![ControllerButton::East]);

        input.accept_browser_controller_sample(&[], [0.0, 0.0], false);
        input.take_tick();
        let mut trigger = vec![BrowserButtonSample::default(); 7];
        trigger[ControllerButton::LeftTrigger as usize] = BrowserButtonSample {
            pressed: false,
            value: 0.2,
        };
        input.accept_browser_controller_sample(&trigger, [0.0, 0.0], false);
        let tick = input.take_tick();
        assert!(tick.bomb_pressed);
        assert_eq!(
            input.controller_pressed(),
            vec![ControllerButton::LeftTrigger]
        );
    }

    #[test]
    fn browser_standard_gamepad_mapping_uses_canonical_stick_and_dpad() {
        let mut input = InputState::default();
        input.set_controller_enabled(true);

        input.accept_browser_controller_sample(&[], [0.0, -1.0], true);
        let stick = input.take_tick();
        assert!(stick.up && stick.up_pressed);

        input.accept_browser_controller_sample(&[], [0.0, 0.0], true);
        input.take_tick();
        let mut dpad = vec![BrowserButtonSample::default(); 16];
        dpad[ControllerButton::DPadUp as usize] = BrowserButtonSample {
            pressed: true,
            value: 1.0,
        };
        input.accept_browser_controller_sample(&dpad, [0.0, 0.0], true);
        let pov = input.take_tick();
        assert!(pov.up && pov.up_pressed);
    }

    #[test]
    fn diagonal_directions_hold_motion_and_keep_source_release_edges() {
        for (diagonal, cardinal, is_up) in [
            (
                [false, true, true, false],
                [false, false, true, false],
                true,
            ),
            (
                [false, true, false, true],
                [false, false, false, true],
                false,
            ),
        ] {
            let mut input = InputState::default();
            let (actions, ui, edges) = input.portable_controller_with_direction_edges(0, diagonal);
            input.accept_gamepads_with_direction_edges(actions, ui, edges);
            let tick = input.take_tick();
            assert!(tick.right);
            assert_eq!(tick.up, is_up);
            assert_eq!(tick.down, !is_up);
            assert!(!tick.up_pressed && !tick.down_pressed);
            assert!(
                !tick.right_released,
                "first diagonal sample has no prior edge"
            );
            assert!(input.ui_pressed().is_empty());

            // Classic clears cardinal direction releases before applying its
            // diagonal holds. Right therefore stays held while also reporting
            // a release edge on every stable NE/SE sample.
            let (actions, ui, edges) = input.portable_controller_with_direction_edges(0, diagonal);
            input.accept_gamepads_with_direction_edges(actions, ui, edges);
            let tick = input.take_tick();
            assert!(tick.right && tick.right_released);
            assert_eq!(tick.up, is_up);
            assert_eq!(tick.down, !is_up);

            // Changing NE to N or SE to S keeps the vertical direction held.
            let (actions, ui, edges) = input.portable_controller_with_direction_edges(0, cardinal);
            input.accept_gamepads_with_direction_edges(actions, ui, edges);
            let tick = input.take_tick();
            assert!(!tick.right);
            assert!(tick.right_released);
            assert_eq!(tick.up, is_up);
            assert_eq!(tick.down, !is_up);
            assert!(!tick.up_pressed && !tick.down_pressed);
            assert!(input.ui_pressed().is_empty());

            let (actions, ui, edges) =
                input.portable_controller_with_direction_edges(0, [false; 4]);
            input.accept_gamepads_with_direction_edges(actions, ui, edges);
            input.take_tick();
            let (actions, ui, edges) = input.portable_controller_with_direction_edges(0, cardinal);
            input.accept_gamepads_with_direction_edges(actions, ui, edges);
            let tick = input.take_tick();
            assert_eq!(tick.up_pressed, is_up, "fresh cardinal Up press");
            assert_eq!(tick.down_pressed, !is_up, "fresh cardinal Down press");
            assert_eq!(
                input.ui_pressed(),
                vec![if is_up { UiButton::Up } else { UiButton::Down }]
            );
        }
    }

    #[test]
    fn physical_music_key_ignores_bindings_repeat_and_synthetic_replays() {
        let mut input = InputState::default();
        input.set_bindings(&BTreeMap::from([("jump".into(), "KeyM".into())]));
        input.music_key(KeyCode::KeyM, true, false, false, true);
        input.key(KeyCode::KeyM, true, false);
        input.key(KeyCode::KeyM, false, false);
        let first = input.take_tick();
        assert!(first.music_pressed && first.jump_pressed);
        assert!(!input.take_tick().music_pressed);
        for (repeat, synthetic) in [(true, false), (false, true)] {
            input.music_key(KeyCode::KeyM, true, repeat, synthetic, true);
            assert!(!input.take_tick().music_pressed);
        }
        input.music_key(KeyCode::KeyN, true, false, false, true);
        assert!(!input.take_tick().music_pressed);
        input.set_bindings(&BTreeMap::from([("jump".into(), "".into())]));
        input.music_key(KeyCode::KeyM, true, false, false, true);
        assert!(input.take_tick().music_pressed);
    }
    #[test]
    fn continuously_paused_music_edge_is_discarded_without_resume_replay() {
        let mut input = InputState::default();
        input.music_key(KeyCode::KeyM, true, false, false, false);
        input.music_key(KeyCode::KeyM, false, false, false, false);
        assert!(!input.take_music_pressed());
        assert!(!input.take_tick().music_pressed);
        // Resume alone does not synthesize a new physical event.
        assert!(!input.take_music_pressed());
        input.music_key(KeyCode::KeyM, true, false, false, true);
        assert!(input.take_music_pressed());
        assert!(!input.take_tick().music_pressed);
        input.music_key(KeyCode::KeyM, true, false, false, true);
        input.clear_edges();
        assert!(!input.take_tick().music_pressed);
    }
    use super::*;

    fn remap_sample(input: &mut InputState, held: u32) {
        let directions = [
            ControllerButton::DPadLeft,
            ControllerButton::DPadRight,
            ControllerButton::DPadUp,
            ControllerButton::DPadDown,
        ]
        .map(|button| held & button.mask() != 0);
        let (actions, ui) = input.portable_controller(held, directions);
        input.accept_controller_capture(held);
        input.accept_gamepads(actions, ui);
    }

    #[test]
    fn controller_remap_defaults_preserve_every_ordinary_button_and_menu_edge() {
        let mut input = InputState::default();
        for button in controller::ALL {
            remap_sample(&mut input, 0);
            input.take_tick();
            input.ui_pressed();
            remap_sample(&mut input, button.mask());
            let tick = input.take_tick();
            assert_eq!(tick.jump, button == ControllerButton::East);
            assert_eq!(tick.jump_pressed, button == ControllerButton::East);
            assert_eq!(tick.attack, button == ControllerButton::South);
            assert_eq!(tick.attack_pressed, button == ControllerButton::South);
            assert_eq!(tick.cycle_pressed, button == ControllerButton::West);
            assert_eq!(tick.run, button == ControllerButton::LeftShoulder);
            assert_eq!(tick.bomb_pressed, button == ControllerButton::LeftTrigger);
            assert_eq!(tick.rope_pressed, button == ControllerButton::RightTrigger);
            assert_eq!(tick.flare_pressed, button == ControllerButton::North);
            assert_eq!(tick.pay_pressed, button == ControllerButton::RightShoulder);
            assert_eq!(tick.left, button == ControllerButton::DPadLeft);
            assert_eq!(tick.right, button == ControllerButton::DPadRight);
            assert_eq!(tick.up, button == ControllerButton::DPadUp);
            assert_eq!(tick.down, button == ControllerButton::DPadDown);
            let expected = match button {
                ControllerButton::South => vec![UiButton::Confirm],
                ControllerButton::East => vec![UiButton::Cancel],
                ControllerButton::Start => vec![UiButton::Pause],
                ControllerButton::DPadLeft => vec![UiButton::Left],
                ControllerButton::DPadRight => vec![UiButton::Right],
                ControllerButton::DPadUp => vec![UiButton::Up],
                ControllerButton::DPadDown => vec![UiButton::Down],
                _ => vec![],
            };
            assert_eq!(input.ui_pressed(), expected);
            remap_sample(&mut input, button.mask());
            let repeat = input.take_tick();
            assert!(!repeat.jump_pressed && !repeat.attack_pressed && !repeat.pay_pressed);
            assert!(input.ui_pressed().is_empty());
        }
    }

    #[test]
    fn controller_remapped_attack_routes_menu_confirm_and_start_remains_remappable() {
        let mut input = InputState::default();
        input.set_controller_bindings(&BTreeMap::from([
            ("jump".into(), "East".into()),
            ("attack".into(), "West".into()),
            ("start".into(), "Mode".into()),
        ]));
        remap_sample(&mut input, 0);
        remap_sample(&mut input, ControllerButton::West.mask());
        let tick = input.take_tick();
        assert!(tick.attack && tick.attack_pressed && !tick.jump);
        assert_eq!(input.ui_pressed(), vec![UiButton::Confirm]);
        remap_sample(&mut input, 0);
        input.take_tick();
        remap_sample(&mut input, ControllerButton::South.mask());
        let tick = input.take_tick();
        assert!(!tick.attack && !tick.jump);
        assert!(input.ui_pressed().is_empty());
        remap_sample(&mut input, 0);
        input.take_tick();
        remap_sample(&mut input, ControllerButton::East.mask());
        let tick = input.take_tick();
        assert!(tick.jump && tick.jump_pressed && !tick.attack);
        assert_eq!(input.ui_pressed(), vec![UiButton::Cancel]);
        remap_sample(&mut input, 0);
        input.take_tick();
        remap_sample(&mut input, ControllerButton::Start.mask());
        assert!(input.ui_pressed().is_empty());
        remap_sample(&mut input, 0);
        remap_sample(&mut input, ControllerButton::Mode.mask());
        assert_eq!(input.ui_pressed(), vec![UiButton::Pause]);
        input.set_controller_enabled(true);
        assert_eq!(input.action_label("attack"), "WEST BUTTON");
        assert_eq!(input.action_label("pause"), "MODE");
        input.set_controller_enabled(false);
        assert_eq!(input.action_label("attack"), "X");
    }

    #[test]
    fn remapped_attack_on_east_does_not_also_emit_the_port_cancel_edge() {
        let mut input = InputState::default();
        input.set_controller_bindings(&BTreeMap::from([("attack".into(), "East".into())]));
        remap_sample(&mut input, 0);
        remap_sample(&mut input, ControllerButton::East.mask());
        assert!(input.take_tick().attack_pressed);
        assert_eq!(input.ui_pressed(), vec![UiButton::Confirm]);
    }

    #[test]
    fn controller_remap_preserves_independent_keyboard_release_and_duplicate_actions() {
        let mut input = InputState::default();
        input.set_controller_bindings(&BTreeMap::from([
            ("attack".into(), "East".into()),
            ("jump".into(), "East".into()),
        ]));
        remap_sample(&mut input, 0);
        input.key(KeyCode::KeyX, true, false);
        input.take_tick();
        remap_sample(&mut input, ControllerButton::East.mask());
        let tick = input.take_tick();
        assert!(tick.attack && tick.attack_pressed && tick.jump && tick.jump_pressed);
        remap_sample(&mut input, 0);
        let tick = input.take_tick();
        assert!(tick.attack && tick.attack_released && tick.jump_released && !tick.jump);
        assert!(!input.take_tick().attack_released);
        remap_sample(&mut input, ControllerButton::East.mask());
        input.take_tick();
        input.key(KeyCode::KeyX, false, false);
        let tick = input.take_tick();
        assert!(tick.attack && tick.attack_released && tick.jump);
    }

    #[test]
    fn controller_remap_capture_requires_all_neutral_and_prioritizes_buttons_before_triggers() {
        let mut input = InputState::default();
        let first = ControllerButton::South.mask() | ControllerButton::LeftTrigger.mask();
        input.accept_controller_capture(first);
        assert_eq!(input.controller_pressed(), vec![ControllerButton::South]);
        input.accept_controller_capture(first | ControllerButton::West.mask());
        input.accept_controller_capture(ControllerButton::West.mask());
        assert!(input.controller_pressed().is_empty());
        input.accept_controller_capture(0);
        input.accept_controller_capture(ControllerButton::LeftTrigger.mask());
        assert_eq!(
            input.controller_pressed(),
            vec![ControllerButton::LeftTrigger]
        );
        input.accept_controller_capture(0);
        input.accept_controller_capture(ControllerButton::RightTrigger.mask());
        assert_eq!(
            input.controller_pressed(),
            vec![ControllerButton::RightTrigger]
        );
        assert!(input.controller_pressed().is_empty());
    }

    #[test]
    fn controller_remap_capture_focus_and_screen_clear_require_release() {
        let mut input = InputState::default();
        let held = ControllerButton::South.mask();
        input.clear();
        input.accept_controller_capture(held);
        assert!(input.controller_pressed().is_empty());
        input.accept_controller_capture(0);
        input.accept_controller_capture(held);
        input.clear_edges();
        input.accept_controller_capture(held);
        assert!(input.controller_pressed().is_empty());
        input.accept_controller_capture(0);
        input.accept_controller_capture(held);
        input.clear_gameplay();
        input.accept_controller_capture(held);
        assert!(input.controller_pressed().is_empty());
        input.accept_controller_capture(0);
        input.accept_controller_capture(held);
        assert_eq!(input.controller_pressed(), vec![ControllerButton::South]);
    }

    #[test]
    fn controller_remap_capture_retains_short_strokes_and_consumes_them_once() {
        let mut input = InputState::default();
        input.accept_controller_capture(ControllerButton::South.mask());
        input.accept_controller_capture(0);
        input.accept_controller_capture(ControllerButton::West.mask());
        input.accept_controller_capture(0);
        assert_eq!(
            input.controller_pressed(),
            vec![ControllerButton::South, ControllerButton::West]
        );
        assert!(input.controller_pressed().is_empty());
    }

    fn mixed_source_floor() -> spelunky_core::Game {
        use spelunky_core::{Area, Game, PlayerState, RulesPreset, Vec2, World};
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world = World::new(40, 30, Area::Mines, 1);
        for x in 0..40 {
            game.world.set_tile(x, 10, "oBrick");
        }
        game.player.pos = Vec2::new(88.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game
    }

    #[test]
    fn mixed_source_horizontal_release_nudges_actual_step_while_other_device_holds() {
        for (action, key, direction) in [
            (Action::Left, KeyCode::ArrowLeft, -1.0),
            (Action::Right, KeyCode::ArrowRight, 1.0),
        ] {
            for release_keyboard in [false, true] {
                let mut game = mixed_source_floor();
                let mut input = InputState::default();
                let mut pad = Actions::default();
                pad.set(action, true);
                input.key(key, true, false);
                input.accept_gamepads(pad, 0);
                game.step(&input.take_tick());
                assert_eq!(
                    game.player.vel.x, 0.0,
                    "source waits two held ticks before acceleration"
                );
                if release_keyboard {
                    input.key(key, false, false);
                } else {
                    input.accept_gamepads(Actions::default(), 0);
                }
                let tick = input.take_tick();
                assert!(tick.left || tick.right);
                game.step(&tick);
                // characterStepEvent release acceleration0.5, ground friction0.6.
                assert!(
                    (game.player.vel.x - direction * 0.3).abs() < 1e-10,
                    "independent release missing: keyboard={release_keyboard} action={action:?} vel={:?}",
                    game.player.vel
                );
                let next = input.take_tick();
                assert!(!next.left_released && !next.right_released);
            }
        }
    }

    #[test]
    fn mixed_source_bow_release_fires_actual_step_while_other_device_still_attacks() {
        for release_keyboard in [false, true] {
            let mut game = mixed_source_floor();
            let mut input = InputState::default();
            let id = game.world.spawn("oBow", game.player.pos);
            game.player.held = Some(id);
            game.world.entity_mut(id).unwrap().held = true;
            game.data.insert("arrows".into(), 3.0);
            let mut pad = Actions::default();
            pad.set(Action::Attack, true);
            input.key(KeyCode::KeyX, true, false);
            input.accept_gamepads(pad, 0);
            game.step(&input.take_tick());
            assert_eq!(game.player.data.get("bow_armed"), Some(&1.0));
            let before = game.player.data["bow_strength"];
            if release_keyboard {
                input.key(KeyCode::KeyX, false, false);
            } else {
                input.accept_gamepads(Actions::default(), 0);
            }
            let tick = input.take_tick();
            assert!(tick.attack);
            game.step(&tick);
            let arrows: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oArrow")
                .collect();
            assert_eq!(
                arrows.len(),
                1,
                "source checkAttackReleased must fire while other device holds"
            );
            assert_eq!(game.data["arrows"], 2.0);
            assert_eq!(game.player.data["bow_armed"], 0.0);
            // Source charges0.2 before release, even in the same Step.
            assert!((arrows[0].vel.x.abs() - (1.0 + before + 0.2)).abs() < 1e-10);
            game.step(&input.take_tick());
            assert_eq!(game.data["arrows"], 2.0, "held other device fired twice");
        }
    }

    #[test]
    fn mixed_source_donation_fresh_press_changes_selection_while_other_device_holds() {
        for keyboard_first in [false, true] {
            for (action, key, initial, first, second) in [
                (Action::Up, KeyCode::ArrowUp, 0, 100, 200),
                (Action::Down, KeyCode::ArrowDown, 500, 400, 300),
            ] {
                let mut game = mixed_source_floor();
                game.world.level = 4;
                game.state = spelunky_core::RunState::Transition;
                game.player.money = 10_000;
                let mut ui = crate::donation_ui::TransitionDonation::begin(&game).unwrap();
                ui.amount = initial;
                let mut input = InputState::default();
                let mut pad = Actions::default();
                pad.set(action, true);
                if keyboard_first {
                    input.key(key, true, false);
                } else {
                    input.accept_gamepads(pad, 0);
                }
                ui.tick(&mut game, &input.take_tick());
                assert_eq!(ui.amount, first);
                if keyboard_first {
                    input.accept_gamepads(pad, 0);
                } else {
                    input.key(key, true, false);
                }
                ui.tick(&mut game, &input.take_tick());
                assert_eq!(ui.amount, second, "fresh second-device press was lost");
                ui.tick(&mut game, &input.take_tick());
                assert_eq!(ui.amount, second, "held selection repeated too early");
            }
        }
    }

    #[test]
    fn mixed_source_opposing_donation_directions_do_not_invent_a_press_when_priority_changes() {
        let mut game = mixed_source_floor();
        game.world.level = 4;
        game.state = spelunky_core::RunState::Transition;
        game.player.money = 10_000;
        let mut ui = crate::donation_ui::TransitionDonation::begin(&game).unwrap();
        ui.amount = 500;
        let mut input = InputState::default();
        input.key(KeyCode::ArrowUp, true, false);
        ui.tick(&mut game, &input.take_tick());
        assert_eq!(ui.amount, 600);
        let mut pad = Actions::default();
        pad.set(Action::Down, true);
        input.accept_gamepads(pad, 0);
        ui.tick(&mut game, &input.take_tick());
        assert_eq!(ui.amount, 500);
        ui.tick(&mut game, &input.take_tick());
        assert_eq!(ui.amount, 500);
        input.key(KeyCode::ArrowUp, false, false);
        ui.tick(&mut game, &input.take_tick());
        assert_eq!(
            ui.amount, 500,
            "held down became a fake press after up released"
        );
    }

    #[test]
    fn mixed_source_donation_short_up_tap_clamps_before_held_down_repeat_at_either_cap() {
        for (money, remaining) in [(1_000, 100_000), (5_000, 1_000)] {
            let mut game = mixed_source_floor();
            game.world.level = 4;
            game.state = spelunky_core::RunState::Transition;
            game.player.money = money;
            game.data
                .insert("tunnel1_remaining".into(), f64::from(remaining));
            let mut ui = crate::donation_ui::TransitionDonation::begin(&game).unwrap();
            let mut input = InputState::default();
            // Reach the cap through real short keyboard presses, without
            // injecting selection or held-repeat counters mid-sequence.
            for _ in 0..10 {
                input.key(KeyCode::ArrowUp, true, false);
                input.key(KeyCode::ArrowUp, false, false);
                ui.tick(&mut game, &input.take_tick());
            }
            assert_eq!(ui.amount, 1_000);
            let mut pad = Actions::default();
            pad.set(Action::Down, true);
            input.accept_gamepads(pad, 0);
            ui.tick(&mut game, &input.take_tick());
            assert_eq!(ui.amount, 900);
            for held_tick in 2..=21 {
                input.key(KeyCode::ArrowUp, true, false);
                input.key(KeyCode::ArrowUp, false, false);
                let tick = input.take_tick();
                assert!(tick.up_pressed && !tick.up && tick.down);
                ui.tick(&mut game, &tick);
                // Fresh Up caps at1000 before Down starts repeating on21.
                assert_eq!(ui.amount, if held_tick == 21 { 900 } else { 1_000 });
            }
            assert_eq!(game.player.money, money);
            assert_eq!(game.data["tunnel1_remaining"], f64::from(remaining));
        }
    }

    #[test]
    fn mixed_source_single_device_bow_release_boundary_and_empty_ammo_paths_stay_valid() {
        for keyboard in [false, true] {
            for ammo in [0.0, 3.0] {
                let mut game = mixed_source_floor();
                let mut input = InputState::default();
                let id = game.world.spawn("oBow", game.player.pos);
                game.player.held = Some(id);
                game.world.entity_mut(id).unwrap().held = true;
                game.data.insert("arrows".into(), ammo);
                let mut pad = Actions::default();
                pad.set(Action::Attack, true);
                if keyboard {
                    input.key(KeyCode::KeyX, true, false);
                } else {
                    input.accept_gamepads(pad, 0);
                }
                // Original Player Step1505 adds 0.2 only while below 12;
                // the final addition can cross that boundary.
                let mut source_strength = 0.0;
                for _ in 0..65 {
                    if source_strength < 12.0 {
                        source_strength += 0.2;
                    }
                    game.step(&input.take_tick());
                }
                if ammo > 0.0 {
                    assert_eq!(game.player.data["bow_strength"], source_strength);
                }
                if keyboard {
                    input.key(KeyCode::KeyX, false, false);
                } else {
                    input.accept_gamepads(Actions::default(), 0);
                }
                game.step(&input.take_tick());
                let arrows: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oArrow")
                    .collect();
                assert_eq!(arrows.len(), usize::from(ammo > 0.0));
                assert_eq!(
                    game.data["arrows"],
                    if ammo > 0.0 { ammo - 1.0 } else { 0.0 }
                );
                if ammo > 0.0 {
                    assert!((arrows[0].vel.x.abs() - (1.0 + source_strength)).abs() < 1e-10);
                }
            }
        }
    }

    #[test]
    fn mixed_source_short_press_repress_and_all_device_releases_are_tick_consumed_once() {
        let mut input = InputState::default();
        input.key(KeyCode::ArrowLeft, true, false);
        input.take_tick();
        input.key(KeyCode::ArrowLeft, false, false);
        input.key(KeyCode::ArrowLeft, true, false);
        input.key(KeyCode::ArrowDown, true, false);
        input.key(KeyCode::ArrowDown, false, false);
        input.key(KeyCode::KeyX, true, false);
        input.key(KeyCode::KeyX, false, false);
        let tick = input.take_tick();
        assert!(
            tick.left
                && tick.left_released
                && tick.down_pressed
                && !tick.down
                && tick.attack_pressed
                && tick.attack_released
                && !tick.attack
        );
        let next = input.take_tick();
        assert!(next.left);
        assert!(!next.left_released && !next.down_pressed && !next.attack_released);
        input.key(KeyCode::ArrowLeft, false, false);
        assert!(input.take_tick().left_released);
        input.clear_edges();
        assert!(!input.take_tick().left_released);
    }

    #[test]
    fn mixed_source_pay_edge_remains_independent_and_confirmation_pays_once() {
        let mut input = InputState::default();
        let mut pad = Actions::default();
        pad.set(Action::Pay, true);
        input.key(KeyCode::KeyP, true, false);
        assert!(input.take_tick().pay_pressed);
        input.accept_gamepads(pad, 0);
        assert!(input.take_tick().pay_pressed);
        assert!(!input.take_tick().pay_pressed);
        input.key(KeyCode::KeyP, false, false);
        input.key(KeyCode::KeyP, true, false);
        assert!(input.take_tick().pay_pressed);
        let mut game = mixed_source_floor();
        game.world.level = 4;
        game.state = spelunky_core::RunState::Transition;
        game.player.money = 500;
        let mut ui = crate::donation_ui::TransitionDonation::begin(&game).unwrap();
        input.key(KeyCode::ArrowUp, true, false);
        ui.tick(&mut game, &input.take_tick());
        assert_eq!(ui.amount, 100);
        input.key(KeyCode::KeyP, false, false);
        input.key(KeyCode::KeyP, true, false);
        assert_eq!(ui.tick(&mut game, &input.take_tick()), 100);
        assert_eq!(ui.tick(&mut game, &input.take_tick()), 0);
        assert_eq!(game.player.money, 400);
    }

    #[test]
    fn mixed_source_device_change_and_screen_clear_do_not_invent_new_edges() {
        let mut input = InputState::default();
        let mut pad = Actions::default();
        for action in [
            Action::Left,
            Action::Right,
            Action::Up,
            Action::Down,
            Action::Attack,
        ] {
            pad.set(action, true);
        }
        input.accept_gamepads(pad, 0);
        input.take_tick();
        input.clear_edges();
        let tick = input.take_tick();
        assert!(
            !tick.left_released
                && !tick.right_released
                && !tick.up_pressed
                && !tick.down_pressed
                && !tick.attack_released
        );
        input.clear_gameplay();
        input.accept_gamepads(pad, 0);
        assert_eq!(input.take_tick(), InputFrame::default());
        input.accept_gamepads(Actions::default(), 0);
        assert_eq!(input.take_tick(), InputFrame::default());
        input.accept_gamepads(pad, 0);
        let tick = input.take_tick();
        assert!(tick.up_pressed && tick.down_pressed && tick.attack_pressed);
        input.accept_gamepads(Actions::default(), 0);
        let tick = input.take_tick();
        assert!(tick.left_released && tick.right_released && tick.attack_released);
    }

    #[test]
    fn mixed_source_historical_serialized_frames_default_new_edges_to_false() {
        let mut old = serde_json::to_value(InputFrame::default()).unwrap();
        for name in [
            "left_released",
            "right_released",
            "up_pressed",
            "down_pressed",
            "attack_released",
        ] {
            old.as_object_mut().unwrap().remove(name);
        }
        assert_eq!(
            serde_json::from_value::<InputFrame>(old).unwrap(),
            InputFrame::default()
        );
    }

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
    fn enabled_controller_selects_prompts_without_a_button_or_last_device_rule() {
        let mut input = InputState::default();
        input.set_bindings(&BTreeMap::from([
            ("attack".into(), "KeyJ".into()),
            ("pay".into(), "".into()),
        ]));
        input.set_controller_enabled(true);
        assert_eq!(input.take_tick(), InputFrame::default());
        assert_eq!(input.action_label("attack"), "SOUTH BUTTON");
        assert_eq!(input.action_label("pay"), "RIGHT SHOULDER");
        input.key(KeyCode::KeyJ, true, false);
        assert_eq!(input.action_label("attack"), "SOUTH BUTTON");
        input.clear_gameplay();
        assert_eq!(input.action_label("attack"), "SOUTH BUTTON");
        input.set_controller_enabled(false);
        assert_eq!(input.action_label("attack"), "J");
        assert_eq!(input.action_label("pay"), "UNBOUND");
    }

    #[test]
    fn controller_enable_gates_gameplay_but_keeps_settings_capture_available() {
        let mut input = InputState::default();
        let south = ControllerButton::South.mask();
        let (actions, ui) = input.portable_controller(south, [false; 4]);
        input.accept_controller_capture(south);
        input.accept_controller_sample(actions, ui);
        let frame = input.take_tick();
        assert!(!frame.attack && !frame.attack_pressed);
        assert!(input.ui_pressed().is_empty());
        assert_eq!(input.controller_pressed(), vec![ControllerButton::South]);

        // A control held when the setting is enabled is applied on the next
        // sample, as it is on Classic's first enabled gamepad step.
        input.accept_controller_capture(0);
        input.set_controller_enabled(true);
        let (actions, ui) = input.portable_controller(south, [false; 4]);
        input.accept_controller_capture(south);
        input.accept_controller_sample(actions, ui);
        let frame = input.take_tick();
        assert!(frame.attack && frame.attack_pressed);
        assert_eq!(input.ui_pressed(), vec![UiButton::Confirm]);
        assert_eq!(input.controller_pressed(), vec![ControllerButton::South]);

        input.set_controller_enabled(false);
        let frame = input.take_tick();
        assert!(!frame.attack && frame.attack_released);
        assert!(input.ui_pressed().is_empty());
        let (actions, ui) = input.portable_controller(south, [false; 4]);
        input.accept_controller_sample(actions, ui);
        assert!(!input.take_tick().attack);
    }

    #[test]
    fn enabling_controller_input_applies_a_button_already_held() {
        let mut input = InputState::default();
        let mut held = Actions::default();
        held.set(Action::Jump, true);

        // While disabled, the physical controller is still sampled, but its
        // gameplay action is gated. Enabling mirrors Classic's next-step poll.
        input.accept_controller_sample(held, 0);
        assert!(!input.take_tick().jump);
        input.set_controller_enabled(true);
        input.accept_controller_sample(held, 0);

        let frame = input.take_tick();
        assert!(frame.jump);
        assert!(frame.jump_pressed);
    }

    #[test]
    fn capped_browser_keeps_controller_taps_sampled_on_skipped_raf_callbacks() {
        let mut input = InputState::default();
        let mut cadence = crate::timing::PresentationCadence::default();
        assert!(cadence.ready(0., Some(30)));
        for callback in 1..=8 {
            let mut actions = Actions::default();
            actions.set(Action::Jump, callback == 1);
            // The real web host samples controllers on both skipped RAFs and
            // rendered updates; these are browser-standard decoded snapshots.
            input.accept_gamepads(actions, if callback == 1 { 1 << 4 } else { 0 });
            if cadence.ready(f64::from(callback) / 240., Some(30)) {
                assert_eq!(callback, 8);
                let tick = input.take_tick();
                assert!(tick.jump_pressed && tick.jump_released && !tick.jump);
                assert_eq!(input.ui_pressed(), vec![UiButton::Confirm]);
                assert_eq!(input.take_tick(), InputFrame::default());
                assert!(input.ui_pressed().is_empty());
            }
        }
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
    fn extended_physical_keys_round_trip_through_settings_bindings() {
        for key in [
            KeyCode::F13,
            KeyCode::F35,
            KeyCode::IntlYen,
            KeyCode::NumpadHash,
        ] {
            assert!(supported_key(key), "{key:?}");
            let value = format!("{key:?}");
            let mut input = InputState::default();
            input.set_bindings(&BTreeMap::from([("jump".into(), value)]));
            input.key(key, true, false);
            assert!(input.take_tick().jump_pressed, "{key:?}");
        }
    }

    #[test]
    fn storage_default_bindings_generate_movement_and_jump() {
        let mut input = InputState::default();
        input.set_bindings(&crate::storage::Settings::default().bindings);
        for (key, direction) in [
            (KeyCode::ArrowLeft, Action::Left),
            (KeyCode::ArrowRight, Action::Right),
            (KeyCode::KeyD, Action::Right),
            (KeyCode::ArrowUp, Action::Up),
            (KeyCode::KeyW, Action::Up),
            (KeyCode::ArrowDown, Action::Down),
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
    fn classic_keyboard_bomb_and_rope_defaults_do_not_move_the_player() {
        let mut input = InputState::default();
        input.set_bindings(&crate::storage::Settings::default().bindings);

        input.key(KeyCode::KeyA, true, false);
        let bomb = input.take_tick();
        assert!(bomb.bomb_pressed);
        assert!(!bomb.left && !bomb.right && !bomb.up && !bomb.down);
        input.key(KeyCode::KeyA, false, false);
        input.take_tick();

        input.key(KeyCode::KeyS, true, false);
        let rope = input.take_tick();
        assert!(rope.rope_pressed);
        assert!(!rope.left && !rope.right && !rope.up && !rope.down);
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
    fn pause_controller_ui_edges_do_not_become_rebound_keyboard_bomb_or_rope() {
        for (action, key) in [("bomb", "ArrowDown"), ("rope", "Enter")] {
            let mut input = InputState::default();
            input.set_bindings(&BTreeMap::from([(action.into(), key.into())]));
            // set_bindings requires a neutral controller sample before new edges.
            input.accept_gamepads(Actions::default(), 0);
            let mut down = Actions::default();
            down.set(Action::Down, true);
            input.accept_gamepads(down, 1 << 1);
            assert_eq!(input.ui_pressed(), [UiButton::Down]);
            let frame = input.take_tick();
            assert!(frame.down_pressed);
            assert!(!frame.bomb_pressed && !frame.rope_pressed);
            input.accept_gamepads(Actions::default(), 0);
            input.take_tick();
            let mut confirm = Actions::default();
            confirm.set(Action::Jump, true);
            input.accept_gamepads(confirm, 1 << 4);
            assert_eq!(input.ui_pressed(), [UiButton::Confirm]);
            let frame = input.take_tick();
            assert!(frame.jump_pressed);
            assert!(!frame.bomb_pressed && !frame.rope_pressed);
            // A genuine rebound physical keyboard press still produces the action.
            input.key(parse_key(key).unwrap(), true, false);
            let frame = input.take_tick();
            assert_eq!(frame.bomb_pressed, action == "bomb");
            assert_eq!(frame.rope_pressed, action == "rope");
        }
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
    fn pause_deactivation_preserves_held_keyboard_levels_without_replaying_edges() {
        let mut input = InputState::default();
        input.key(KeyCode::ArrowRight, true, false);
        input.key(KeyCode::KeyZ, true, false);
        let entering_pause = input.take_tick();
        assert!(entering_pause.right && entering_pause.jump);
        assert!(entering_pause.jump_pressed);

        // Classic reactivates the same instances, so keyboard_check still sees
        // the physical key level on the next active Step.
        input.clear_gameplay_preserving_keyboard();

        let resumed = input.take_tick();
        assert!(resumed.right, "held ArrowRight should survive pause");
        assert!(resumed.jump, "held Z should survive pause");
        assert!(!resumed.jump_pressed, "resume must not synthesize a press");

        input.key(KeyCode::ArrowRight, false, false);
        let released = input.take_tick();
        assert!(!released.right && released.right_released);
    }

    #[test]
    fn pause_deactivation_keeps_controller_release_guard_and_keyboard_level() {
        let mut input = InputState::default();
        let mut actions = Actions::default();
        actions.set(Action::Jump, true);
        input.accept_gamepads(actions, 0);
        input.key(KeyCode::ArrowRight, true, false);
        input.take_tick();

        input.clear_gameplay_preserving_keyboard();
        let resumed = input.take_tick();
        assert!(resumed.right);
        assert!(!resumed.jump && !resumed.jump_pressed);

        input.accept_gamepads(actions, 0);
        assert!(!input.take_tick().jump);
        input.accept_gamepads(Actions::default(), 0);
        input.take_tick();
        input.accept_gamepads(actions, 0);
        assert!(input.take_tick().jump_pressed);
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

pub(crate) fn startup_controller_enabled(saved_setting: bool, gamepad_available: bool) -> bool {
    saved_setting && gamepad_available
}
