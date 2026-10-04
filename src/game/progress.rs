//! State of the current run ([`Progress`]) and state which persists across
//! runs and restarts of the game ([`Meta`]).

use std::collections::BTreeSet;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

const SAVE_FILE: &str = "savegame.yaml";

/// Number of [`Occupant`](super::script::Occupant)s behind the locked doors.
pub const OCCUPANT_COUNT: usize = 4;

/// Number of cats of Ulu which can be collected.
pub const CAT_COUNT: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
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
    /// Path of the item's icon in the assets.
    pub fn icon(self) -> &'static str {
        match self {
            Self::Knife => "icons/knife.png",
            Self::Candle => "icons/candle.png",
            Self::BurningCandle => "icons/burning_candle.png",
            Self::StinkyCheese => "icons/stinky_cheese.png",
            Self::MeltedCheese => "icons/melted_cheese.png",
            Self::CreasedFeather => "icons/creased_feather.png",
            Self::PerfectFeather => "icons/perfect_feather.png",
            Self::CreasedRavenFeather => "icons/creased_raven_feather.png",
            Self::PerfectRavenFeather => "icons/perfect_raven_feather.png",
            Self::Wine => "icons/wine.png",
            Self::RedHerring => "icons/red_herring.png",
        }
    }

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum Carpet {
    #[default]
    Clean,
    Bloody,
    RolledIn,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
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

    /// A hint how the achievement is obtained, without giving the solution away.
    pub fn description(self) -> &'static str {
        match self {
            Self::GodOfSleep => "Some prefer to face their destiny with their eyes closed.",
            Self::MeowthyCultist => {
                "Prepare a flawless ritual for a god who already knows your taste."
            }
            Self::WhosTheRealMaster => {
                "Earn Ulu's highest praise without any help from the magister."
            }
            Self::CatUluGuidesMe => "Prepare a ritual that is even better than flawless.",
            Self::RichHarvest => "Prepare a ritual that truly pleases Ulu.",
            Self::FoodOfUlu => "Get harvested, even though your ritual had its flaws.",
            Self::CatGrass => "Offer Ulu a ritual that is hard to digest.",
            Self::Unworthy => "Stumble around in the dark once too often.",
            Self::JustAQuietPeep => {
                "Make sure nobody complains about running in the library again."
            }
            Self::YouDontFoolMe => "See through some well-meant advice.",
            Self::WisdomOfAThousandMice => "Learn everything the magister knows about the ritual.",
            Self::PraiseCatUluForEternity => "Meet every one of the cats of Ulu.",
        }
    }

    pub fn secret(self) -> bool {
        matches!(
            self,
            Self::CatUluGuidesMe | Self::JustAQuietPeep | Self::PraiseCatUluForEternity
        )
    }
}

/// A reason why Ulu likes the ritual more or less.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RitualNote {
    /// Change of the ritual counter. Negative is better, 0 is just praise.
    pub change: i32,
    pub text: &'static str,
}

/// State of the current run. Reset whenever a new game is started.
#[derive(Resource, Debug)]
pub struct Progress {
    pub inventory: Vec<Item>,
    /// The lower, the better Ulu likes the ritual.
    pub ritual_counter: i32,
    /// Why the ritual counter changed.
    pub ritual_notes: Vec<RitualNote>,
    /// Ulu was summoned.
    pub ritual_performed: bool,
    /// How often the player tripped in the dark ritual room.
    pub trip_count: u32,

    pub knows_candle: bool,
    pub knows_magister: bool,
    /// The librarian hinted that sleeping might be a good idea.
    pub meta_knowledge: bool,
    pub tried_to_cut: bool,
    /// How often the player tried to open the locked door of each occupant.
    pub locked_door_attempts: [usize; OCCUPANT_COUNT],
    pub cheese_taken: bool,
    pub wine_received: bool,
    pub mirror_done: bool,
    pub magister_topics: BTreeSet<Topic>,

    pub librarian_met: bool,
    /// The librarian noticed the player, so he can't be surprised from behind
    /// anymore, even if the player didn't talk to him.
    pub librarian_noticed: bool,
    pub librarian_gave_candle: bool,
    pub librarian_gone: bool,
    /// The librarian is gone because the player killed him.
    pub librarian_killed: bool,
    pub magister_talked: bool,

    pub fire_lit: bool,
    pub bed_destroyed: bool,
    pub carpet: Carpet,
    pub circle: RitualCircle,
    pub circle_feather: Option<Item>,
    pub circle_candle: bool,
    pub circle_incense: Option<Item>,
    /// What the invocation circle was drawn with.
    pub circle_blood: Option<Item>,

    /// Achievements unlocked for the first time during this run.
    pub new_achievements: Vec<Achievement>,
    pub ending: Option<Ending>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            inventory: Vec::new(),
            ritual_counter: 0,
            ritual_notes: Vec::new(),
            ritual_performed: false,
            trip_count: 0,
            knows_candle: false,
            knows_magister: false,
            meta_knowledge: false,
            tried_to_cut: false,
            locked_door_attempts: [0; OCCUPANT_COUNT],
            cheese_taken: false,
            wine_received: false,
            mirror_done: false,
            magister_topics: BTreeSet::new(),
            librarian_met: false,
            librarian_noticed: false,
            librarian_gave_candle: false,
            librarian_gone: false,
            librarian_killed: false,
            magister_talked: false,
            fire_lit: true,
            bed_destroyed: false,
            carpet: Carpet::Clean,
            circle: RitualCircle::None,
            circle_feather: None,
            circle_candle: false,
            circle_incense: None,
            circle_blood: None,
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

    /// How well the ritual was prepared, from 1 to 5 stars. `None` if Ulu was
    /// never summoned. Matches the thresholds of the summoning endings.
    pub fn ritual_stars(&self) -> Option<u8> {
        let stars = match self.ritual_counter {
            ..=0 => 5,
            1 => 4,
            2 => 3,
            3 => 2,
            _ => 1,
        };
        self.ritual_performed.then_some(stars)
    }

    /// The reasons for the rating of the ritual: everything which changed the
    /// ritual counter plus praise for what was done well.
    pub fn ritual_assessment(&self) -> Vec<RitualNote> {
        let praise = |text| RitualNote { change: 0, text };
        let mut notes = Vec::new();
        if self.circle_candle {
            notes.push(praise("The candle burned brightly in the circle."));
        }
        if self.circle_feather == Some(Item::PerfectRavenFeather) {
            notes.push(praise("The raven feather was flawless."));
        }
        if self.circle_incense == Some(Item::StinkyCheese) {
            notes.push(praise("The incense was ripe and wonderfully pungent."));
        }
        if self.circle_blood == Some(Item::RedHerring) {
            notes.push(praise(
                "The pentagram was drawn in fish blood. Ulu didn't mind.",
            ));
        }
        notes.extend(self.ritual_notes.iter().copied());
        notes
    }

    /// Whether there is light in the dark ritual room: The player carries the
    /// burning candle or placed it on the invocation circle.
    pub fn has_light(&self) -> bool {
        self.has(Item::BurningCandle) || self.circle_candle
    }

    /// The player can sneak up on the librarian and stab him from behind.
    pub fn can_stab_librarian(&self) -> bool {
        !self.librarian_met && !self.librarian_noticed && self.has(Item::Knife)
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
