use bevy::{app::AppExit, prelude::*};

use crate::{
    AppState,
    game::{
        audio::{PlaySfx, Sfx},
        input::{Action, DeviceText, InputDevice, MenuInput},
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
            .init_resource::<MenuSelection>()
            // Systems to handle the main menu screen
            .add_systems(OnEnter(MenuState::Main), main_menu_setup)
            .add_systems(OnEnter(MenuState::Achievements), achievements_setup)
            .add_systems(OnEnter(MenuState::Credits), credits_setup)
            .add_systems(Update, menu_navigation.run_if(in_state(AppState::Menu)));
    }
}

const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

const MENU_BG: Color = Color::srgb(0.1, 0.1, 0.1);

fn main_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>, meta: Res<Meta>) {
    log::info!("main_menu_setup");
    commands.insert_resource(MenuSelection::default());
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    // Common style for all buttons on the screen
    let button_node = Node {
        width: Val::Px(320.0),
        height: Val::Px(60.0),
        margin: UiRect::all(Val::Px(10.0)),
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
                    Text::new("The Harvest"),
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
                    button_node.clone(),
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::Achievements,
                    children![(
                        Text::new("Achievements"),
                        button_text_font.clone(),
                        TextColor(TEXT_COLOR)
                    )],
                ),
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(NORMAL_BUTTON),
                    MenuButtonAction::Credits,
                    children![(
                        Text::new("Credits"),
                        button_text_font.clone(),
                        TextColor(TEXT_COLOR)
                    )],
                ),
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

// State used for the current menu screen
#[derive(SubStates, Clone, Copy, Default, Eq, PartialEq, Debug, Hash)]
#[source(AppState = AppState::Menu)]
enum MenuState {
    #[default]
    Main,
    Achievements,
    Credits,
}

// All actions that can be triggered from a button click
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MenuButtonAction {
    Play,
    Achievements,
    Credits,
    BackToMainMenu,
    Quit,
}

impl MenuState {
    /// The buttons of the screen from top to bottom.
    fn buttons(self) -> &'static [MenuButtonAction] {
        match self {
            Self::Main => &[
                MenuButtonAction::Play,
                MenuButtonAction::Achievements,
                MenuButtonAction::Credits,
                MenuButtonAction::Quit,
            ],
            Self::Achievements | Self::Credits => &[MenuButtonAction::BackToMainMenu],
        }
    }
}

/// Index of the highlighted button in [`MenuState::buttons`].
#[derive(Resource, Default)]
struct MenuSelection(usize);

fn achievements_setup(mut commands: Commands, asset_server: Res<AssetServer>, meta: Res<Meta>) {
    commands.insert_resource(MenuSelection::default());
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
    let entries: Vec<_> = Achievement::ALL
        .iter()
        .map(|achievement| {
            let unlocked = meta.achievements.contains(achievement);
            let hidden = achievement.secret() && !unlocked;
            let (title, description) = if hidden {
                ("???", "A secret achievement. Keep playing to discover it.")
            } else {
                (achievement.title(), achievement.description())
            };
            let icon = if unlocked {
                "icons/achievement.png"
            } else {
                "icons/achievement_locked.png"
            };
            let (title_color, description_color) = if unlocked {
                (Color::WHITE, Color::srgb(0.7, 0.7, 0.7))
            } else {
                (Color::srgb(0.55, 0.55, 0.55), Color::srgb(0.42, 0.42, 0.42))
            };
            (
                Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.13, 0.12, 0.13)),
                children![
                    (
                        ImageNode::new(asset_server.load(icon)),
                        Node {
                            width: Val::Px(44.0),
                            height: Val::Px(44.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ),
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 1.0,
                            ..default()
                        },
                        children![
                            text(title, 18.0, title_color),
                            text(description, 14.0, description_color),
                        ],
                    ),
                ],
            )
        })
        .collect();

    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            DespawnOnExit(MenuState::Achievements),
        ))
        .id();
    let panel = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(MENU_BG),
            ChildOf(root),
        ))
        .id();
    commands.spawn((text("Achievements", 36.0, TEXT_COLOR), ChildOf(panel)));
    commands.spawn((
        text(
            &format!(
                "{} of {} unlocked",
                meta.achievements.len(),
                Achievement::ALL.len()
            ),
            18.0,
            Color::srgb(0.95, 0.85, 0.55),
        ),
        ChildOf(panel),
    ));
    let grid = commands
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: vec![GridTrack::px(440.0); 2],
                column_gap: Val::Px(10.0),
                row_gap: Val::Px(8.0),
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    for entry in entries {
        commands.spawn((entry, ChildOf(grid)));
    }
    commands.spawn((
        Button,
        Node {
            width: Val::Px(200.0),
            height: Val::Px(50.0),
            margin: UiRect::top(Val::Px(6.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(NORMAL_BUTTON),
        MenuButtonAction::BackToMainMenu,
        ChildOf(panel),
        children![text("Back", 28.0, TEXT_COLOR)],
    ));
}

/// The people who made the game and what they did.
const CREDITS: [(&str, &str); 5] = [
    ("Michael P. Jung", "Story & Software Engineer"),
    ("Tim Markmann", "Story & Level Design"),
    ("Tom Haase", "Music & Sounds"),
    ("Manuel Terranova", "Story & Dialogs"),
    ("Claude AI", "Junior Developer"),
];

fn credits_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(MenuSelection::default());
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
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            DespawnOnExit(MenuState::Credits),
        ))
        .id();
    let panel = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(6.0),
                padding: UiRect::axes(Val::Px(60.0), Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(MENU_BG),
            ChildOf(root),
        ))
        .id();
    commands.spawn((text("Credits", 36.0, TEXT_COLOR), ChildOf(panel)));
    commands.spawn((
        text("ULU - The Harvest", 20.0, Color::srgb(0.85, 0.2, 0.15)),
        Node {
            margin: UiRect::bottom(Val::Px(14.0)),
            ..default()
        },
        ChildOf(panel),
    ));
    for (name, role) in CREDITS {
        commands.spawn((text(name, 26.0, Color::WHITE), ChildOf(panel)));
        commands.spawn((
            text(role, 17.0, Color::srgb(0.75, 0.55, 0.2)),
            Node {
                margin: UiRect::bottom(Val::Px(10.0)),
                ..default()
            },
            ChildOf(panel),
        ));
    }
    commands.spawn((
        Button,
        Node {
            width: Val::Px(200.0),
            height: Val::Px(50.0),
            margin: UiRect::top(Val::Px(14.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(NORMAL_BUTTON),
        MenuButtonAction::BackToMainMenu,
        ChildOf(panel),
        children![text("Back", 28.0, TEXT_COLOR)],
    ));
}

/// Navigates the menu with keyboard, gamepad or mouse.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn menu_navigation(
    input: MenuInput,
    state: Res<State<MenuState>>,
    mut selection: ResMut<MenuSelection>,
    mut buttons: Query<(Ref<Interaction>, &MenuButtonAction, &mut BackgroundColor), With<Button>>,
    mut app_exit: MessageWriter<AppExit>,
    mut sfx: MessageWriter<PlaySfx>,
    mut menu_state: ResMut<NextState<MenuState>>,
    mut game_state: ResMut<NextState<AppState>>,
) {
    let actions = input.read();
    let screen = state.get().buttons();
    let count = screen.len();
    if actions.up {
        selection.0 = (selection.0 + count - 1) % count;
    }
    if actions.down {
        selection.0 = (selection.0 + 1) % count;
    }
    selection.0 = selection.0.min(count - 1);
    let mut chosen = actions.confirm.then_some(screen[selection.0]);
    if actions.back && *state.get() != MenuState::Main {
        chosen = Some(MenuButtonAction::BackToMainMenu);
    }
    for (interaction, action, mut color) in &mut buttons {
        let index = screen.iter().position(|a| a == action);
        if interaction.is_changed() {
            match *interaction {
                Interaction::Pressed => chosen = Some(*action),
                Interaction::Hovered => selection.0 = index.unwrap_or(selection.0),
                Interaction::None => {}
            }
        }
        color.0 = if *interaction == Interaction::Pressed {
            PRESSED_BUTTON
        } else if index == Some(selection.0) {
            HOVERED_BUTTON
        } else {
            NORMAL_BUTTON
        };
    }

    let Some(action) = chosen else {
        return;
    };
    sfx.write(PlaySfx(Sfx::Click));
    match action {
        MenuButtonAction::Quit => {
            app_exit.write(AppExit::Success);
        }
        MenuButtonAction::Play => game_state.set(AppState::Game),
        MenuButtonAction::Achievements => menu_state.set(MenuState::Achievements),
        MenuButtonAction::Credits => menu_state.set(MenuState::Credits),
        MenuButtonAction::BackToMainMenu => menu_state.set(MenuState::Main),
    }
}
