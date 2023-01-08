use std::time::Duration;

use bevy::{
    prelude::{Bundle, Component},
    time::TimerMode,
    utils::HashMap,
};

use crate::systems::animation::AnimationTimer;

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

#[derive(Bundle)]
pub struct AnimationBundle {
    animation: Animation,
    timer: AnimationTimer,
    state: AnimationState,
}

impl AnimationBundle {
    pub fn new(frames: HashMap<String, Vec<(usize, Duration)>>, animation: &'static str) -> Self {
        Self {
            animation: Animation { frames },
            timer: AnimationTimer::from_seconds(0.0, TimerMode::Repeating),
            state: AnimationState {
                animation,
                restart: true,
                index: 0,
            },
        }
    }
}
