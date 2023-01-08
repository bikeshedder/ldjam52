// Source: https://github.com/StarArawn/bevy_ecs_tilemap/pull/381

// Limitations:
//   Some Tiled tilesets use a single image (a.k.a spritesheet) and then find the image based on
//   caclculated pixel offsets within that image. Other tilesets use a separate image per tile in
//   the tileset. This loader is compatible with either style but will not work with maps that mix
//   the two styles.
//   * Only finite tile layers are loaded. Infinite tile layers and object layers will be skipped.

use std::io::BufReader;

use bevy::prelude::{BuildChildren, SpatialBundle};
use bevy::sprite::{Anchor, Sprite, SpriteBundle};
use bevy::utils::default;
use bevy::{
    asset::{AssetLoader, AssetPath, LoadedAsset},
    log,
    prelude::{
        AddAsset, Added, AssetEvent, Assets, Bundle, Commands, Component, EventReader, Handle,
        Image, Plugin, Query, Res, Transform,
    },
    reflect::TypeUuid,
};

use anyhow::Result;

#[derive(Default)]
pub struct TiledMapPlugin;

impl Plugin for TiledMapPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_asset::<TiledMap>()
            .add_asset_loader(TiledLoader)
            .add_system(process_loaded_maps);
    }
}

#[derive(Debug, TypeUuid)]
#[uuid = "e51081d0-6168-4881-a1c6-4249b2000d7f"]
pub struct TiledMap {
    pub map: ::tiled::Map,
    pub tilesets: Vec<TiledTileset>,
}

#[derive(Debug, Default)]
pub struct TiledTileset {
    pub images: Vec<Handle<Image>>,
}

#[derive(Component, Default)]
pub struct TiledTile {}

#[derive(Default, Bundle)]
pub struct TiledMapBundle {
    pub tiled_map: Handle<TiledMap>,
    //pub transform: Transform,
    //pub global_transform: GlobalTransform,
    //pub visibility: Visibility,
}

pub struct TiledLoader;

impl AssetLoader for TiledLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let mut loader = tiled::Loader::new();
            let mut tilesets = Vec::<TiledTileset>::new();
            let mut dependencies = Vec::<AssetPath>::new();
            let map = loader
                .load_tmx_map_from(BufReader::new(bytes), load_context.path())
                .map_err(|e| anyhow::anyhow!("Could not load TMX map: {e}"))?;
            for tileset in map.tilesets().iter() {
                let mut tileset_images = Vec::<(u32, Handle<Image>)>::new();
                if tileset.image.is_some() {
                    panic!("Tilesets with a texture atlas are not supported");
                };
                for (tile_id, tile) in tileset.tiles() {
                    if let Some(img) = &tile.image {
                        let asset_path = AssetPath::new(img.source.clone(), None);
                        log::debug!("Loading tile image from {asset_path:?} as image ({tile_id})");
                        tileset_images.push((tile_id, load_context.get_handle(asset_path.clone())));
                        dependencies.push(asset_path);
                    }
                }
                tileset_images.sort();
                tilesets.push(TiledTileset {
                    images: tileset_images.into_iter().map(|(_, tile)| tile).collect(),
                });
            }
            log::info!("Loaded map: {}", load_context.path().display());
            let loaded_asset = LoadedAsset::new(TiledMap { map, tilesets });
            load_context.set_default_asset(loaded_asset.with_dependencies(dependencies));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["tmx"];
        EXTENSIONS
    }
}

pub fn process_loaded_maps(
    mut commands: Commands,
    mut map_events: EventReader<AssetEvent<TiledMap>>,
    maps: Res<Assets<TiledMap>>,
    new_maps: Query<&Handle<TiledMap>, Added<Handle<TiledMap>>>,
    mut map_query: Query<&Handle<TiledMap>>,
) {
    let mut changed_maps = Vec::<Handle<TiledMap>>::default();
    for event in map_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                log::info!("Map added!");
                changed_maps.push(handle.clone());
            }
            AssetEvent::Modified { handle } => {
                log::info!("Map changed!");
                changed_maps.push(handle.clone());
            }
            AssetEvent::Removed { handle } => {
                log::info!("Map removed!");
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_maps.retain(|changed_handle| changed_handle == handle);
            }
        }
    }

    // If we have new map entities add them to the changed_maps list.
    /* FIXME this causes an infinite loop
    for new_map_handle in new_maps.iter() {
        changed_maps.push(new_map_handle.clone_weak());
    }
     */

    for changed_map in changed_maps.iter() {
        for map_handle in map_query.iter_mut() {
            if map_handle != changed_map {
                continue;
            }
            let Some(tiled_map) = maps.get(map_handle) else {
                continue;
            };

            let map_entity = commands
                .spawn((
                    TiledMapBundle {
                        tiled_map: map_handle.clone(),
                        ..default()
                    },
                    SpatialBundle::default(),
                ))
                .id();

            let map = &tiled_map.map;

            for (layer_index, layer) in map.layers().enumerate() {
                let layer_entity = commands.spawn(SpatialBundle::default()).id();
                commands.entity(map_entity).add_child(layer_entity);

                let tiled::LayerType::TileLayer(tile_layer) = layer.layer_type() else {
                    log::info!(
                        "Skipping layer {} because only tile layers are supported.",
                        layer.id()
                    );
                    continue;
                };

                let tiled::TileLayer::Finite(layer_data) = tile_layer else {
                    log::info!(
                        "Skipping layer {} because only finite layers are supported.",
                        layer.id()
                    );
                    continue;
                };

                for x in 0..map.width {
                    for y in 0..map.height {
                        let Some(layer_tile) = layer_data.get_tile(x as i32, y as i32) else {
                            continue;
                        };
                        let tiled_tileset = &tiled_map.tilesets[layer_tile.tileset_index()];
                        let image_handle = tiled_tileset.images[layer_tile.id() as usize].clone();
                        let tileset = &tiled_map.map.tilesets()[layer_tile.tileset_index()];
                        commands.entity(map_entity).with_children(|parent| {
                            parent.spawn(SpriteBundle {
                                texture: image_handle,
                                transform: iso_to_screen(
                                    &map,
                                    x,
                                    y,
                                    layer_index,
                                    tileset.offset_x,
                                    tileset.offset_y,
                                ),
                                sprite: Sprite {
                                    flip_x: layer_tile.flip_h,
                                    flip_y: layer_tile.flip_v,
                                    anchor: Anchor::BottomLeft,
                                    // FIXME flip_d ?
                                    ..default()
                                },
                                ..default()
                            });
                        });
                    }
                }
            }
        }
    }
}

fn iso_to_screen(
    map: &tiled::Map,
    x: u32,
    y: u32,
    layer_index: usize,
    offset_x: i32,
    offset_y: i32,
) -> Transform {
    let x = x as f32;
    let y = y as f32;
    let z = layer_index as f32;
    let tile_width = map.tile_width as f32;
    let tile_height = map.tile_height as f32;
    let offset_x = offset_x as f32;
    let offset_y = offset_y as f32;
    Transform::from_xyz(
        ((x - y) * tile_width) / 2.0 + offset_x,
        -(((x + y) * tile_height) / 2.0 + offset_y),
        (x + y) + z * 10.0 + 64.0,
    )
}
