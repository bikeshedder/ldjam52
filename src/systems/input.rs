use bevy::prelude::*;

use crate::components::player::{Player, PlayerInput};

pub fn player_input(
    key: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut player: Single<&mut Player>,
) {
    let mut input = PlayerInput::from_keys(&key);
    input.merge(gamepads.iter().map(PlayerInput::from_gamepad));
    player.input = input;
}
