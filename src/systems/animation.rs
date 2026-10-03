use bevy::prelude::*;

use crate::components::animation::{Animation, AnimationState};

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

impl Default for AnimationTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.0, TimerMode::Repeating))
    }
}

pub fn animation_system(
    time: Res<Time>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut Sprite,
        &Animation,
        &mut AnimationState,
    )>,
) {
    for (mut timer, mut sprite, animation, mut state) in &mut query {
        let frames = &animation.frames[state.animation];
        if state.restart {
            state.restart = false;
            state.index = 0;
            timer.reset();
        } else {
            timer.tick(time.delta());
            if !timer.just_finished() {
                continue;
            }
            state.index = (state.index + 1) % frames.len();
        }
        let (atlas_index, duration) = frames[state.index];
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = atlas_index;
        }
        timer.set_duration(duration);
    }
}
