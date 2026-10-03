//! The actual game: exploring the cult's home, talking to its members and
//! preparing the ritual to summon Ulu.

pub mod audio;
pub mod dialogue;
pub mod ending;
pub mod hud;
pub mod input;
pub mod iso;
pub mod progress;
pub mod rooms;
pub mod script;
pub mod world;

use bevy::prelude::*;

use crate::{AppState, components::player::Player};
use dialogue::StartDialogue;
use progress::{Meta, Progress};
use script::Node;

/// What the player is currently doing while in [`AppState::Game`].
#[derive(SubStates, Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
#[source(AppState = AppState::Game)]
pub enum Phase {
    #[default]
    Exploring,
    Dialogue,
    Paused,
    /// Moving to another room.
    Transition,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<Phase>()
            .insert_resource(Meta::load())
            .init_resource::<Progress>()
            .add_plugins((
                audio::AudioPlugin,
                dialogue::DialoguePlugin,
                ending::EndingPlugin,
                hud::HudPlugin,
                input::InputPlugin,
                rooms::RoomsPlugin,
                world::WorldPlugin,
            ))
            .add_systems(OnEnter(AppState::Game), start_intro)
            .add_systems(OnExit(Phase::Exploring), stop_player)
            .add_systems(Update, save_meta.run_if(resource_changed::<Meta>));
    }
}

fn start_intro(mut dialogue: MessageWriter<StartDialogue>) {
    dialogue.write(StartDialogue(Node::Intro));
}

/// The player stands still while talking or meditating.
fn stop_player(mut player: Query<&mut Player>) {
    for mut player in &mut player {
        player.input = default();
    }
}

fn save_meta(meta: Res<Meta>) {
    if !meta.is_added() {
        meta.save();
    }
}
