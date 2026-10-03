//! Runs the dialogue [`script`](super::script) and displays it.

use bevy::prelude::*;

use super::{
    Phase,
    audio::{PlaySfx, Sfx},
    hud::ScreenFx,
    input::MenuInput,
    progress::{Meta, Progress},
    script::{self, Ctx, Effect, Line, Next, Scene, Speaker},
};
use crate::AppState;

/// Characters revealed per second.
const TEXT_SPEED: f32 = 90.0;

/// Request to start a dialogue at the given node.
#[derive(Message, Clone, Copy, Debug)]
pub struct StartDialogue(pub script::Node);

#[derive(Resource, Default)]
pub struct Dialogue {
    scene: Scene,
    line: usize,
    /// The line currently on screen. Kept when entering a scene without lines.
    shown: Option<Line>,
    revealed: f32,
    /// The selected answer. Nothing is selected initially, so pressing the
    /// confirm key to skip the text never picks an answer by accident.
    selected: Option<usize>,
    /// The player tried to confirm without selecting an answer.
    needs_selection: bool,
    /// Incremented whenever the displayed choices change.
    choices_version: u32,
}

impl Dialogue {
    pub fn speaker(&self) -> Option<Speaker> {
        self.shown.as_ref().map(|line| line.speaker)
    }

    fn fully_revealed(&self) -> bool {
        self.shown
            .as_ref()
            .is_none_or(|line| self.revealed as usize >= line.text.chars().count())
    }

    fn on_last_line(&self) -> bool {
        self.line + 1 >= self.scene.lines.len()
    }

    fn choices(&self) -> &[script::Choice] {
        match &self.scene.next {
            Next::Choices(choices) if self.on_last_line() && self.fully_revealed() => choices,
            _ => &[],
        }
    }
}

#[derive(Component)]
struct DialogueBox;
#[derive(Component)]
struct SpeakerText;
#[derive(Component)]
struct BodyText;
#[derive(Component)]
struct Hint;
#[derive(Component)]
struct ChoiceList(u32);
#[derive(Component)]
struct ChoiceButton(usize);

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<StartDialogue>()
            .init_resource::<Dialogue>()
            .add_systems(OnEnter(AppState::Game), spawn_dialogue_box)
            .add_systems(
                Update,
                (
                    start_dialogue.run_if(in_state(AppState::Game)),
                    (dialogue_input, reveal_text)
                        .chain()
                        .run_if(in_state(Phase::Dialogue)),
                    (update_text, update_hint, update_choices, style_choices)
                        .chain()
                        .run_if(in_state(AppState::Game)),
                )
                    .chain(),
            )
            .add_systems(OnEnter(Phase::Dialogue), show_box(true))
            .add_systems(OnExit(Phase::Dialogue), show_box(false));
    }
}

/// Advances the dialogue. Sounds and changes to [`Meta`] are collected and
/// applied by [`Runner::finish`].
struct Runner<'a> {
    dialogue: &'a mut Dialogue,
    progress: &'a mut Progress,
    meta: &'a mut Meta,
    screen: &'a mut ScreenFx,
    phase: &'a mut NextState<Phase>,
    app_state: &'a mut NextState<AppState>,
    sfx: Vec<Sfx>,
    meta_changed: bool,
}

impl<'a> Runner<'a> {
    fn new(
        dialogue: &'a mut Dialogue,
        progress: &'a mut Progress,
        meta: &'a mut Meta,
        screen: &'a mut ScreenFx,
        phase: &'a mut NextState<Phase>,
        app_state: &'a mut NextState<AppState>,
    ) -> Self {
        Self {
            dialogue,
            progress,
            meta,
            screen,
            phase,
            app_state,
            sfx: Vec::new(),
            meta_changed: false,
        }
    }

    /// Returns the collected sounds and whether [`Meta`] was changed.
    fn finish(self) -> (Vec<Sfx>, bool) {
        (self.sfx, self.meta_changed)
    }

    /// Enters the given node, following `Goto`s of scenes without lines.
    fn enter(&mut self, mut node: script::Node) {
        loop {
            if node == script::Node::Exit {
                self.close();
                return;
            }
            let mut cx = Ctx::new(self.progress, self.meta);
            let scene = script::run(node, &mut cx);
            self.meta_changed |= cx.meta_changed;
            for effect in cx.effects {
                match effect {
                    Effect::Sfx(sfx) => self.sfx.push(sfx),
                    Effect::Fade => self.screen.pulse(),
                    Effect::Blackout(black) => self.screen.blackout = black,
                }
            }
            if scene.lines.is_empty() {
                match scene.next {
                    Next::Goto(next) => {
                        node = next;
                        continue;
                    }
                    Next::End => {
                        self.close();
                        return;
                    }
                    Next::Ending(_) | Next::Choices(_) => {}
                }
            }
            self.dialogue.line = 0;
            self.dialogue.revealed = 0.0;
            self.dialogue.selected = None;
            self.dialogue.needs_selection = false;
            self.dialogue.choices_version += 1;
            if let Some(line) = scene.lines.first() {
                self.dialogue.shown = Some(line.clone());
            } else {
                self.dialogue.revealed = f32::MAX;
            }
            self.dialogue.scene = scene;
            self.phase.set(Phase::Dialogue);
            return;
        }
    }

    /// Continues after the last line of the current scene.
    fn finish_scene(&mut self) {
        match self.dialogue.scene.next.clone() {
            Next::End => self.close(),
            Next::Goto(node) => self.enter(node),
            Next::Choices(choices) => {
                if let Some(choice) = self.dialogue.selected.and_then(|i| choices.get(i)) {
                    self.sfx.push(Sfx::Click);
                    self.enter(choice.node);
                }
            }
            Next::Ending(ending) => {
                self.progress.ending = Some(ending);
                self.app_state.set(AppState::Ending);
            }
        }
    }

    fn close(&mut self) {
        self.dialogue.shown = None;
        self.screen.blackout = false;
        self.phase.set(Phase::Exploring);
    }
}

fn apply(
    sounds: Vec<Sfx>,
    meta_changed: bool,
    meta: &mut ResMut<Meta>,
    sfx: &mut MessageWriter<PlaySfx>,
) {
    if meta_changed {
        meta.set_changed();
    }
    for sound in sounds {
        sfx.write(PlaySfx(sound));
    }
}

#[allow(clippy::too_many_arguments)]
fn start_dialogue(
    mut requests: MessageReader<StartDialogue>,
    mut dialogue: ResMut<Dialogue>,
    mut progress: ResMut<Progress>,
    mut meta: ResMut<Meta>,
    mut screen: ResMut<ScreenFx>,
    mut sfx: MessageWriter<PlaySfx>,
    mut phase: ResMut<NextState<Phase>>,
    mut app_state: ResMut<NextState<AppState>>,
) {
    // Only one dialogue can run at a time.
    let Some(StartDialogue(node)) = requests.read().last().copied() else {
        return;
    };
    let mut runner = Runner::new(
        &mut dialogue,
        &mut progress,
        meta.bypass_change_detection(),
        &mut screen,
        &mut phase,
        &mut app_state,
    );
    runner.enter(node);
    let (sounds, meta_changed) = runner.finish();
    apply(sounds, meta_changed, &mut meta, &mut sfx);
}

#[allow(clippy::too_many_arguments)]
fn dialogue_input(
    input: MenuInput,
    choice_buttons: Query<(&Interaction, &ChoiceButton), Changed<Interaction>>,
    box_interaction: Query<&Interaction, (Changed<Interaction>, With<DialogueBox>)>,
    mut dialogue: ResMut<Dialogue>,
    mut progress: ResMut<Progress>,
    mut meta: ResMut<Meta>,
    mut screen: ResMut<ScreenFx>,
    mut sfx: MessageWriter<PlaySfx>,
    mut phase: ResMut<NextState<Phase>>,
    mut app_state: ResMut<NextState<AppState>>,
) {
    let menu = input.read();
    let mut confirm = menu.confirm || box_interaction.iter().any(|i| *i == Interaction::Pressed);

    let choice_count = dialogue.choices().len();
    if choice_count > 0 {
        if menu.up {
            dialogue.selected = Some(
                dialogue
                    .selected
                    .map_or(choice_count - 1, |i| (i + choice_count - 1) % choice_count),
            );
        }
        if menu.down {
            dialogue.selected = Some(dialogue.selected.map_or(0, |i| (i + 1) % choice_count));
        }
        for (interaction, ChoiceButton(index)) in &choice_buttons {
            match interaction {
                Interaction::Hovered => dialogue.selected = Some(*index),
                Interaction::Pressed => {
                    dialogue.selected = Some(*index);
                    confirm = true;
                }
                Interaction::None => {}
            }
        }
    }

    if !confirm {
        return;
    }
    if !dialogue.fully_revealed() {
        dialogue.revealed = f32::MAX;
        return;
    }
    if !dialogue.on_last_line() {
        dialogue.line += 1;
        dialogue.revealed = 0.0;
        dialogue.shown = dialogue.scene.lines.get(dialogue.line).cloned();
        return;
    }
    if choice_count > 0 && dialogue.selected.is_none() {
        dialogue.needs_selection = true;
        return;
    }
    let mut runner = Runner::new(
        &mut dialogue,
        &mut progress,
        meta.bypass_change_detection(),
        &mut screen,
        &mut phase,
        &mut app_state,
    );
    runner.finish_scene();
    let (sounds, meta_changed) = runner.finish();
    apply(sounds, meta_changed, &mut meta, &mut sfx);
}

fn reveal_text(time: Res<Time>, mut dialogue: ResMut<Dialogue>) {
    if !dialogue.fully_revealed() {
        dialogue.revealed += time.delta_secs() * TEXT_SPEED;
    }
}

fn speaker_color(speaker: Speaker) -> Color {
    match speaker {
        Speaker::Narrator => Color::srgb(0.7, 0.7, 0.7),
        Speaker::Player => Color::srgb(0.95, 0.85, 0.55),
        Speaker::Librarian => Color::srgb(0.6, 0.8, 0.95),
        Speaker::Magister => Color::srgb(0.8, 0.65, 0.95),
        Speaker::Ulu => Color::srgb(1.0, 0.2, 0.15),
    }
}

fn spawn_dialogue_box(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::FlexEnd,
            padding: UiRect::all(Val::Px(24.0)),
            ..default()
        },
        GlobalZIndex(40),
        Visibility::Hidden,
        DespawnOnExit(AppState::Game),
        DialogueBox,
        Button,
        children![(
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(1100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Px(28.0), Val::Px(20.0)),
                row_gap: Val::Px(10.0),
                border: UiRect::all(Val::Px(3.0)),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.03, 0.06, 0.92)),
            BorderColor::all(Color::srgb(0.45, 0.12, 0.1)),
            children![
                (
                    Text::default(),
                    TextFont {
                        font: font.clone().into(),
                        font_size: 22.0.into(),
                        ..default()
                    },
                    SpeakerText,
                ),
                (
                    Text::default(),
                    TextFont {
                        font: font.clone().into(),
                        font_size: 24.0.into(),
                        ..default()
                    },
                    TextColor(Color::srgb(0.93, 0.92, 0.9)),
                    BodyText,
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    ChoiceList(0),
                ),
                (
                    Text::default(),
                    TextFont {
                        font: font.into(),
                        font_size: 16.0.into(),
                        ..default()
                    },
                    TextColor(Color::srgb(0.5, 0.5, 0.5)),
                    Node {
                        align_self: AlignSelf::FlexEnd,
                        ..default()
                    },
                    Hint,
                ),
            ],
        )],
    ));
}

fn show_box(show: bool) -> impl Fn(Single<&mut Visibility, With<DialogueBox>>) {
    move |mut visibility| {
        **visibility = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

#[allow(clippy::type_complexity)]
fn update_text(
    dialogue: Res<Dialogue>,
    speaker: Single<(&mut Text, &mut TextColor), (With<SpeakerText>, Without<BodyText>)>,
    mut body: Single<&mut Text, (With<BodyText>, Without<SpeakerText>)>,
) {
    if !dialogue.is_changed() {
        return;
    }
    let (mut speaker_text, mut speaker_color_) = speaker.into_inner();
    let Some(line) = &dialogue.shown else {
        speaker_text.0.clear();
        body.0.clear();
        return;
    };
    speaker_text.0 = line.speaker.name().to_string();
    speaker_color_.0 = speaker_color(line.speaker);
    let revealed: String = line.text.chars().take(dialogue.revealed as usize).collect();
    if body.0 != revealed {
        body.0 = revealed;
    }
}

#[allow(clippy::type_complexity)]
fn update_hint(
    dialogue: Res<Dialogue>,
    hint: Single<
        (&mut Text, &mut TextColor),
        (With<Hint>, Without<BodyText>, Without<SpeakerText>),
    >,
) {
    if !dialogue.is_changed() {
        return;
    }
    let (mut text, mut color) = hint.into_inner();
    let hint = if !dialogue.fully_revealed() {
        ""
    } else if dialogue.choices().is_empty() {
        "[Space] continue"
    } else if dialogue.selected.is_some() {
        "[Space] confirm"
    } else {
        "[Up/Down] choose an answer"
    };
    if text.0 != hint {
        text.0 = hint.to_string();
    }
    color.0 = if dialogue.needs_selection && dialogue.selected.is_none() {
        Color::srgb(1.0, 0.75, 0.4)
    } else {
        Color::srgb(0.5, 0.5, 0.5)
    };
}

fn update_choices(
    mut commands: Commands,
    dialogue: Res<Dialogue>,
    list: Single<(Entity, &mut ChoiceList)>,
    asset_server: Res<AssetServer>,
) {
    let (entity, mut list) = list.into_inner();
    // Choices appear once the text is fully revealed, so the version alone is not enough.
    let version = dialogue.choices_version * 2 + u32::from(!dialogue.choices().is_empty());
    if list.0 == version {
        return;
    }
    list.0 = version;
    commands.entity(entity).despawn_related::<Children>();
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    for (index, choice) in dialogue.choices().iter().enumerate() {
        commands.spawn((
            Button,
            Node {
                padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChoiceButton(index),
            ChildOf(entity),
            children![(
                Text::new(&choice.text),
                TextFont {
                    font: font.clone().into(),
                    font_size: 21.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.75, 0.75)),
            )],
        ));
    }
}

fn style_choices(
    dialogue: Res<Dialogue>,
    mut buttons: Query<(&ChoiceButton, &mut BackgroundColor, &Children)>,
    mut texts: Query<&mut TextColor>,
) {
    for (ChoiceButton(index), mut background, children) in &mut buttons {
        let selected = dialogue.selected == Some(*index);
        background.0 = if selected {
            Color::srgba(0.5, 0.12, 0.1, 0.6)
        } else {
            Color::NONE
        };
        for child in children {
            if let Ok(mut color) = texts.get_mut(*child) {
                color.0 = if selected {
                    Color::WHITE
                } else {
                    Color::srgb(0.7, 0.7, 0.7)
                };
            }
        }
    }
}
