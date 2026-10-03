//! Input for navigating menus and dialogues.

use bevy::{ecs::system::SystemParam, prelude::*};

#[derive(Default, Clone, Copy, Debug)]
pub struct MenuActions {
    pub up: bool,
    pub down: bool,
    pub confirm: bool,
    pub back: bool,
}

#[derive(SystemParam)]
pub struct MenuInput<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    gamepads: Query<'w, 's, &'static Gamepad>,
}

impl MenuInput<'_, '_> {
    pub fn read(&self) -> MenuActions {
        let keys = &self.keys;
        let mut actions = MenuActions {
            up: keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]),
            down: keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]),
            confirm: keys.any_just_pressed([KeyCode::Space, KeyCode::Enter]),
            back: keys.just_pressed(KeyCode::Escape),
        };
        for gamepad in &self.gamepads {
            actions.up |= gamepad.just_pressed(GamepadButton::DPadUp);
            actions.down |= gamepad.just_pressed(GamepadButton::DPadDown);
            actions.confirm |= gamepad.just_pressed(GamepadButton::South);
            actions.back |= gamepad.any_just_pressed([GamepadButton::Start, GamepadButton::East]);
        }
        actions
    }
}
