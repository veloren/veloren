use crate::{path::Chaser, state::Time};
use specs::{Component, Entity as EcsEntity};
use specs_idvs::IDVStorage;
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Alignment {
    /// Wild animals and gentle giants
    Wild,
    /// Dungeon cultists and bandits
    Enemy,
    /// Friendly folk in villages
    Npc,
    /// Farm animals and pets of villagers
    Tame,
    /// Pets you've tamed with a collar
    Owned(EcsEntity),
}

impl Alignment {
    // Always attacks
    pub fn hostile_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Enemy, Alignment::Enemy) => false,
            (Alignment::Enemy, _) => true,
            (_, Alignment::Enemy) => true,
            _ => false,
        }
    }

    // Never attacks
    pub fn passive_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Enemy, Alignment::Enemy) => true,
            (Alignment::Owned(a), Alignment::Owned(b)) if a == b => true,
            (Alignment::Npc, Alignment::Npc) => true,
            (Alignment::Npc, Alignment::Tame) => true,
            (Alignment::Tame, Alignment::Npc) => true,
            (Alignment::Tame, Alignment::Tame) => true,
            _ => false,
        }
    }
}

impl Component for Alignment {
    type Storage = IDVStorage<Self>;
}

#[derive(Clone, Debug, Default)]
pub struct Agent {
    pub patrol_origin: Option<Vec3<f32>>,
    pub activity: Activity,
    /// Does the agent talk when e.g. hit by the player
    // TODO move speech patterns into a Behavior component
    pub can_speak: bool,
}

impl Agent {
    pub fn with_patrol_origin(mut self, origin: Vec3<f32>) -> Self {
        self.patrol_origin = Some(origin);
        self
    }

    pub fn new(origin: Vec3<f32>, can_speak: bool) -> Self {
        let patrol_origin = Some(origin);
        Agent {
            patrol_origin,
            can_speak,
            ..Default::default()
        }
    }
}

impl Component for Agent {
    type Storage = IDVStorage<Self>;
}

#[derive(Clone, Debug)]
pub enum Activity {
    Idle(Vec2<f32>),
    Follow(EcsEntity, Chaser),
    Attack {
        target: EcsEntity,
        chaser: Chaser,
        time: f64,
        been_close: bool,
        powerup: f32,
    },
}

impl Activity {
    pub fn is_follow(&self) -> bool {
        match self {
            Activity::Follow(_, _) => true,
            _ => false,
        }
    }

    pub fn is_attack(&self) -> bool {
        match self {
            Activity::Attack { .. } => true,
            _ => false,
        }
    }
}

impl Default for Activity {
    fn default() -> Self { Activity::Idle(Vec2::zero()) }
}

/// Default duration in seconds of speech bubbles
pub const SPEECH_BUBBLE_DURATION: f64 = 5.0;

/// The contents of a speech bubble
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SpeechBubbleMessage {
    /// This message was said by a player and needs no translation
    Plain(String),
    /// This message was said by an NPC. The fields are a i18n key and a random
    /// u16 index
    Localized(String, u16),
}

/// Adds a speech bubble to the entity
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpeechBubble {
    pub message: SpeechBubbleMessage,
    pub timeout: Option<Time>,
    // TODO add icon enum for player chat type / npc quest+trade
}
impl SpeechBubble {
    pub fn npc_new(i18n_key: String, now: Time) -> Self {
        let message = SpeechBubbleMessage::Localized(i18n_key, rand::random());
        let timeout = Some(Time(now.0 + SPEECH_BUBBLE_DURATION));
        Self { message, timeout }
    }

    pub fn player_new(message: String, now: Time) -> Self {
        let message = SpeechBubbleMessage::Plain(message);
        let timeout = Some(Time(now.0 + SPEECH_BUBBLE_DURATION));
        Self { message, timeout }
    }

    pub fn message<F>(&self, i18n_variation: F) -> String
    where
        F: Fn(String, u16) -> String,
    {
        match &self.message {
            SpeechBubbleMessage::Plain(m) => m.to_string(),
            SpeechBubbleMessage::Localized(k, i) => i18n_variation(k.to_string(), *i),
        }
    }
}
