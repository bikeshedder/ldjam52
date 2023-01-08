use std::time::Duration;

use bevy::{
    asset::LoadState,
    prelude::{AssetServer, Assets, Handle, Image, Res, ResMut, State},
    sprite::{TextureAtlas, TextureAtlasBuilder},
    utils::HashMap,
};

use crate::{
    data::entity_types::{EntityImage, EntityTypes, Loaded, LoadedAnimations},
    AppState, ImageHandles,
};

pub fn load_textures(
    mut entity_types: ResMut<EntityTypes>,
    mut image_handles: ResMut<ImageHandles>,
    asset_server: Res<AssetServer>,
) {
    for (name, entity_type) in entity_types.types.iter_mut() {
        match &entity_type.image {
            EntityImage::Static(image) => {
                let handle = asset_server.load::<Image, _>(&format!("entity_types/{name}/{image}"));
                image_handles.add(handle.clone());
                entity_type.loaded = Some(Loaded::Static(handle));
            }
            EntityImage::Animations(animations) => {
                for animation in animations.values() {
                    for frame in animation.iter() {
                        let image = &frame.image;
                        let handle =
                            asset_server.load::<Image, _>(&format!("entity_types/{name}/{image}"));
                        image_handles.add(handle.clone());
                        entity_type.loaded = Some(Loaded::Static(handle));
                    }
                }
            }
            _ => unimplemented!(),
        }
    }
}

pub fn check_textures(
    mut state: ResMut<State<AppState>>,
    image_handles: ResMut<ImageHandles>,
    asset_server: Res<AssetServer>,
    mut entity_types: ResMut<EntityTypes>,
    mut textures: ResMut<Assets<Image>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    if asset_server.get_group_load_state(image_handles.handles.iter().map(|handle| handle.id()))
        != LoadState::Loaded
    {
        return;
    }

    state.set(AppState::Menu).unwrap();

    for (name, entity_type) in entity_types.types.iter_mut() {
        match &entity_type.image {
            EntityImage::Static(_) => {
                // The handle was already assigned in the load_textures method.
            }
            EntityImage::Animations(animations) => {
                let mut atlas_builder = TextureAtlasBuilder::default();
                let frame_handles: HashMap<String, Vec<(Handle<Image>, Duration)>> = animations
                    .iter()
                    .map(|(animation_name, frames)| {
                        (
                            animation_name.clone(),
                            frames
                                .iter()
                                .map(|frame| {
                                    let file_name =
                                        format!("entity_types/{}/{}", name, frame.image);
                                    let handle = asset_server.get_handle(&file_name);
                                    let texture = textures.get(&handle).unwrap();
                                    atlas_builder.add_texture(handle.clone(), texture);
                                    (handle, Duration::from_millis(frame.duration))
                                })
                                .collect(),
                        )
                    })
                    .collect();
                let atlas = atlas_builder.finish(&mut textures).unwrap();
                let atlas_handle = texture_atlases.add(atlas);
                let atlas = texture_atlases.get(&atlas_handle).unwrap();
                entity_type.loaded = Some(Loaded::Animations(LoadedAnimations {
                    atlas: atlas_handle,
                    frames: frame_handles
                        .into_iter()
                        .map(|(animation_name, frames)| {
                            (
                                animation_name,
                                frames
                                    .into_iter()
                                    .map(|(handle, duration)| {
                                        (atlas.get_texture_index(&handle).unwrap(), duration)
                                    })
                                    .collect(),
                            )
                        })
                        .collect(),
                }));
            }
            _ => unimplemented!(),
        }
    }
}
