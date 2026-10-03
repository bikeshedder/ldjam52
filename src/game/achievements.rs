//! Pop-ups for newly unlocked achievements.

use std::collections::VecDeque;

use bevy::prelude::*;

use super::{
    audio::{PlaySfx, Sfx},
    progress::{Achievement, Progress},
};

const SLIDE_SECS: f32 = 0.35;
const SHOW_SECS: f32 = 4.5;
const WIDTH: f32 = 380.0;
const MARGIN: f32 = 16.0;

/// Achievements waiting to be shown.
#[derive(Resource, Default)]
struct Queue {
    pending: VecDeque<Achievement>,
    /// Number of achievements of the current game which were already queued.
    seen: usize,
}

/// The pop-up currently on screen.
#[derive(Component)]
struct Toast {
    age: f32,
}

pub struct AchievementsPlugin;

impl Plugin for AchievementsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Queue>().add_systems(
            Update,
            (queue_achievements, show_toast, animate_toast).chain(),
        );
    }
}

fn queue_achievements(progress: Res<Progress>, mut queue: ResMut<Queue>) {
    let new = &progress.new_achievements;
    // A new game started.
    if new.len() < queue.seen {
        queue.seen = 0;
    }
    let unseen: Vec<_> = new[queue.seen..].to_vec();
    queue.seen = new.len();
    queue.pending.extend(unseen);
}

fn show_toast(
    mut commands: Commands,
    mut queue: ResMut<Queue>,
    toasts: Query<(), With<Toast>>,
    asset_server: Res<AssetServer>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    if !toasts.is_empty() {
        return;
    }
    let Some(achievement) = queue.pending.pop_front() else {
        return;
    };
    sfx.write(PlaySfx(Sfx::Feather));
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text = |content: &str, size: f32, color: Color| {
        (
            Text::new(content),
            TextFont {
                font: font.clone().into(),
                font_size: size.into(),
                ..default()
            },
            TextColor(color),
        )
    };
    // Not tied to a state, so it stays visible when the game ends.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(MARGIN),
            right: Val::Px(-WIDTH - MARGIN),
            width: Val::Px(WIDTH),
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.07, 0.05, 0.08, 0.95)),
        BorderColor::all(Color::srgb(0.75, 0.55, 0.2)),
        GlobalZIndex(100),
        Toast { age: 0.0 },
        children![
            (
                ImageNode::new(asset_server.load("icons/achievement.png")),
                Node {
                    width: Val::Px(56.0),
                    height: Val::Px(56.0),
                    flex_shrink: 0.0,
                    ..default()
                },
            ),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    flex_shrink: 1.0,
                    ..default()
                },
                children![
                    text("Achievement unlocked", 14.0, Color::srgb(0.75, 0.55, 0.2)),
                    text(achievement.title(), 20.0, Color::WHITE),
                    text(achievement.description(), 15.0, Color::srgb(0.7, 0.7, 0.7)),
                ],
            ),
        ],
    ));
}

/// Slides the pop-up in from the right, keeps it for a while and slides it out.
fn animate_toast(
    mut commands: Commands,
    time: Res<Time>,
    mut toasts: Query<(Entity, &mut Toast, &mut Node)>,
) {
    for (entity, mut toast, mut node) in &mut toasts {
        toast.age += time.delta_secs();
        let shown = if toast.age < SLIDE_SECS {
            toast.age / SLIDE_SECS
        } else if toast.age < SLIDE_SECS + SHOW_SECS {
            1.0
        } else {
            1.0 - (toast.age - SLIDE_SECS - SHOW_SECS) / SLIDE_SECS
        };
        if shown <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        // Ease out, so it comes in quickly and settles softly.
        let eased = 1.0 - (1.0 - shown.clamp(0.0, 1.0)).powi(3);
        node.right = Val::Px(MARGIN - (1.0 - eased) * (WIDTH + 2.0 * MARGIN));
    }
}
