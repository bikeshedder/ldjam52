use bevy::{
    camera::ScalingMode,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};
use data::entity_types::load_entity_types;
use game::{GamePlugin, Phase};
use plugins::{menu::Menu, tiled::TiledMapPlugin};
use systems::{
    animation::animation_system,
    camera::camera_system,
    input::player_input,
    player::player_system,
    textures::{ImageHandles, check_textures, load_textures},
};

mod components;
mod data;
#[cfg(feature = "dev")]
mod dev;
mod game;
mod helpers;
mod plugins;
mod systems;

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Loading,
    Menu,
    Game,
    Ending,
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedHorizontal {
                viewport_width: 1600.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn main() -> anyhow::Result<()> {
    let entity_types = load_entity_types()?;

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: String::from("ULU - Harvest"),
            present_mode: PresentMode::Immediate,
            ..default()
        }),
        ..default()
    }))
    .add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
        Menu,
        TiledMapPlugin,
    ))
    .init_state::<AppState>()
    .add_plugins(GamePlugin)
    .insert_resource(ClearColor(Color::BLACK))
    .init_resource::<ImageHandles>()
    .insert_resource(entity_types)
    .add_systems(Startup, setup)
    .add_systems(OnEnter(AppState::Loading), load_textures)
    .add_systems(Update, check_textures.run_if(in_state(AppState::Loading)))
    .add_systems(
        Update,
        (
            player_input.run_if(in_state(Phase::Exploring)),
            (player_system, animation_system, camera_system).run_if(in_state(AppState::Game)),
        )
            .chain(),
    );
    #[cfg(feature = "dev")]
    app.add_plugins(dev::scenario::ScenarioPlugin);
    if app.run().is_error() {
        std::process::exit(1);
    }
    Ok(())
}
