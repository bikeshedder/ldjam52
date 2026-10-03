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
/// Distance the player keeps to walls and furniture behind it, in map cells.
const PLAYER_MARGIN: f32 = 0.35;
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
            walkable.is_free(pos, PLAYER_MARGIN)
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
        let moved = if can_stand(feet + step) {
            Some(step)
        } else {
            slide(step, feet, &solids).find(|step| can_stand(feet + *step))
        };
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

/// Directions of the map axes on screen. Walls run along them.
const MAP_X: Vec2 = Vec2::new(64.0, -32.0);
const MAP_Y: Vec2 = Vec2::new(-64.0, -32.0);

/// Splits a step into its parts along the map's x and y axes.
fn split_along_map_axes(step: Vec2) -> (Vec2, Vec2) {
    let det = MAP_X.perp_dot(MAP_Y);
    (
        MAP_X * step.perp_dot(MAP_Y) / det,
        MAP_Y * MAP_X.perp_dot(step) / det,
    )
}

/// Alternative steps when the full step is blocked, longest first: the parts of
/// the step along the walls and around round obstacles. This keeps the part of
/// the movement which is still possible instead of stopping at walls.
fn slide(
    step: Vec2,
    feet: Vec2,
    solids: &Query<(&Transform, &Solid), Without<Player>>,
) -> impl Iterator<Item = Vec2> {
    let (along_x, along_y) = split_along_map_axes(step);
    let mut candidates = vec![along_x, along_y];
    // Around round obstacles: drop the part of the step towards them.
    for (transform, solid) in solids {
        let center = transform.translation.truncate() + FEET_OFFSET * transform.scale.y;
        let to_center = center - feet;
        if to_center.length() < solid.radius * 1.5 {
            let normal = to_center.normalize_or_zero();
            candidates.push(step - normal * step.dot(normal).max(0.0));
        }
    }
    candidates.retain(|candidate| candidate.length() > step.length() * 0.05);
    candidates.sort_by(|a, b| b.length().total_cmp(&a.length()));
    candidates.into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_split_along_the_map_axes() {
        // Moving straight up on screen goes diagonally against both map axes.
        let (along_x, along_y) = split_along_map_axes(Vec2::new(0.0, 10.0));
        assert!((along_x + along_y - Vec2::new(0.0, 10.0)).length() < 1e-4);
        assert!(along_x.perp_dot(MAP_X).abs() < 1e-4);
        assert!(along_y.perp_dot(MAP_Y).abs() < 1e-4);
        // A step along a wall stays as it is.
        let (along_x, along_y) = split_along_map_axes(MAP_X);
        assert!((along_x - MAP_X).length() < 1e-4 && along_y.length() < 1e-4);
    }
}
