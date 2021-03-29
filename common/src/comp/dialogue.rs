use vek::{Vec2, Vec3};

use super::Item;

#[derive(Clone, Debug)]
pub struct AskedLocation {
    pub name: String,
    pub origin: Vec2<i32>,
}

#[derive(Clone, Debug)]
pub enum PersonType {
    Merchant,
    Villager { name: String },
}

#[derive(Clone, Debug)]
pub struct AskedPerson {
    pub person_type: PersonType,
    pub origin: Option<Vec3<f32>>,
}

impl AskedPerson {
    pub fn name(&self) -> String {
        match &self.person_type {
            PersonType::Merchant => "The Merchant".to_string(),
            PersonType::Villager { name } => name.clone(),
        }
    }
}

/// Conversation subject
#[derive(Clone, Debug)]
pub enum Subject {
    /// Using simple interaction with NPC
    /// This is meant to be the default behavior of talking
    /// NPC will throw a random dialogue to you
    Regular,
    /// Asking for trading
    /// Ask the person to trade with you
    /// NPC will either invite you to trade, or decline
    Trade,
    /// Inquiring the mood of the NPC
    /// NPC will explain what his mood is, and why.
    /// Can lead to potential quests if the NPC has a bad mood
    /// Else it'll just be flavor text explaining why he got this mood
    Mood,
    /// Asking for a location
    /// NPC will either know where this location is, or not
    /// It'll tell you which direction and approx what distance it is from you
    Location(AskedLocation),
    /// Asking for a person's location
    /// NPC will either know where this person is, or not
    /// It'll tell you which direction and approx what distance it is from you
    Person(AskedPerson),
    /// Asking for work
    /// NPC will give you a quest if his mood is bad enough
    /// So either it'll tell you something to do, or just say that he got
    /// nothing
    Work,
}

/// Context of why a NPC has a specific mood (good, neutral, bad, ...)
#[derive(Clone, Debug)]
pub enum MoodContext {
    /// The weather is good, sunny, appeasing, etc...
    GoodWeather,
    /// Someone completed a quest and enlightened this NPC's day
    QuestSucceeded { hero: String, quest_desc: String },

    /// Normal day, same as yesterday, nothing relevant to say about it, that's
    /// everyday life
    EverydayLife,
    /// Need one or more items in order to complete a personal task, or for
    /// working
    NeedItem { item: Item, quantity: u16 },

    /// A personal good has been robbed! Gotta find a replacement
    MissingItem { item: Item },
}

// Note: You can add in-between states if needed
/// NPC mood status indicator
#[derive(Clone, Debug)]
pub enum MoodState {
    /// The NPC is happy!
    Good(MoodContext),
    /// The NPC is having a normal day
    Neutral(MoodContext),
    /// The NPC got a pretty bad day. He may even need player's help!
    Bad(MoodContext),
}

// TODO: dialogue localization
impl MoodState {
    pub fn describe(&self) -> String {
        match self {
            MoodState::Good(context) => format!("I'm so happy, {}", context.describe()),
            MoodState::Neutral(context) => context.describe(),
            MoodState::Bad(context) => {
                format!("I'm mad, {}", context.describe())
            },
        }
    }
}

// TODO: dialogue localization
impl MoodContext {
    pub fn describe(&self) -> String {
        match &self {
            MoodContext::GoodWeather => "The weather is great today!".to_string(),
            MoodContext::QuestSucceeded { hero, quest_desc } => {
                format!("{} helped me on {}", hero, quest_desc)
            },
            &MoodContext::EverydayLife => "Life's going as always.".to_string(),
            MoodContext::NeedItem { item, quantity } => {
                format!("I need {} {}!", quantity, item.name())
            },
            &MoodContext::MissingItem { item } => {
                format!("Someone robbed my {}!", item.name())
            },
        }
    }
}
