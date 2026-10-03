use bevy::{
    input::{ButtonInput, gamepad::Gamepad},
    math::Vec2,
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
        let direction = limit(Vec2::new(key_right + key_left, key_up + key_down));
        Self {
            x: direction.x,
            y: direction.y,
            interact: key.any_just_pressed([KeyCode::Space, KeyCode::Enter]),
            back: key.just_pressed(KeyCode::Escape),
        }
    }
    pub fn from_gamepad(gamepad: &Gamepad) -> Self {
        let stick = gamepad.left_stick();
        let stick = Vec2::new(deadzone(stick.x), deadzone(stick.y));
        let direction = limit(stick + gamepad.dpad());
        Self {
            x: direction.x,
            y: direction.y,
            interact: gamepad.just_pressed(GamepadButton::South),
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
        let direction = limit(Vec2::new(self.x, self.y));
        self.x = direction.x;
        self.y = direction.y;
    }
}

/// Limits the length of a direction to 1, so moving diagonally with digital
/// inputs (keys, D-Pad) isn't faster than with a fully tilted analog stick.
fn limit(direction: Vec2) -> Vec2 {
    direction.clamp_length_max(1.0)
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
        }
    }
}
