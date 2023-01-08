use bevy::{math::Vec3, prelude::Component};

#[derive(Component, Debug)]
pub struct Interaction {
    pub name: String,
    pub center: Vec3,
    pub max_distance: u16,
}
