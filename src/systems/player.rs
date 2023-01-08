use bevy::{
    math::vec3,
    prelude::{Query, Res, Transform},
    time::Time,
};

use crate::components::player::{Player, PlayerDirection, PlayerState};

pub const PLAYER_SPEED: f32 = 600.0;

pub fn player_system(time: Res<Time>, mut query: Query<(&mut Player, &mut Transform)>) {
    let (mut player, mut transform) = query.single_mut();
    let delta = time.delta().as_secs_f32();

    if let Some(direction) = player.primary_direction() {
        player.direction = direction;
        match direction {
            PlayerDirection::Left => {
                player.state = PlayerState::Walk;
            }
            PlayerDirection::Right => {
                player.state = PlayerState::Walk;
            }
            PlayerDirection::Up => {
                player.state = PlayerState::Walk;
            }
            PlayerDirection::Down => {
                player.state = PlayerState::Walk;
            }
        }
    } else {
        player.state = PlayerState::Idle;
    }

    if player.input.interact {
        player.state = PlayerState::Interact;
    }

    if player.state == PlayerState::Walk {
        // move player to new position
        transform.translation = vec3(
            transform.translation.x + player.input.x * PLAYER_SPEED * delta,
            transform.translation.y + player.input.y * PLAYER_SPEED * delta,
            512.0, // FIXME this needs to be updated depending on the world position
        );
    }
}
