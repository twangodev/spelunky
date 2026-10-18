//! Portable controller buttons and the Classic control-wizard defaults.

use std::collections::BTreeMap;

/// Positions follow the browser standard gamepad layout, including its Mode
/// button. Native adapters translate their named buttons into these positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Button {
    South,
    East,
    West,
    North,
    LeftShoulder,
    RightShoulder,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Mode,
}

pub const ALL: [Button; 17] = [
    Button::South,
    Button::East,
    Button::West,
    Button::North,
    Button::LeftShoulder,
    Button::RightShoulder,
    Button::LeftTrigger,
    Button::RightTrigger,
    Button::Select,
    Button::Start,
    Button::LeftStick,
    Button::RightStick,
    Button::DPadUp,
    Button::DPadDown,
    Button::DPadLeft,
    Button::DPadRight,
    Button::Mode,
];

impl Button {
    pub fn name(self) -> &'static str {
        match self {
            Self::South => "South",
            Self::East => "East",
            Self::West => "West",
            Self::North => "North",
            Self::LeftShoulder => "LeftShoulder",
            Self::RightShoulder => "RightShoulder",
            Self::LeftTrigger => "LeftTrigger",
            Self::RightTrigger => "RightTrigger",
            Self::Select => "Select",
            Self::Start => "Start",
            Self::LeftStick => "LeftStick",
            Self::RightStick => "RightStick",
            Self::DPadUp => "DPadUp",
            Self::DPadDown => "DPadDown",
            Self::DPadLeft => "DPadLeft",
            Self::DPadRight => "DPadRight",
            Self::Mode => "Mode",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::South => "SOUTH BUTTON",
            Self::East => "EAST BUTTON",
            Self::West => "WEST BUTTON",
            Self::North => "NORTH BUTTON",
            Self::LeftShoulder => "LEFT SHOULDER",
            Self::RightShoulder => "RIGHT SHOULDER",
            Self::LeftTrigger => "LEFT TRIGGER",
            Self::RightTrigger => "RIGHT TRIGGER",
            Self::Select => "SELECT",
            Self::Start => "START",
            Self::LeftStick => "LEFT STICK",
            Self::RightStick => "RIGHT STICK",
            Self::DPadUp => "D-PAD UP",
            Self::DPadDown => "D-PAD DOWN",
            Self::DPadLeft => "D-PAD LEFT",
            Self::DPadRight => "D-PAD RIGHT",
            Self::Mode => "MODE",
        }
    }

    pub fn mask(self) -> u32 {
        1 << self as u8
    }
}

pub fn parse(name: &str) -> Option<Button> {
    ALL.into_iter().find(|button| button.name() == name)
}

/// Classic's B1..B10 wizard values translated to standard physical positions.
pub const ACTIONS: [(&str, &str, Button); 9] = [
    ("jump", "JUMP", Button::East),
    ("attack", "ACTION", Button::South),
    ("cycle", "SWITCH", Button::West),
    ("run", "RUN", Button::LeftShoulder),
    ("bomb", "BOMB", Button::LeftTrigger),
    ("rope", "ROPE", Button::RightTrigger),
    ("flare", "FLARE", Button::North),
    ("pay", "PURCHASE", Button::RightShoulder),
    ("start", "START", Button::Start),
];

pub fn default_bindings() -> BTreeMap<String, String> {
    ACTIONS
        .into_iter()
        .map(|(action, _, button)| (action.into(), button.name().into()))
        .collect()
}

/// Classic's `oGlobals` chooses joystick 1, using joystick 2 only when
/// joystick 1 is absent. Higher-index devices are never selected.
pub(crate) fn classic_gamepad_slot(
    connected_slots: impl IntoIterator<Item = usize>,
) -> Option<usize> {
    let mut primary_connected = false;
    let mut secondary_connected = false;
    for slot in connected_slots {
        match slot {
            0 => primary_connected = true,
            1 => secondary_connected = true,
            _ => {}
        }
    }
    if primary_connected {
        Some(0)
    } else if secondary_connected {
        Some(1)
    } else {
        None
    }
}

/// Missing or unsupported values keep the defaults. The source permits
/// duplicate assignments, but has no controller-unbind operation.
pub fn validated_bindings(bindings: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    ACTIONS
        .into_iter()
        .map(|(action, _, default)| {
            let button = bindings
                .get(action)
                .and_then(|name| parse(name))
                .unwrap_or(default);
            (action.into(), button.name().into())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_buttons_round_trip_and_match_standard_browser_indices() {
        for (index, button) in ALL.into_iter().enumerate() {
            assert_eq!(parse(button.name()), Some(button));
            assert_eq!(button.mask(), 1u32 << index);
            assert!(!button.label().is_empty());
        }
        for unsupported in ["", "B1", "Button18", "south"] {
            assert_eq!(parse(unsupported), None);
        }
    }

    #[test]
    fn classic_control_wizard_numbers_translate_to_standard_button_positions() {
        assert_eq!(
            default_bindings(),
            BTreeMap::from([
                ("jump".into(), "East".into()),
                ("attack".into(), "South".into()),
                ("cycle".into(), "West".into()),
                ("run".into(), "LeftShoulder".into()),
                ("bomb".into(), "LeftTrigger".into()),
                ("rope".into(), "RightTrigger".into()),
                ("flare".into(), "North".into()),
                ("pay".into(), "RightShoulder".into()),
                ("start".into(), "Start".into()),
            ])
        );
    }

    #[test]
    fn classic_gamepad_selection_prefers_pad_one_and_uses_pad_two_only_as_fallback() {
        assert_eq!(classic_gamepad_slot([0, 1]), Some(0));
        assert_eq!(classic_gamepad_slot([1, 0]), Some(0));
        assert_eq!(classic_gamepad_slot([1]), Some(1));
        assert_eq!(classic_gamepad_slot([2]), None);
        assert_eq!(classic_gamepad_slot([1, 2]), Some(1));
    }
}
