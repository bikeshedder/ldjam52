//! The game world: characters, interactable objects and visual changes caused
//! by the player's actions.

use bevy::prelude::*;

use super::{
    Phase,
    audio::{PlaySfx, Sfx},
    dialogue::StartDialogue,
    hud::Prompt,
    iso::{FEET_OFFSET, cell_center, character_translation, depth_at},
    light::RoomLight,
    outline::Highlighted,
    progress::{Carpet, Item, Progress, RitualCircle},
    rooms::{
        Area, EnterRoom, Room, RoomEntity, RoomMaps, RoomSpawned, RoomSpawner, START_ROOM,
        START_SPAWN,
    },
    script::{Node, Occupant},
};
use crate::{
    AppState,
    components::{
        animation::{Animation, AnimationState},
        player::Player,
    },
    data::entity_types::{EntityType, EntityTypes, Loaded},
    plugins::tiled::{MapTile, TiledMap, flat_depth},
    systems::animation::AnimationTimer,
};

const FIREPLACE: &str = "tilesets/village_interiors/Iso_Deco_Fireplace_01.png";
const FIREPLACE_BURNING: &str = "tilesets/macgreg/Iso_Deco_Fireplace_01_burning.png";
const PENTAGRAM: &str = "tilesets/macgreg/Pentagram.png";

/// Center of the fire in the fireplace image, measured from its bottom-left corner.
const FIRE_OFFSET: Vec2 = Vec2::new(110.0, 95.0);
const FIRE_GLOW_SIZE: Vec2 = Vec2::new(360.0, 300.0);
const FIRE_GLOW_COLOR: Vec3 = Vec3::new(1.0, 0.6, 0.15);
/// Seconds per pulse of the fire glow.
const FIRE_GLOW_PERIOD: f32 = 2.4;

const LIBRARIAN_SPEED: f32 = 70.0;
const LIBRARIAN_SCALE: f32 = 0.95;
const LIBRARY: &str = "library";
const LIBRARIAN_SIGHT: f32 = 230.0;
const DEFAULT_RADIUS: f32 = 120.0;
const DOOR_RADIUS: f32 = 100.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Bed,
    Mirror,
    Diary,
    Chest,
    Table,
    Fireplace,
    Ritual,
    Magister,
    Librarian,
    /// The door with the given index in [`Room::doors`].
    Door(usize),
}

impl Target {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "bed" => Self::Bed,
            "mirror" => Self::Mirror,
            "diary" => Self::Diary,
            "chest" => Self::Chest,
            "table" => Self::Table,
            "fireplace" => Self::Fireplace,
            "ritual" => Self::Ritual,
            _ => return None,
        })
    }
}

/// Something the player can interact with when standing close to it.
#[derive(Component)]
pub struct Interactable {
    target: Target,
    radius: f32,
    /// Map cells whose tiles are highlighted when the interaction is available.
    area: Option<Area>,
}

/// The interactable the player can currently interact with.
#[derive(Resource, Default)]
struct Focus(Option<Entity>);

/// Map layers whose tiles are highlighted.
const HIGHLIGHT_LAYERS: [&str; 3] = ["Furniture", "Props", "Carpet"];

/// Characters which block the player's movement.
#[derive(Component)]
pub struct Solid {
    pub radius: f32,
}

/// Sprites whose depth is updated from the position of their feet.
#[derive(Component)]
pub struct DepthSorted;

#[derive(Component)]
struct Tint(Color);

#[derive(Component)]
struct Librarian {
    /// Prevents the librarian from addressing the player again right away.
    cooldown: bool,
    /// The player sneaked up on the librarian with a knife, so he doesn't turn around.
    unaware: bool,
    /// He already talked to the player during this visit of the library, so he
    /// doesn't address the player on his own again.
    addressed: bool,
}

/// Where the librarian is on his patrol through the library. This is kept
/// outside of the library, so he keeps walking while the player is elsewhere.
#[derive(Resource, Default)]
pub struct LibrarianPatrol {
    /// Patrol route in world coordinates of the library.
    route: Vec<Vec2>,
    /// Position of his feet.
    position: Vec2,
    target: usize,
    wait: f32,
    facing: Vec2,
}

impl LibrarianPatrol {
    /// Where somebody stands who is `distance` behind the librarian.
    #[cfg(feature = "dev")]
    pub fn behind(&self, distance: f32) -> Vec2 {
        self.position - self.facing * distance
    }

    fn new(route: Vec<Vec2>) -> Self {
        Self {
            position: route.first().copied().unwrap_or_default(),
            target: 1 % route.len().max(1),
            wait: 0.0,
            facing: Vec2::new(-1.0, -0.5).normalize(),
            route,
        }
    }

    /// Walks between the waypoints. Returns the walking direction, or `None`
    /// while waiting at a waypoint.
    fn walk(&mut self, delta: f32) -> Option<Vec2> {
        if self.wait > 0.0 || self.route.is_empty() {
            self.wait -= delta;
            return None;
        }
        let to_target = self.route[self.target] - self.position;
        let step = LIBRARIAN_SPEED * delta;
        if to_target.length() <= step {
            self.position += to_target;
            self.target = (self.target + 1) % self.route.len();
            self.wait = 2.5;
            None
        } else {
            let direction = to_target.normalize();
            self.position += direction * step;
            self.facing = direction;
            Some(direction)
        }
    }
}

#[derive(Component)]
struct Magister;

/// Visual indicators of the world state.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Marker {
    BloodStain,
    Pentagram,
    PlacedCandle,
    PlacedFeather,
    PlacedIncense,
    BedFeathers,
}

/// Warm light pulsing from the fireplace while the fire is burning.
#[derive(Component)]
struct FireGlow;

/// Where the player was during the last frame.
#[derive(Resource, Default)]
struct RoomTracker {
    room: String,
    on_carpet: bool,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoomTracker>()
            .init_resource::<Focus>()
            .init_resource::<LibrarianPatrol>()
            .add_systems(
                OnEnter(AppState::Game),
                (setup_world, enter_start_room).chain(),
            )
            .add_systems(OnExit(Phase::Exploring), (clear_prompt, stop_librarian))
            .add_systems(
                Update,
                (
                    spawn_room_contents.run_if(on_message::<RoomSpawned>),
                    remove_librarian.run_if(in_state(Phase::Exploring)),
                    (librarian_patrol, dark_room, interact)
                        .chain()
                        .after(crate::systems::input::player_input)
                        .run_if(in_state(Phase::Exploring)),
                    (
                        apply_tint,
                        update_depth,
                        update_light,
                        update_world_visuals,
                        (spawn_fire_glow, update_fire_glow).chain(),
                        face_player,
                        update_highlight,
                    )
                        .run_if(in_state(AppState::Game)),
                )
                    .chain(),
            );
    }
}

/// Spawns an entity based on its entity type definition.
pub fn spawn_entity(
    commands: &mut Commands,
    entity_type: &EntityType,
    translation: Vec3,
    animation_name: Option<&'static str>,
    extra: impl Bundle,
) -> Entity {
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
    };
    entity_cmds.insert(extra);
    entity_cmds.id()
}

fn setup_world(
    mut commands: Commands,
    entity_types: Res<EntityTypes>,
    mut progress: ResMut<Progress>,
    mut tracker: ResMut<RoomTracker>,
    mut patrol: ResMut<LibrarianPatrol>,
    room_maps: Res<RoomMaps>,
    maps: Res<Assets<TiledMap>>,
) {
    *progress = Progress::default();
    *tracker = RoomTracker::default();
    let route = Room::load(LIBRARY, &room_maps, &maps)
        .and_then(|room| room.npcs.into_iter().find(|npc| npc.name == "librarian"))
        .map(|npc| npc.path.into_iter().map(cell_center).collect())
        .unwrap_or_default();
    *patrol = LibrarianPatrol::new(route);
    spawn_entity(
        &mut commands,
        &entity_types["player"],
        Vec3::ZERO,
        Some("idle_down"),
        (
            Player::default(),
            DepthSorted,
            DespawnOnExit(AppState::Game),
        ),
    );
}

fn enter_start_room(mut spawner: RoomSpawner) {
    spawner.spawn(START_ROOM, START_SPAWN);
}

/// Spawns the characters and interactable objects of the room which was just entered.
#[allow(clippy::too_many_arguments)]
fn spawn_room_contents(
    mut commands: Commands,
    room: Res<Room>,
    patrol: Res<LibrarianPatrol>,
    entity_types: Res<EntityTypes>,
    progress: Res<Progress>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mouse = &entity_types["player"];
    if room.name == LIBRARY && progress.librarian_killed {
        spawn_blood_stain(&mut commands, &mut meshes, &mut materials, patrol.position);
    }
    for object in &room.interactables {
        let Some(target) = Target::from_name(&object.name) else {
            warn!(
                "Unknown interactable {:?} in room {}",
                object.name, room.name
            );
            continue;
        };
        commands.spawn((
            Transform::from_translation(cell_center(object.path[0]).extend(0.0)),
            Interactable {
                target,
                radius: object.radius.unwrap_or(DEFAULT_RADIUS),
                area: object.area,
            },
            RoomEntity,
            DespawnOnExit(AppState::Game),
        ));
    }
    for (index, door) in room.doors.iter().enumerate() {
        commands.spawn((
            Transform::from_translation(cell_center(door.area.center()).extend(0.0)),
            Interactable {
                target: Target::Door(index),
                radius: DOOR_RADIUS,
                // Doors at the front of a room stand next to their area.
                area: Some(door.area.expanded(1.0)),
            },
            RoomEntity,
            DespawnOnExit(AppState::Game),
        ));
    }
    for npc in &room.npcs {
        let translation = character_translation(npc.path[0]);
        match npc.name.as_str() {
            "magister" => {
                spawn_entity(
                    &mut commands,
                    mouse,
                    translation,
                    Some("idle_up"),
                    (
                        Transform::from_translation(translation).with_scale(Vec3::splat(1.15)),
                        Magister,
                        Tint(Color::srgb(0.85, 0.7, 1.0)),
                        Solid { radius: 40.0 },
                        DepthSorted,
                        Interactable {
                            target: Target::Magister,
                            radius: npc.radius.unwrap_or(130.0),
                            area: None,
                        },
                        RoomEntity,
                        DespawnOnExit(AppState::Game),
                    ),
                );
            }
            // The librarian is wherever his patrol took him meanwhile.
            "librarian" if !progress.librarian_gone => {
                let translation = (patrol.position - FEET_OFFSET * LIBRARIAN_SCALE).extend(0.0);
                spawn_entity(
                    &mut commands,
                    mouse,
                    translation,
                    Some("idle_down"),
                    (
                        Transform::from_translation(translation)
                            .with_scale(Vec3::splat(LIBRARIAN_SCALE)),
                        Librarian {
                            cooldown: false,
                            unaware: false,
                            addressed: false,
                        },
                        Tint(Color::srgb(0.7, 0.72, 0.8)),
                        Solid { radius: 35.0 },
                        DepthSorted,
                        Interactable {
                            target: Target::Librarian,
                            radius: npc.radius.unwrap_or(140.0),
                            area: None,
                        },
                        RoomEntity,
                        DespawnOnExit(AppState::Game),
                    ),
                );
            }
            "librarian" => {}
            name => warn!("Unknown NPC {name:?} in room {}", room.name),
        }
    }
    spawn_markers(
        &mut commands,
        &asset_server,
        meshes,
        materials,
        room.interactable("ritual")
            .map(|object| cell_center(object.path[0])),
        room.interactable("bed")
            .map(|object| cell_center(object.path[0])),
    );
}

fn apply_tint(mut sprites: Query<(&Tint, &mut Sprite), Added<Tint>>) {
    for (tint, mut sprite) in &mut sprites {
        sprite.color = tint.0;
    }
}

fn update_depth(mut sprites: Query<&mut Transform, With<DepthSorted>>) {
    for mut transform in &mut sprites {
        let feet = transform.translation.truncate() + FEET_OFFSET * transform.scale.y;
        transform.translation.z = depth_at(feet);
    }
}

fn feet(transform: &Transform) -> Vec2 {
    transform.translation.truncate() + FEET_OFFSET * transform.scale.y
}

fn clear_prompt(mut prompt: ResMut<Prompt>, mut focus: ResMut<Focus>) {
    prompt.0 = None;
    focus.0 = None;
}

/// Highlights the sprite of the focused interactable, or the map tiles it covers.
fn update_highlight(
    mut commands: Commands,
    focus: Res<Focus>,
    interactables: Query<(&Interactable, Has<Sprite>)>,
    tiles: Query<(Entity, &MapTile, &Visibility)>,
    new_tiles: Query<(), Added<MapTile>>,
    highlighted: Query<Entity, With<Highlighted>>,
) {
    if !focus.is_changed() && new_tiles.is_empty() {
        return;
    }
    let targets: Vec<Entity> = match focus.0.and_then(|e| Some((e, interactables.get(e).ok()?))) {
        Some((entity, (_, true))) => vec![entity],
        Some((_, (interactable, false))) => interactable
            .area
            .map(|area| {
                let is_door = matches!(interactable.target, Target::Door(_));
                tiles
                    .iter()
                    .filter(|(_, tile, visibility)| {
                        area.contains_cell(tile.cell)
                            && HIGHLIGHT_LAYERS.contains(&tile.layer.as_str())
                            && **visibility != Visibility::Hidden
                            && (!is_door || tile.image.contains("Door"))
                    })
                    .map(|(entity, ..)| entity)
                    .collect()
            })
            .unwrap_or_default(),
        None => Vec::new(),
    };
    for entity in &highlighted {
        if !targets.contains(&entity) {
            commands.entity(entity).try_remove::<Highlighted>();
        }
    }
    for entity in targets {
        if !highlighted.contains(entity) {
            commands.entity(entity).try_insert(Highlighted);
        }
    }
}

fn librarian_from_behind(facing: Vec2, librarian_pos: Vec2, player_pos: Vec2) -> bool {
    (player_pos - librarian_pos).normalize_or_zero().dot(facing) < -0.2
}

#[allow(clippy::too_many_arguments)]
fn interact(
    player: Single<(&Player, &Transform)>,
    mut interactables: Query<(Entity, &Interactable, &Transform, Option<&mut Librarian>)>,
    progress: Res<Progress>,
    mut prompt: ResMut<Prompt>,
    mut focus: ResMut<Focus>,
    room: Res<Room>,
    patrol: Res<LibrarianPatrol>,
    mut dialogue: MessageWriter<StartDialogue>,
    mut enter: MessageWriter<EnterRoom>,
) {
    let (player, player_transform) = *player;
    let player_pos = feet(player_transform);
    let nearest = interactables
        .iter_mut()
        .filter(|(_, interactable, ..)| match interactable.target {
            Target::Ritual => progress.has_light(),
            Target::Librarian => !progress.librarian_gone,
            _ => true,
        })
        .map(|(entity, interactable, transform, librarian)| {
            let pos = if librarian.is_some() || interactable.target == Target::Magister {
                feet(transform)
            } else {
                transform.translation.truncate()
            };
            (
                entity,
                interactable,
                pos,
                librarian,
                pos.distance(player_pos),
            )
        })
        .filter(|(_, interactable, _, _, distance)| *distance <= interactable.radius)
        .min_by(|a, b| a.4.total_cmp(&b.4));

    let focused = nearest.as_ref().map(|(entity, ..)| *entity);
    if focus.0 != focused {
        focus.0 = focused;
    }
    let Some((_, interactable, pos, mut librarian, _)) = nearest else {
        prompt.0 = None;
        return;
    };
    let behind = librarian
        .as_ref()
        .is_some_and(|_| librarian_from_behind(patrol.facing, pos, player_pos));
    if let Target::Door(index) = interactable.target {
        prompt.0 = Some("Open the door".to_string());
        if player.input.interact
            && let Some(door) = room.doors.get(index)
        {
            match &door.target {
                Some(target) => {
                    enter.write(EnterRoom {
                        room: target.clone(),
                        spawn: room.name.clone(),
                    });
                }
                None => {
                    let occupant = Occupant::from_name(&door.name);
                    dialogue.write(StartDialogue(Node::LockedDoor(occupant)));
                }
            }
        }
        return;
    }
    let (label, node) = match interactable.target {
        Target::Bed => ("Examine the bed", Node::Bed),
        Target::Mirror => ("Look out of the window", Node::Mirror),
        Target::Diary => ("Read your diary", Node::Diary),
        Target::Chest => ("Search the chest", Node::Chest),
        Target::Table => ("Look at the table", Node::Table),
        Target::Fireplace => ("Examine the fireplace", Node::Fireplace),
        Target::Ritual => match (progress.carpet, progress.circle) {
            (Carpet::Clean | Carpet::Bloody, _) => ("Examine the carpet", Node::Ritual),
            (_, RitualCircle::None) => ("Examine the floor", Node::Ritual),
            _ => ("Examine the invocation circle", Node::Ritual),
        },
        Target::Magister => ("Talk to the magister", Node::Magister),
        Target::Librarian if behind => ("Approach the librarian from behind", Node::LibrarianBack),
        Target::Librarian => ("Talk to the librarian", Node::Librarian),
        Target::Door(_) => unreachable!("handled above"),
    };
    prompt.0 = Some(label.to_string());
    if player.input.interact {
        if let Some(librarian) = &mut librarian {
            librarian.cooldown = true;
            librarian.addressed = true;
            // He only stays unaware while the player can still stab him in the
            // back. Otherwise he notices the player and turns around.
            librarian.unaware = behind && progress.can_stab_librarian();
        }
        dialogue.write(StartDialogue(node));
    }
}

/// Moves the librarian along his patrol, also while the player is in another room.
fn librarian_patrol(
    time: Res<Time>,
    mut patrol: ResMut<LibrarianPatrol>,
    player: Single<&Transform, (With<Player>, Without<Librarian>)>,
    librarian: Option<
        Single<(
            &mut Librarian,
            &mut Transform,
            &mut AnimationState,
            &mut Sprite,
        )>,
    >,
    progress: Res<Progress>,
    mut dialogue: MessageWriter<StartDialogue>,
) {
    if progress.librarian_gone {
        return;
    }
    let Some(librarian) = librarian else {
        // He is somewhere in the library while the player is elsewhere.
        patrol.walk(time.delta_secs());
        return;
    };
    let (mut librarian, mut transform, mut animation, mut sprite) = librarian.into_inner();
    let pos = patrol.position;
    let player_pos = feet(&player);
    let distance = pos.distance(player_pos);

    // Notice the player.
    if librarian.cooldown {
        librarian.cooldown = distance < LIBRARIAN_SIGHT * 1.5;
    } else if distance < LIBRARIAN_SIGHT {
        if !librarian.addressed && !librarian_from_behind(patrol.facing, pos, player_pos) {
            librarian.cooldown = true;
            librarian.addressed = true;
            librarian.unaware = false;
            dialogue.write(StartDialogue(Node::Librarian));
            return;
        }
        if distance < 80.0 && progress.can_stab_librarian() {
            // The player ran into the librarian from behind with a knife in hand.
            librarian.cooldown = true;
            librarian.unaware = true;
            dialogue.write(StartDialogue(Node::LibrarianBack));
            return;
        }
    }

    match patrol.walk(time.delta_secs()) {
        Some(direction) => {
            animation.start(if direction.y > 0.0 {
                "walk_up"
            } else {
                "walk_down"
            });
            sprite.flip_x = direction.x < 0.0;
        }
        None => animation.start(if patrol.facing.y > 0.0 {
            "idle_up"
        } else {
            "idle_down"
        }),
    }
    let z = transform.translation.z;
    transform.translation = (patrol.position - FEET_OFFSET * transform.scale.y).extend(z);
}

/// The librarian doesn't move while the player isn't exploring (e.g. during
/// dialogues or while changing rooms), so he stops his walking animation.
fn stop_librarian(
    patrol: Res<LibrarianPatrol>,
    mut librarian: Query<&mut AnimationState, With<Librarian>>,
) {
    for mut animation in &mut librarian {
        animation.start(if patrol.facing.y > 0.0 {
            "idle_up"
        } else {
            "idle_down"
        });
    }
}

/// Characters look at the player while talking.
#[allow(clippy::type_complexity)]
fn face_player(
    phase: Res<State<Phase>>,
    mut patrol: ResMut<LibrarianPatrol>,
    player: Single<&Transform, With<Player>>,
    mut npcs: Query<
        (
            &Transform,
            &mut AnimationState,
            &mut Sprite,
            Option<&mut Librarian>,
        ),
        (Or<(With<Librarian>, With<Magister>)>, Without<Player>),
    >,
) {
    let talking = *phase.get() == Phase::Dialogue;
    for (transform, mut animation, mut sprite, librarian) in &mut npcs {
        let to_player = player.translation.truncate() - transform.translation.truncate();
        let unaware = librarian.as_ref().is_some_and(|l| l.unaware);
        if talking && to_player.length() < 250.0 && !unaware {
            animation.start(if to_player.y > 0.0 {
                "idle_up"
            } else {
                "idle_down"
            });
            sprite.flip_x = to_player.x < 0.0;
            if librarian.is_some() {
                patrol.facing = to_player.normalize_or_zero();
            }
        } else if librarian.is_none() {
            // The magister is reading at the bookshelf.
            animation.start("idle_up");
            sprite.flip_x = true;
        }
    }
}

/// Removes the librarian once he is gone. This happens after the dialogue in
/// which he died or left.
fn remove_librarian(
    mut commands: Commands,
    progress: Res<Progress>,
    patrol: Res<LibrarianPatrol>,
    librarian: Query<Entity, With<Librarian>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if progress.librarian_gone {
        for entity in &librarian {
            commands.entity(entity).despawn();
            if progress.librarian_killed {
                spawn_blood_stain(&mut commands, &mut meshes, &mut materials, patrol.position);
            }
        }
    }
}

/// A pool of blood on the floor where the librarian died. His patrol stopped
/// there, so its position is where he fell.
fn spawn_blood_stain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    position: Vec2,
) {
    let blood = materials.add(Color::srgba(0.42, 0.02, 0.03, 0.9));
    let dark = materials.add(Color::srgba(0.25, 0.0, 0.02, 0.9));
    // Several overlapping blobs, squashed to lie flat on the isometric floor.
    let stain = commands
        .spawn((
            Transform::from_translation(position.extend(flat_depth(2) + 0.05))
                .with_scale(Vec3::new(1.0, 0.5, 1.0)),
            Visibility::default(),
            RoomEntity,
            DespawnOnExit(AppState::Game),
        ))
        .id();
    for (x, y, radius, material) in [
        (0.0, 0.0, 30.0, &blood),
        (-26.0, 10.0, 18.0, &blood),
        (24.0, -12.0, 20.0, &blood),
        (8.0, 22.0, 12.0, &blood),
        (-4.0, 2.0, 16.0, &dark),
        (44.0, -26.0, 6.0, &blood),
        (-46.0, 20.0, 5.0, &blood),
    ] {
        commands.spawn((
            Mesh2d(meshes.add(Circle::new(radius))),
            MeshMaterial2d(material.clone()),
            Transform::from_xyz(x, y, 0.0),
            ChildOf(stain),
        ));
    }
}

fn dark_room(
    room: Res<Room>,
    player: Single<&Transform, With<Player>>,
    progress: Res<Progress>,
    mut tracker: ResMut<RoomTracker>,
    mut dialogue: MessageWriter<StartDialogue>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let dark = room.dark && !progress.has_light();
    let on_carpet = room
        .trip
        .is_some_and(|trip| trip.contains_world(feet(&player)));
    if tracker.room != room.name {
        tracker.room = room.name.clone();
        if dark {
            dialogue.write(StartDialogue(Node::DarkRoom));
        } else if room.dark {
            // The candle lights the way.
            sfx.write(PlaySfx(Sfx::RoomIsNowBright));
        }
    } else if on_carpet && !tracker.on_carpet && dark && progress.carpet != Carpet::RolledIn {
        dialogue.write(StartDialogue(Node::Trip));
    }
    tracker.on_carpet = on_carpet;
}

/// Radius of the light of the candle carried by the player.
const CANDLE_LIGHT: f32 = 260.0;
/// Radius of the light of the candle placed on the invocation circle.
const PLACED_CANDLE_LIGHT: f32 = 420.0;

/// The light comes from the candle: around the player while carrying it, from
/// the invocation circle once it is placed there.
fn update_light(
    room: Res<Room>,
    progress: Res<Progress>,
    player: Single<&Transform, With<Player>>,
    mut light: ResMut<RoomLight>,
) {
    let ritual = room
        .interactable("ritual")
        .map(|ritual| cell_center(ritual.path[0]));
    let (center, radius) = match ritual {
        Some(center) if progress.circle_candle => (center, PLACED_CANDLE_LIGHT),
        _ if progress.has(Item::BurningCandle) => (feet(&player), CANDLE_LIGHT),
        _ => (feet(&player), 0.0),
    };
    light.set_if_neq(RoomLight {
        dark: room.dark,
        center,
        radius,
    });
}

fn spawn_markers(
    commands: &mut Commands,
    asset_server: &AssetServer,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    ritual: Option<Vec2>,
    bed: Option<Vec2>,
) {
    let flat = flat_depth(2);
    if let Some(bed) = bed {
        let parent = commands
            .spawn((
                Transform::from_translation((bed + Vec2::new(0.0, -50.0)).extend(flat + 0.2)),
                Visibility::Hidden,
                Marker::BedFeathers,
                RoomEntity,
                DespawnOnExit(AppState::Game),
            ))
            .id();
        let feather = meshes.add(Ellipse::new(11.0, 4.0));
        let white = materials.add(Color::srgb(0.95, 0.95, 0.92));
        for (x, y, angle) in [
            (-60.0, 10.0, 0.3),
            (-22.0, -18.0, -0.5),
            (18.0, 12.0, 1.1),
            (64.0, -6.0, -0.2),
            (5.0, 30.0, 0.8),
            (-38.0, -34.0, 1.4),
            (40.0, -30.0, 2.0),
            (-80.0, -12.0, -1.0),
            (90.0, 20.0, 0.6),
        ] {
            commands.spawn((
                Mesh2d(feather.clone()),
                MeshMaterial2d(white.clone()),
                Transform::from_xyz(x, y, 0.0).with_rotation(Quat::from_rotation_z(angle)),
                ChildOf(parent),
            ));
        }
    }
    let Some(center) = ritual else {
        return;
    };
    let mut marker = |marker: Marker, bundle: (Mesh2d, Color, Transform)| {
        let (mesh, color, transform) = bundle;
        commands.spawn((
            mesh,
            MeshMaterial2d(materials.add(color)),
            transform,
            Visibility::Hidden,
            marker,
            RoomEntity,
            DespawnOnExit(AppState::Game),
        ));
    };

    marker(
        Marker::BloodStain,
        (
            Mesh2d(meshes.add(Ellipse::new(22.0, 9.0))),
            Color::srgba(0.45, 0.02, 0.02, 0.85),
            Transform::from_translation((center + Vec2::new(-30.0, 8.0)).extend(flat + 0.05)),
        ),
    );
    marker(
        Marker::PlacedFeather,
        (
            Mesh2d(meshes.add(Ellipse::new(16.0, 4.0))),
            Color::srgb(0.05, 0.05, 0.07),
            Transform::from_translation((center + Vec2::new(-70.0, 0.0)).extend(flat + 0.2))
                .with_rotation(Quat::from_rotation_z(0.4)),
        ),
    );
    marker(
        Marker::PlacedIncense,
        (
            Mesh2d(meshes.add(Ellipse::new(13.0, 8.0))),
            Color::srgb(0.95, 0.8, 0.3),
            Transform::from_translation((center + Vec2::new(70.0, 0.0)).extend(flat + 0.2)),
        ),
    );

    // The center of the circle is near the bottom of the image.
    let pentagram_pos = center + Vec2::new(1.0, 98.0);
    commands.spawn((
        Sprite::from_image(asset_server.load(PENTAGRAM)),
        Transform::from_translation(pentagram_pos.extend(flat + 0.1)),
        Visibility::Hidden,
        Marker::Pentagram,
        RoomEntity,
        DespawnOnExit(AppState::Game),
    ));

    let candle_pos = center + Vec2::new(0.0, 30.0);
    commands.spawn((
        Sprite::from_image(
            asset_server.load("tilesets/village_interiors/Iso_Deco_Candleabra_01_Lit_01.png"),
        ),
        Transform::from_translation(candle_pos.extend(depth_at(candle_pos - Vec2::new(0.0, 60.0))))
            .with_scale(Vec3::splat(0.6)),
        Visibility::Hidden,
        Marker::PlacedCandle,
        RoomEntity,
        DespawnOnExit(AppState::Game),
    ));
}

fn update_world_visuals(
    room: Res<Room>,
    progress: Res<Progress>,
    asset_server: Res<AssetServer>,
    new_tiles: Query<(), Added<MapTile>>,
    mut tiles: Query<(&MapTile, &mut Sprite, &mut Visibility), Without<Marker>>,
    mut markers: Query<(&Marker, &mut Visibility)>,
) {
    if !progress.is_changed() && new_tiles.is_empty() {
        return;
    }
    for (tile, mut sprite, mut visibility) in &mut tiles {
        if tile.image.contains("Fireplace") {
            sprite.image = asset_server.load(if progress.fire_lit {
                FIREPLACE_BURNING
            } else {
                FIREPLACE
            });
        }
        // The carpet in the ritual room can be rolled in.
        if tile.layer == "Carpet" && room.interactable("ritual").is_some() {
            visibility.set_if_neq(if progress.carpet == Carpet::RolledIn {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            });
        }
    }
    for (marker, mut visibility) in &mut markers {
        let visible = match marker {
            Marker::BloodStain => progress.carpet == Carpet::Bloody,
            Marker::Pentagram => progress.circle != RitualCircle::None,
            Marker::PlacedCandle => progress.circle_candle,
            Marker::PlacedFeather => progress.circle_feather.is_some(),
            Marker::PlacedIncense => progress.circle_incense.is_some(),
            Marker::BedFeathers => progress.bed_destroyed,
        };
        visibility.set_if_neq(if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

/// A soft round glow, fully opaque in the center and transparent at the edge.
fn glow_image() -> Image {
    const SIZE: u32 = 64;
    let data = (0..SIZE * SIZE)
        .flat_map(|i| {
            let p = Vec2::new((i % SIZE) as f32, (i / SIZE) as f32) + 0.5;
            let d = (p / SIZE as f32 * 2.0 - 1.0).length().min(1.0);
            let alpha = (1.0 - d * d).powi(2);
            [255, 255, 255, (alpha * 255.0) as u8]
        })
        .collect();
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

/// Puts a glow behind every fireplace, so it lights up the wall and floor around it.
fn spawn_fire_glow(
    mut commands: Commands,
    tiles: Query<(Entity, &MapTile), Added<MapTile>>,
    mut images: ResMut<Assets<Image>>,
    mut glow: Local<Option<Handle<Image>>>,
) {
    for (entity, tile) in &tiles {
        if !tile.image.contains("Fireplace") {
            continue;
        }
        let image = glow.get_or_insert_with(|| images.add(glow_image())).clone();
        commands.spawn((
            Sprite {
                image,
                custom_size: Some(FIRE_GLOW_SIZE),
                color: Color::NONE,
                ..default()
            },
            Transform::from_translation(FIRE_OFFSET.extend(-0.05)),
            Visibility::Hidden,
            FireGlow,
            ChildOf(entity),
        ));
    }
}

fn update_fire_glow(
    time: Res<Time>,
    progress: Res<Progress>,
    mut glows: Query<(&mut Sprite, &mut Visibility), With<FireGlow>>,
) {
    let phase = time.elapsed_secs() / FIRE_GLOW_PERIOD * std::f32::consts::TAU;
    // A slow pulse with a slight flicker on top.
    let pulse = 0.5 + 0.4 * phase.sin() + 0.1 * (phase * 3.7).sin();
    let alpha = 0.18 + 0.12 * pulse;
    for (mut sprite, mut visibility) in &mut glows {
        visibility.set_if_neq(if progress.fire_lit {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let [r, g, b] = FIRE_GLOW_COLOR.to_array();
        sprite.color = Color::srgba(r, g, b, alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn librarian_walks_and_waits_at_waypoints() {
        let mut patrol = LibrarianPatrol::new(vec![Vec2::ZERO, Vec2::new(100.0, 0.0)]);
        assert_eq!(patrol.walk(1.0), Some(Vec2::X));
        assert_eq!(patrol.position, Vec2::new(LIBRARIAN_SPEED, 0.0));
        // Reaches the waypoint and waits there before walking back.
        assert_eq!(patrol.walk(1.0), None);
        assert_eq!(patrol.position, Vec2::new(100.0, 0.0));
        assert_eq!(patrol.walk(2.0), None);
        assert_eq!(patrol.walk(1.0), None);
        assert_eq!(patrol.walk(1.0), Some(Vec2::NEG_X));
    }
}
