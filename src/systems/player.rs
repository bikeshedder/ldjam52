use bevy::prelude::*;

use crate::{
    components::{
        animation::AnimationState,
        player::{Player, PlayerDirection, PlayerState},
    },
    game::{
        audio::{PlaySfx, Sfx},
        iso::{FEET_OFFSET, Walkable},
        world::Solid,
    },
};

pub const PLAYER_SPEED_X: f32 = 300.0;
pub const PLAYER_SPEED_Y: f32 = 150.0;
const STEP_INTERVAL: f32 = 0.3;
const STEPS: [Sfx; 3] = [Sfx::Step1, Sfx::Step2, Sfx::Step3];

#[derive(Default)]
pub struct Footsteps {
    timer: f32,
    index: usize,
}

#[allow(clippy::type_complexity)]
pub fn player_system(
    time: Res<Time>,
    walkable: Res<Walkable>,
    query: Single<(
        &mut Player,
        &mut Transform,
        &mut AnimationState,
        &mut Sprite,
    )>,
    solids: Query<(&Transform, &Solid), Without<Player>>,
    mut footsteps: Local<Footsteps>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let (mut player, mut transform, mut animation, mut sprite) = query.into_inner();
    let delta = time.delta().as_secs_f32();

    if player.is_moving() {
        player.state = PlayerState::Walk;
        player.direction = player.primary_direction();
    } else {
        player.state = PlayerState::Idle;
    }

    if player.input.interact {
        player.state = PlayerState::Interact;
    }

    if player.state == PlayerState::Walk {
        let feet = transform.translation.truncate() + FEET_OFFSET;
        let can_stand = |pos: Vec2| {
            walkable.is_walkable(pos)
                && solids.iter().all(|(solid_transform, solid)| {
                    let solid_feet = solid_transform.translation.truncate()
                        + FEET_OFFSET * solid_transform.scale.y;
                    // Moving away from a solid is always allowed so nobody gets stuck.
                    pos.distance(solid_feet) >= solid.radius
                        || pos.distance(solid_feet) > feet.distance(solid_feet)
                })
        };
        let step = Vec2::new(
            player.input.x * PLAYER_SPEED_X,
            player.input.y * PLAYER_SPEED_Y,
        ) * delta;
        // Slide along walls if the full step is blocked.
        let moved = [step, Vec2::new(step.x, 0.0), Vec2::new(0.0, step.y)]
            .into_iter()
            .find(|step| can_stand(feet + *step));
        if let Some(step) = moved {
            transform.translation += step.extend(0.0);
            footsteps.timer -= delta;
            if footsteps.timer <= 0.0 {
                footsteps.timer = STEP_INTERVAL;
                footsteps.index = (footsteps.index + 1) % STEPS.len();
                sfx.write(PlaySfx(STEPS[footsteps.index]));
            }
        }
    } else {
        footsteps.timer = 0.0;
    }

    animation.start(match (player.state, player.direction) {
        (PlayerState::Walk, PlayerDirection::NE) => "walk_up",
        (PlayerState::Walk, PlayerDirection::NW) => "walk_up",
        (PlayerState::Walk, PlayerDirection::SE) => "walk_down",
        (PlayerState::Walk, PlayerDirection::SW) => "walk_down",
        (_, PlayerDirection::NE) => "idle_up",
        (_, PlayerDirection::NW) => "idle_up",
        (_, PlayerDirection::SE) => "idle_down",
        (_, PlayerDirection::SW) => "idle_down",
    });

    sprite.flip_x = matches!(player.direction, PlayerDirection::NW | PlayerDirection::SW);
}
