//! Glowing outlines around highlighted sprites.

use bevy::{
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite::Anchor,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};

const OUTLINE_WIDTH: f32 = 7.0;
const OUTLINE_COLOR: LinearRgba = LinearRgba::new(1.0, 0.8, 0.35, 1.0);

/// Add this to an entity with a [`Sprite`] to draw an outline around it.
#[derive(Component)]
pub struct Highlighted;

/// The outline mesh of a highlighted sprite. It is a child of the sprite.
#[derive(Component)]
struct Outline;

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct OutlineMaterial {
    #[uniform(0)]
    color: LinearRgba,
    /// Region of the texture shown by the sprite in UV coordinates.
    #[uniform(0)]
    uv_rect: Vec4,
    /// Outline width, horizontal flip and sprite size (see `outline.wgsl`).
    #[uniform(0)]
    params: Vec4,
    #[texture(1)]
    #[sampler(2)]
    image: Handle<Image>,
}

impl Material2d for OutlineMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/outline.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

pub struct OutlinePlugin;

impl Plugin for OutlinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<OutlineMaterial>::default())
            .add_systems(
                PostUpdate,
                (remove_outlines, add_outlines, update_outlines).chain(),
            );
    }
}

/// Size of the sprite in pixels and the region of its texture in UV coordinates.
fn sprite_geometry(
    sprite: &Sprite,
    images: &Assets<Image>,
    layouts: &Assets<TextureAtlasLayout>,
) -> Option<(Vec2, Vec4)> {
    let image_size = images.get(&sprite.image)?.size().as_vec2();
    let rect = sprite
        .texture_atlas
        .as_ref()
        .and_then(|atlas| atlas.texture_rect(layouts))
        .map(|rect| rect.as_rect())
        .unwrap_or(Rect::from_corners(Vec2::ZERO, image_size));
    let size = sprite.custom_size.unwrap_or(rect.size());
    let uv_rect = Vec4::new(
        rect.min.x / image_size.x,
        rect.min.y / image_size.y,
        rect.max.x / image_size.x,
        rect.max.y / image_size.y,
    );
    Some((size, uv_rect))
}

fn params(size: Vec2, flip_x: bool) -> Vec4 {
    Vec4::new(
        OUTLINE_WIDTH,
        if flip_x { 1.0 } else { 0.0 },
        size.x,
        size.y,
    )
}

fn add_outlines(
    mut commands: Commands,
    sprites: Query<(Entity, &Sprite, &Anchor), Added<Highlighted>>,
    images: Res<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OutlineMaterial>>,
) {
    for (entity, sprite, anchor) in &sprites {
        let Some((size, uv_rect)) = sprite_geometry(sprite, &images, &layouts) else {
            continue;
        };
        commands.spawn((
            Mesh2d(meshes.add(Rectangle::from_size(size + 2.0 * OUTLINE_WIDTH))),
            MeshMaterial2d(materials.add(OutlineMaterial {
                color: OUTLINE_COLOR,
                uv_rect,
                params: params(size, sprite.flip_x),
                image: sprite.image.clone(),
            })),
            // The mesh is centered while the sprite is positioned by its anchor.
            // Drawn slightly behind the sprite.
            Transform::from_translation((-anchor.0 * size).extend(-0.04)),
            Outline,
            ChildOf(entity),
        ));
    }
}

fn remove_outlines(
    mut commands: Commands,
    mut removed: RemovedComponents<Highlighted>,
    children: Query<&Children>,
    outlines: Query<(), With<Outline>>,
) {
    for entity in removed.read() {
        // The sprite might have been despawned together with its outline.
        let Ok(children) = children.get(entity) else {
            continue;
        };
        for child in children.iter().filter(|child| outlines.contains(*child)) {
            commands.entity(child).despawn();
        }
    }
}

/// Keeps the outlines of animated or flipped sprites in sync.
///
/// Objects made of several tiles are highlighted together. All their outlines
/// are drawn behind the rearmost tile, so they don't cover the other tiles.
fn update_outlines(
    sprites: Query<(&Sprite, &GlobalTransform, &Children), With<Highlighted>>,
    mut outlines: Query<(&MeshMaterial2d<OutlineMaterial>, &mut Transform), With<Outline>>,
    images: Res<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    mut materials: ResMut<Assets<OutlineMaterial>>,
) {
    let rearmost = sprites
        .iter()
        .map(|(_, transform, _)| transform.translation().z)
        .reduce(f32::min);
    for (sprite, transform, children) in &sprites {
        let Some((size, uv_rect)) = sprite_geometry(sprite, &images, &layouts) else {
            continue;
        };
        let params = params(size, sprite.flip_x);
        for child in children {
            let Ok((material, mut outline_transform)) = outlines.get_mut(*child) else {
                continue;
            };
            if let Some(rearmost) = rearmost {
                let z = rearmost - transform.translation().z - 0.04;
                if outline_transform.translation.z != z {
                    outline_transform.translation.z = z;
                }
            }
            let unchanged = materials
                .get(&material.0)
                .is_some_and(|m| m.uv_rect == uv_rect && m.params == params);
            if !unchanged && let Some(mut material) = materials.get_mut(&material.0) {
                material.uv_rect = uv_rect;
                material.params = params;
            }
        }
    }
}
