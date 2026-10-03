//! The game world: characters, interactable objects and visual changes caused
//! by the player's actions.

use bevy::prelude::*;

use super::{
    Phase,
    dialogue::StartDialogue,
    hud::{Darkness, Prompt},
    iso::{FEET_OFFSET, cell_center, character_translation, depth_at},
    outline::Highlighted,
    progress::{Carpet, Item, Progress, RitualCircle},
    rooms::{Area, Room, RoomEntity, RoomSpawned, RoomSpawner, START_ROOM, START_SPAWN},
    script::Node,
};
use crate::{
    AppState,
    components::{
        animation::{Animation, AnimationState},
        player::Player,
    },
    data::entity_types::{EntityType, EntityTypes, Loaded},
    plugins::tiled::{MapTile, flat_depth},
    systems::animation::AnimationTimer,
};

const LIBRARIAN_SPEED: f32 = 70.0;
const LIBRARIAN_SIGHT: f32 = 230.0;
const DEFAULT_RADIUS: f32 = 120.0;

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
    /// Patrol route in world coordinates.
    patrol: Vec<Vec2>,
    target: usize,
    wait: f32,
    facing: Vec2,
    /// Prevents the librarian from addressing the player again right away.
    cooldown: bool,
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
            .add_systems(
                OnEnter(AppState::Game),
                (setup_world, enter_start_room).chain(),
            )
            .add_systems(OnExit(Phase::Exploring), clear_prompt)
            .add_systems(
                Update,
                (
                    spawn_room_contents.run_if(on_message::<RoomSpawned>),
                    (librarian_patrol, dark_room, interact)
                        .chain()
                        .after(crate::systems::input::player_input)
                        .run_if(in_state(Phase::Exploring)),
                    (
                        apply_tint,
                        update_depth,
                        update_darkness,
                        update_world_visuals,
                        remove_librarian,
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
        _ => unimplemented!(),
    };
    if let Some(interaction) = &entity_type.interaction {
        entity_cmds.insert(crate::components::interaction::Interaction {
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
    entity_cmds.id()
}

fn setup_world(
    mut commands: Commands,
    entity_types: Res<EntityTypes>,
    mut progress: ResMut<Progress>,
    mut tracker: ResMut<RoomTracker>,
) {
    *progress = Progress::default();
    *tracker = RoomTracker::default();
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
fn spawn_room_contents(
    mut commands: Commands,
    room: Res<Room>,
    entity_types: Res<EntityTypes>,
    progress: Res<Progress>,
    asset_server: Res<AssetServer>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    let mouse = &entity_types["player"];
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
            "librarian" if !progress.librarian_gone => {
                spawn_entity(
                    &mut commands,
                    mouse,
                    translation,
                    Some("idle_down"),
                    (
                        Transform::from_translation(translation).with_scale(Vec3::splat(0.95)),
                        Librarian {
                            patrol: npc.path.iter().map(|cell| cell_center(*cell)).collect(),
                            target: 1 % npc.path.len(),
                            wait: 0.0,
                            facing: Vec2::new(-1.0, -0.5).normalize(),
                            cooldown: false,
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
                tiles
                    .iter()
                    .filter(|(_, tile, visibility)| {
                        area.contains_cell(tile.cell)
                            && HIGHLIGHT_LAYERS.contains(&tile.layer.as_str())
                            && **visibility != Visibility::Hidden
                    })
                    .map(|(entity, ..)| entity)
                    .collect()
            })
            .unwrap_or_default(),
        None => Vec::new(),
    };
    for entity in &highlighted {
        if !targets.contains(&entity) {
            commands.entity(entity).remove::<Highlighted>();
        }
    }
    for entity in targets {
        if !highlighted.contains(entity) {
            commands.entity(entity).insert(Highlighted);
        }
    }
}

fn librarian_from_behind(librarian: &Librarian, librarian_pos: Vec2, player_pos: Vec2) -> bool {
    (player_pos - librarian_pos)
        .normalize_or_zero()
        .dot(librarian.facing)
        < -0.2
}

fn interact(
    player: Single<(&Player, &Transform)>,
    interactables: Query<(Entity, &Interactable, &Transform, Option<&Librarian>)>,
    progress: Res<Progress>,
    mut prompt: ResMut<Prompt>,
    mut focus: ResMut<Focus>,
    mut dialogue: MessageWriter<StartDialogue>,
) {
    let (player, player_transform) = *player;
    let player_pos = feet(player_transform);
    let nearest = interactables
        .iter()
        .filter(|(_, interactable, ..)| interactable.target != Target::Ritual || progress.room_lit)
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
    let Some((_, interactable, pos, librarian, _)) = nearest else {
        prompt.0 = None;
        return;
    };
    let behind = librarian.is_some_and(|l| librarian_from_behind(l, pos, player_pos));
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
    };
    prompt.0 = Some(label.to_string());
    if player.input.interact {
        dialogue.write(StartDialogue(node));
    }
}

fn librarian_patrol(
    time: Res<Time>,
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
    let Some(librarian) = librarian else {
        return;
    };
    let (mut librarian, mut transform, mut animation, mut sprite) = librarian.into_inner();
    let pos = feet(&transform);
    let player_pos = feet(&player);
    let distance = pos.distance(player_pos);

    // Notice the player.
    if librarian.cooldown {
        librarian.cooldown = distance < LIBRARIAN_SIGHT * 1.5;
    } else if distance < LIBRARIAN_SIGHT {
        if !librarian_from_behind(&librarian, pos, player_pos) {
            librarian.cooldown = true;
            dialogue.write(StartDialogue(Node::Librarian));
            return;
        }
        if distance < 80.0 && !progress.librarian_met && progress.has(Item::Knife) {
            // The player ran into the librarian from behind with a knife in hand.
            librarian.cooldown = true;
            dialogue.write(StartDialogue(Node::LibrarianBack));
            return;
        }
    }

    // Walk between the waypoints.
    if librarian.wait > 0.0 {
        librarian.wait -= time.delta_secs();
        let up = librarian.facing.y > 0.0;
        animation.start(if up { "idle_up" } else { "idle_down" });
        return;
    }
    let target = librarian.patrol[librarian.target];
    let to_target = target - pos;
    let step = LIBRARIAN_SPEED * time.delta_secs();
    if to_target.length() <= step {
        transform.translation += to_target.extend(0.0);
        librarian.target = (librarian.target + 1) % librarian.patrol.len();
        librarian.wait = 2.5;
    } else {
        let direction = to_target.normalize();
        transform.translation += (direction * step).extend(0.0);
        librarian.facing = direction;
        animation.start(if direction.y > 0.0 {
            "walk_up"
        } else {
            "walk_down"
        });
        sprite.flip_x = direction.x < 0.0;
    }
}

/// Characters look at the player while talking.
#[allow(clippy::type_complexity)]
fn face_player(
    phase: Res<State<Phase>>,
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
        if talking && to_player.length() < 250.0 {
            animation.start(if to_player.y > 0.0 {
                "idle_up"
            } else {
                "idle_down"
            });
            sprite.flip_x = to_player.x < 0.0;
            if let Some(mut librarian) = librarian {
                librarian.facing = to_player.normalize_or_zero();
            }
        } else if librarian.is_none() {
            // The magister is reading at the bookshelf.
            animation.start("idle_up");
            sprite.flip_x = true;
        }
    }
}

fn remove_librarian(
    mut commands: Commands,
    progress: Res<Progress>,
    phase: Res<State<Phase>>,
    librarian: Query<Entity, With<Librarian>>,
) {
    if progress.librarian_gone && *phase.get() == Phase::Exploring {
        for entity in &librarian {
            commands.entity(entity).despawn();
        }
    }
}

fn dark_room(
    room: Res<Room>,
    player: Single<&Transform, With<Player>>,
    progress: Res<Progress>,
    mut tracker: ResMut<RoomTracker>,
    mut dialogue: MessageWriter<StartDialogue>,
) {
    let dark = room.dark && !progress.room_lit;
    let on_carpet = room
        .trip
        .is_some_and(|trip| trip.contains_world(feet(&player)));
    if tracker.room != room.name {
        tracker.room = room.name.clone();
        if dark {
            dialogue.write(StartDialogue(Node::DarkRoom));
        }
    } else if on_carpet
        && !tracker.on_carpet
        && dark
        && !progress.has(Item::BurningCandle)
        && progress.carpet != Carpet::RolledIn
    {
        dialogue.write(StartDialogue(Node::Trip));
    }
    tracker.on_carpet = on_carpet;
}

fn update_darkness(room: Res<Room>, progress: Res<Progress>, mut darkness: ResMut<Darkness>) {
    let dark = room.dark && !progress.room_lit;
    if darkness.0 != dark {
        darkness.0 = dark;
    }
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

    // The pentagram is drawn in a flat plane which is squashed to match the
    // isometric perspective.
    let blood = materials.add(Color::srgba(0.6, 0.0, 0.02, 0.9));
    let pentagram = commands
        .spawn((
            Transform::from_translation(center.extend(flat + 0.1))
                .with_scale(Vec3::new(1.0, 0.5, 1.0)),
            Visibility::Hidden,
            Marker::Pentagram,
            RoomEntity,
            DespawnOnExit(AppState::Game),
        ))
        .id();
    let radius = 110.0;
    commands.spawn((
        Mesh2d(meshes.add(Annulus::new(radius - 4.0, radius))),
        MeshMaterial2d(blood.clone()),
        ChildOf(pentagram),
    ));
    let points: Vec<Vec2> = (0..5)
        .map(|i| {
            let angle = std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / 5.0;
            Vec2::from_angle(angle) * (radius - 4.0)
        })
        .collect();
    for i in 0..5 {
        let (a, b) = (points[i], points[(i + 2) % 5]);
        let mid = (a + b) / 2.0;
        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(a.distance(b), 4.0))),
            MeshMaterial2d(blood.clone()),
            Transform::from_translation(mid.extend(0.0))
                .with_rotation(Quat::from_rotation_z((b - a).to_angle())),
            ChildOf(pentagram),
        ));
    }

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
    new_tiles: Query<(), Added<MapTile>>,
    mut tiles: Query<(&MapTile, &mut Sprite, &mut Visibility), Without<Marker>>,
    mut markers: Query<(&Marker, &mut Visibility)>,
) {
    if !progress.is_changed() && new_tiles.is_empty() {
        return;
    }
    for (tile, mut sprite, mut visibility) in &mut tiles {
        if tile.image.contains("Fireplace") {
            sprite.color = if progress.fire_lit {
                Color::WHITE
            } else {
                Color::srgb(0.45, 0.45, 0.5)
            };
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
