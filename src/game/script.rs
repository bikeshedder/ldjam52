//! The dialogue script of the game.
//!
//! Every [`Node`] is a piece of dialogue. Entering a node applies its effects
//! (items, flags, sounds, ...) and yields a [`Scene`]: the lines to display and
//! what happens afterwards.

use super::{
    audio::Sfx,
    progress::{
        Achievement, CAT_COUNT, Carpet, Ending, Item, Meta, Progress, RitualCircle, RitualNote,
        TOPIC_COUNT, Topic,
    },
};

/// Name of the librarian ("NPC L" in the concept).
pub const LIBRARIAN: &str = "Edam";
/// Name of the magister ("NPC M" in the concept).
pub const MAGISTER: &str = "Roquefort";

/// The servants of Ulu living behind the locked doors in the hall.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub enum Occupant {
    /// Sound asleep. He will miss the Great Harvest.
    Sleeper,
    /// Awake, meditating and chanting.
    Chanter,
    /// Awake, copying the holy book of Ulu with a late snack.
    Scribe,
    /// A locked door without a known occupant.
    Unknown,
}

impl Occupant {
    /// The occupant of the door with the given name in the room map.
    pub fn from_name(name: &str) -> Self {
        match name {
            "sleeper" => Self::Sleeper,
            "chanter" => Self::Chanter,
            "scribe" => Self::Scribe,
            _ => Self::Unknown,
        }
    }

    /// Messages shown when trying to open the door, one after another.
    fn messages(self) -> &'static [&'static str] {
        match self {
            Self::Sleeper => &[
                "The door is locked. Somebody behind it is snoring loudly.",
                "Still locked. The snoring hasn't stopped for a moment.",
                "Locked. The snoring pauses... and continues even louder.",
                "Locked. Whoever sleeps in there will sleep right through the Great Harvest.",
            ],
            Self::Chanter => &[
                "The door is locked. You can hear muffled chanting from the other side.",
                "Locked. A small sign on the door reads: \"Do not disturb - meditating\".",
                "Locked. The chanting goes on: \"Ulu... Ulu... Ulu...\"",
                "Locked. The chanting stops. You hold your breath. Then it starts again.",
            ],
            Self::Scribe => &[
                "The door is locked. Somewhere behind it, a quill is scratching over parchment.",
                "Locked. A faint smell of old cheese seeps through the keyhole. Somebody is having a late snack.",
                "Locked. You hear somebody muttering verses of the holy book of Ulu while writing.",
                "Locked. The scratching stops. \"Not now, I'm almost done with this chapter!\" a voice calls from inside.",
            ],
            Self::Unknown => &["The door is locked."],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speaker {
    Narrator,
    Player,
    Librarian,
    Magister,
    Ulu,
}

impl Speaker {
    pub fn name(self) -> &'static str {
        match self {
            Self::Narrator => "",
            Self::Player => "You",
            Self::Librarian => LIBRARIAN,
            Self::Magister => MAGISTER,
            Self::Ulu => "Ulu",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Line {
    pub speaker: Speaker,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Choice {
    pub text: String,
    pub node: Node,
}

#[derive(Clone, Debug, Default)]
pub enum Next {
    #[default]
    End,
    Goto(Node),
    Choices(Vec<Choice>),
    Ending(Ending),
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub lines: Vec<Line>,
    pub next: Next,
}

impl Scene {
    fn line(mut self, speaker: Speaker, text: impl Into<String>) -> Self {
        self.lines.push(Line {
            speaker,
            text: text.into(),
        });
        self
    }
    fn n(self, text: impl Into<String>) -> Self {
        self.line(Speaker::Narrator, text)
    }
    fn c(self, text: impl Into<String>) -> Self {
        self.line(Speaker::Player, text)
    }
    fn l(self, text: impl Into<String>) -> Self {
        self.line(Speaker::Librarian, text)
    }
    fn m(self, text: impl Into<String>) -> Self {
        self.line(Speaker::Magister, text)
    }
    fn u(self, text: impl Into<String>) -> Self {
        self.line(Speaker::Ulu, text)
    }
    fn choice(self, text: impl Into<String>, node: Node) -> Self {
        self.choice_if(true, text, node)
    }
    fn choice_if(mut self, condition: bool, text: impl Into<String>, node: Node) -> Self {
        if condition {
            let choice = Choice {
                text: text.into(),
                node,
            };
            match &mut self.next {
                Next::Choices(choices) => choices.push(choice),
                _ => self.next = Next::Choices(vec![choice]),
            }
        }
        self
    }
    fn goto(mut self, node: Node) -> Self {
        self.next = Next::Goto(node);
        self
    }
    fn ending(mut self, ending: Ending) -> Self {
        self.next = Next::Ending(ending);
        self
    }
}

/// Side effects of entering a node which are not covered by [`Progress`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Sfx(Sfx),
    /// Briefly fade the screen to black and back.
    Fade,
    /// Turn the screen black (`true`) or reveal it again (`false`).
    Blackout(bool),
    /// Leave the room through the door the player came in.
    LeaveRoom,
}

pub struct Ctx<'a> {
    pub p: &'a mut Progress,
    pub meta: &'a mut Meta,
    pub effects: Vec<Effect>,
    pub meta_changed: bool,
}

impl<'a> Ctx<'a> {
    pub fn new(p: &'a mut Progress, meta: &'a mut Meta) -> Self {
        Self {
            p,
            meta,
            effects: Vec::new(),
            meta_changed: false,
        }
    }
    fn has(&self, item: Item) -> bool {
        self.p.has(item)
    }
    fn sfx(&mut self, sfx: Sfx) {
        self.effects.push(Effect::Sfx(sfx));
    }
    fn fade(&mut self) {
        self.effects.push(Effect::Fade);
    }
    fn blackout(&mut self) {
        self.effects.push(Effect::Blackout(true));
    }
    fn achieve(&mut self, achievement: Achievement) {
        if self.meta.achievements.insert(achievement) {
            self.p.new_achievements.push(achievement);
            self.meta_changed = true;
        }
    }
    fn cat(&mut self, cat: u8) {
        if self.meta.cats.insert(cat) {
            self.meta_changed = true;
        }
        if self.meta.cats.len() >= CAT_COUNT {
            self.achieve(Achievement::PraiseCatUluForEternity);
        }
    }
    /// Changes the ritual counter and remembers why.
    fn ritual(&mut self, change: i32, text: &'static str) {
        self.p.ritual_counter += change;
        self.p.ritual_notes.push(RitualNote { change, text });
    }
    fn librarian_candle(&mut self) {
        self.p.give(Item::Candle);
        self.p.librarian_gave_candle = true;
        self.ritual(
            1,
            "The candle was a gift from the librarian, not a sign of your own devotion.",
        );
    }
    fn ask(&mut self, topic: Topic) {
        self.p.magister_topics.insert(topic);
        if self.p.magister_topics.len() >= TOPIC_COUNT {
            self.achieve(Achievement::WisdomOfAThousandMice);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub enum Node {
    /// Ends the dialogue.
    Exit,

    Intro,
    Diary,
    LockedDoor(Occupant),

    // Bed
    Bed,
    BedSleep,
    BedTear,
    BedCut,

    // Fireplace
    Fireplace,
    FireExtinguish,
    FireBask,
    FireIgnite,
    FireSoot,

    // Table with cheese
    Table,
    TableTakeCheese,

    // Chest
    Chest,
    ChestKnife,

    // Reflection in the window
    Mirror,
    Mirror1,
    Mirror2,
    Mirror3,
    MirrorVision,
    MirrorQuiet,
    MirrorKnife,

    // Librarian
    Librarian,
    LibrarianBack,
    LibA,
    LibB,
    LibC,
    LibKnownA,
    LibKnownB,
    LibKnownC,
    LibTrap,
    LibRest,
    LibNoHelp,
    LibMurder,

    // Magister
    Magister,
    MagGreet,
    MagHelp,
    MagFromLibrarianB,
    MagFromLibrarianC,
    MagIngredient,
    MagAgain,
    MagTopics,
    MagMore,
    MagBye,
    MagFeather,
    MagCandle,
    MagCandleUnlit,
    MagLightFire,
    MagCandleRefused,
    MagCircle,
    MagCircleChoices,
    MagCircleFloor,
    MagIncense,
    MagIncenseMelted,
    MagBlood,
    MagWine,
    MagGiveWine,
    MagKnife,

    // Dark ritual room
    DarkRoom,
    DarkRoomLeave,
    Trip,

    // Ritual: carpet, floor and circle
    Ritual,
    CarpetRollIn,
    FloorDraw,
    FloorCut,
    FloorCutFail,
    FloorHerring,
    PlaceFeather,
    PlaceCandle,
    PlaceUnlitCandle,
    PlaceIncense,
    Summon,
    EndMeowthy,
    EndRichHarvest,
    EndFoodOfUlu,
    EndCatGrass,
}

pub fn run(node: Node, cx: &mut Ctx) -> Scene {
    let s = Scene::default();
    match node {
        Node::Exit => s,

        Node::Intro => {
            cx.blackout();
            s.u("The Harvest is 'nigh! My sheeple, be prepared. Tonight you shall join me. Remember the ritual...")
                .u("The Great Harvest is tonight! Be prepared...")
                .n("You wake up with a start. Two glowing eyes are still burning in your mind.")
        }

        Node::LockedDoor(occupant) => {
            // A different message every time.
            let attempts = &mut cx.p.locked_door_attempts[occupant as usize];
            let messages = occupant.messages();
            let message = messages[*attempts % messages.len()];
            *attempts += 1;
            s.n(message)
        }

        Node::Diary => s
            .n("Your diary lies open on the crate next to your bed. The last entry is written in a hasty, shaky handwriting:")
            .n("\"Ulu spoke to me again. The Great Harvest is near. For the invocation I will need a burning candle, the feather of a raven and some incense. And the circle of invocation, drawn in blood.\"")
            .c("I should not forget anything. Ulu is watching."),

        // ------------------------------------------------------------------
        // Bed
        Node::Bed => {
            if cx.p.bed_destroyed {
                s.c("This is my bed. The pillow is torn and feathers are scattered everywhere. I don't really need the pillow to sleep, though.")
                    .choice("Sleep", Node::BedSleep)
                    .choice("I am not tired now", Node::Exit)
            } else {
                let knife = cx.has(Item::Knife);
                s.c("This is my bed.")
                    .choice("Sleep", Node::BedSleep)
                    .choice("Tear the pillow", Node::BedTear)
                    .choice_if(knife, "Cut the pillow", Node::BedCut)
                    .choice("I am not tired now", Node::Exit)
            }
        }
        Node::BedSleep => {
            cx.sfx(Sfx::Sleep);
            cx.blackout();
            if cx.p.meta_knowledge && !cx.meta.confirmed_meta_knowledge {
                cx.meta.confirmed_meta_knowledge = true;
                cx.meta_changed = true;
            }
            cx.achieve(Achievement::GodOfSleep);
            cx.cat(1);
            s.n("You lie down on your bed. While falling asleep you hear a voice in your head.")
                .u("I see: You don't want to be harvested. Come back next month, when you are ready, my food.")
                .ending(Ending::Sleep)
        }
        Node::BedTear => {
            cx.sfx(Sfx::PillowTear);
            cx.p.bed_destroyed = true;
            cx.p.give(Item::CreasedFeather);
            s.n("You take the pillow and tear it apart. This is more difficult than you would have thought. You need to push, pull and tear with all of your strength. Finally, the feathers are coming out. They look a little creased but you are sure they will serve the purpose.")
        }
        Node::BedCut => {
            cx.sfx(Sfx::PillowCut);
            cx.p.bed_destroyed = true;
            cx.p.give(Item::PerfectFeather);
            s.n("You cautiously cut the pillow and take out the most beautiful feather. This is it: The perfect feather. Ulu will be pleased.")
        }

        // ------------------------------------------------------------------
        // Fireplace
        Node::Fireplace => {
            if cx.p.fire_lit {
                let mut s = s;
                if cx.has(Item::StinkyCheese) {
                    cx.p.replace(Item::StinkyCheese, Item::MeltedCheese);
                    cx.ritual(1, "The incense melted into a sticky clump by the fire.");
                    s = s.n("The heat of the fire is getting to the cheese in your pocket. It melts into a soft and even smellier clump.");
                }
                let candle = cx.has(Item::Candle);
                s.c("The fireplace is warm and cozy. I'd love to take some time basking at the fireplace.")
                    .choice("Extinguish the fire", Node::FireExtinguish)
                    .choice("Bask, relax and warm up", Node::FireBask)
                    .choice_if(candle, "Ignite the candle", Node::FireIgnite)
                    .choice("That was long enough. I need to move on.", Node::Exit)
            } else {
                let feather = cx.p.has_any(&[Item::CreasedFeather, Item::PerfectFeather]);
                s.n("The fireplace is empty and cold. The dark black soot almost has a hypnotic effect on you.")
                    .choice_if(feather, "Soil the feather in the soot", Node::FireSoot)
                    .choice("I need to move on.", Node::Exit)
            }
        }
        Node::FireExtinguish => {
            cx.sfx(Sfx::FireExtinguish);
            cx.fade();
            cx.p.fire_lit = false;
            s.n("Using a nearby basket full of water you carefully extinguish the fire. Now it's much colder than before. Was it really worth it?")
        }
        Node::FireBask => {
            cx.fade();
            s.n("You take some time to sit down and bask at the fire. Your whole body feels refreshed and warm now.")
        }
        Node::FireIgnite => {
            cx.sfx(Sfx::FireLightUp);
            cx.p.replace(Item::Candle, Item::BurningCandle);
            s.n("You carefully light the candle at the fireplace. You feel warm now all over your body.")
        }
        Node::FireSoot => {
            cx.sfx(Sfx::Feather);
            if cx.has(Item::CreasedFeather) {
                cx.p.replace(Item::CreasedFeather, Item::CreasedRavenFeather);
                cx.ritual(1, "The raven feather was torn and creased.");
            } else {
                cx.p.replace(Item::PerfectFeather, Item::PerfectRavenFeather);
            }
            s.n("You drag the feather through the soot of the fireplace. It looks like a feather of a black bird, now.")
        }

        // ------------------------------------------------------------------
        // Table with cheese
        Node::Table => {
            let can_take = !cx.p.cheese_taken;
            s.n("You see food and drinks on the table. Ulu never needs to suffer hunger and is always surrounded by food. For this reason, in the faith of Ulu, there should always be food available for everyone at any time. You can see some bread, water, juice, some nuts and various kinds of cheese. There is a plate with a big chunk of cheese that exudes a strong smell which makes your mouth water.")
                .c("Hmmm, the cheese is very stinky. It will be very yummy when it's ripe enough, I can't wait to eat it.")
                .choice("I think I should wait longer for the cheese to get more ripe and more stinky. I will have a sip of water and some bread instead.", Node::Exit)
                .choice_if(can_take, "I think the cheese is old and stinky enough. I will take it and eat it later.", Node::TableTakeCheese)
        }
        Node::TableTakeCheese => {
            cx.p.cheese_taken = true;
            cx.p.give(Item::StinkyCheese);
            s.n("You wrap the stinky cheese in a napkin and put it into your pocket.")
        }

        // ------------------------------------------------------------------
        // Chest
        Node::Chest => {
            cx.sfx(Sfx::TreasureSearch);
            let s = s
                .n("You open the chest in search of anything helpful.")
                .c("Let's see if I can find anything helpful here.");
            if cx.has(Item::Knife) {
                s.choice("There is nothing of use in here.", Node::Exit)
            } else {
                s.choice("There is a sharp knife, I will take it.", Node::ChestKnife)
                    .choice("I'd rather leave the knife where it is.", Node::Exit)
            }
        }
        Node::ChestKnife => {
            cx.sfx(Sfx::Knife);
            cx.p.give(Item::Knife);
            s.n("You carefully take the knife and put it into your pocket.")
        }

        // ------------------------------------------------------------------
        // Reflection in the window
        Node::Mirror => s
            .n("You see your reflection in the window. It's the image of a young cultist mouse wearing the clothes that most servants of Ulu wear.")
            .choice("Examine yourself", Node::Mirror1)
            .choice("I don't have time for this", Node::Exit),
        Node::Mirror1 => s
            .n("You look determined, but you also look tired. Ulu has been haunting your dreams in the past weeks. Today is the Great Harvest and you will finally become one with Ulu. A feeling of joy and anticipation fills you.")
            .choice("I should leave now and get ready for the ritual. There is no time to waste.", Node::Exit)
            .choice("Examine yourself further", Node::Mirror2),
        Node::Mirror2 => s
            .n("Looking closer at yourself you see how dark your eyes are. There have been more and more nights in which you could not even get to sleep. Studying, praising, meditating, preparing day after day, you could literally feel being closer and closer to Ulu. On the other hand, being a servant of Ulu has clearly taken its toll on you. Soon you will fulfill your destiny.")
            .choice("A truly devoted servant of Ulu is not afraid of anything. What else can I find out about myself?", Node::Mirror3)
            .choice("Ulu, I hear your call. I should prepare the ritual.", Node::Exit),
        Node::Mirror3 => {
            let magister = cx.p.magister_talked;
            let vision = !magister && !cx.p.mirror_done;
            s.n("What else is there to see in your reflection?")
                .choice("There is nothing else to see. I need to move on.", Node::Exit)
                .choice_if(magister, "The magister told me I should not waste any precious time. I need to prepare the ritual.", Node::Exit)
                .choice_if(vision, "I believe I can see Ulu in my reflection.", Node::MirrorVision)
        }
        Node::MirrorVision => {
            let s = s.n("Your reflection slowly transforms into a fog in which the only thing you can see are some red glowing eyes.");
            if cx.has(Item::Knife) {
                s.u("Oh, my patient servant, I see you. The Great Harvest is soon and I see you are well prepared for your sacrifice. Are you ready to give your everything to me? Your heart, your flesh and bones, your mind and even your soul?")
                    .choice("[Remain quiet and don't say anything. You are not worthy to speak to a god.]", Node::MirrorQuiet)
                    .choice("[You take out your knife] I am willing to sacrifice everything to you.", Node::MirrorKnife)
            } else {
                cx.p.mirror_done = true;
                cx.p.give(Item::RedHerring);
                cx.cat(8);
                s.u("Oh, my ever devoted servant, I can see you. Don't waste any more of your time, the Great Harvest is soon. Can't you see this is leading to nowhere? Or maybe it isn't? Who am I to judge?")
                    .n("The fog turns fully red and slowly transforms into the shape of a fish. It seems as if the fish sprang to life as it moves and swims in the reflection, then it changes its direction and directly swims towards you. With a loud splash it lands on the floor just in front of you. You don't know what just happened but you pick up the red herring. A sign of Ulu?")
            }
        }
        Node::MirrorQuiet => {
            cx.p.mirror_done = true;
            cx.p.give(Item::RedHerring);
            cx.cat(8);
            s.u("Yees. I can feel it. You will be ready. Soon, we will become one. Be careful not to be distracted on your holy mission. Let me hand you a little reminder...")
                .n("The fog turns fully red and slowly transforms into the shape of a fish. It seems as if the fish sprang to life as it moves and swims in the reflection, then it changes its direction and directly swims towards you. With a loud splash it lands on the floor just in front of you. You don't know what just happened but you pick up the red herring. This is a clear sign of Ulu, but you are not sure how to use it.")
        }
        Node::MirrorKnife => {
            cx.p.mirror_done = true;
            cx.p.give(Item::RedHerring);
            cx.cat(8);
            s.u("It's too early now, my food. I see you are fully devoted to me and ready to defend me against any spark of disbelief. If your faith is strong and pure you will find me without any other mouse.")
                .n("You see the fog transforming into a red herring that swims out of the reflection right into your hand. Something about this whole scene just felt so familiar. You will make sure to choose your actions wisely. Soon you will be harvested by the great Ulu.")
        }

        // ------------------------------------------------------------------
        // Librarian
        Node::Librarian => librarian(cx, s),
        Node::LibrarianBack => {
            if !cx.p.librarian_met && cx.has(Item::Knife) {
                s.n("He didn't notice you, yet.")
                    .choice("[Use the knife]", Node::LibMurder)
                    .choice("Leave", Node::Exit)
            } else if cx.p.librarian_met {
                s.l("Hey, kiddo. Is this you standing in my back? What are you up to? Why don't you come over, so I can see you?")
                    .n("He's old and grumpy and constantly annoys you by complaining about running, but you know he's a good guy.")
                    .c(format!("You are right, {LIBRARIAN}, I will come around. I will try to walk slowly, please don't shout at me again!"))
            } else {
                librarian(cx, s)
            }
        }
        Node::LibA => {
            cx.p.librarian_met = true;
            cx.librarian_candle();
            s.n("The librarian smiles.")
                .l("The Great Harvest! You are right, it's tonight. Do you already have everything you need for the invocation? Do you already have a candle? If not, take this one. Good luck and may Ulu be with you.")
        }
        Node::LibB => {
            cx.p.librarian_met = true;
            cx.p.knows_magister = true;
            cx.librarian_candle();
            s.l("Guardian of wisdom? Hahaha, you are funny, kiddo.")
                .c("No, I am serious. The Great Harvest is tonight. Is there any way you can help me?")
                .l("Help you? Hahaha. You received the call of Ulu and don't know what to do?")
                .c("Yes, I am sorry. I must have eaten too much cheese yesterday.")
                .l(format!("Hmm... Talk to {MAGISTER}, he can explain it much better than me. You'll definitely need a candle. Here you go. Good luck and may Ulu be with you."))
        }
        Node::LibC => {
            cx.p.librarian_met = true;
            cx.p.meta_knowledge = true;
            s.l(format!("Did you eat too much cheese, kiddo? I'm {LIBRARIAN}, the librarian. I take care of the books here. Go and get some sleep. I am sure tomorrow you will remember everything."))
                .c("You think I should simply go to bed? What about the Great Harvest?")
                .l("I think you should rather get some sleep. (He blinks.)")
                .c("I understand. Thanks for your time. May Ulu be with you.")
        }
        Node::LibKnownA => {
            cx.p.librarian_met = true;
            cx.librarian_candle();
            s.l("This is no excuse for running around in my library! You know the rules. No running.")
                .c("Yes. No running. Will you give me a candle now?")
                .l("I should rather cut off your ears for your crudeness.")
                .c("(gasps) ...")
                .l("Though... I am in a good mood today. Here you go, take a candle. Good luck and may Ulu be with you.")
        }
        Node::LibKnownB => {
            cx.p.librarian_met = true;
            cx.librarian_candle();
            s.n("The librarian smiles.")
                .l("The Great Harvest! You are right, it's tonight. Do you already have everything you need for the invocation? Do you already have a candle? If not, take this one. Good luck and may Ulu be with you.")
        }
        Node::LibKnownC => {
            cx.p.librarian_met = true;
            cx.librarian_candle();
            s.l("Guardian of wisdom? Hahaha, you are funny, kiddo.")
                .c(format!("No, I am serious. The Great Harvest is tonight. I talked to {MAGISTER}. He explained to me exactly what I need for the ritual. He also told me you have some candles. Can I get one, please?"))
                .l(format!("{MAGISTER}, hm? He's a good friend of mine, you know. He has been a magister of Ulu for more than half his life now."))
                .c("Half his life? Wow. That must be a long time.")
                .l("Let me think. I guess it is around one year already. Time passes so quickly.")
                .c("...")
                .l("Ah, yes! The candle. Here you go. Good luck and may Ulu be with you.")
        }
        Node::LibTrap => {
            cx.librarian_candle();
            cx.achieve(Achievement::YouDontFoolMe);
            s.l("A trap? I don't know what you are talking about.")
                .c("You knew I would miss the harvest if I slept again.")
                .l("Yes, but how do you know this? (He blinks.)")
                .c("I am confused.")
                .l("Don't you worry. Here you go. Take the candle. Think about what I said. Good luck and may Ulu be with you.")
        }
        Node::LibRest => {
            cx.p.give(Item::Candle);
            cx.p.librarian_gave_candle = true;
            cx.p.librarian_gone = true;
            cx.ritual(
                1,
                "The candle came from the librarian's pouch, not from your own devotion.",
            );
            s.l("You think I should rest. Why?")
                .c("Isn't it exhausting to guard the library all night? You also need to take time to rest.")
                .l("Hm. I think you are right, a small nap can't hurt.")
                .n("The librarian goes over to a table, puts a small pouch that was hanging from his belt on it and leaves the room. As curiosity overcomes you, you open the pouch, find a candle inside and take it. This will definitely be handy later.")
        }
        Node::LibNoHelp => s.l(format!("Sorry, but I cannot help you further right now. Ask {MAGISTER} if you need any help. He can explain everything much better: the Great Harvest, what you need to do and so on.")),
        Node::LibMurder => {
            cx.sfx(Sfx::Knife);
            cx.p.give(Item::Candle);
            cx.p.librarian_gone = true;
            cx.p.librarian_killed = true;
            cx.ritual(-1, "The librarian's silence pleased Ulu.");
            cx.achieve(Achievement::JustAQuietPeep);
            cx.cat(7);
            s.n("The librarian makes a quiet peep and sinks silently to the floor. You take the bloody knife out of his dead body. In his pockets you find a candle. You are sure that you will need it.")
                .n("Did you really have to kill this mouse? You are still not sure why you just killed him, but you make sure to discreetly dispose of his body.")
                .u("Well done, my food. This old rat would never have sacrificed himself for the Great Harvest. Soon you will fulfill your destiny.")
        }

        // ------------------------------------------------------------------
        // Magister
        Node::Magister => {
            if cx.p.magister_talked {
                s.n("The magister is still reading the holy book of Ulu.")
                    .choice(format!("Magister {MAGISTER}? Excuse me for disturbing you. Can I ask again for your help?"), Node::MagAgain)
                    .choice("I should rather not disturb the magister.", Node::Exit)
            } else {
                s.n("You see a tall mouse, wearing the noble clothes of a scholar, focused on reading a big folio. You can't see the cover but you assume that it is the holy book of Ulu.")
                    .n(format!("This must be magister {MAGISTER}. He is the head of the cult and will hopefully be able to spare some of his precious time to help you prepare the ritual. Should you ask him for help or try to remember the ritual and prepare everything yourself?"))
                    .choice(format!("Magister {MAGISTER}? Excuse me for disturbing you. Can I ask some questions?"), Node::MagGreet)
                    .choice("I should rather not disturb the magister.", Node::Exit)
            }
        }
        Node::MagGreet => {
            cx.p.magister_talked = true;
            let met = cx.p.librarian_met;
            let knows = cx.p.knows_magister;
            s.n("The magister turns around and a big smile spreads across his face as he recognises you.")
                .m("Hello, dear child. How can I help you?")
                .choice_if(!met, "I don't know. I just woke up and I don't really know what to do. I think I heard the call of Ulu. The Great Harvest is tonight, right?", Node::MagHelp)
                .choice_if(met && knows, format!("I just talked to {LIBRARIAN}. He says you can explain everything to me."), Node::MagFromLibrarianB)
                .choice_if(met && !knows, format!("I just talked to {LIBRARIAN} but he could not help me. Can you help me to understand what I need to do now?"), Node::MagFromLibrarianC)
                .choice("I require your help to gather a specific ingredient that I need for the Great Harvest.", Node::MagIngredient)
                .choice("Sorry for disturbing you. May the Great Ulu be with you.", Node::MagBye)
        }
        Node::MagHelp => {
            cx.p.knows_candle = true;
            s.m("You received the Call of Ulu? My child, this is a great blessing! Food of Ulu, I am so proud of you. Soon you will become one with the Great Ulu. You will be part of his flesh and bones. There is no greater honor for a servant of Ulu!")
                .c("It said the Great Harvest is tonight.")
                .m("Yes, indeed. The monthly Great Harvest is tonight, so you should make haste.")
                .c("What do I need to do?")
                .m("Listen closely: You will need three carefully chosen items for the invocation of Ulu: a burning candle, the feather of a raven and some incense.")
                .c("Where do I find those items?")
                .m(format!("Ulu will guide you. Be curious, you should find everything nearby. If you need any help feel free to reach out to me at any time. I am sure {LIBRARIAN} has the candle you will need. He is the librarian here, as you already know. (He blinks.)"))
                .c("What else do I need?")
                .m("You will also need to find a proper place for the ritual, so you will need a circle of invocation.")
                .c("Is there anything else I need to know?")
                .m("Yes, you should choose your actions wisely. There is no time to waste. The Great Harvest is tonight and we all don't want you to miss it.")
                .c("Thanks a lot. May the Great Ulu be with you.")
                .goto(Node::MagBye)
        }
        Node::MagFromLibrarianB => s
            .m(format!("Good old {LIBRARIAN}. He was already the librarian of our small community when I was a little baby mouse, you know. And when I was a little kid he was always complaining about me running around. He even does so today, when he only sees my back and doesn't recognize me. He's very kind though and he can even help you achieve your goal. So he sent you to me as you received your call of Ulu?"))
            .c("Yes. In my dream somebody talked to me before I woke up.")
            .goto(Node::MagHelp),
        Node::MagFromLibrarianC => s
            .m(format!("So, you talked to {LIBRARIAN}. He could not help you? Yes, he's not a mouse of many words. Believe me, my child, beneath his hard crust there is a very soft core. His heart is pure, by Ulu, I know it. I assume you came to me as you received the Call of Ulu?"))
            .c("Yes. In my dream somebody talked to me before I woke up.")
            .goto(Node::MagHelp),
        Node::MagIngredient => s
            .m("You know what you need to do?")
            .choice("No, not really. Somebody spoke to me in my dreams.", Node::MagHelp)
            .choice("Yes, I want to summon Ulu for the Great Harvest.", Node::MagTopics),
        Node::MagAgain => s
            .m("Aaah. Food of Ulu. How can I help you?")
            .choice("I require your help to gather a specific ingredient that I need for the Great Harvest.", Node::MagTopics)
            .choice("Sorry for disturbing you. May the Great Ulu be with you.", Node::MagBye),
        Node::MagTopics => topics(s.m("What exactly are you struggling with?")),
        Node::MagMore => topics(s.m("I am glad that I could help you. Is there any other part of the ritual you need support with?")),
        Node::MagBye => s.m("Make haste, child. The Great Harvest starts soon. May Ulu be with you, food of Ulu."),
        Node::MagFeather => {
            cx.ask(Topic::Feather);
            s.m("A feather of a raven, yes. The holy book of Ulu tells us that we need one, as Ulu can catch and eat almost all kinds of birds but the preferred ones are ravens. Their feathers are black just like the darkness that surrounds Ulu and the dark character that Ulu has.")
                .c("I have never seen any raven here, how should I get one of their feathers?")
                .m("To be honest, we are not sure if it really has to be the feather of a raven or if it's sufficient to simply have a black feather. The ancient versions of the holy book of Ulu were written at a time when the only known birds were ravens, as they have always been a huge threat for our kind. Servants of Ulu would collect the feathers of their dead bodies after they were defeated by Ulu and would wear them as a sign of faith and strength. In our ancient language, \"black feather\" and \"raven feather\" are just the same.")
                .c("I understand, I will need a black feather. This should be manageable. Thanks a lot for your help, magister.")
                .goto(Node::MagMore)
        }
        Node::MagCandle => {
            cx.ask(Topic::Candle);
            cx.p.knows_candle = true;
            let s = s.m(format!("A candle to cast light on the darkness that surrounds Ulu. Didn't you already talk to {LIBRARIAN}? You can find him in the library. Make haste and don't waste any precious time."));
            if cx.has(Item::BurningCandle) || cx.p.circle_candle {
                s.choice("I already have a burning candle. Thanks for your help, magister.", Node::MagMore)
            } else if cx.has(Item::Candle) {
                s.choice("I already have a candle, but I think something is missing.", Node::MagCandleUnlit)
            } else if cx.p.librarian_met {
                s.choice(format!("I already talked to {LIBRARIAN}, but he could not help me."), Node::MagCandleRefused)
            } else {
                s.choice(format!("I understand, I will ask {LIBRARIAN} for a candle. Thanks for your help, magister."), Node::MagMore)
            }
        }
        Node::MagCandleUnlit => {
            let lit = cx.p.fire_lit;
            s.m("Did you already ignite the candle? Ulu loves darkness, but our minor kind needs some light to see.")
                .choice_if(!lit, "I don't know how to ignite the candle. The candle holders are too high for me and the fireplace is empty.", Node::MagLightFire)
                .choice_if(lit, format!("I need a burning candle, I understand. Thanks for your help, magister {MAGISTER}."), Node::MagMore)
        }
        Node::MagLightFire => {
            cx.sfx(Sfx::FireLightUp);
            cx.fade();
            cx.p.fire_lit = true;
            s.m("We have no fire burning? Now I understand why I started to feel cold. Give me just a moment, I will start a fire.")
                .n("The magister leaves for a moment. When he comes back, his paws are covered in soot.")
                .c("Thanks for your help, magister.")
                .goto(Node::MagMore)
        }
        Node::MagCandleRefused => s
            .m(format!("{LIBRARIAN} didn't want to help you? He has a hard crust, but he is soft inside. If you ask kindly, I am sure he will give you the candle you need."))
            .choice(format!("I understand, I will ask {LIBRARIAN} for a candle. Thanks for your help, magister."), Node::MagMore),
        Node::MagCircle => {
            cx.ask(Topic::Circle);
            circle_choices(s.m("You know the circle of invocation, right? It's a pentagram: a circle including a star with five corners. You will need to find some empty floor and draw the circle on it using blood."))
        }
        Node::MagCircleChoices => circle_choices(s),
        Node::MagCircleFloor => s
            .m("Follow the call of Ulu, my child. I am sure you will find a proper place if you look around. You should look for a dark and quiet place.")
            .goto(Node::MagCircleChoices),
        Node::MagIncense => {
            cx.ask(Topic::Incense);
            let s = s.m("Ulu sees you, Ulu hears you and Ulu smells you, and I am sure you can feel the presence of Ulu, my child. I can always feel the presence of Ulu in my heart. The most important thing about incense is an intense odor. Usually, we use scented candles or incense sticks. If you can't find those, maybe you will find something else to replace them.");
            if cx.has(Item::MeltedCheese) {
                s.choice("I think I have something, but I am not sure.", Node::MagIncenseMelted)
            } else if cx.has(Item::StinkyCheese) || cx.p.circle_incense.is_some() {
                s.choice("I think I already have what I need. Thanks for your help.", Node::MagMore)
            } else {
                s.choice("I will look around and hope I will find something. Thanks for your help.", Node::MagMore)
            }
        }
        Node::MagIncenseMelted => s
            .m("Just try it, food of Ulu. I am sure it will work.")
            .goto(Node::MagMore),
        Node::MagBlood => {
            cx.ask(Topic::Blood);
            let tried = cx.p.tried_to_cut;
            s.m("Food of Ulu, don't you want to share your blood with the Great Ulu?")
                .choice("I am not sure, what do you think I should do?", Node::MagWine)
                .choice("I want to be harvested, but I can't cut myself. Do you have any ideas?", Node::MagWine)
                .choice_if(tried, "I assure you, I really tried, but I cannot do it.", Node::MagWine)
                .choice("I will try. Thanks for your help.", Node::MagMore)
        }
        Node::MagWine => {
            let received = cx.p.wine_received;
            s.m("It's not a shame if you can't cut yourself. We can use a simple replacement for your blood. The book of Ulu tells us that we need the essence of life to summon Ulu. Apart from us mice, there are other species living in this world. It does not even have to be an animal. If you smash some grapes they also bleed their juice. Why don't you just use some wine instead of your blood?")
                .choice_if(!received, "I don't have any wine. Where can I get some?", Node::MagGiveWine)
                .choice_if(received, "I already have some wine. I will make sure to use it. Thanks.", Node::MagMore)
        }
        Node::MagGiveWine => {
            cx.p.wine_received = true;
            cx.p.give(Item::Wine);
            s.m("Here you go, I still have a bag of cheap wine. You will need a knife to cut it open and then you are ready to draw the circle of invocation.")
                .c("Thanks for your help, magister. I will make sure to use the wine.")
                .goto(Node::MagMore)
        }
        Node::MagKnife => {
            cx.ask(Topic::Knife);
            s.m("A knife... Let me think about it... There must be a knife nearby, but I don't remember where it is. It's probably my age, you know. Why don't you just look around a bit, my child? Be careful not to hurt anyone once you have it.")
                .c("I will be careful. Thanks for your support, magister.")
                .goto(Node::MagMore)
        }

        // ------------------------------------------------------------------
        // Dark ritual room
        Node::DarkRoom => {
            cx.sfx(Sfx::EnterDarkRoom);
            s.c("It's very dark here. I should be careful.")
                .choice("I am not afraid of the dark. Ulu will guide me.", Node::Exit)
                .choice("I'd better leave this room.", Node::DarkRoomLeave)
        }
        Node::DarkRoomLeave => {
            cx.effects.push(Effect::LeaveRoom);
            s.n("You turn around and feel your way back to the door.")
        }
        Node::Trip => {
            cx.sfx(Sfx::EnterDarkRoomAua);
            cx.p.trip_count += 1;
            if cx.p.carpet == Carpet::Clean {
                cx.p.carpet = Carpet::Bloody;
            }
            if cx.p.trip_count >= 10 {
                cx.blackout();
                cx.achieve(Achievement::Unworthy);
                cx.cat(6);
                s.c("Ooouch! I can't get up anymore.")
                    .n("Your consciousness slowly fades away and you fall into a deep sleep.")
                    .u("Unworthy fool! How shall I harvest somebody who's too stupid to walk?")
                    .ending(Ending::Unworthy)
            } else {
                s.c("Ooouch! That hurt. I must have tripped over the carpet. Now my knee is bleeding a little. I hope it will not leave a stain.")
            }
        }

        // ------------------------------------------------------------------
        // Ritual: carpet, floor and circle
        Node::Ritual => ritual(cx, s),
        Node::CarpetRollIn => {
            cx.p.carpet = Carpet::RolledIn;
            s.n("You roll in the carpet and push it into a corner.")
        }
        Node::FloorDraw => {
            if cx.has(Item::Wine) {
                cx.sfx(Sfx::PentagramDraw);
                cx.p.take(Item::Wine);
                cx.p.circle = RitualCircle::Unfinished;
                cx.p.circle_blood = Some(Item::Wine);
                cx.ritual(
                    1,
                    "The pentagram was drawn in cheap wine instead of your own blood.",
                );
                s.n("You take out the bag of wine the magister gave to you and cut it with the knife. Then you cautiously spill the wine on the clean floor, drawing a red circle and a star with five corners. This is a decent pentagram. Ulu will be pleased with you.")
            } else if cx.has(Item::RedHerring) {
                s.c("Ok, now is the time to draw the pentagram.")
                    .choice("Cut yourself", Node::FloorCut)
                    .choice("Use the red herring", Node::FloorHerring)
            } else {
                s.goto(Node::FloorCut)
            }
        }
        Node::FloorCut => s
            .c("Ok. Now is the time. I will need to cut my artery.")
            .c("I will not have much time to live after that and I will need to draw the full circle and place all the items on it to summon Ulu.")
            .choice("No, I will not cut myself, I just cannot do it. Maybe the magister can help me.", Node::Exit)
            .choice("DO IT!", Node::FloorCutFail),
        Node::FloorCutFail => {
            cx.p.tried_to_cut = true;
            s.n("You take the knife and hold it above your arm. Your whole body is trembling and you start to sweat. You want to be harvested today and become one with the great Ulu, but you just cannot cut yourself. There has to be another way.")
        }
        Node::FloorHerring => {
            cx.sfx(Sfx::PentagramDraw);
            cx.p.take(Item::RedHerring);
            cx.p.circle = RitualCircle::Unfinished;
            cx.p.circle_blood = Some(Item::RedHerring);
            s.n("You cautiously cut the red herring and are quite surprised to see red blood coming out of it. You use it to paint the pentagram on the floor. Let's hope Ulu will be satisfied even if you didn't use your own blood.")
        }
        Node::PlaceFeather => {
            cx.sfx(Sfx::Feather);
            if cx.has(Item::PerfectRavenFeather) {
                cx.p.take(Item::PerfectRavenFeather);
                cx.p.circle_feather = Some(Item::PerfectRavenFeather);
                check_circle(cx);
                s.n("You place the perfect feather of a raven on the circle.")
                    .c("It's actually not coming from a raven, but it will serve its purpose.")
            } else {
                cx.p.take(Item::CreasedRavenFeather);
                cx.p.circle_feather = Some(Item::CreasedRavenFeather);
                check_circle(cx);
                s.n("You place the creased feather of a raven on the circle.")
                    .c("Maybe I should not have torn the pillow with my bare paws. I hope it works.")
            }
        }
        Node::PlaceCandle => {
            cx.p.take(Item::BurningCandle);
            cx.p.circle_candle = true;
            check_circle(cx);
            s.n("You place the burning candle on the circle. It glows brightly and warm.")
                .c("Ulu will be pleased.")
        }
        Node::PlaceUnlitCandle => s.c("I can't place the candle yet, I should ignite it first."),
        Node::PlaceIncense => {
            if cx.has(Item::StinkyCheese) {
                cx.p.take(Item::StinkyCheese);
                cx.p.circle_incense = Some(Item::StinkyCheese);
                check_circle(cx);
                s.n("You place the chunk of stinky cheese on the circle. You'd still love to eat this delicious piece of cheese.")
                    .c("Maybe Ulu won't mind if I take a bite during the Great Harvest.")
            } else {
                cx.p.take(Item::MeltedCheese);
                cx.p.circle_incense = Some(Item::MeltedCheese);
                check_circle(cx);
                s.n("You fiddle the clump of melted stinky cheese out of your pocket and place it on the circle as well as you can.")
                    .c("Maybe I should not have gone so close to the fire with the cheese in my pocket.")
            }
        }
        Node::Summon => {
            cx.sfx(Sfx::Meow);
            cx.p.ritual_performed = true;
            let counter = cx.p.ritual_counter;
            let rich_harvest = cx.meta.cats.contains(&3);
            let next = if counter < 1 && rich_harvest {
                Node::EndMeowthy
            } else if counter < 2 && !rich_harvest {
                Node::EndRichHarvest
            } else if counter < 4 {
                Node::EndFoodOfUlu
            } else {
                Node::EndCatGrass
            };
            s.n("As you speak the words of invocation the floor starts to tremble and all the summoning ingredients are pulled into the circle, which has become a sphere to another world. Your heart is pounding quicker and quicker as you realize that the moment of the Great Harvest - of your harvest - has finally come. You hear the loud meow of a giant cat. This is it. Cat Ulu is coming...")
                .goto(next)
        }
        Node::EndMeowthy => {
            cx.blackout();
            cx.achieve(Achievement::MeowthyCultist);
            if !cx.p.magister_talked {
                cx.achieve(Achievement::WhosTheRealMaster);
            }
            if cx.p.ritual_counter < 0 {
                cx.achieve(Achievement::CatUluGuidesMe);
            }
            cx.cat(2);
            s.n("A giant paw is dashing out of the glowing invocation circle. It quietly lands to your left and is followed by another paw that gently lands to your right. The only thing you feel is pure joy and love. An even bigger head of a dark cat emerges from the circle and lies down in front of you, its eyes just slightly above you. For some moments the cat simply breathes, you can feel its breath tickling your nostrils. You can feel and hear Cat Ulu purring.")
                .n("It feels like an eternity of pleasure until you hear the familiar voice of Cat Ulu in your head. The voice you have become used to listening to.")
                .u("Look at this rare and precious being. I, the great Cat Ulu, acknowledge your presence and praise you. You have shown the truest and purest devotion. You let no one guide you but your infinite faith. I will feast upon your devotion for the harvests to come. Feel free to become one with me whenever you wish, my meowthy mouse cultist.")
                .n("You hear a trembling and loud meow and Cat Ulu opens its mouth, showing its beautiful white and sharp teeth. In the distance you can see a dark tunnel, but beyond it you can also see light. Infinite love and pleasure await you and you proudly move forward. Forward to fulfill your final and truest destiny.")
                .ending(Ending::MeowthyCultist)
        }
        Node::EndRichHarvest => {
            cx.blackout();
            cx.achieve(Achievement::RichHarvest);
            cx.cat(3);
            s.n("A giant paw is dashing out of the glowing invocation circle and pushes you down onto the floor. Then another paw comes out and lands on the space next to you. You don't feel any pain, there is no fear, no gravity, just excitement, joy and the pleasure of fulfillment. An even bigger head of a dark cat emerges from the circle, moves above you and looks straight at you while the paw releases its pressure. You look into glowing cat eyes. Cat Ulu truly sees you, your soul and your whole essence. You hear a voice in your head.")
                .u("My pure and devoted servant. I knew I could count on you. We will now become one and you will experience the power of a god. I am very pleased with you, you are the only mouse I will feast on tonight.")
                .n("You are filled with pure joy and love. This is the moment you have been working for your whole life, your whole destiny was to become one with Ulu. This is now your end, but it's not the final end. Ulu will help you explore other worlds and possibilities.")
                .ending(Ending::RichHarvest)
        }
        Node::EndFoodOfUlu => {
            cx.blackout();
            cx.achieve(Achievement::FoodOfUlu);
            cx.cat(4);
            s.n("A giant paw is dashing out of the glowing invocation circle and grabs you. While being carried into the circle you hear a voice in your head.")
                .u("My minor servant, the time of the Great Harvest is now. You are my food and will now become one with me. I will feast upon you and many others tonight.")
                .n("You are close to exploding with joy and excitement. This is the moment you have been working towards for your whole life. Now your end has come and you slowly fade away into a never-ending sleep.")
                .ending(Ending::FoodOfUlu)
        }
        Node::EndCatGrass => {
            cx.blackout();
            cx.achieve(Achievement::CatGrass);
            cx.cat(5);
            s.n("A giant paw is dashing out of the glowing invocation circle and grabs you. You are filled with excitement and joy and happily glide down a slippery tunnel, landing in a warm and cozy dark pool. It's only a short moment until you are suddenly pulled out again and find yourself back in the ritual chamber. Before you can realize what just happened, you hear a voice in your head.")
                .u("Meow, do you want to poison me? Your clumsiness is just too big and your faith is smaller than a grain of dust! I will come back next month for the next harvest. Maybe you will be ripe enough then to become my food.")
                .ending(Ending::CatGrass)
        }
    }
}

fn librarian(cx: &mut Ctx, s: Scene) -> Scene {
    let p = &cx.p;
    if !p.librarian_met {
        let s = s.l("Hey! Running is forbidden in the library!");
        if p.knows_candle {
            s.choice("I know, but it's urgent. I need candles for the Great Harvest.", Node::LibKnownA)
                .choice(format!("Oh, sorry. I am in a hurry as the Great Harvest is today. I heard you have some candles, {LIBRARIAN}. Can I borrow one? Please?"), Node::LibKnownB)
                .choice(format!("Please excuse my inappropriate behaviour, {LIBRARIAN}, guardian of great wisdom and power. I have come to thee to request a candle to participate in the Great Harvest."), Node::LibKnownC)
                .choice("Sorry!", Node::Exit)
        } else {
            s.choice("Oh, sorry. I am in a hurry as the Great Harvest is today. Can you help me with this?", Node::LibA)
                .choice("Excuse me, guardian of wisdom. The Great Harvest is tonight and I have received my call.", Node::LibB)
                .choice("Who are you?", Node::LibC)
                .choice("Sorry!", Node::Exit)
        }
    } else if p.got_candle() {
        s.l("Hey! Running is forbidden in the library!")
            .l(format!("Oh, it's you, kiddo. Sorry, but I cannot help you further. Ask {MAGISTER} if you need any help."))
            .choice("OK!", Node::Exit)
    } else if p.knows_candle {
        s.l("Hey! Running is forbidden in the library! Oh, it's you, kiddo.")
            .choice("I need candles for the Great Harvest.", Node::LibKnownA)
            .choice(format!("I am in a hurry as the Great Harvest is today. I heard you have some candles, {LIBRARIAN}. Can I borrow one? Please?"), Node::LibKnownB)
            .choice(format!("Please excuse my inappropriate behaviour, {LIBRARIAN}, guardian of great wisdom and power. I have come to thee to request a candle to participate in the Great Harvest."), Node::LibKnownC)
            .choice("Never mind!", Node::Exit)
    } else if cx.meta.confirmed_meta_knowledge && p.meta_knowledge {
        s.l("Hey! Running is forbidden in the library! Oh, it's you, kiddo. Did you have a good sleep? (He blinks.)")
            .choice("That was a trap! You lied to me!", Node::LibTrap)
            .choice("Yes, thanks a lot. I think you should also take some rest.", Node::LibRest)
            .choice("Yes, but I still don't know what to do.", Node::LibNoHelp)
            .choice("Yes!", Node::Exit)
    } else {
        s.l("Hey! Running is forbidden in the library! Oh, it's you, kiddo.")
            .goto(Node::LibNoHelp)
    }
}

fn topics(s: Scene) -> Scene {
    s.choice("Where can I find a feather of a raven?", Node::MagFeather)
        .choice("Where can I find a candle?", Node::MagCandle)
        .choice("How can I make an invocation circle to summon Ulu?", Node::MagCircle)
        .choice("Where can I find some incense?", Node::MagIncense)
        .choice("How do I get some blood?", Node::MagBlood)
        .choice("Where can I find a knife?", Node::MagKnife)
        .choice(format!("This is all I needed to know. Thanks a lot for your help, magister {MAGISTER}. May the Great Ulu be with you."), Node::MagBye)
}

fn circle_choices(s: Scene) -> Scene {
    s.choice("Where do I find some empty floor?", Node::MagCircleFloor)
        .choice("Where can I find some blood?", Node::MagBlood)
        .choice("Yes, I can do this. Thanks for your help.", Node::MagMore)
}

fn check_circle(cx: &mut Ctx) {
    let p = &mut cx.p;
    if p.circle_feather.is_some() && p.circle_candle && p.circle_incense.is_some() {
        p.circle = RitualCircle::Ready;
    }
}

fn ritual(cx: &mut Ctx, s: Scene) -> Scene {
    let p = &cx.p;
    match (p.carpet, p.circle) {
        (Carpet::Clean, _) => s
            .n("The carpet lies exactly in the center of the room.")
            .choice("I will roll it in.", Node::CarpetRollIn)
            .choice("I will leave it as it is.", Node::Exit),
        (Carpet::Bloody, _) => s
            .c("Urgh, I hope nobody will notice the blood stain on the carpet. Maybe I should roll it in to hide the stain.")
            .choice("I will roll it in.", Node::CarpetRollIn)
            .choice("I will leave it as it is.", Node::Exit),
        (Carpet::RolledIn, RitualCircle::None) => {
            let knife = p.has(Item::Knife);
            s.n("The floor is empty and clean.")
                .choice("I will leave it as it is.", Node::Exit)
                .choice_if(knife, "Let's draw the invocation circle!", Node::FloorDraw)
        }
        (Carpet::RolledIn, RitualCircle::Unfinished) => {
            let feather = p.circle_feather.is_none();
            let candle = !p.circle_candle;
            let incense = p.circle_incense.is_none();
            s.n("You examine the invocation circle. It's a well shaped pentagram. You still require the invocation ingredients.")
                .choice_if(feather && p.has(Item::PerfectRavenFeather), "Place the perfect feather of a raven", Node::PlaceFeather)
                .choice_if(feather && p.has(Item::CreasedRavenFeather), "Place the creased feather of a raven", Node::PlaceFeather)
                .choice_if(candle && p.has(Item::BurningCandle), "Place the burning candle", Node::PlaceCandle)
                .choice_if(candle && p.has(Item::Candle), "Place the candle", Node::PlaceUnlitCandle)
                .choice_if(incense && p.has(Item::StinkyCheese), "Place the stinky cheese", Node::PlaceIncense)
                .choice_if(incense && p.has(Item::MeltedCheese), "Place the melted stinky cheese", Node::PlaceIncense)
                .choice("I still need to gather some materials.", Node::Exit)
        }
        (Carpet::RolledIn, RitualCircle::Ready) => s
            .n("You examine the invocation circle. It's a well shaped pentagram. All the ingredients required for the ritual are prepared: a burning candle, incense and the feather of a raven.")
            .choice("I am ready to summon Ulu.", Node::Summon)
            .choice("I need to check something first.", Node::Exit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every change of the ritual counter has a reason.
    fn assert_notes_match_counter(p: &Progress) {
        let sum: i32 = p.ritual_notes.iter().map(|note| note.change).sum();
        assert_eq!(sum, p.ritual_counter);
    }

    /// Plays the script like a player would.
    struct Sim {
        p: Progress,
        meta: Meta,
        choices: Vec<Choice>,
        ending: Option<Ending>,
        sfx: Vec<Sfx>,
    }

    impl Sim {
        fn new(meta: Meta) -> Self {
            Self {
                p: Progress::default(),
                meta,
                choices: Vec::new(),
                ending: None,
                sfx: Vec::new(),
            }
        }

        fn enter(&mut self, mut node: Node) -> &mut Self {
            loop {
                let mut cx = Ctx::new(&mut self.p, &mut self.meta);
                let scene = run(node, &mut cx);
                for effect in cx.effects {
                    if let Effect::Sfx(sfx) = effect {
                        self.sfx.push(sfx);
                    }
                }
                self.choices.clear();
                match scene.next {
                    Next::Goto(next) => node = next,
                    Next::Choices(choices) => {
                        self.choices = choices;
                        return self;
                    }
                    Next::End => return self,
                    Next::Ending(ending) => {
                        self.ending = Some(ending);
                        return self;
                    }
                }
            }
        }

        fn choose(&mut self, prefix: &str) -> &mut Self {
            let choice = self
                .choices
                .iter()
                .find(|c| c.text.starts_with(prefix))
                .unwrap_or_else(|| {
                    let texts: Vec<_> = self.choices.iter().map(|c| &c.text).collect();
                    panic!("No choice {prefix:?} in {texts:?}")
                })
                .clone();
            self.enter(choice.node)
        }

        fn has_choice(&self, prefix: &str) -> bool {
            self.choices.iter().any(|c| c.text.starts_with(prefix))
        }

        /// Gathers everything for a low ritual counter.
        fn perfect_preparation(&mut self) -> &mut Self {
            self.enter(Node::Chest).choose("There is a sharp knife");
            self.enter(Node::Bed).choose("Cut the pillow");
            self.enter(Node::Mirror)
                .choose("Examine yourself")
                .choose("Examine yourself further")
                .choose("A truly devoted")
                .choose("I believe I can see Ulu")
                .choose("[Remain quiet");
            assert!(self.p.has(Item::RedHerring));
            self
        }

        fn light_candle_and_soil_feather(&mut self) -> &mut Self {
            self.enter(Node::Fireplace).choose("Ignite the candle");
            assert!(self.p.has(Item::BurningCandle));
            self.enter(Node::Fireplace).choose("Extinguish the fire");
            self.enter(Node::Fireplace).choose("Soil the feather");
            self
        }

        fn do_ritual(&mut self) -> &mut Self {
            assert!(self.p.has_light());
            self.enter(Node::Ritual).choose("I will roll it in");
            self.enter(Node::Ritual)
                .choose("Let's draw the invocation circle");
            if !self.choices.is_empty() {
                self.choose("Use the red herring");
            }
            assert_eq!(self.p.circle, RitualCircle::Unfinished);
            self.enter(Node::Ritual).choose("Place the perfect feather");
            self.enter(Node::Ritual).choose("Place the burning candle");
            self.enter(Node::Ritual).choose("Place the stinky cheese");
            assert_eq!(self.p.circle, RitualCircle::Ready);
            self.enter(Node::Ritual).choose("I am ready to summon Ulu");
            self
        }
    }

    #[test]
    fn rich_harvest_then_meowthy_cultist() {
        let mut sim = Sim::new(Meta::default());
        sim.perfect_preparation();
        sim.enter(Node::Librarian)
            .choose("Oh, sorry. I am in a hurry");
        sim.light_candle_and_soil_feather();
        sim.enter(Node::Table).choose("I think the cheese is old");
        sim.do_ritual();
        assert_eq!(sim.p.ritual_counter, 1);
        assert_eq!(sim.ending, Some(Ending::RichHarvest));
        assert_eq!(sim.p.ritual_stars(), Some(4));
        assert_notes_match_counter(&sim.p);
        assert!(sim.meta.achievements.contains(&Achievement::RichHarvest));

        // A second run with the knowledge of the first one.
        let mut sim = Sim::new(sim.meta);
        sim.perfect_preparation();
        sim.enter(Node::LibrarianBack).choose("[Use the knife]");
        assert!(sim.p.librarian_gone && sim.p.has(Item::Candle));
        sim.light_candle_and_soil_feather();
        sim.enter(Node::Table).choose("I think the cheese is old");
        sim.do_ritual();
        assert_eq!(sim.p.ritual_counter, -1);
        assert_eq!(sim.ending, Some(Ending::MeowthyCultist));
        assert_eq!(sim.p.ritual_stars(), Some(5));
        assert_notes_match_counter(&sim.p);
        let notes = sim.p.ritual_assessment();
        assert!(notes.iter().any(|note| note.change < 0));
        assert!(notes.iter().any(|note| note.text.contains("flawless")));
        for achievement in [
            Achievement::MeowthyCultist,
            Achievement::WhosTheRealMaster,
            Achievement::CatUluGuidesMe,
            Achievement::JustAQuietPeep,
        ] {
            assert!(
                sim.p.new_achievements.contains(&achievement),
                "{achievement:?}"
            );
        }
    }

    #[test]
    fn cat_grass_with_magisters_help() {
        let mut sim = Sim::new(Meta::default());
        sim.enter(Node::Magister)
            .choose("Magister")
            .choose("I don't know. I just woke up");
        assert!(sim.p.knows_candle);
        sim.enter(Node::Librarian).choose("I know, but it's urgent");
        sim.enter(Node::Magister)
            .choose("Magister")
            .choose("I require your help")
            .choose("How do I get some blood?")
            .choose("I am not sure")
            .choose("I don't have any wine")
            .choose("This is all I needed");
        assert!(sim.p.has(Item::Wine));
        sim.enter(Node::Chest).choose("There is a sharp knife");
        sim.enter(Node::Bed).choose("Tear the pillow");
        sim.enter(Node::Table).choose("I think the cheese is old");
        // Visiting the burning fireplace melts the cheese.
        sim.enter(Node::Fireplace).choose("Ignite the candle");
        assert!(sim.p.has(Item::MeltedCheese));
        sim.enter(Node::Fireplace).choose("Extinguish the fire");
        sim.enter(Node::Fireplace).choose("Soil the feather");
        assert!(sim.p.has_light());
        sim.enter(Node::Ritual).choose("I will roll it in");
        sim.enter(Node::Ritual)
            .choose("Let's draw the invocation circle");
        assert_eq!(sim.p.circle, RitualCircle::Unfinished);
        sim.enter(Node::Ritual).choose("Place the creased feather");
        sim.enter(Node::Ritual).choose("Place the burning candle");
        sim.enter(Node::Ritual)
            .choose("Place the melted stinky cheese");
        sim.enter(Node::Ritual).choose("I am ready to summon Ulu");
        assert_eq!(sim.p.ritual_counter, 4);
        assert_eq!(sim.ending, Some(Ending::CatGrass));
        assert_eq!(sim.p.ritual_stars(), Some(1));
        assert_notes_match_counter(&sim.p);
        assert_eq!(sim.p.ritual_notes.len(), 4);
    }

    #[test]
    fn sleeping_unlocks_the_librarians_trap() {
        let mut sim = Sim::new(Meta::default());
        sim.enter(Node::Librarian).choose("Who are you?");
        assert!(!sim.p.got_candle());
        sim.enter(Node::Bed).choose("Sleep");
        assert_eq!(sim.ending, Some(Ending::Sleep));
        assert_eq!(sim.p.ritual_stars(), None);
        assert!(sim.meta.confirmed_meta_knowledge);

        let mut sim = Sim::new(sim.meta);
        sim.enter(Node::Librarian).choose("Who are you?");
        sim.enter(Node::Librarian).choose("That was a trap!");
        assert!(sim.p.has(Item::Candle));
        assert!(sim.p.new_achievements.contains(&Achievement::YouDontFoolMe));
    }

    #[test]
    fn locked_doors_show_their_own_varying_messages() {
        let mut sim = Sim::new(Meta::default());
        let mut message = |occupant| {
            let mut cx = Ctx::new(&mut sim.p, &mut sim.meta);
            let scene = run(Node::LockedDoor(occupant), &mut cx);
            assert_eq!(scene.lines.len(), 1);
            scene.lines[0].text.clone()
        };
        for occupant in [Occupant::Sleeper, Occupant::Chanter, Occupant::Scribe] {
            let messages = occupant.messages();
            // Trying another door in between doesn't skip any message.
            let shown: Vec<_> = (0..messages.len() + 1)
                .map(|_| {
                    message(Occupant::Unknown);
                    message(occupant)
                })
                .collect();
            assert!(shown.windows(2).all(|pair| pair[0] != pair[1]));
            assert_eq!(shown[..messages.len()], *messages);
            assert_eq!(shown[0], shown[messages.len()]);
        }
    }

    #[test]
    fn tripping_ten_times_is_unworthy() {
        let mut sim = Sim::new(Meta::default());
        for _ in 0..9 {
            sim.enter(Node::Trip);
            assert_eq!(sim.ending, None);
        }
        assert_eq!(sim.p.carpet, Carpet::Bloody);
        sim.enter(Node::Trip);
        assert_eq!(sim.ending, Some(Ending::Unworthy));
        assert_eq!(sim.p.ritual_stars(), None);
    }

    #[test]
    fn asking_all_topics_grants_wisdom() {
        let mut sim = Sim::new(Meta::default());
        sim.enter(Node::Magister)
            .choose("Magister")
            .choose("I require")
            .choose("Yes, I want");
        for topic in [
            "Where can I find a feather",
            "Where can I find a candle",
            "How can I make",
            "Where can I find some incense",
            "How do I get some blood",
            "Where can I find a knife",
        ] {
            sim.choose(topic);
            // Leave sub-dialogues until the topic list is shown again.
            while !sim.has_choice("This is all I needed") {
                let first = sim.choices.last().unwrap().text.clone();
                sim.choose(&first);
            }
        }
        assert!(
            sim.meta
                .achievements
                .contains(&Achievement::WisdomOfAThousandMice)
        );
    }
}
