//! Plays scripted scenarios for testing, e.g. to get to a specific situation,
//! take screenshots and check the result:
//!
//! ```text
//! cargo run --features dev -- --scenario scenarios/smoke.yaml
//! ```
//!
//! A scenario is a YAML list of steps, executed one after another:
//!
//! ```yaml
//! - start_game                     # leave the main menu
//! - skip_dialogues: true           # close every dialogue right away
//! - wait: 2.0                      # seconds
//! - enter_room: [library, hall]    # room, and the room the player comes from
//! - teleport: [6.0, 2.0]           # map cell of the player's feet
//! - give: BurningCandle
//! - take: BurningCandle
//! - set: { carpet: RolledIn, circle_candle: true }
//! - press: Space                   # for one frame
//! - hold: [[KeyD, KeyS], 0.4]      # keys, seconds
//! - screenshot: name               # saved in the screenshot directory
//! - expect_room: hall
//! - expect_phase: Exploring
//! - log                            # room, phase and position of the player
//! - exit
//! ```
//!
//! Screenshots are written to `target/scenario` or the directory given with
//! `--screenshots`. A failed `expect_*` step exits with an error.

use std::path::PathBuf;

use bevy::{
    input::InputSystems,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use serde::Deserialize;

use crate::{
    AppState,
    components::player::Player,
    game::{
        Phase,
        hud::ScreenFx,
        iso::{FEET_OFFSET, character_translation, world_to_cell},
        progress::{Carpet, Item, Progress, RitualCircle},
        rooms::{EnterRoom, Room},
    },
};

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum Step {
    StartGame,
    SkipDialogues(bool),
    Wait(f32),
    EnterRoom(String, String),
    Teleport(f32, f32),
    Give(Item),
    Take(Item),
    Set(ProgressChanges),
    Press(String),
    Hold(Vec<String>, f32),
    Screenshot(String),
    ExpectRoom(String),
    ExpectPhase(String),
    Log,
    Exit,
}

/// Changes to the [`Progress`] of the current game.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
struct ProgressChanges {
    carpet: Option<Carpet>,
    circle: Option<RitualCircle>,
    circle_candle: Option<bool>,
    fire_lit: Option<bool>,
    bed_destroyed: Option<bool>,
    librarian_met: Option<bool>,
    librarian_gone: Option<bool>,
    knows_candle: Option<bool>,
    ritual_counter: Option<i32>,
}

impl ProgressChanges {
    fn apply(&self, progress: &mut Progress) {
        let Self {
            carpet,
            circle,
            circle_candle,
            fire_lit,
            bed_destroyed,
            librarian_met,
            librarian_gone,
            knows_candle,
            ritual_counter,
        } = self.clone();
        if let Some(v) = carpet {
            progress.carpet = v;
        }
        if let Some(v) = circle {
            progress.circle = v;
        }
        if let Some(v) = circle_candle {
            progress.circle_candle = v;
        }
        if let Some(v) = fire_lit {
            progress.fire_lit = v;
        }
        if let Some(v) = bed_destroyed {
            progress.bed_destroyed = v;
        }
        if let Some(v) = librarian_met {
            progress.librarian_met = v;
        }
        if let Some(v) = librarian_gone {
            progress.librarian_gone = v;
        }
        if let Some(v) = knows_candle {
            progress.knows_candle = v;
        }
        if let Some(v) = ritual_counter {
            progress.ritual_counter = v;
        }
    }
}

#[derive(Resource)]
struct Scenario {
    name: String,
    steps: Vec<Step>,
    next: usize,
    wait_until: f32,
    skip_dialogues: bool,
    held: Vec<KeyCode>,
    hold_until: f32,
    screenshots: PathBuf,
}

pub struct ScenarioPlugin;

impl Plugin for ScenarioPlugin {
    fn build(&self, app: &mut App) {
        let mut args = std::env::args().skip(1);
        let mut path = None;
        let mut screenshots = PathBuf::from("target/scenario");
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scenario" => path = args.next(),
                "--screenshots" => {
                    screenshots = args.next().map(PathBuf::from).unwrap_or(screenshots)
                }
                _ => {}
            }
        }
        let Some(path) = path else {
            return;
        };
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Reading scenario {path:?} failed: {e}"));
        let steps: Vec<Step> = serde_saphyr::from_str(&content)
            .unwrap_or_else(|e| panic!("Parsing scenario {path:?} failed: {e}"));
        std::fs::create_dir_all(&screenshots).expect("creating the screenshot directory");
        info!("Playing scenario {path:?} with {} steps", steps.len());
        app.insert_resource(Scenario {
            name: path,
            steps,
            next: 0,
            // Give the game some time to load.
            wait_until: 3.0,
            skip_dialogues: false,
            held: Vec::new(),
            hold_until: 0.0,
            screenshots,
        })
        .add_systems(PreUpdate, play_scenario.after(InputSystems));
    }
}

fn key_code(name: &str) -> KeyCode {
    match name {
        "Space" => KeyCode::Space,
        "Enter" => KeyCode::Enter,
        "Escape" => KeyCode::Escape,
        "Up" | "ArrowUp" => KeyCode::ArrowUp,
        "Down" | "ArrowDown" => KeyCode::ArrowDown,
        "Left" | "ArrowLeft" => KeyCode::ArrowLeft,
        "Right" | "ArrowRight" => KeyCode::ArrowRight,
        "W" | "KeyW" => KeyCode::KeyW,
        "A" | "KeyA" => KeyCode::KeyA,
        "S" | "KeyS" => KeyCode::KeyS,
        "D" | "KeyD" => KeyCode::KeyD,
        other => panic!("Unknown key {other:?} in scenario"),
    }
}

#[allow(clippy::too_many_arguments)]
fn play_scenario(
    mut commands: Commands,
    time: Res<Time>,
    mut scenario: ResMut<Scenario>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut app_state: ResMut<NextState<AppState>>,
    phase: Option<Res<State<Phase>>>,
    mut next_phase: ResMut<NextState<Phase>>,
    mut screen: Option<ResMut<ScreenFx>>,
    mut progress: ResMut<Progress>,
    room: Res<Room>,
    mut player: Query<&mut Transform, With<Player>>,
    mut enter: MessageWriter<EnterRoom>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    // Keys pressed by the scenario are released when they're no longer held.
    for key in keys.get_pressed().copied().collect::<Vec<_>>() {
        if !scenario.held.contains(&key) || now >= scenario.hold_until {
            keys.release(key);
        }
    }
    if now >= scenario.hold_until {
        scenario.held.clear();
    }
    let phase = phase.map(|phase| *phase.get());
    if scenario.skip_dialogues && phase == Some(Phase::Dialogue) {
        next_phase.set(Phase::Exploring);
        if let Some(screen) = &mut screen {
            screen.blackout = false;
        }
    }

    while now >= scenario.wait_until {
        let Some(step) = scenario.steps.get(scenario.next).cloned() else {
            info!("Scenario {} finished", scenario.name);
            exit.write(AppExit::Success);
            scenario.wait_until = f32::MAX;
            return;
        };
        scenario.next += 1;
        info!("Scenario step {}: {step:?}", scenario.next);
        let fail = |message: String| {
            error!("Scenario step failed: {message}");
            AppExit::error()
        };
        match step {
            Step::StartGame => {
                app_state.set(AppState::Game);
                scenario.wait_until = now + 2.0;
            }
            Step::SkipDialogues(skip) => scenario.skip_dialogues = skip,
            Step::Wait(seconds) => scenario.wait_until = now + seconds,
            Step::EnterRoom(target, from) => {
                enter.write(EnterRoom {
                    room: target,
                    spawn: from,
                });
                // Wait for the transition to finish.
                scenario.wait_until = now + 2.0;
            }
            Step::Teleport(x, y) => {
                for mut transform in &mut player {
                    transform.translation = character_translation(Vec2::new(x, y));
                }
                scenario.wait_until = now + 0.3;
            }
            Step::Give(item) => progress.give(item),
            Step::Take(item) => progress.take(item),
            Step::Set(changes) => changes.apply(&mut progress),
            Step::Press(key) => {
                keys.press(key_code(&key));
                // Released in the next frame.
                scenario.wait_until = now + 0.3;
            }
            Step::Hold(names, seconds) => {
                let held: Vec<KeyCode> = names.iter().map(|name| key_code(name)).collect();
                for key in &held {
                    keys.press(*key);
                }
                scenario.held = held;
                scenario.hold_until = now + seconds;
                scenario.wait_until = now + seconds + 0.1;
            }
            Step::Screenshot(name) => {
                let path = scenario.screenshots.join(format!("{name}.png"));
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path));
                scenario.wait_until = now + 0.3;
            }
            Step::ExpectRoom(expected) if room.name != expected => {
                exit.write(fail(format!(
                    "expected room {expected:?}, but is {:?}",
                    room.name
                )));
                scenario.wait_until = f32::MAX;
                return;
            }
            Step::ExpectPhase(expected) if format!("{phase:?}") != format!("Some({expected})") => {
                exit.write(fail(format!("expected phase {expected}, but is {phase:?}")));
                scenario.wait_until = f32::MAX;
                return;
            }
            Step::ExpectRoom(_) | Step::ExpectPhase(_) => {}
            Step::Log => {
                let cell = player
                    .iter()
                    .next()
                    .map(|transform| world_to_cell(transform.translation.truncate() + FEET_OFFSET));
                info!(
                    "room {:?}, phase {phase:?}, player at cell {cell:?}",
                    room.name
                );
            }
            Step::Exit => {
                exit.write(AppExit::Success);
                scenario.wait_until = f32::MAX;
                return;
            }
        }
    }
}
