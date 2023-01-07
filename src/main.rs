use bevy::{prelude::*, render::camera::{ScalingMode}};
use bevy_ecs_tilemap::TilemapPlugin;
use components::{player::Player};
use systems::{input::player_input, player::player_system, camera::camera_system};

use crate::tiled::TiledMapPlugin;

mod components;
mod systems;
mod tiled;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        {
            let mut bundle = Camera2dBundle::default();
            let proj = &mut bundle.projection;
            proj.scaling_mode = ScalingMode::FixedHorizontal(1920.0);
            bundle
        },
    ));
    commands.spawn((
        Player::default(),
        SpriteBundle {
            texture: asset_server.load("entities/player.png"),
            ..Default::default()
        }
    ));

    let map_handle: Handle<tiled::TiledMap> = asset_server.load("maps/bedroom.tmx");

    commands.spawn(tiled::TiledMapBundle {
        tiled_map: map_handle,
        ..Default::default()
    });
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(TilemapPlugin)
        .add_plugin(TiledMapPlugin)
        .add_startup_system(setup)
        .add_system(player_input)
        .add_system(player_system)
        .add_system(camera_system)
        .run();
}
