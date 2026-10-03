use std::{collections::HashMap, ops::Index, path::PathBuf, time::Duration};

use anyhow::Context;
use bevy::{
    image::TextureAtlasLayout,
    prelude::{Handle, Image, Resource},
};
use serde::Deserialize;

use super::common::{Position, Rect, Size};

#[derive(Default, Resource)]
pub struct EntityTypes {
    pub types: HashMap<String, EntityType>,
}

impl Index<&str> for EntityTypes {
    type Output = EntityType;
    fn index(&self, index: &str) -> &Self::Output {
        self.types.get(index).unwrap()
    }
}

#[derive(Deserialize, Debug)]
pub struct EntityType {
    #[serde(flatten)]
    pub size: Size,
    pub collision: Option<Rect>,
    pub interaction: Option<Interaction>,
    #[serde(flatten)]
    pub image: EntityImage,
    #[serde(skip)]
    pub loaded: Option<Loaded>,
}

#[derive(Debug)]
pub enum Loaded {
    Static(Handle<Image>),
    Animation(LoadedAnimation),
    Animations(LoadedAnimations),
}

#[derive(Debug)]
pub struct LoadedAnimation {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub frames: Vec<(usize, Duration)>,
}

#[derive(Debug)]
pub struct LoadedAnimations {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub frames: bevy::platform::collections::HashMap<String, Vec<(usize, Duration)>>,
}

#[derive(Deserialize, Debug)]
pub enum EntityImage {
    #[serde(rename = "image")]
    Static(String),
    #[serde(rename = "animation")]
    Animation(Frames),
    #[serde(rename = "animations")]
    Animations(HashMap<String, Frames>),
}

pub type Frames = Vec<Frame>;

#[derive(Deserialize, Debug)]
pub struct Frame {
    pub image: String,
    pub duration: u64,
    #[serde(skip)]
    pub index: usize,
}

#[derive(Deserialize, Debug)]
pub struct Interaction {
    pub name: String,
    pub position: Position,
    pub max_distance: u16,
}

pub fn load_entity_types() -> anyhow::Result<EntityTypes> {
    let mut types = HashMap::new();
    for (path, content) in entity_type_files()? {
        // Skip non-yaml files
        if path.extension().is_none_or(|ext| ext != "yaml") {
            continue;
        }
        let entity_type: EntityType = serde_saphyr::from_slice(&content)
            .with_context(|| format!("Parsing {path:?} failed"))?;
        let entity_name = path.file_stem().unwrap().to_string_lossy().into_owned();
        types.insert(entity_name, entity_type);
    }
    Ok(EntityTypes { types })
}

/// The files in `assets/entity_types`, read from disk.
#[cfg(not(feature = "embed"))]
fn entity_type_files() -> anyhow::Result<Vec<(PathBuf, Vec<u8>)>> {
    let dir = "assets/entity_types";
    let mut files = Vec::new();
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("Reading directory {dir:?} failed"))?
    {
        let path = entry?.path();
        // Skip non-regular files
        if !path.is_file() {
            continue;
        }
        let content = std::fs::read(&path).with_context(|| format!("Reading {path:?} failed"))?;
        files.push((path, content));
    }
    Ok(files)
}

/// The files in `assets/entity_types`, embedded into the binary.
#[cfg(feature = "embed")]
fn entity_type_files() -> anyhow::Result<Vec<(PathBuf, Vec<u8>)>> {
    static DIR: include_dir::Dir =
        include_dir::include_dir!("$CARGO_MANIFEST_DIR/assets/entity_types");
    Ok(DIR
        .files()
        .map(|file| (file.path().to_path_buf(), file.contents().to_vec()))
        .collect())
}
