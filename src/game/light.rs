//! Darkness and candle light in dark rooms.

use bevy::{
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};

use crate::AppState;

/// Darkness outside of the light in a dark room.
const DARKNESS: f32 = 0.94;
/// Drawn in front of everything in the world.
const LIGHT_Z: f32 = 900.0;
/// Size of the darkness layer. It follows the camera.
const LAYER_SIZE: f32 = 6000.0;

/// The light in the current room, set by the game. Changes are faded in.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub struct RoomLight {
    pub dark: bool,
    /// Center of the light in world coordinates.
    pub center: Vec2,
    /// Radius of the light in pixels. 0 if there is no light.
    pub radius: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct LightMaterial {
    /// Center, radius and darkness (see `light.wgsl`).
    #[uniform(0)]
    params: Vec4,
}

impl Material2d for LightMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/light.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Component)]
struct DarknessLayer;

pub struct LightPlugin;

impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<LightMaterial>::default())
            .init_resource::<RoomLight>()
            .add_systems(OnEnter(AppState::Game), spawn_darkness_layer)
            .add_systems(
                PostUpdate,
                update_darkness_layer.run_if(in_state(AppState::Game)),
            );
    }
}

fn spawn_darkness_layer(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LightMaterial>>,
) {
    commands.insert_resource(RoomLight::default());
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(LAYER_SIZE, LAYER_SIZE))),
        MeshMaterial2d(materials.add(LightMaterial { params: Vec4::ZERO })),
        Transform::from_xyz(0.0, 0.0, LIGHT_Z),
        DarknessLayer,
        DespawnOnExit(AppState::Game),
    ));
}

/// Follows the camera and fades the darkness and light towards [`RoomLight`].
fn update_darkness_layer(
    time: Res<Time>,
    light: Res<RoomLight>,
    camera: Single<&GlobalTransform, With<Camera2d>>,
    layer: Single<(&mut Transform, &MeshMaterial2d<LightMaterial>), With<DarknessLayer>>,
    mut materials: ResMut<Assets<LightMaterial>>,
) {
    let (mut transform, material) = layer.into_inner();
    let camera = camera.translation();
    transform.translation = camera.truncate().extend(LIGHT_Z);
    let Some(mut material) = materials.get_mut(&material.0) else {
        return;
    };
    let params = &mut material.params;
    let fade = (time.delta_secs() * 3.0).min(1.0);
    let darkness = if light.dark { DARKNESS } else { 0.0 };
    params.w += (darkness - params.w) * fade;
    params.z += (light.radius - params.z) * fade;
    // Light moving with the player follows immediately, a new light source fades over.
    let center = Vec2::new(params.x, params.y);
    let center = if center.distance(light.center) < 200.0 {
        light.center
    } else {
        center.lerp(light.center, fade)
    };
    params.x = center.x;
    params.y = center.y;
}
