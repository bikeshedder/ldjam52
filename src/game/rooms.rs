//! The rooms of the game. Every room is a separate Tiled map and rooms are
//! connected by doors. Only the current room is spawned.
//!
//! Each map has an object layer called `Objects` describing the room:
//!
//! * `door` (rectangle): Opening it from within the rectangle's area leads to the
//!   room named like the object. Doors with the `locked` property set to `true`
//!   can't be opened, their name says who lives behind them.
//! * `spawn` (point): Where the player appears when coming from the room named
//!   like the object. `start` is used when starting a new game.
//! * `interactable` (point or rectangle): Something the player can interact
//!   with. The name selects the dialogue, the `radius` property the interaction
//!   distance. The tiles covered by a rectangle are highlighted when the
//!   interaction is available.
//! * `npc` (point or polyline): A character. A polyline is used as patrol route.
//! * `trip` (rectangle): The carpet the player trips over in the dark.
//!
//! Rooms with the map property `dark` set to `true` are dark until lit.

use bevy::{platform::collections::HashMap, prelude::*};

use super::{
    Phase,
    hud::ScreenFx,
    iso::{Walkable, character_translation, world_to_cell},
};
use crate::{components::player::Player, plugins::tiled::TiledMap};

/// All rooms of the game. They are loaded from `assets/rooms/<name>.tmx`.
pub const ROOMS: [&str; 6] = [
    "bedroom",
    "hall",
    "library",
    "study",
    "dining_hall",
    "ritual_room",
];
pub const START_ROOM: &str = "bedroom";
pub const START_SPAWN: &str = "start";

/// The maps of all rooms.
#[derive(Resource)]
pub struct RoomMaps(pub HashMap<String, Handle<TiledMap>>);

impl RoomMaps {
    pub fn load(asset_server: &AssetServer) -> Self {
        Self(
            ROOMS
                .iter()
                .map(|room| {
                    (
                        room.to_string(),
                        asset_server.load(format!("rooms/{room}.tmx")),
                    )
                })
                .collect(),
        )
    }
}

/// An area of map cells. Coordinates use the [`iso`](super::iso) convention
/// of cell centers being integer coordinates.
#[derive(Clone, Copy, Debug)]
pub struct Area {
    min: Vec2,
    max: Vec2,
}

impl Area {
    pub fn contains_cell(&self, cell: IVec2) -> bool {
        let cell = cell.as_vec2();
        cell.cmpge(self.min).all() && cell.cmplt(self.max).all()
    }

    pub fn expanded(&self, cells: f32) -> Self {
        Self {
            min: self.min - cells,
            max: self.max + cells,
        }
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) / 2.0
    }

    pub fn contains_world(&self, pos: Vec2) -> bool {
        let cell = world_to_cell(pos);
        cell.cmpge(self.min).all() && cell.cmplt(self.max).all()
    }
}

#[derive(Clone, Debug)]
pub struct Door {
    pub name: String,
    /// The room behind the door. `None` if the door is locked.
    pub target: Option<String>,
    pub area: Area,
}

#[derive(Clone, Debug)]
pub struct RoomObject {
    pub name: String,
    /// Position (or patrol route) in map cells.
    pub path: Vec<Vec2>,
    pub radius: Option<f32>,
    pub area: Option<Area>,
}

/// The room the player is currently in.
#[derive(Resource, Clone, Debug, Default)]
pub struct Room {
    pub name: String,
    pub dark: bool,
    pub spawns: HashMap<String, Vec2>,
    pub doors: Vec<Door>,
    pub trip: Option<Area>,
    pub interactables: Vec<RoomObject>,
    pub npcs: Vec<RoomObject>,
}

impl Room {
    fn from_map(name: &str, tiled_map: &TiledMap) -> Self {
        let map = &tiled_map.map;
        let unit = map.tile_height as f32;
        // Tiled measures object positions in tile heights from the top corner of
        // the map. Cell centers are integer coordinates in the game.
        let cell = |x: f32, y: f32| Vec2::new(x, y) / unit - 0.5;
        let mut room = Room {
            name: name.to_string(),
            dark: matches!(
                map.properties.get("dark"),
                Some(tiled::PropertyValue::BoolValue(true))
            ),
            ..default()
        };
        let objects = map
            .layers()
            .filter(|layer| layer.name == "Objects")
            .filter_map(|layer| layer.as_object_layer());
        for object in objects.flat_map(|layer| layer.objects().collect::<Vec<_>>()) {
            let origin = cell(object.x, object.y);
            let area = match object.shape {
                tiled::ObjectShape::Rect { width, height } => Some(Area {
                    min: origin,
                    max: origin + Vec2::new(width, height) / unit,
                }),
                _ => None,
            };
            let path = match &object.shape {
                tiled::ObjectShape::Polyline { points } => points
                    .iter()
                    .map(|(x, y)| origin + Vec2::new(*x, *y) / unit)
                    .collect(),
                _ => vec![area.map_or(origin, |area| area.center())],
            };
            let radius = match object.properties.get("radius") {
                Some(tiled::PropertyValue::FloatValue(radius)) => Some(*radius),
                Some(tiled::PropertyValue::IntValue(radius)) => Some(*radius as f32),
                _ => None,
            };
            let room_object = RoomObject {
                name: object.name.clone(),
                path,
                radius,
                area,
            };
            match (object.user_type.as_str(), area) {
                ("door", Some(area)) => {
                    let locked = matches!(
                        object.properties.get("locked"),
                        Some(tiled::PropertyValue::BoolValue(true))
                    );
                    room.doors.push(Door {
                        name: object.name.clone(),
                        target: (!locked).then(|| object.name.clone()),
                        area,
                    });
                }
                ("trip", Some(area)) => room.trip = Some(area),
                ("spawn", _) => {
                    room.spawns.insert(object.name.clone(), origin);
                }
                ("interactable", _) => room.interactables.push(room_object),
                ("npc", _) => room.npcs.push(room_object),
                (kind, _) => warn!(
                    "Ignoring object {:?} of type {kind:?} in room {name}",
                    object.name
                ),
            }
        }
        room
    }

    /// Reads the room with the given name from its map.
    pub fn load(name: &str, room_maps: &RoomMaps, maps: &Assets<TiledMap>) -> Option<Self> {
        let map = maps.get(room_maps.0.get(name)?)?;
        Some(Self::from_map(name, map))
    }

    pub fn interactable(&self, name: &str) -> Option<&RoomObject> {
        self.interactables.iter().find(|object| object.name == name)
    }
}

/// Marks entities which belong to the current room.
#[derive(Component)]
pub struct RoomEntity;

/// Request to move the player to another room.
#[derive(Message, Clone, Debug)]
pub struct EnterRoom {
    pub room: String,
    pub spawn: String,
}

/// A room change waiting for the screen to turn black.
#[derive(Resource, Default)]
struct Transition(Option<EnterRoom>);

/// Sent after a room was spawned.
#[derive(Message, Clone, Debug)]
pub struct RoomSpawned;

pub struct RoomsPlugin;

impl Plugin for RoomsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EnterRoom>()
            .add_message::<RoomSpawned>()
            .init_resource::<Room>()
            .init_resource::<Transition>()
            .add_systems(
                Update,
                (
                    start_transition,
                    finish_transition.run_if(in_state(Phase::Transition)),
                )
                    .chain()
                    .run_if(in_state(crate::AppState::Game)),
            );
    }
}

fn start_transition(
    mut requests: MessageReader<EnterRoom>,
    mut transition: ResMut<Transition>,
    mut screen: ResMut<ScreenFx>,
    mut phase: ResMut<NextState<Phase>>,
) {
    if let Some(request) = requests.read().last() {
        transition.0 = Some(request.clone());
        screen.blackout = true;
        phase.set(Phase::Transition);
    }
}

fn finish_transition(
    mut transition: ResMut<Transition>,
    mut screen: ResMut<ScreenFx>,
    mut phase: ResMut<NextState<Phase>>,
    mut spawner: RoomSpawner,
) {
    if !screen.is_black() {
        return;
    }
    let Some(request) = transition.0.take() else {
        return;
    };
    spawner.spawn(&request.room, &request.spawn);
    screen.blackout = false;
    phase.set(Phase::Exploring);
}

/// Replaces the current room with another one.
#[derive(bevy::ecs::system::SystemParam)]
pub struct RoomSpawner<'w, 's> {
    commands: Commands<'w, 's>,
    maps: Res<'w, Assets<TiledMap>>,
    room_maps: Res<'w, RoomMaps>,
    room_entities: Query<'w, 's, Entity, With<RoomEntity>>,
    player: Query<'w, 's, &'static mut Transform, With<Player>>,
    room: ResMut<'w, Room>,
    spawned: MessageWriter<'w, RoomSpawned>,
}

impl RoomSpawner<'_, '_> {
    pub fn spawn(&mut self, name: &str, spawn: &str) {
        let Some(handle) = self.room_maps.0.get(name) else {
            error!("Unknown room {name:?}");
            return;
        };
        let tiled_map = self.maps.get(handle).expect("rooms are loaded");
        let room = Room::from_map(name, tiled_map);
        for entity in &self.room_entities {
            self.commands.entity(entity).despawn();
        }
        self.commands.spawn((
            crate::plugins::tiled::TiledMapHandle(handle.clone()),
            RoomEntity,
            DespawnOnExit(crate::AppState::Game),
        ));
        self.commands.insert_resource(Walkable::from_map(tiled_map));

        let spawn_cell = room.spawns.get(spawn).copied().unwrap_or_else(|| {
            warn!("Room {name:?} has no spawn point {spawn:?}");
            room.spawns.values().next().copied().unwrap_or_default()
        });
        for mut transform in &mut self.player {
            transform.translation = character_translation(spawn_cell);
        }
        *self.room = room;
        self.spawned.write(RoomSpawned);
    }
}
