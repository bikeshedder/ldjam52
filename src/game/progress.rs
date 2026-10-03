//! State of the current run ([`Progress`]) and state which persists across
//! runs and restarts of the game ([`Meta`]).

use std::collections::BTreeSet;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

const SAVE_FILE: &str = "savegame.yaml";

/// Number of cats of Ulu which can be collected.
pub const CAT_COUNT: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Knife,
    Candle,
    BurningCandle,
    StinkyCheese,
    MeltedCheese,
    CreasedFeather,
    PerfectFeather,
    CreasedRavenFeather,
    PerfectRavenFeather,
    Wine,
    RedHerring,
}

impl Item {
    pub fn name(self) -> &'static str {
        match self {
            Self::Knife => "Knife",
            Self::Candle => "Candle",
            Self::BurningCandle => "Burning candle",
            Self::StinkyCheese => "Stinky cheese",
            Self::MeltedCheese => "Melted stinky cheese",
            Self::CreasedFeather => "Creased feather",
            Self::PerfectFeather => "Perfect feather",
            Self::CreasedRavenFeather => "Creased raven feather",
            Self::PerfectRavenFeather => "Perfect raven feather",
            Self::Wine => "Bag of wine",
            Self::RedHerring => "Red herring",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Carpet {
    #[default]
    Clean,
    Bloody,
    RolledIn,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RitualCircle {
    #[default]
    None,
    Unfinished,
    Ready,
}

/// Topics the magister can help with. Asking about all of them unlocks an
/// achievement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Topic {
    Feather,
    Candle,
    Circle,
    Incense,
    Blood,
    Knife,
}

pub const TOPIC_COUNT: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    Sleep,
    MeowthyCultist,
    RichHarvest,
    FoodOfUlu,
    CatGrass,
    Unworthy,
}

impl Ending {
    pub fn title(self) -> &'static str {
        match self {
            Self::Sleep => "Sleeping Through the Harvest",
            Self::MeowthyCultist => "The Meowthy Cultist",
            Self::RichHarvest => "A Rich Harvest",
            Self::FoodOfUlu => "Food of Ulu",
            Self::CatGrass => "Cat Grass",
            Self::Unworthy => "Unworthy",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Achievement {
    GodOfSleep,
    MeowthyCultist,
    WhosTheRealMaster,
    CatUluGuidesMe,
    RichHarvest,
    FoodOfUlu,
    CatGrass,
    Unworthy,
    JustAQuietPeep,
    YouDontFoolMe,
    WisdomOfAThousandMice,
    PraiseCatUluForEternity,
}

impl Achievement {
    pub const ALL: [Self; 12] = [
        Self::GodOfSleep,
        Self::MeowthyCultist,
        Self::WhosTheRealMaster,
        Self::CatUluGuidesMe,
        Self::RichHarvest,
        Self::FoodOfUlu,
        Self::CatGrass,
        Self::Unworthy,
        Self::JustAQuietPeep,
        Self::YouDontFoolMe,
        Self::WisdomOfAThousandMice,
        Self::PraiseCatUluForEternity,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::GodOfSleep => "God of Sleep",
            Self::MeowthyCultist => "Meowthy Cultist",
            Self::WhosTheRealMaster => "Who's the real master?",
            Self::CatUluGuidesMe => "Cat Ulu guides me",
            Self::RichHarvest => "Rich Harvest",
            Self::FoodOfUlu => "Food of Ulu",
            Self::CatGrass => "Cat grass",
            Self::Unworthy => "Unworthy",
            Self::JustAQuietPeep => "Just a quiet peep",
            Self::YouDontFoolMe => "You don't fool me!",
            Self::WisdomOfAThousandMice => "Wisdom of a thousand mice",
            Self::PraiseCatUluForEternity => "Praise Cat Ulu for eternity",
        }
    }

    pub fn secret(self) -> bool {
        matches!(
            self,
            Self::CatUluGuidesMe | Self::JustAQuietPeep | Self::PraiseCatUluForEternity
        )
    }
}

/// State of the current run. Reset whenever a new game is started.
#[derive(Resource, Debug)]
pub struct Progress {
    pub inventory: Vec<Item>,
    /// The lower, the better Ulu likes the ritual.
    pub ritual_counter: i32,
    /// How often the player tripped in the dark ritual room.
    pub trip_count: u32,

    pub knows_candle: bool,
    pub knows_magister: bool,
    /// The librarian hinted that sleeping might be a good idea.
    pub meta_knowledge: bool,
    pub tried_to_cut: bool,
    pub cheese_taken: bool,
    pub wine_received: bool,
    pub mirror_done: bool,
    pub magister_topics: BTreeSet<Topic>,

    pub librarian_met: bool,
    pub librarian_gave_candle: bool,
    pub librarian_gone: bool,
    pub magister_talked: bool,

    pub fire_lit: bool,
    pub bed_destroyed: bool,
    pub room_lit: bool,
    pub carpet: Carpet,
    pub circle: RitualCircle,
    pub circle_feather: Option<Item>,
    pub circle_candle: bool,
    pub circle_incense: Option<Item>,

    /// Achievements unlocked for the first time during this run.
    pub new_achievements: Vec<Achievement>,
    pub ending: Option<Ending>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            inventory: Vec::new(),
            ritual_counter: 0,
            trip_count: 0,
            knows_candle: false,
            knows_magister: false,
            meta_knowledge: false,
            tried_to_cut: false,
            cheese_taken: false,
            wine_received: false,
            mirror_done: false,
            magister_topics: BTreeSet::new(),
            librarian_met: false,
            librarian_gave_candle: false,
            librarian_gone: false,
            magister_talked: false,
            fire_lit: true,
            bed_destroyed: false,
            room_lit: false,
            carpet: Carpet::Clean,
            circle: RitualCircle::None,
            circle_feather: None,
            circle_candle: false,
            circle_incense: None,
            new_achievements: Vec::new(),
            ending: None,
        }
    }
}

impl Progress {
    pub fn has(&self, item: Item) -> bool {
        self.inventory.contains(&item)
    }

    pub fn has_any(&self, items: &[Item]) -> bool {
        items.iter().any(|item| self.has(*item))
    }

    pub fn give(&mut self, item: Item) {
        if !self.has(item) {
            self.inventory.push(item);
        }
    }

    pub fn take(&mut self, item: Item) {
        self.inventory.retain(|i| *i != item);
    }

    pub fn replace(&mut self, old: Item, new: Item) {
        match self.inventory.iter_mut().find(|i| **i == old) {
            Some(slot) => *slot = new,
            None => self.give(new),
        }
    }

    /// Whether the player ever had (or still has) a candle.
    pub fn got_candle(&self) -> bool {
        self.has_any(&[Item::Candle, Item::BurningCandle]) || self.circle_candle
    }
}

/// Progress which persists across runs and is saved to disk.
#[derive(Resource, Debug, Default, Serialize, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub achievements: BTreeSet<Achievement>,
    #[serde(default)]
    pub cats: BTreeSet<u8>,
    /// The player slept after the librarian suggested it. Unlocks new dialogue
    /// options in later runs.
    #[serde(default)]
    pub confirmed_meta_knowledge: bool,
}

impl Meta {
    pub fn load() -> Self {
        let Ok(content) = std::fs::read_to_string(SAVE_FILE) else {
            return Self::default();
        };
        serde_saphyr::from_str(&content).unwrap_or_else(|e| {
            warn!("Ignoring invalid save file {SAVE_FILE:?}: {e}");
            Self::default()
        })
    }

    pub fn save(&self) {
        let result = serde_saphyr::to_string(self)
            .map_err(|e| e.to_string())
            .and_then(|yaml| std::fs::write(SAVE_FILE, yaml).map_err(|e| e.to_string()));
        if let Err(e) = result {
            warn!("Could not write save file {SAVE_FILE:?}: {e}");
        }
    }
}
