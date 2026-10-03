use bevy::{
    input::{ButtonInput, gamepad::Gamepad},
    math::Vec3,
    prelude::{Component, GamepadButton, KeyCode},
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
    pub fn from_keys(key: &ButtonInput<KeyCode>) -> Self {
        let key_left = key_to_analog(key, &[KeyCode::KeyA, KeyCode::ArrowLeft], -1.0);
        let key_right = key_to_analog(key, &[KeyCode::KeyD, KeyCode::ArrowRight], 1.0);
        let key_up = key_to_analog(key, &[KeyCode::KeyW, KeyCode::ArrowUp], 1.0);
        let key_down = key_to_analog(key, &[KeyCode::KeyS, KeyCode::ArrowDown], -1.0);
        Self {
            x: key_right + key_left,
            y: key_up + key_down,
            interact: key.pressed(KeyCode::Space),
            back: key.just_pressed(KeyCode::Escape),
        }
    }
    pub fn from_gamepad(gamepad: &Gamepad) -> Self {
        let stick = gamepad.left_stick();
        let dpad = gamepad.dpad();
        Self {
            x: (deadzone(stick.x) + dpad.x).clamp(-1.0, 1.0),
            y: (deadzone(stick.y) + dpad.y).clamp(-1.0, 1.0),
            interact: gamepad.pressed(GamepadButton::South),
            back: gamepad.just_pressed(GamepadButton::East),
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
    if value.abs() > 0.2 { value } else { 0.0 }
}

fn key_to_analog(key: &ButtonInput<KeyCode>, codes: &[KeyCode], value: f32) -> f32 {
    let pressed = codes.iter().any(|&code| key.pressed(code));
    if pressed { value } else { 0.0 }
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
