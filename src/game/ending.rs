//! The ending screen showing the achievements and credits.

use bevy::prelude::*;

use super::{
    audio::{PlaySfx, Sfx},
    input::{Action, DeviceText, MenuInput},
    progress::{Achievement, CAT_COUNT, Meta, Progress},
};
use crate::AppState;

pub struct EndingPlugin;

impl Plugin for EndingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Ending), spawn_ending_screen)
            .add_systems(Update, leave_ending.run_if(in_state(AppState::Ending)));
    }
}

fn spawn_ending_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    progress: Res<Progress>,
    meta: Res<Meta>,
) {
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
            TextLayout::justify(Justify::Center),
        )
    };
    let gray = Color::srgb(0.55, 0.55, 0.55);
    let gold = Color::srgb(0.95, 0.85, 0.55);
    let title = progress.ending.map_or("The End", |ending| ending.title());

    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            DespawnOnExit(AppState::Ending),
        ))
        .id();
    commands.spawn((text("The End", 24.0, gray), ChildOf(root)));
    commands.spawn((
        text(title, 54.0, Color::srgb(0.9, 0.2, 0.15)),
        ChildOf(root),
    ));

    // The rating of the ritual and the reasons for it.
    let ritual = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(6.0),
                margin: UiRect::vertical(Val::Px(8.0)),
                padding: UiRect::axes(Val::Px(28.0), Val::Px(14.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.05, 0.08)),
            BorderColor::all(Color::srgb(0.45, 0.12, 0.1)),
            ChildOf(root),
        ))
        .id();
    commands.spawn((text("Your ritual", 20.0, gold), ChildOf(ritual)));
    match progress.ritual_stars() {
        Some(stars) => {
            let row = commands
                .spawn((
                    Node {
                        column_gap: Val::Px(4.0),
                        ..default()
                    },
                    ChildOf(ritual),
                ))
                .id();
            for i in 0..5 {
                let icon = if i < stars {
                    "icons/star.png"
                } else {
                    "icons/star_empty.png"
                };
                commands.spawn((
                    ImageNode::new(asset_server.load(icon)),
                    Node {
                        width: Val::Px(40.0),
                        height: Val::Px(40.0),
                        ..default()
                    },
                    ChildOf(row),
                ));
            }
            let notes = progress.ritual_assessment();
            let list = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(3.0),
                        ..default()
                    },
                    ChildOf(ritual),
                ))
                .id();
            for note in notes {
                let (sign, color) = if note.change > 0 {
                    ("-", Color::srgb(0.9, 0.35, 0.3))
                } else {
                    ("+", Color::srgb(0.45, 0.8, 0.4))
                };
                commands.spawn((
                    Node {
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    ChildOf(list),
                    children![
                        text(sign, 18.0, color),
                        text(note.text, 18.0, Color::srgb(0.85, 0.85, 0.85)),
                    ],
                ));
            }
        }
        None => {
            commands.spawn((
                text("The ritual was never performed.", 18.0, gray),
                ChildOf(ritual),
            ));
        }
    }

    commands.spawn((
        text(
            &format!(
                "Achievements: {} / {}      Cats of Ulu: {} / {CAT_COUNT}",
                meta.achievements.len(),
                Achievement::ALL.len(),
                meta.cats.len(),
            ),
            20.0,
            gold,
        ),
        ChildOf(root),
    ));
    // The full list is in the main menu, only show what was unlocked now.
    if !progress.new_achievements.is_empty() {
        let names: Vec<_> = progress
            .new_achievements
            .iter()
            .map(|achievement| achievement.title())
            .collect();
        commands.spawn((
            text(&format!("New: {}", names.join(", ")), 18.0, Color::WHITE),
            ChildOf(root),
        ));
    }
    commands.spawn((
        text("ULU - The Harvest\nThanks for playing!", 20.0, gray),
        Node {
            margin: UiRect::top(Val::Px(8.0)),
            ..default()
        },
        ChildOf(root),
    ));
    commands.spawn((
        text("", 18.0, gray),
        DeviceText(|device| format!("{} Return to the main menu", device.label(Action::Confirm))),
        ChildOf(root),
    ));
}

fn leave_ending(
    input: MenuInput,
    mouse: Res<ButtonInput<MouseButton>>,
    mut app_state: ResMut<NextState<AppState>>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let actions = input.read();
    if actions.confirm || actions.back || mouse.just_pressed(MouseButton::Left) {
        sfx.write(PlaySfx(Sfx::Click));
        app_state.set(AppState::Menu);
    }
}
