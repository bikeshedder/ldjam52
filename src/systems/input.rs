use bevy::prelude::*;

use crate::components::player::{Player, PlayerInput};

pub fn player_input(
    key: Res<Input<KeyCode>>,
    gamepads: Res<Gamepads>,
    gamepad_axis: Res<Axis<GamepadAxis>>,
    gamepad_button: Res<Input<GamepadButton>>,
    mut query: Query<&mut Player>,
) {
    let mut player = query.single_mut();
    let mut input = PlayerInput::from_keys(key);
    input.merge(
        gamepads
            .iter()
            .map(|gamepad| PlayerInput::from_gamepad(gamepad, &gamepad_axis, &gamepad_button)),
    );
    player.input = input;
}
