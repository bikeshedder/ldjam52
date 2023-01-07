use bevy::{prelude::*, render::camera::ScalingMode};
use components::player::Player;
use systems::{input::player_input, player::player_system};

mod components;
mod systems;

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
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(player_input)
        .add_system(player_system)
        .run();
}
