//! Input for navigating menus and dialogues.

use bevy::{ecs::system::SystemParam, prelude::*};

#[derive(Default, Clone, Copy, Debug)]
pub struct MenuActions {
    pub up: bool,
    pub down: bool,
    pub confirm: bool,
    pub back: bool,
    /// Open the pause menu.
    pub pause: bool,
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
            pause: keys.just_pressed(KeyCode::Escape),
        };
        for gamepad in &self.gamepads {
            actions.up |= gamepad.just_pressed(GamepadButton::DPadUp);
            actions.down |= gamepad.just_pressed(GamepadButton::DPadDown);
            actions.confirm |= gamepad.just_pressed(GamepadButton::South);
            actions.back |= gamepad.any_just_pressed([GamepadButton::Start, GamepadButton::East]);
            actions.pause |= gamepad.just_pressed(GamepadButton::Start);
        }
        actions
    }
}

/// The input device the player used last. Input hints are shown for it.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputDevice {
    #[default]
    Keyboard,
    Gamepad,
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Confirm,
    Choose,
    Pause,
}

impl InputDevice {
    /// Label of the button for the given action.
    pub fn label(self, action: Action) -> &'static str {
        match (self, action) {
            (Self::Keyboard, Action::Confirm) => "[Space]",
            (Self::Keyboard, Action::Choose) => "[Up/Down]",
            (Self::Keyboard, Action::Pause) => "[Esc]",
            (Self::Gamepad, Action::Confirm) => "(A)",
            (Self::Gamepad, Action::Choose) => "(D-Pad)",
            (Self::Gamepad, Action::Pause) => "(Start)",
        }
    }
}

/// A text which depends on the [`InputDevice`]. It is updated whenever the
/// player switches between keyboard and gamepad.
#[derive(Component)]
pub struct DeviceText(pub fn(InputDevice) -> String);

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputDevice>()
            .add_systems(
                PreUpdate,
                detect_input_device.after(bevy::input::InputSystems),
            )
            .add_systems(Update, update_device_texts);
    }
}

fn detect_input_device(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    gamepads: Query<&Gamepad>,
    mut device: ResMut<InputDevice>,
) {
    let gamepad_used = gamepads.iter().any(|gamepad| {
        gamepad.get_just_pressed().next().is_some() || gamepad.left_stick().length() > 0.5
    });
    let keyboard_used =
        keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some();
    if gamepad_used {
        device.set_if_neq(InputDevice::Gamepad);
    } else if keyboard_used {
        device.set_if_neq(InputDevice::Keyboard);
    }
}

fn update_device_texts(device: Res<InputDevice>, mut texts: Query<(Ref<DeviceText>, &mut Text)>) {
    for (device_text, mut text) in &mut texts {
        if device.is_changed() || device_text.is_added() {
            text.0 = (device_text.0)(*device);
        }
    }
}
