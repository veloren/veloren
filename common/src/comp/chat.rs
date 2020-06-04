use crate::sync::Uid;
use specs::Component;
use specs_idvs::IDVStorage;
use std::time::{Duration, Instant};

/// A player's current chat mode.
#[derive(Copy, Clone, Debug)]
pub enum ChatMode {
    /// Private message to another player (by uuid)
    Tell(Uid),
    /// Talk to players within shouting distance
    Say,
    /// Talk to players in your region of the world
    Region,
    /// Talk to your current group of players
    Group,
    /// Talk to your faction
    Faction,
    /// Talk to every player on the server
    World,
}

impl Component for ChatMode {
    type Storage = IDVStorage<Self>;
}

impl ChatMode {
    /// Create a message from your current chat mode and uuid.
    pub fn msg_from(&self, from: Uid, message: String) -> ChatMsg {
        let chat_type = match self {
            ChatMode::Tell(to) => ChatType::Tell(from, *to),
            ChatMode::Say => ChatType::Say(from),
            ChatMode::Region => ChatType::Region(from),
            ChatMode::Group => ChatType::Group(from),
            ChatMode::Faction => ChatType::Faction(from),
            ChatMode::World => ChatType::World(from),
        };
        ChatMsg { chat_type, message }
    }
}

/// List of chat types. Note that this is a superset of `ChatMode`; this is
/// because `ChatType::Kill`, `ChatType::Broadcast`, and `ChatType::Private`
/// cannot be sent by players.
#[derive(Copy, Debug, Clone, Serialize, Deserialize)]
pub enum ChatType {
    /// Tell all players something (such as players connecting or alias changes)
    Broadcast,
    /// Private messages from the server (such as results of chat commands)
    Private,
    /// Inform players that someone died
    Kill,
    /// One-on-one chat (from, to)
    Tell(Uid, Uid),
    /// Chat with nearby players
    Say(Uid),
    /// Group chat
    Group(Uid),
    /// Factional chat
    Faction(Uid),
    /// Regional chat
    Region(Uid),
    /// World chat
    World(Uid),
    /// Messages sent from NPCs (Not shown in chat but as speech bubbles)
    ///
    /// The u16 field is a random number for selecting localization variants.
    Npc(Uid, u16),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMsg {
    pub chat_type: ChatType,
    pub message: String,
}

impl ChatMsg {
    pub fn npc(uid: Uid, message: String) -> Self {
        let chat_type = ChatType::Npc(uid, rand::random());
        Self { chat_type, message }
    }

    pub fn to_bubble(&self) -> Option<(SpeechBubble, Uid)> {
        let tuple = match self.chat_type {
            ChatType::Broadcast => None,
            ChatType::Private => None,
            ChatType::Kill => None,
            ChatType::Tell(u, _) => Some((SpeechBubbleIcon::Tell, u, None)),
            ChatType::Say(u) => Some((SpeechBubbleIcon::Say, u, None)),
            ChatType::Group(u) => Some((SpeechBubbleIcon::Group, u, None)),
            ChatType::Faction(u) => Some((SpeechBubbleIcon::Faction, u, None)),
            ChatType::Region(u) => Some((SpeechBubbleIcon::Region, u, None)),
            ChatType::World(u) => Some((SpeechBubbleIcon::World, u, None)),
            ChatType::Npc(u, r) => Some((SpeechBubbleIcon::None, u, Some(r))),
        };
        tuple.map(|(icon, from, npc_rand)| {
            if let Some(r) = npc_rand {
                (SpeechBubble::npc_new(self.message.clone(), r, icon), from)
            } else {
                (SpeechBubble::player_new(self.message.clone(), icon), from)
            }
        })
    }
}

/// Player groups are useful when forming raiding parties and coordinating
/// gameplay.
///
/// Groups are currently just an associated String (the group's name)
pub struct Group(String);
impl Component for Group {
    type Storage = IDVStorage<Self>;
}

/// Player factions are used to coordinate pvp vs hostile factions or segment
/// chat from the world
///
/// Factions are currently just an associated String (the faction's name)
pub struct Faction(String);
impl Component for Faction {
    type Storage = IDVStorage<Self>;
}

/// The contents of a speech bubble
pub enum SpeechBubbleMessage {
    /// This message was said by a player and needs no translation
    Plain(String),
    /// This message was said by an NPC. The fields are a i18n key and a random
    /// u16 index
    Localized(String, u16),
}

pub enum SpeechBubbleIcon {
    // One for each chat mode
    Tell,
    Say,
    Region,
    Group,
    Faction,
    World,
    // For NPCs
    Quest, // TODO not implemented
    Trade, // TODO not implemented
    None,  // No icon (default for npcs)
}

/// Adds a speech bubble above the character
pub struct SpeechBubble {
    pub message: SpeechBubbleMessage,
    pub icon: SpeechBubbleIcon,
    pub timeout: Instant,
}

impl SpeechBubble {
    /// Default duration in seconds of speech bubbles
    pub const DEFAULT_DURATION: f64 = 5.0;

    pub fn npc_new(i18n_key: String, r: u16, icon: SpeechBubbleIcon) -> Self {
        let message = SpeechBubbleMessage::Localized(i18n_key, r);
        let timeout = Instant::now() + Duration::from_secs_f64(SpeechBubble::DEFAULT_DURATION);
        Self {
            message,
            timeout,
            icon,
        }
    }

    pub fn player_new(message: String, icon: SpeechBubbleIcon) -> Self {
        let message = SpeechBubbleMessage::Plain(message);
        let timeout = Instant::now() + Duration::from_secs_f64(SpeechBubble::DEFAULT_DURATION);
        Self {
            message,
            timeout,
            icon,
        }
    }

    pub fn message<F>(&self, i18n_variation: F) -> String
    where
        F: Fn(String, u16) -> String,
    {
        match &self.message {
            SpeechBubbleMessage::Plain(m) => m.to_string(),
            SpeechBubbleMessage::Localized(k, i) => i18n_variation(k.to_string(), *i).to_string(),
        }
    }
}
