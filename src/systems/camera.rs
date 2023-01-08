use bevy::prelude::{Camera, Query, Transform, Without};

use crate::components::player::Player;

pub fn camera_system(
    mut camera_query: Query<(&Camera, &mut Transform, Without<Player>)>,
    player_query: Query<(&Player, &Transform, Without<Camera>)>,
) {
    let (_, player_transform, _) = player_query.single();
    if let Ok((_, mut transform, _)) = camera_query.get_single_mut() {
        //transform.translation.x = player_transform.translation.x.clamp(-1920.0, 1920.0);
        //transform.translation.y = player_transform.translation.y.clamp(-1080.0, 1080.0);
        transform.translation.x = player_transform.translation.x;
        transform.translation.y = player_transform.translation.y;
    }
}
