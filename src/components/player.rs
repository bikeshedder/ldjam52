use bevy::{
    input::{Axis, Input},
    math::Vec3,
    prelude::{
        Component, Gamepad, GamepadAxis, GamepadAxisType, GamepadButton, GamepadButtonType,
        KeyCode, Res,
    },
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PlayerState {
    Idle,
    Walk,
    Interact,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PlayerDirection {
    NE,
    NW,
    SE,
    SW,
}

#[derive(Debug, Default)]
pub struct PlayerInput {
    pub x: f32,
    pub y: f32,
    pub interact: bool,
    pub back: bool,
}

impl PlayerInput {
    pub fn from_keys(key: Res<Input<KeyCode>>) -> Self {
        let key_left = key_to_analog(&key, &[KeyCode::A, KeyCode::Left], -1.0);
        let key_right = key_to_analog(&key, &[KeyCode::D, KeyCode::Right], 1.0);
        let key_up = key_to_analog(&key, &[KeyCode::W, KeyCode::Up], 1.0);
        let key_down = key_to_analog(&key, &[KeyCode::S, KeyCode::Down], -1.0);
        Self {
            x: key_right + key_left,
            y: key_up + key_down,
            interact: key.pressed(KeyCode::Space),
            back: key.just_pressed(KeyCode::Escape),
        }
    }
    pub fn from_gamepad(
        gamepad: Gamepad,
        axis: &Res<Axis<GamepadAxis>>,
        button: &Res<Input<GamepadButton>>,
    ) -> Self {
        let axis_lx = GamepadAxis {
            gamepad,
            axis_type: GamepadAxisType::LeftStickX,
        };
        let axis_ly = GamepadAxis {
            gamepad,
            axis_type: GamepadAxisType::LeftStickY,
        };
        let dpad_left = GamepadButton {
            gamepad,
            button_type: GamepadButtonType::DPadLeft,
        };
        let dpad_right = GamepadButton {
            gamepad,
            button_type: GamepadButtonType::DPadRight,
        };
        let dpad_up = GamepadButton {
            gamepad,
            button_type: GamepadButtonType::DPadUp,
        };
        let dpad_down = GamepadButton {
            gamepad,
            button_type: GamepadButtonType::DPadDown,
        };
        let interact = GamepadButton {
            gamepad,
            button_type: GamepadButtonType::South,
        };
        let back = GamepadButton {
            gamepad,
            button_type: GamepadButtonType::East,
        };
        let dpadx = match (button.pressed(dpad_left), button.pressed(dpad_right)) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        };
        let dpady = match (button.pressed(dpad_up), button.pressed(dpad_down)) {
            (true, false) => 1.0,
            (false, true) => -1.0,
            _ => 0.0,
        };
        Self {
            x: (deadzone(axis.get(axis_lx).unwrap_or(0.0)) + dpadx).clamp(-1.0, 1.0),
            y: (deadzone(axis.get(axis_ly).unwrap_or(0.0)) + dpady).clamp(-1.0, 1.0),
            interact: button.pressed(interact),
            back: button.just_pressed(back),
        }
    }
    pub fn merge(&mut self, inputs: impl Iterator<Item = PlayerInput>) {
        for input in inputs {
            self.x += input.x;
            self.y += input.y;
            self.interact |= input.interact;
            self.back |= input.back;
        }
        self.x = self.x.clamp(-1.0, 1.0);
        self.y = self.y.clamp(-1.0, 1.0);
    }
}

fn deadzone(value: f32) -> f32 {
    if value.abs() > 0.2 {
        value
    } else {
        0.0
    }
}

fn key_to_analog(key: &Res<Input<KeyCode>>, codes: &[KeyCode], value: f32) -> f32 {
    let pressed = codes.iter().any(|&code| key.pressed(code));
    if pressed {
        value
    } else {
        0.0
    }
}

#[derive(Component, Debug)]
pub struct Player {
    pub input: PlayerInput,
    pub state: PlayerState,
    pub direction: PlayerDirection,
    pub center: Vec3,
}

impl Player {
    pub fn primary_direction(&self) -> PlayerDirection {
        match (self.input.x >= 0.0, self.input.y > 0.0) {
            (true, true) => PlayerDirection::NE,
            (false, true) => PlayerDirection::NW,
            (true, false) => PlayerDirection::SE,
            (false, false) => PlayerDirection::SW,
        }
    }
    pub fn is_moving(&self) -> bool {
        self.input.x != 0.0 || self.input.y != 0.0
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            input: PlayerInput::default(),
            state: PlayerState::Idle,
            direction: PlayerDirection::SE,
            center: Vec3::new(0.0, -40.0, 0.0),
        }
    }
}
