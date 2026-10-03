//! Sound effects and music.

use bevy::{audio::Volume, prelude::*};

use crate::AppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sfx {
    Click,
    EnterDarkRoom,
    EnterDarkRoomAua,
    Feather,
    FireExtinguish,
    FireLightUp,
    Knife,
    Meow,
    PentagramDraw,
    PillowCut,
    PillowTear,
    RoomIsNowBright,
    Sleep,
    Step1,
    Step2,
    Step3,
    TreasureSearch,
}

impl Sfx {
    const ALL: [Self; 17] = [
        Self::Click,
        Self::EnterDarkRoom,
        Self::EnterDarkRoomAua,
        Self::Feather,
        Self::FireExtinguish,
        Self::FireLightUp,
        Self::Knife,
        Self::Meow,
        Self::PentagramDraw,
        Self::PillowCut,
        Self::PillowTear,
        Self::RoomIsNowBright,
        Self::Sleep,
        Self::Step1,
        Self::Step2,
        Self::Step3,
        Self::TreasureSearch,
    ];

    fn path(self) -> &'static str {
        match self {
            Self::Click => "audio/Click.ogg",
            Self::EnterDarkRoom => "audio/EnterDarkRoom.ogg",
            Self::EnterDarkRoomAua => "audio/EnterDarkRoomAua.ogg",
            Self::Feather => "audio/Feather.ogg",
            Self::FireExtinguish => "audio/FireExtinguish.ogg",
            Self::FireLightUp => "audio/FireLightUp.ogg",
            Self::Knife => "audio/Knife.ogg",
            Self::Meow => "audio/Meow.ogg",
            Self::PentagramDraw => "audio/PentagramDraw.ogg",
            Self::PillowCut => "audio/PillowCut.ogg",
            Self::PillowTear => "audio/PillowTear.ogg",
            Self::RoomIsNowBright => "audio/RoomIsNowBright.ogg",
            Self::Sleep => "audio/Sleep.ogg",
            Self::Step1 => "audio/Step_01.ogg",
            Self::Step2 => "audio/Step_02.ogg",
            Self::Step3 => "audio/Step_03.ogg",
            Self::TreasureSearch => "audio/TreasureSearch.ogg",
        }
    }

    fn volume(self) -> f32 {
        match self {
            Self::Step1 | Self::Step2 | Self::Step3 => 0.35,
            Self::Click => 0.6,
            _ => 1.0,
        }
    }
}

/// Request to play a sound effect.
#[derive(Message, Clone, Copy, Debug)]
pub struct PlaySfx(pub Sfx);

#[derive(Resource)]
struct SfxHandles(Vec<Handle<AudioSource>>);

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfx>()
            .add_systems(Startup, load_sfx)
            .add_systems(Update, play_sfx)
            .add_systems(
                OnEnter(AppState::Menu),
                music("audio/Harvest_WakeUpSlow.ogg", AppState::Menu),
            )
            .add_systems(
                OnEnter(AppState::Game),
                music("audio/02_Harvest_WakeUpPanic.ogg", AppState::Game),
            )
            .add_systems(
                OnEnter(AppState::Ending),
                music("audio/Harvest_WakeUpSlow.ogg", AppState::Ending),
            );
    }
}

fn load_sfx(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Keep the handles alive so the sounds don't need to be loaded on first use.
    commands.insert_resource(SfxHandles(
        Sfx::ALL
            .iter()
            .map(|sfx| asset_server.load(sfx.path()))
            .collect(),
    ));
}

fn play_sfx(
    mut commands: Commands,
    mut requests: MessageReader<PlaySfx>,
    handles: Res<SfxHandles>,
) {
    for PlaySfx(sfx) in requests.read() {
        let index = Sfx::ALL.iter().position(|s| s == sfx).unwrap();
        commands.spawn((
            AudioPlayer::new(handles.0[index].clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(sfx.volume())),
        ));
    }
}

fn music(path: &'static str, state: AppState) -> impl Fn(Commands, Res<AssetServer>) {
    move |mut commands: Commands, asset_server: Res<AssetServer>| {
        commands.spawn((
            AudioPlayer::new(asset_server.load(path)),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.5)),
            DespawnOnExit(state),
        ));
    }
}
