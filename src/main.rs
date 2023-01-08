use bevy::prelude::*;
use components::player::Player;
use plugins::tiled;
use systems::{camera::camera_system, input::player_input, player::player_system};

use crate::tiled::TiledMapPlugin;

mod components;
mod plugins;
mod systems;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    /*
    commands.spawn((
        {
            let mut bundle = Camera2dBundle::default();
            let proj = &mut bundle.projection;
            proj.scaling_mode = ScalingMode::FixedHorizontal(1920.0);
            bundle
        },
    ));
    */
    commands.spawn((
        Player::default(),
        SpriteBundle {
            texture: asset_server.load("entities/player.png"),
            // FIXME use iso_to_screen function instead
            transform: Transform::from_xyz(0.0, 0.0, 100.0),
            ..Default::default()
        },
    ));

    let map_handle: Handle<tiled::TiledMap> = asset_server.load("map.tmx");

    commands.spawn(tiled::TiledMapBundle {
        tiled_map: map_handle,
        ..default()
    });
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: String::from("LDJAM52"),
                ..default()
            },
            ..default()
        }))
        .add_plugin(TiledMapPlugin)
        .add_startup_system(setup)
        .add_system(player_input)
        .add_system(player_system)
        .add_system(camera_system)
        .run();
}
