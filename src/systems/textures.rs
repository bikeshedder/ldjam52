use std::time::Duration;

use anyhow::Context;
use bevy::{
    image::{TextureAtlasBuilder, TextureAtlasLayout},
    platform::collections::HashMap,
    prelude::*,
};

use crate::{
    AppState,
    data::entity_types::{EntityImage, EntityTypes, Loaded, LoadedAnimations},
};

/// Handles of all entity type images which need to be loaded before leaving
/// the [`AppState::Loading`] state, keyed by asset path.
#[derive(Default, Resource)]
pub struct ImageHandles {
    handles: HashMap<String, Handle<Image>>,
}

fn image_path(entity_name: &str, image: &str) -> String {
    format!("entity_types/{entity_name}/{image}")
}

pub fn load_textures(
    mut entity_types: ResMut<EntityTypes>,
    mut image_handles: ResMut<ImageHandles>,
    asset_server: Res<AssetServer>,
) {
    for (name, entity_type) in entity_types.types.iter_mut() {
        match &entity_type.image {
            EntityImage::Static(image) => {
                let path = image_path(name, image);
                let handle = asset_server.load(&path);
                image_handles.handles.insert(path, handle.clone());
                entity_type.loaded = Some(Loaded::Static(handle));
            }
            EntityImage::Animations(animations) => {
                for frame in animations.values().flatten() {
                    let path = image_path(name, &frame.image);
                    let handle = asset_server.load(&path);
                    image_handles.handles.insert(path, handle);
                }
            }
            _ => unimplemented!(),
        }
    }
}

pub fn check_textures(
    mut next_state: ResMut<NextState<AppState>>,
    image_handles: Res<ImageHandles>,
    asset_server: Res<AssetServer>,
    mut entity_types: ResMut<EntityTypes>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) -> Result {
    for (path, handle) in &image_handles.handles {
        if asset_server.load_state(handle).is_failed() {
            return Err(anyhow::anyhow!("Loading image {path:?} failed").into());
        }
    }
    if !image_handles
        .handles
        .values()
        .all(|handle| asset_server.is_loaded_with_dependencies(handle))
    {
        return Ok(());
    }

    for (name, entity_type) in entity_types.types.iter_mut() {
        match &entity_type.image {
            EntityImage::Static(_) => {
                // The handle was already assigned in the load_textures method.
            }
            EntityImage::Animations(animations) => {
                let mut atlas_builder = TextureAtlasBuilder::default();
                let mut frame_handles = HashMap::<String, Vec<(AssetId<Image>, Duration)>>::new();
                for (animation_name, frames) in animations {
                    let frames = frames
                        .iter()
                        .map(|frame| {
                            let id = image_handles.handles[&image_path(name, &frame.image)].id();
                            let image = images.get(id).context("Image missing")?;
                            atlas_builder.add_texture(Some(id), image);
                            Ok((id, Duration::from_millis(frame.duration)))
                        })
                        .collect::<anyhow::Result<_>>()?;
                    frame_handles.insert(animation_name.clone(), frames);
                }
                let (layout, sources, image) = atlas_builder.build()?;
                entity_type.loaded = Some(Loaded::Animations(LoadedAnimations {
                    image: images.add(image),
                    layout: layouts.add(layout),
                    frames: frame_handles
                        .into_iter()
                        .map(|(animation_name, frames)| {
                            let frames = frames
                                .into_iter()
                                .map(|(id, duration)| {
                                    (sources.texture_index(id).unwrap(), duration)
                                })
                                .collect();
                            (animation_name, frames)
                        })
                        .collect(),
                }));
            }
            _ => unimplemented!(),
        }
    }

    next_state.set(AppState::Menu);
    Ok(())
}
