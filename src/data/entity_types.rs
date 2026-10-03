use std::{collections::HashMap, ops::Index, time::Duration};

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
    let dir = "assets/entity_types";
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("Reading directory {dir:?} failed"))?
    {
        let path = entry?.path();
        // Skip non-regular and non-yaml files
        if !path.is_file() || path.extension().is_none_or(|ext| ext != "yaml") {
            continue;
        }
        let file =
            std::fs::File::open(&path).with_context(|| format!("Reading {path:?} failed"))?;
        let entity_type: EntityType =
            serde_saphyr::from_reader(file).with_context(|| format!("Parsing {path:?} failed"))?;
        let entity_name = path.file_stem().unwrap().to_string_lossy().into_owned();
        types.insert(entity_name, entity_type);
    }
    Ok(EntityTypes { types })
}
