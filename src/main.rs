use bevy::{ecs::system::EntityCommands, prelude::*};
use components::{
    animation::{Animation, AnimationState},
    interaction::Interaction,
    player::Player,
};
use data::entity_types::{load_entity_types, EntityType, EntityTypes, Loaded};
use helpers::z_index;
use plugins::tiled;
use systems::{
    animation::{animation_system, AnimationTimer},
    camera::camera_system,
    input::player_input,
    player::player_system,
    textures::{check_textures, load_textures},
};

use crate::tiled::TiledMapPlugin;

mod components;
mod data;
mod helpers;
mod plugins;
mod systems;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    Loading,
    Game,
}

#[derive(Default, Resource)]
pub struct ImageHandles {
    handles: Vec<Handle<Image>>,
}

impl ImageHandles {
    pub fn add(&mut self, handle: Handle<Image>) -> usize {
        let index = self.handles.len();
        self.handles.push(handle);
        index
    }
}

fn spawn_entity(
    commands: &mut Commands,
    entity_type: &EntityType,
    translation: Vec3,
    animation_name: Option<&'static str>,
    f: fn(cmd: &mut EntityCommands),
) {
    let mut entity_cmds = match entity_type.loaded.as_ref().unwrap() {
        Loaded::Static(handle) => commands.spawn(SpriteBundle {
            texture: handle.clone(),
            transform: Transform {
                translation,
                ..Default::default()
            },
            ..Default::default()
        }),
        Loaded::Animations(animations) => {
            let cmd = commands.spawn((
                SpriteSheetBundle {
                    texture_atlas: animations.atlas.clone(),
                    // FIXME pass optional initial frame
                    sprite: TextureAtlasSprite {
                        index: animation_name
                            .map(|name| animations.frames[name][0].0)
                            .unwrap(),
                        ..Default::default()
                    },
                    transform: Transform {
                        translation,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Animation {
                    frames: animations.frames.clone(),
                },
                AnimationState {
                    animation: animation_name.unwrap(),
                    restart: true,
                    index: 0,
                },
                AnimationTimer::from_seconds(0.0, TimerMode::Repeating),
            ));
            cmd
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
    f(&mut entity_cmds);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, entity_types: Res<EntityTypes>) {
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

    spawn_entity(
        &mut commands,
        &entity_types["player"],
        Vec3::default(),
        Some("idle_down"),
        |cmd| {
            cmd.insert(Player::default());
        },
    );

    let map_handle: Handle<tiled::TiledMap> = asset_server.load("map.tmx");

    commands.spawn(tiled::TiledMapBundle {
        tiled_map: map_handle,
        ..default()
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entity_types = load_entity_types()?;
    App::new()
        .init_resource::<ImageHandles>()
        .insert_resource(entity_types)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: String::from("LDJAM52"),
                ..default()
            },
            ..default()
        }))
        .add_plugin(TiledMapPlugin)
        .add_state(AppState::Loading)
        .add_system_set(SystemSet::on_enter(AppState::Loading).with_system(load_textures))
        .add_system_set(SystemSet::on_update(AppState::Loading).with_system(check_textures))
        .add_system_set(SystemSet::on_enter(AppState::Game).with_system(setup))
        .add_system_set(
            SystemSet::on_update(AppState::Game)
                .with_system(player_input)
                .with_system(player_system)
                .with_system(camera_system)
                .with_system(animation_system),
        )
        .run();
    Ok(())
}
