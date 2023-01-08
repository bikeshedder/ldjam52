use bevy::{
    math::vec3,
    prelude::{Query, Res, Transform},
    sprite::TextureAtlasSprite,
    time::Time,
};

use crate::components::{
    animation::AnimationState,
    player::{Player, PlayerDirection, PlayerState},
};

pub const PLAYER_SPEED_X: f32 = 300.0;
pub const PLAYER_SPEED_Y: f32 = 150.0;

pub fn player_system(
    time: Res<Time>,
    mut query: Query<(
        &mut Player,
        &mut Transform,
        &mut AnimationState,
        &mut TextureAtlasSprite,
    )>,
) {
    let (mut player, mut transform, mut animation, mut sprite) = query.single_mut();
    let delta = time.delta().as_secs_f32();

    if player.is_moving() {
        player.state = PlayerState::Walk;
        player.direction = player.primary_direction();
    } else {
        player.state = PlayerState::Idle;
    }

    if player.input.interact {
        player.state = PlayerState::Interact;
    }

    if player.state == PlayerState::Walk {
        // move player to new position
        transform.translation = vec3(
            transform.translation.x + player.input.x * PLAYER_SPEED_X * delta,
            transform.translation.y + player.input.y * PLAYER_SPEED_Y * delta,
            512.0, // FIXME this needs to be updated depending on the world position
        );
    }

    animation.start(match (player.state, player.direction) {
        (PlayerState::Walk, PlayerDirection::NE) => "walk_up",
        (PlayerState::Walk, PlayerDirection::NW) => "walk_up",
        (PlayerState::Walk, PlayerDirection::SE) => "walk_down",
        (PlayerState::Walk, PlayerDirection::SW) => "walk_down",
        (_, PlayerDirection::NE) => "idle_up",
        (_, PlayerDirection::NW) => "idle_up",
        (_, PlayerDirection::SE) => "idle_down",
        (_, PlayerDirection::SW) => "idle_down",
    });

    sprite.flip_x = matches!(player.direction, PlayerDirection::NW | PlayerDirection::SW);
}
