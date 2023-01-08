use std::{default, time::Duration};

use bevy::{animation, asset::LoadState, prelude::*, utils::HashMap};
use components::{
    animation::{Animation, AnimationBundle, AnimationState},
    player::Player,
};
use plugins::tiled;
use systems::{
    animation::{animation_system, AnimationTimer},
    camera::camera_system,
    input::player_input,
    player::player_system,
};

use crate::tiled::TiledMapPlugin;

mod components;
mod plugins;
mod systems;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    Loading,
    Game,
}

#[derive(Resource, Default)]
struct EntitySprites {
    handles: Vec<HandleUntyped>,
}

fn load_textures(mut entity_sprites: ResMut<EntitySprites>, asset_server: Res<AssetServer>) {
    entity_sprites.handles = asset_server.load_folder("entities/player").unwrap();
}

fn check_textures(
    mut state: ResMut<State<AppState>>,
    entity_sprites: ResMut<EntitySprites>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded =
        asset_server.get_group_load_state(entity_sprites.handles.iter().map(|handle| handle.id))
    {
        state.set(AppState::Game).unwrap();
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    entity_sprites: Res<EntitySprites>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut textures: ResMut<Assets<Image>>,
) {
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

    for handle in &entity_sprites.handles {
        log::info!("{:?}", handle);
    }

    let image_handles: Vec<Handle<Image>> = (0..=9)
        .map(|n| format!("entities/player/LD_23_Mouse_Walkcycle_{n:02}.png"))
        .map(|n| asset_server.get_handle(n))
        .collect::<Vec<_>>();

    let mut builder = TextureAtlasBuilder::default();
    for handle in image_handles {
        let image = textures.get(&handle).unwrap();
        builder.add_texture(handle, image);
    }

    let atlas = builder.finish(&mut textures).unwrap();
    let atlas_handle = texture_atlases.add(atlas);

    // let mut atlas_builder = TextureAtlasBuilder::default();
    // for handle in

    // atlas_builder
    //     .atlas_builder
    //     .add_texture(asset_server.load("entities/player/LD_23_Mouse_Walkcycle_00.png"));

    commands.spawn((
        Player::default(),
        SpriteSheetBundle {
            texture_atlas: atlas_handle,
            sprite: TextureAtlasSprite {
                index: 0,
                ..default()
            },
            // FIXME use iso_to_screen function instead
            transform: Transform::from_xyz(0.0, 0.0, 100.0),
            ..Default::default()
        },
        AnimationBundle::new(
            HashMap::from([(
                "walk_ne".to_owned(),
                (0..=9).map(|i| (i, Duration::from_millis(150))).collect(),
            )]),
            "walk_ne",
        ),
    ));

    let map_handle: Handle<tiled::TiledMap> = asset_server.load("map.tmx");

    commands.spawn(tiled::TiledMapBundle {
        tiled_map: map_handle,
        ..default()
    });
}

fn main() {
    App::new()
        .init_resource::<EntitySprites>()
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
}
