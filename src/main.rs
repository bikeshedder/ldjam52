use bevy::{
    camera::ScalingMode,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};
use components::{
    animation::{Animation, AnimationState},
    interaction::Interaction,
    player::Player,
};
use data::entity_types::{EntityType, EntityTypes, Loaded, load_entity_types};
use plugins::{
    menu::Menu,
    tiled::{TiledMapHandle, TiledMapPlugin},
};
use systems::{
    animation::{AnimationTimer, animation_system},
    camera::camera_system,
    input::player_input,
    interaction::InteractionText,
    player::player_system,
    textures::{ImageHandles, check_textures, load_textures},
};

mod components;
mod data;
mod helpers;
mod plugins;
mod systems;

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Loading,
    Menu,
    Game,
}

fn spawn_entity(
    commands: &mut Commands,
    entity_type: &EntityType,
    translation: Vec3,
    animation_name: Option<&'static str>,
    extra: impl Bundle,
) {
    let transform = Transform::from_translation(translation);
    let mut entity_cmds = match entity_type.loaded.as_ref().unwrap() {
        Loaded::Static(handle) => commands.spawn((Sprite::from_image(handle.clone()), transform)),
        Loaded::Animations(animations) => {
            let animation_name = animation_name.unwrap();
            commands.spawn((
                Sprite::from_atlas_image(
                    animations.image.clone(),
                    TextureAtlas {
                        layout: animations.layout.clone(),
                        index: animations.frames[animation_name][0].0,
                    },
                ),
                transform,
                Animation {
                    frames: animations.frames.clone(),
                },
                AnimationState {
                    animation: animation_name,
                    restart: true,
                    index: 0,
                },
                AnimationTimer::default(),
            ))
        }
        _ => unimplemented!(),
    };
    if let Some(interaction) = &entity_type.interaction {
        entity_cmds.insert(Interaction {
            name: interaction.name.clone(),
            center: Vec3::new(
                translation.x - f32::from(entity_type.size.width) / 2.0
                    + f32::from(interaction.position.x),
                translation.y + f32::from(entity_type.size.height) / 2.0
                    - f32::from(interaction.position.y),
                0.0,
            ),
            max_distance: interaction.max_distance,
        });
    }
    entity_cmds.insert(extra);
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedHorizontal {
                viewport_width: 1920.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn game_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    entity_types: Res<EntityTypes>,
) {
    spawn_entity(
        &mut commands,
        &entity_types["player"],
        Vec3::default(),
        Some("idle_down"),
        (Player::default(), DespawnOnExit(AppState::Game)),
    );

    commands.spawn((
        TiledMapHandle(asset_server.load("map.tmx")),
        DespawnOnExit(AppState::Game),
    ));

    commands.spawn((
        Text::new("Text Example"),
        TextFont {
            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
            font_size: 30.0.into(),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            margin: UiRect::all(Val::Px(5.0)),
            ..default()
        },
        InteractionText,
        DespawnOnExit(AppState::Game),
    ));
}

fn main() -> anyhow::Result<()> {
    let entity_types = load_entity_types()?;

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("LDJAM52"),
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
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<ImageHandles>()
        .insert_resource(entity_types)
        .add_systems(Startup, setup)
        .add_systems(OnEnter(AppState::Loading), load_textures)
        .add_systems(Update, check_textures.run_if(in_state(AppState::Loading)))
        .add_systems(OnEnter(AppState::Game), game_setup)
        .add_systems(
            Update,
            (player_input, player_system, animation_system, camera_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        )
        .run();
    Ok(())
}
