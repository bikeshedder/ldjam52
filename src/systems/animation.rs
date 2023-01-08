use bevy::{
    prelude::{Component, Query, Res},
    sprite::TextureAtlasSprite,
    time::{Time, Timer, TimerMode},
};

use crate::components::animation::{Animation, AnimationState};

#[derive(Component)]
pub struct AnimationTimer {
    pub timer: Timer,
}

impl AnimationTimer {
    pub fn from_seconds(duration: f32, mode: TimerMode) -> Self {
        Self {
            timer: Timer::from_seconds(duration, mode),
        }
    }
}

pub fn animation_system(
    time: Res<Time>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
        &Animation,
        &mut AnimationState,
    )>,
) {
    for (mut timer, mut sprite, animation, mut state) in query.iter_mut() {
        let update = if state.restart {
            state.restart = false;
            true
        } else {
            timer.timer.tick(time.delta());
            timer.timer.finished()
        };
        if update {
            let frames = &animation.frames[state.animation];
            state.index = (state.index + 1) % frames.len();
            let (atlas_index, duration) = frames[state.index];
            sprite.index = atlas_index;
            timer.timer.set_duration(duration);
        }
    }
}
