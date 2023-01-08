use std::time::Duration;

use bevy::{prelude::Component, utils::HashMap};

#[derive(Component, Debug)]
pub struct Animation {
    pub frames: HashMap<String, Vec<(usize, Duration)>>,
}

#[derive(Component, Debug)]
pub struct AnimationState {
    pub animation: &'static str,
    pub restart: bool,
    pub index: usize,
}

impl AnimationState {
    pub fn start(&mut self, animation: &'static str) {
        if animation != self.animation {
            self.animation = animation;
            self.restart = true;
            self.index = 0;
        }
    }
}
