//! Overlays and the action bar.

use bevy::prelude::*;

use super::{
    Phase,
    audio::{PlaySfx, Sfx},
    dialogue::Dialogue,
    input::MenuInput,
    progress::Progress,
    script::Speaker,
};
use crate::AppState;

/// Duration of a short fade to black and back.
const PULSE_SECS: f32 = 1.4;

/// Fading the screen to black.
#[derive(Resource)]
pub struct ScreenFx {
    pub blackout: bool,
    pulse: f32,
    alpha: f32,
}

impl Default for ScreenFx {
    fn default() -> Self {
        // Every game starts in the dark, dreaming of Ulu.
        Self {
            blackout: true,
            pulse: 0.0,
            alpha: 1.0,
        }
    }
}

impl ScreenFx {
    pub fn pulse(&mut self) {
        self.pulse = PULSE_SECS;
    }

    pub fn is_black(&self) -> bool {
        self.alpha >= 1.0
    }
}

/// Whether the player is in a room without light.
#[derive(Resource, Default)]
pub struct Darkness(pub bool);

/// Text of the interaction available to the player.
#[derive(Resource, Default)]
pub struct Prompt(pub Option<String>);

#[derive(Component)]
struct FadeOverlay;
#[derive(Component)]
struct DarknessOverlay;
#[derive(Component)]
struct UluEyes;
#[derive(Component)]
struct PromptText;
#[derive(Component)]
struct InventoryText;
#[derive(Component)]
struct PauseMenu;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum PauseButton {
    Resume,
    MainMenu,
}

#[derive(Resource, Default)]
struct PauseSelection(usize);

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Darkness>()
            .init_resource::<Prompt>()
            .init_resource::<PauseSelection>()
            .add_systems(OnEnter(AppState::Game), (reset_hud, spawn_hud))
            .add_systems(
                Update,
                (
                    update_fade,
                    update_darkness,
                    update_eyes,
                    update_action_bar,
                    pause.run_if(in_state(Phase::Exploring)),
                    pause_menu.run_if(in_state(Phase::Paused)),
                )
                    .run_if(in_state(AppState::Game)),
            )
            .add_systems(OnEnter(Phase::Paused), spawn_pause_menu);
    }
}

fn reset_hud(mut commands: Commands) {
    commands.insert_resource(ScreenFx::default());
    commands.insert_resource(Darkness::default());
    commands.insert_resource(Prompt::default());
}

fn full_screen() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    }
}

fn spawn_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn((
        full_screen(),
        BackgroundColor(Color::NONE),
        GlobalZIndex(10),
        DarknessOverlay,
        DespawnOnExit(AppState::Game),
    ));
    commands.spawn((
        full_screen(),
        BackgroundColor(Color::BLACK),
        GlobalZIndex(20),
        FadeOverlay,
        DespawnOnExit(AppState::Game),
    ));
    let eye = || {
        (
            Node {
                width: Val::Px(70.0),
                height: Val::Px(34.0),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(Color::srgb(1.0, 0.15, 0.05)),
            BoxShadow::new(
                Color::srgba(1.0, 0.1, 0.0, 0.8),
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Px(2.0),
                Val::Px(16.0),
            ),
        )
    };
    commands.spawn((
        Node {
            top: Val::Percent(22.0),
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(110.0),
            ..full_screen()
        },
        GlobalZIndex(25),
        Visibility::Hidden,
        UluEyes,
        DespawnOnExit(AppState::Game),
        children![eye(), eye()],
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            right: Val::Px(16.0),
            top: Val::Px(12.0),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        },
        GlobalZIndex(30),
        DespawnOnExit(AppState::Game),
        children![
            (
                Text::default(),
                TextFont {
                    font: font.clone().into(),
                    font_size: 20.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.8, 0.7)),
                TextShadow::default(),
                InventoryText,
            ),
            (
                Text::default(),
                TextFont {
                    font: font.into(),
                    font_size: 22.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.6)),
                TextShadow::default(),
                PromptText,
            ),
        ],
    ));
}

fn update_fade(
    time: Res<Time>,
    mut fx: ResMut<ScreenFx>,
    mut overlay: Single<&mut BackgroundColor, With<FadeOverlay>>,
) {
    let dt = time.delta_secs();
    fx.pulse = (fx.pulse - dt).max(0.0);
    let target = if fx.blackout { 1.0 } else { 0.0 };
    let alpha = fx.alpha + (target - fx.alpha).clamp(-dt * 1.5, dt * 1.5);
    fx.alpha = alpha;
    let pulse = (std::f32::consts::PI * fx.pulse / PULSE_SECS).sin();
    overlay.0 = Color::BLACK.with_alpha(alpha.max(pulse));
}

fn update_darkness(
    time: Res<Time>,
    darkness: Res<Darkness>,
    mut overlay: Single<&mut BackgroundColor, With<DarknessOverlay>>,
) {
    let current = overlay.0.alpha();
    let target = if darkness.0 { 0.93 } else { 0.0 };
    let dt = time.delta_secs() * 2.0;
    overlay.0 = Color::BLACK.with_alpha(current + (target - current).clamp(-dt, dt));
}

fn update_eyes(
    time: Res<Time>,
    dialogue: Res<Dialogue>,
    phase: Res<State<Phase>>,
    eyes: Single<(&mut Visibility, &Children), With<UluEyes>>,
    mut colors: Query<&mut BackgroundColor>,
) {
    let (mut visibility, children) = eyes.into_inner();
    let visible = *phase.get() == Phase::Dialogue && dialogue.speaker() == Some(Speaker::Ulu);
    *visibility = if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let glow = 0.75 + 0.25 * (time.elapsed_secs() * 3.0).sin();
    for child in children {
        if let Ok(mut color) = colors.get_mut(*child) {
            color.0 = Color::srgb(glow, 0.15 * glow, 0.05);
        }
    }
}

fn update_action_bar(
    progress: Res<Progress>,
    prompt: Res<Prompt>,
    phase: Res<State<Phase>>,
    mut prompt_text: Single<&mut Text, (With<PromptText>, Without<InventoryText>)>,
    mut inventory_text: Single<&mut Text, (With<InventoryText>, Without<PromptText>)>,
) {
    let prompt = match (&prompt.0, phase.get()) {
        (Some(prompt), Phase::Exploring) => format!("[Space] {prompt}"),
        _ => String::new(),
    };
    if prompt_text.0 != prompt {
        prompt_text.0 = prompt;
    }
    if progress.is_changed() {
        inventory_text.0 = if progress.inventory.is_empty() {
            String::from("Inventory: -")
        } else {
            let items: Vec<_> = progress.inventory.iter().map(|item| item.name()).collect();
            format!("Inventory: {}", items.join(", "))
        };
    }
}

fn pause(input: MenuInput, mut phase: ResMut<NextState<Phase>>) {
    if input.read().back {
        phase.set(Phase::Paused);
    }
}

fn spawn_pause_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut selection: ResMut<PauseSelection>,
) {
    selection.0 = 0;
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let button = |label: &str, action: PauseButton| {
        (
            Button,
            Node {
                width: Val::Px(320.0),
                padding: UiRect::all(Val::Px(12.0)),
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            action,
            children![(
                Text::new(label),
                TextFont {
                    font: font.clone().into(),
                    font_size: 28.0.into(),
                    ..default()
                },
            )],
        )
    };
    commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(16.0),
            ..full_screen()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        GlobalZIndex(50),
        PauseMenu,
        DespawnOnExit(Phase::Paused),
        children![
            (
                Text::new("You meditate..."),
                TextFont {
                    font: font.clone().into(),
                    font_size: 44.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.3, 0.2)),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ),
            button("Continue", PauseButton::Resume),
            button("Return to main menu", PauseButton::MainMenu),
        ],
    ));
}

fn pause_menu(
    input: MenuInput,
    mut selection: ResMut<PauseSelection>,
    mut buttons: Query<(&Interaction, &PauseButton, &mut BackgroundColor)>,
    mut phase: ResMut<NextState<Phase>>,
    mut app_state: ResMut<NextState<AppState>>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    const BUTTONS: [PauseButton; 2] = [PauseButton::Resume, PauseButton::MainMenu];
    let actions = input.read();
    if actions.back {
        phase.set(Phase::Exploring);
        return;
    }
    if actions.up || actions.down {
        selection.0 = (selection.0 + 1) % BUTTONS.len();
    }
    let mut chosen = actions.confirm.then_some(BUTTONS[selection.0]);
    for (interaction, button, mut color) in &mut buttons {
        let index = BUTTONS.iter().position(|b| b == button).unwrap();
        match interaction {
            Interaction::Pressed => chosen = Some(*button),
            Interaction::Hovered => selection.0 = index,
            Interaction::None => {}
        }
        color.0 = if index == selection.0 {
            Color::srgb(0.5, 0.12, 0.1)
        } else {
            Color::srgb(0.15, 0.15, 0.15)
        };
    }
    match chosen {
        Some(PauseButton::Resume) => {
            sfx.write(PlaySfx(Sfx::Click));
            phase.set(Phase::Exploring);
        }
        Some(PauseButton::MainMenu) => {
            sfx.write(PlaySfx(Sfx::Click));
            app_state.set(AppState::Menu);
        }
        None => {}
    }
}
