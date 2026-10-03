use bevy::prelude::*;

use crate::components::player::Player;

pub fn camera_system(
    mut camera_transform: Single<&mut Transform, (With<Camera>, Without<Player>)>,
    player_transform: Single<&Transform, With<Player>>,
) {
    //camera_transform.translation.x = player_transform.translation.x.clamp(-1920.0, 1920.0);
    //camera_transform.translation.y = player_transform.translation.y.clamp(-1080.0, 1080.0);
    camera_transform.translation.x = player_transform.translation.x;
    camera_transform.translation.y = player_transform.translation.y;
}
