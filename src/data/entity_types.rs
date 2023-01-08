use std::{borrow::Cow, ops::Index, time::Duration};

use bevy::{
    prelude::{Handle, Image, Resource},
    sprite::TextureAtlas,
    utils::HashMap,
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
    pub atlas: Handle<TextureAtlas>,
    pub frames: Vec<(usize, Duration)>,
}

#[derive(Debug)]
pub struct LoadedAnimations {
    pub atlas: Handle<TextureAtlas>,
    pub frames: HashMap<String, Vec<(usize, Duration)>>,
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

pub fn load_entity_types() -> Result<EntityTypes, anyhow::Error> {
    let mut entity_types: HashMap<String, EntityType> = HashMap::default();
    let dir = "assets/entity_types";
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Reading directory {:?} failed: {:?}", dir, e))
    {
        let entry = entry?;
        if !entry.metadata()?.is_file() {
            // Skip non-regular files
            continue;
        }
        let path = entry.path();
        let ext = path.extension().map(|ext| ext.to_string_lossy());
        if ext != Some(Cow::Borrowed("yaml")) {
            // Skip non-yaml files
            continue;
        }
        let file = std::fs::File::open(path.clone())
            .unwrap_or_else(|e| panic!("Reading {:?} failed: {:?}", path, e));
        let entity_type: EntityType = serde_yaml::from_reader(file)
            .unwrap_or_else(|e| panic!("Parsing {:?} failed: {:?}", path, e));
        let entity_name = path.file_stem().unwrap().to_string_lossy();
        entity_types.insert(entity_name.to_string(), entity_type);
    }
    Ok(EntityTypes {
        types: entity_types,
    })
}
