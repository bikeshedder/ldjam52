// Source: https://github.com/StarArawn/bevy_ecs_tilemap/pull/381

// Limitations:
//   * Only tilesets using a separate image per tile are supported.
//   * Only finite tile layers are loaded. Infinite tile layers are skipped.
//   * Object layers are only rendered if they contain tile objects. Other objects
//     are left for the game to interpret.

use std::{
    io::{Cursor, ErrorKind},
    path::{Component as PathComponent, Path, PathBuf},
};

use bevy::{
    asset::{AssetLoader, LoadContext, ReadAssetBytesError, io::Reader},
    platform::collections::{HashMap, HashSet},
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

/// A single tile spawned from a map layer.
#[derive(Component, Debug, Clone)]
pub struct MapTile {
    pub layer: String,
    pub cell: IVec2,
    /// File name of the tile's image.
    pub image: String,
}

/// Depth for flat tiles (floors, carpets, ...) which are always drawn below upright ones.
pub fn flat_depth(layer_index: usize) -> f32 {
    layer_index as f32 * 0.1
}

/// Depth for upright sprites. `cell_sum` is the sum of the (fractional) map cell
/// coordinates, `bias` orders sprites standing in the same cell.
pub fn upright_depth(cell_sum: f32, bias: f32) -> f32 {
    10.0 + cell_sum * 10.0 + bias
}

/// Whether tiles with this image lie flat on the floor.
pub fn is_flat_image(image: &str) -> bool {
    image.contains("Glow_Floor")
}

/// Whether tiles with this image are small items lying on a table or shelf
/// (e.g. mugs). They don't block movement, the furniture below them does.
pub fn is_tabletop_image(image: &str) -> bool {
    image.contains("Dishes") || image.contains("BookPile")
}

/// Whether tiles with this image hang on the wall behind their cell (e.g.
/// paintings). They are drawn with the wall, so nothing can walk behind them.
pub fn is_wall_hanging_image(image: &str) -> bool {
    image.contains("Painting")
}

#[derive(Debug, thiserror::Error)]
pub enum TiledLoaderError {
    #[error("Could not read TMX map: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not read a resource of the TMX map: {0}")]
    ReadAsset(#[from] ReadAssetBytesError),
    #[error("Could not load TMX map: {0}")]
    Tiled(#[from] tiled::Error),
    #[error("Tilesets with a texture atlas are not supported")]
    TextureAtlasTileset,
}

/// Resolves `.` and `..` components, as the asset server expects normalized paths.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            PathComponent::ParentDir => {
                normalized.pop();
            }
            PathComponent::CurDir => {}
            other => normalized.push(other),
        }
    }
    normalized
}

/// A [`tiled::ResourceReader`] serving the map and the external tilesets which
/// were read ahead of time, as `tiled` can't read files asynchronously.
struct PrefetchedResources(HashMap<PathBuf, Vec<u8>>);

impl tiled::ResourceReader for PrefetchedResources {
    type Resource = Cursor<Vec<u8>>;
    type Error = std::io::Error;

    fn read_from(&mut self, path: &Path) -> Result<Self::Resource, Self::Error> {
        match self.0.get(&normalize_path(path)) {
            Some(bytes) => Ok(Cursor::new(bytes.clone())),
            None => Err(std::io::Error::new(
                ErrorKind::NotFound,
                format!("Resource was not prefetched: {}", path.display()),
            )),
        }
    }
}

/// Finds the external tilesets referenced by a map.
fn external_tilesets(tmx: &str) -> impl Iterator<Item = &str> {
    tmx.split("source=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .filter(|source| source.ends_with(".tsx"))
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
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();

        let mut resources = HashMap::new();
        let tilesets: Vec<PathBuf> = external_tilesets(&String::from_utf8_lossy(&bytes))
            .map(|source| normalize_path(&dir.join(source)))
            .collect();
        for tileset in tilesets {
            let tileset_bytes = load_context.read_asset_bytes(tileset.clone()).await?;
            resources.insert(tileset, tileset_bytes);
        }
        resources.insert(path.clone(), bytes);

        let map = tiled::Loader::with_reader(PrefetchedResources(resources)).load_tmx_map(&path)?;
        let mut tilesets = Vec::new();
        for tileset in map.tilesets() {
            if tileset.image.is_some() {
                return Err(TiledLoaderError::TextureAtlasTileset);
            }
            let mut images = HashMap::new();
            for (tile_id, tile) in tileset.tiles() {
                if let Some(img) = &tile.image {
                    images.insert(tile_id, load_context.load(normalize_path(&img.source)));
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
            AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id } => {
                changed_maps.push(*id);
            }
            AssetEvent::Removed { id } => {
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

fn image_name(tile: Option<tiled::Tile>) -> String {
    tile.and_then(|tile| tile.image.as_ref().map(|image| image.source.clone()))
        .and_then(|source| {
            source
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_default()
}

fn spawn_map_tiles(commands: &mut Commands, map_entity: Entity, tiled_map: &TiledMap) {
    let map = &tiled_map.map;
    let walls = wall_cells(map);

    for (layer_index, layer) in map.layers().enumerate() {
        let layer_entity = commands
            .spawn((
                Transform::default(),
                Visibility::default(),
                ChildOf(map_entity),
            ))
            .id();
        match layer.layer_type() {
            tiled::LayerType::Tiles(tiled::TileLayer::Finite(layer_data)) => {
                let walls = walls
                    .as_ref()
                    .filter(|(walls_index, _)| layer_index > *walls_index)
                    .map(|(_, cells)| cells);
                spawn_tile_layer(
                    commands,
                    layer_entity,
                    tiled_map,
                    &layer,
                    &layer_data,
                    layer_index,
                    walls,
                );
            }
            tiled::LayerType::Objects(objects) => {
                spawn_tile_objects(commands, layer_entity, tiled_map, &objects, layer_index);
            }
            _ => log::info!(
                "Skipping layer {} because only finite tile and object layers are supported.",
                layer.id()
            ),
        }
    }
}

/// Index of the layer called `Walls` and the cells covered by it.
fn wall_cells(map: &tiled::Map) -> Option<(usize, HashSet<IVec2>)> {
    map.layers().enumerate().find_map(|(index, layer)| {
        let tiled::LayerType::Tiles(tiled::TileLayer::Finite(data)) = layer.layer_type() else {
            return None;
        };
        (layer.name == "Walls").then(|| {
            let cells = (0..map.height as i32)
                .flat_map(|y| (0..map.width as i32).map(move |x| IVec2::new(x, y)))
                .filter(|cell| data.get_tile(cell.x, cell.y).is_some())
                .collect();
            (index, cells)
        })
    })
}

/// Spawns the tiles of a tile layer. `walls` are the wall cells if the layer is
/// above the walls layer: Tiles mounted on walls (e.g. windows) are drawn in
/// front of the whole wall like Tiled draws them.
#[allow(clippy::too_many_arguments)]
fn spawn_tile_layer(
    commands: &mut Commands,
    layer_entity: Entity,
    tiled_map: &TiledMap,
    layer: &tiled::Layer,
    layer_data: &tiled::FiniteTileLayer,
    layer_index: usize,
    walls: Option<&HashSet<IVec2>>,
) {
    let map = &tiled_map.map;
    for x in 0..map.width {
        for y in 0..map.height {
            let Some(layer_tile) = layer_data.get_tile(x as i32, y as i32) else {
                continue;
            };
            let image = image_name(layer_tile.get_tile());
            let tileset = layer_tile.get_tileset();
            let is_flat =
                matches!(layer.name.as_str(), "Floor" | "Carpet") || is_flat_image(&image);
            let on_wall =
                walls.is_some_and(|walls| walls.contains(&IVec2::new(x as i32, y as i32)));
            let z = if is_flat {
                flat_depth(layer_index)
            } else if on_wall {
                upright_depth((x + y + 1) as f32, layer_index as f32 * 0.1)
            } else if is_wall_hanging_image(&image) {
                // In front of the wall, behind furniture standing against it.
                upright_depth((x + y) as f32 - 1.0, layer_index as f32 * 0.1 - 0.05)
            } else {
                upright_depth((x + y) as f32, layer_index as f32 * 0.1)
            };
            commands.spawn((
                Sprite {
                    image: tiled_map.tilesets[layer_tile.tileset_index()].images[&layer_tile.id()]
                        .clone(),
                    flip_x: layer_tile.flip_h,
                    flip_y: layer_tile.flip_v,
                    // FIXME flip_d ?
                    ..default()
                },
                Anchor::BOTTOM_LEFT,
                iso_to_screen(map, x, y, z, tileset.offset_x, tileset.offset_y),
                MapTile {
                    layer: layer.name.clone(),
                    cell: IVec2::new(x as i32, y as i32),
                    image,
                },
                ChildOf(layer_entity),
            ));
        }
    }
}

/// Spawns the tile objects of an object layer. In isometric maps, the position
/// of a tile object is the bottom center of its image.
fn spawn_tile_objects(
    commands: &mut Commands,
    layer_entity: Entity,
    tiled_map: &TiledMap,
    objects: &tiled::ObjectLayer,
    layer_index: usize,
) {
    let map = &tiled_map.map;
    let (tile_width, tile_height) = (map.tile_width as f32, map.tile_height as f32);
    for object in objects.objects() {
        let (Some(data), Some(tile)) = (object.tile_data(), object.get_tile()) else {
            continue;
        };
        let tiled::TilesetLocation::Map(tileset_index) = *data.tileset_location() else {
            continue;
        };
        let Some(image) = tiled_map.tilesets[tileset_index].images.get(&data.id()) else {
            continue;
        };
        let tileset = tile.get_tileset();
        // Position in map cells and on screen (as Tiled draws it, y pointing down).
        let cell = Vec2::new(object.x, object.y) / tile_height;
        let screen = Vec2::new(
            (cell.x - cell.y) * tile_width / 2.0 + tileset.offset_x as f32,
            (cell.x + cell.y) * tile_height / 2.0 + tileset.offset_y as f32,
        );
        let custom_size = match object.shape {
            tiled::ObjectShape::Rect { width, height } => Some(Vec2::new(width, height)),
            _ => None,
        };
        // Tile objects are usually placed on top of other things (e.g. a mug on a
        // table), so they are drawn in front of the cell they stand on.
        let z = upright_depth(cell.x + cell.y, 0.2 + layer_index as f32 * 0.1);
        commands.spawn((
            Sprite {
                image: image.clone(),
                flip_x: data.flip_h,
                flip_y: data.flip_v,
                custom_size,
                ..default()
            },
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(screen.x + tile_width / 2.0, tile_height - screen.y, z),
            ChildOf(layer_entity),
        ));
    }
}

fn iso_to_screen(
    map: &tiled::Map,
    x: u32,
    y: u32,
    z: f32,
    offset_x: i32,
    offset_y: i32,
) -> Transform {
    let x = x as f32;
    let y = y as f32;
    let tile_width = map.tile_width as f32;
    let tile_height = map.tile_height as f32;
    let offset_x = offset_x as f32;
    let offset_y = offset_y as f32;
    Transform::from_xyz(
        ((x - y) * tile_width) / 2.0 + offset_x,
        -(((x + y) * tile_height) / 2.0 + offset_y),
        z,
    )
}
