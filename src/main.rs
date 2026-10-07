// Don't open a console window next to the game in release builds on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{
    asset::AssetMetaCheck,
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
mod plugins;
mod systems;
mod version;

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
    if std::env::args().any(|arg| arg == "--version") {
        println!("ULU - The Harvest {}", version::long());
        return Ok(());
    }
    let entity_types = load_entity_types()?;

    let mut app = App::new();
    // Must be added before the `AssetPlugin` in `DefaultPlugins`
    #[cfg(feature = "embed")]
    app.add_plugins(bevy_embedded_assets::EmbeddedAssetPlugin {
        mode: bevy_embedded_assets::PluginMode::ReplaceDefault,
    });
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: String::from("ULU - The Harvest"),
                    // Don't wait for vsync where the platform allows it. The
                    // browser doesn't, so `Immediate` would fail there.
                    present_mode: PresentMode::AutoNoVsync,
                    // Draw into the canvas of `web/index.html` and fill the page
                    #[cfg(target_arch = "wasm32")]
                    canvas: Some("#game".into()),
                    #[cfg(target_arch = "wasm32")]
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                // There are no `.meta` files, so don't look for them. On the web
                // every lookup would be a failed HTTP request.
                meta_check: AssetMetaCheck::Never,
                ..default()
            }),
    )
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
    info!("ULU - The Harvest {}", version::long());
    if app.run().is_error() {
        std::process::exit(1);
    }
    Ok(())
}
