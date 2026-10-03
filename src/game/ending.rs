//! The ending screen showing the achievements and credits.

use bevy::prelude::*;

use super::{
    audio::{PlaySfx, Sfx},
    input::MenuInput,
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
    let text = |content: String, size: f32, color: Color| {
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
    let title = progress.ending.map_or("The End", |ending| ending.title());

    let achievements: Vec<String> = Achievement::ALL
        .iter()
        .map(|achievement| {
            let unlocked = meta.achievements.contains(achievement);
            let name = if unlocked || !achievement.secret() {
                achievement.title()
            } else {
                "???"
            };
            let new = if progress.new_achievements.contains(achievement) {
                "  (new!)"
            } else {
                ""
            };
            format!("{} {name}{new}", if unlocked { "[x]" } else { "[ ]" })
        })
        .collect();

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            DespawnOnExit(AppState::Ending),
        ))
        .with_children(|parent| {
            parent.spawn(text("The End".into(), 26.0, gray));
            parent.spawn(text(title.into(), 60.0, Color::srgb(0.9, 0.2, 0.15)));
            parent.spawn(text(
                format!(
                    "Achievements: {} / {}      Cats of Ulu: {} / {CAT_COUNT}",
                    meta.achievements.len(),
                    Achievement::ALL.len(),
                    meta.cats.len(),
                ),
                26.0,
                Color::srgb(0.95, 0.85, 0.55),
            ));
            parent.spawn(text(
                achievements.join("\n"),
                20.0,
                Color::srgb(0.85, 0.85, 0.85),
            ));
            parent.spawn(text(
                "ULU - made for Ludum Dare 52, theme \"Harvest\"\nThanks for playing!".into(),
                22.0,
                gray,
            ));
            parent.spawn(text("[Space] Return to the main menu".into(), 18.0, gray));
        });
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
