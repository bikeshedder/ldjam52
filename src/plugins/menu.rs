use bevy::{app::AppExit, prelude::*};

use crate::{
    AppState,
    game::{
        audio::{PlaySfx, Sfx},
        input::{Action, DeviceText, InputDevice},
        progress::{Achievement, CAT_COUNT, Meta},
    },
};

#[derive(Default)]
pub struct Menu;

impl Plugin for Menu {
    fn build(&self, app: &mut App) {
        app
            // The menu state only exists while in `AppState::Menu`, starting at `MenuState::Main`.
            .add_sub_state::<MenuState>()
            // Systems to handle the main menu screen
            .add_systems(OnEnter(MenuState::Main), main_menu_setup)
            .add_systems(
                Update,
                (menu_action, button_system).run_if(in_state(AppState::Menu)),
            );
    }
}

const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const HOVERED_PRESSED_BUTTON: Color = Color::srgb(0.25, 0.65, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

const MENU_BG: Color = Color::srgb(0.1, 0.1, 0.1);

fn main_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>, meta: Res<Meta>) {
    log::info!("main_menu_setup");
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    // Common style for all buttons on the screen
    let button_node = Node {
        width: Val::Px(250.0),
        height: Val::Px(65.0),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    /*
    let button_icon_node = Node {
        width: Val::Px(30.0),
        // This takes the icons out of the flexbox flow, to be positioned exactly
        position_type: PositionType::Absolute,
        // The icon will be close to the left border of the button
        left: Val::Px(10.0),
        ..default()
    };
     */
    let button_text_font = TextFont {
        font: font.clone().into(),
        font_size: 40.0.into(),
        ..default()
    };

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        DespawnOnExit(MenuState::Main),
        children![(
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(MENU_BG),
            children![
                // Display the game name
                (
                    Text::new("ULU"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: 80.0.into(),
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Node {
                        margin: UiRect::top(Val::Px(30.0)),
                        ..default()
                    },
                ),
                (
                    Text::new("Harvest"),
                    TextFont {
                        font: font.clone().into(),
                        font_size: 32.0.into(),
                        ..default()
                    },
                    TextColor(Color::srgb(0.85, 0.2, 0.15)),
                    Node {
                        margin: UiRect::bottom(Val::Px(30.0)),
                        ..default()
                    },
                ),
                // Display a button for each action available from the main menu:
                // - new game
                // - quit
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::Play,
                    children![
                        /*
                        (
                            ImageNode::new(asset_server.load("textures/Game Icons/right.png")),
                            button_icon_node.clone(),
                        ),
                         */
                        (
                            Text::new("New game"),
                            button_text_font.clone(),
                            TextColor(TEXT_COLOR),
                        ),
                    ],
                ),
                /*
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::Settings,
                    children![(
                        Text::new("Settings"),
                        button_text_font.clone(),
                        TextColor(TEXT_COLOR),
                    )],
                ),
                 */
                (
                    Button,
                    button_node,
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::Quit,
                    children![(Text::new("Quit"), button_text_font, TextColor(TEXT_COLOR))],
                ),
                (
                    Text::new(format!(
                        "Achievements: {} / {}    Cats of Ulu: {} / {CAT_COUNT}",
                        meta.achievements.len(),
                        Achievement::ALL.len(),
                        meta.cats.len()
                    )),
                    TextFont {
                        font: font.clone().into(),
                        font_size: 20.0.into(),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.85, 0.55)),
                    Node {
                        margin: UiRect::top(Val::Px(20.0)),
                        ..default()
                    },
                ),
                (
                    Text::default(),
                    DeviceText(|device| {
                        let movement = match device {
                            InputDevice::Keyboard => "WASD / arrow keys",
                            InputDevice::Gamepad => "left stick / D-Pad",
                        };
                        format!(
                            "Move: {movement}    Action: {}    Meditate: {}",
                            device.label(Action::Confirm),
                            device.label(Action::Pause)
                        )
                    }),
                    TextFont {
                        font: font.clone().into(),
                        font_size: 16.0.into(),
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.6, 0.6)),
                    Node {
                        margin: UiRect::top(Val::Px(12.0)),
                        ..default()
                    },
                ),
            ],
        )],
    ));
}

#[derive(Component)]
struct SelectedOption;

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, Option<&SelectedOption>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, selected) in &mut interaction_query {
        *color = match (*interaction, selected) {
            (Interaction::Pressed, _) | (Interaction::None, Some(_)) => PRESSED_BUTTON.into(),
            (Interaction::Hovered, Some(_)) => HOVERED_PRESSED_BUTTON.into(),
            (Interaction::Hovered, None) => HOVERED_BUTTON.into(),
            (Interaction::None, None) => NORMAL_BUTTON.into(),
        }
    }
}

// State used for the current menu screen
#[derive(SubStates, Clone, Copy, Default, Eq, PartialEq, Debug, Hash)]
#[source(AppState = AppState::Menu)]
enum MenuState {
    #[default]
    Main,
    Settings,
    SettingsDisplay,
    SettingsSound,
}

// All actions that can be triggered from a button click
#[derive(Component)]
enum MenuButtonAction {
    Play,
    Settings,
    SettingsDisplay,
    SettingsSound,
    BackToMainMenu,
    BackToSettings,
    Quit,
}

fn menu_action(
    interaction_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut app_exit: MessageWriter<AppExit>,
    mut sfx: MessageWriter<PlaySfx>,
    mut menu_state: ResMut<NextState<MenuState>>,
    mut game_state: ResMut<NextState<AppState>>,
) {
    for (interaction, menu_button_action) in &interaction_query {
        if *interaction == Interaction::Pressed {
            sfx.write(PlaySfx(Sfx::Click));
            match menu_button_action {
                MenuButtonAction::Quit => {
                    app_exit.write(AppExit::Success);
                }
                MenuButtonAction::Play => game_state.set(AppState::Game),
                MenuButtonAction::Settings => menu_state.set(MenuState::Settings),
                MenuButtonAction::SettingsDisplay => menu_state.set(MenuState::SettingsDisplay),
                MenuButtonAction::SettingsSound => menu_state.set(MenuState::SettingsSound),
                MenuButtonAction::BackToMainMenu => menu_state.set(MenuState::Main),
                MenuButtonAction::BackToSettings => menu_state.set(MenuState::Settings),
            }
        }
    }
}
