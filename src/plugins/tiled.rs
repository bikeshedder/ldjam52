// Source: https://github.com/StarArawn/bevy_ecs_tilemap/pull/381

// Limitations:
//   Some Tiled tilesets use a single image (a.k.a spritesheet) and then find the image based on
//   caclculated pixel offsets within that image. Other tilesets use a separate image per tile in
//   the tileset. This loader is compatible with either style but will not work with maps that mix
//   the two styles.
//   * Only finite tile layers are loaded. Infinite tile layers and object layers will be skipped.

use std::{
    io::{Cursor, ErrorKind},
    path::{Path, PathBuf},
};

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    platform::collections::HashMap,
    prelude::*,
    sprite::Anchor,
};

#[derive(Default)]
pub struct TiledMapPlugin;

impl Plugin for TiledMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<TiledMap>()
            .register_asset_loader(TiledLoader)
            .add_systems(Update, process_loaded_maps);
    }
}

#[derive(Asset, TypePath, Debug)]
pub struct TiledMap {
    pub map: tiled::Map,
    pub tilesets: Vec<TiledTileset>,
}

#[derive(Debug, Default)]
pub struct TiledTileset {
    pub images: HashMap<u32, Handle<Image>>,
}

/// Spawn an entity with this component to display a Tiled map. The tiles are
/// spawned as children of this entity once the map has been loaded.
#[derive(Component, Debug, Clone)]
#[require(Transform, Visibility)]
pub struct TiledMapHandle(pub Handle<TiledMap>);

#[derive(Debug, thiserror::Error)]
pub enum TiledLoaderError {
    #[error("Could not read TMX map: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not load TMX map: {0}")]
    Tiled(#[from] tiled::Error),
    #[error("Tilesets with a texture atlas are not supported")]
    TextureAtlasTileset,
}

/// A [`tiled::ResourceReader`] which serves the already read map file. External
/// resources (e.g. `.tsx` tilesets) are not supported.
struct BytesResourceReader<'a> {
    path: &'a Path,
    bytes: &'a [u8],
}

impl<'a> tiled::ResourceReader for BytesResourceReader<'a> {
    type Resource = Cursor<&'a [u8]>;
    type Error = std::io::Error;

    fn read_from(&mut self, path: &Path) -> Result<Self::Resource, Self::Error> {
        if path == self.path {
            Ok(Cursor::new(self.bytes))
        } else {
            Err(std::io::Error::new(
                ErrorKind::NotFound,
                format!("External resources are not supported: {}", path.display()),
            ))
        }
    }
}

#[derive(TypePath)]
pub struct TiledLoader;

impl AssetLoader for TiledLoader {
    type Asset = TiledMap;
    type Settings = ();
    type Error = TiledLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let path = PathBuf::from(load_context.path().path());
        let map = tiled::Loader::with_reader(BytesResourceReader {
            path: &path,
            bytes: &bytes,
        })
        .load_tmx_map(&path)?;
        let mut tilesets = Vec::new();
        for tileset in map.tilesets() {
            if tileset.image.is_some() {
                return Err(TiledLoaderError::TextureAtlasTileset);
            }
            let mut images = HashMap::new();
            for (tile_id, tile) in tileset.tiles() {
                if let Some(img) = &tile.image {
                    log::debug!(
                        "Loading tile image from {:?} as image ({tile_id})",
                        img.source
                    );
                    images.insert(tile_id, load_context.load(img.source.clone()));
                }
            }
            tilesets.push(TiledTileset { images });
        }
        log::info!("Loaded map: {}", load_context.path());
        Ok(TiledMap { map, tilesets })
    }

    fn extensions(&self) -> &[&str] {
        &["tmx"]
    }
}

pub fn process_loaded_maps(
    mut commands: Commands,
    mut map_events: MessageReader<AssetEvent<TiledMap>>,
    maps: Res<Assets<TiledMap>>,
    map_query: Query<(Entity, &TiledMapHandle)>,
    new_maps: Query<Entity, Added<TiledMapHandle>>,
) {
    let mut changed_maps = Vec::<AssetId<TiledMap>>::new();
    for event in map_events.read() {
        match event {
            AssetEvent::LoadedWithDependencies { id } => {
                log::info!("Map added!");
                changed_maps.push(*id);
            }
            AssetEvent::Modified { id } => {
                log::info!("Map changed!");
                changed_maps.push(*id);
            }
            AssetEvent::Removed { id } => {
                log::info!("Map removed!");
                // if the map was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_maps.retain(|changed_id| changed_id != id);
            }
            AssetEvent::Added { .. } | AssetEvent::Unused { .. } => {}
        }
    }

    for (map_entity, map_handle) in &map_query {
        // Map entities spawned after their map has finished loading won't receive an
        // asset event, so they are (re)built as soon as they are added.
        if !changed_maps.contains(&map_handle.0.id()) && !new_maps.contains(map_entity) {
            continue;
        }
        let Some(tiled_map) = maps.get(&map_handle.0) else {
            continue;
        };
        commands.entity(map_entity).despawn_related::<Children>();
        spawn_map_tiles(&mut commands, map_entity, tiled_map);
    }
}

fn spawn_map_tiles(commands: &mut Commands, map_entity: Entity, tiled_map: &TiledMap) {
    let map = &tiled_map.map;

    for (layer_index, layer) in map.layers().enumerate() {
        let tiled::LayerType::Tiles(tile_layer) = layer.layer_type() else {
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

        let layer_entity = commands
            .spawn((
                Transform::default(),
                Visibility::default(),
                ChildOf(map_entity),
            ))
            .id();

        for x in 0..map.width {
            for y in 0..map.height {
                let Some(layer_tile) = layer_data.get_tile(x as i32, y as i32) else {
                    continue;
                };
                let tiled_tileset = &tiled_map.tilesets[layer_tile.tileset_index()];
                let image_handle = tiled_tileset.images[&layer_tile.id()].clone();
                let tileset = layer_tile.get_tileset();
                commands.spawn((
                    Sprite {
                        image: image_handle,
                        flip_x: layer_tile.flip_h,
                        flip_y: layer_tile.flip_v,
                        // FIXME flip_d ?
                        ..default()
                    },
                    Anchor::BOTTOM_LEFT,
                    iso_to_screen(map, x, y, layer_index, tileset.offset_x, tileset.offset_y),
                    ChildOf(layer_entity),
                ));
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
