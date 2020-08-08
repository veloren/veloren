use crate::{comp::group::Group, msg::ServerMsg, sync::Uid};
use serde::{Deserialize, Serialize};
use specs::Component;
use specs_idvs::IdvStorage;
use std::time::{Duration, Instant};

/// A player's current chat mode. These are chat types that can only be sent by
/// the player.
#[derive(Clone, Debug)]
pub enum ChatMode {
    /// Private message to another player (by uuid)
    Tell(Uid),
    /// Talk to players within shouting distance
    Say,
    /// Talk to players in your region of the world
    Region,
    /// Talk to your current group of players
    Group(Group),
    /// Talk to your faction
    Faction(String),
    /// Talk to every player on the server
    World,
}

impl Component for ChatMode {
    type Storage = IdvStorage<Self>;
}

impl ChatMode {
    /// Create a message from your current chat mode and uuid.
    pub fn new_message(&self, from: Uid, message: String) -> UnresolvedChatMsg {
        let chat_type = match self {
            ChatMode::Tell(to) => ChatType::Tell(from, *to),
            ChatMode::Say => ChatType::Say(from),
            ChatMode::Region => ChatType::Region(from),
            ChatMode::Group(group) => ChatType::Group(from, *group),
            ChatMode::Faction(faction) => ChatType::Faction(from, faction.clone()),
            ChatMode::World => ChatType::World(from),
        };
        UnresolvedChatMsg { chat_type, message }
    }
}

impl Default for ChatMode {
    fn default() -> Self { Self::World }
}

/// List of chat types. Each one is colored differently and has its own icon.
///
/// This is a superset of `SpeechBubbleType`, which is a superset of `ChatMode`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatType<G> {
    /// A player came online
    Online,
    /// A player went offline
    Offline,
    /// The result of chat commands
    CommandInfo,
    /// A chat command failed
    CommandError,
    /// Inform players that someone died
    Kill,
    /// Server notifications to a group, such as player join/leave
    GroupMeta(G),
    /// Server notifications to a faction, such as player join/leave
    FactionMeta(String),
    /// One-on-one chat (from, to)
    Tell(Uid, Uid),
    /// Chat with nearby players
    Say(Uid),
    /// Group chat
    Group(Uid, G),
    /// Factional chat
    Faction(Uid, String),
    /// Regional chat
    Region(Uid),
    /// World chat
    World(Uid),
    /// Messages sent from NPCs (Not shown in chat but as speech bubbles)
    ///
    /// The u16 field is a random number for selecting localization variants.
    Npc(Uid, u16),
    /// Anything else
    Meta,
    // Looted items
    Loot,
}

impl<G> ChatType<G> {
    pub fn chat_msg<S>(self, msg: S) -> GenericChatMsg<G>
    where
        S: Into<String>,
    {
        GenericChatMsg {
            chat_type: self,
            message: msg.into(),
        }
    }
}
impl ChatType<String> {
    pub fn server_msg<S>(self, msg: S) -> ServerMsg
    where
        S: Into<String>,
    {
        ServerMsg::ChatMsg(self.chat_msg(msg))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericChatMsg<G> {
    pub chat_type: ChatType<G>,
    pub message: String,
}

pub type ChatMsg = GenericChatMsg<String>;
pub type UnresolvedChatMsg = GenericChatMsg<Group>;

impl<G> GenericChatMsg<G> {
    pub const NPC_DISTANCE: f32 = 100.0;
    pub const REGION_DISTANCE: f32 = 1000.0;
    pub const SAY_DISTANCE: f32 = 100.0;

    pub fn npc(uid: Uid, message: String) -> Self {
        let chat_type = ChatType::Npc(uid, rand::random());
        Self { chat_type, message }
    }

    pub fn map_group<T>(self, mut f: impl FnMut(G) -> T) -> GenericChatMsg<T> {
        let chat_type = match self.chat_type {
            ChatType::Online => ChatType::Online,
            ChatType::Offline => ChatType::Offline,
            ChatType::CommandInfo => ChatType::CommandInfo,
            ChatType::CommandError => ChatType::CommandError,
            ChatType::Loot => ChatType::Loot,
            ChatType::FactionMeta(a) => ChatType::FactionMeta(a),
            ChatType::GroupMeta(g) => ChatType::GroupMeta(f(g)),
            ChatType::Kill => ChatType::Kill,
            ChatType::Tell(a, b) => ChatType::Tell(a, b),
            ChatType::Say(a) => ChatType::Say(a),
            ChatType::Group(a, g) => ChatType::Group(a, f(g)),
            ChatType::Faction(a, b) => ChatType::Faction(a, b),
            ChatType::Region(a) => ChatType::Region(a),
            ChatType::World(a) => ChatType::World(a),
            ChatType::Npc(a, b) => ChatType::Npc(a, b),
            ChatType::Meta => ChatType::Meta,
        };

        GenericChatMsg {
            chat_type,
            message: self.message,
        }
    }

    pub fn to_bubble(&self) -> Option<(SpeechBubble, Uid)> {
        let icon = self.icon();
        if let ChatType::Npc(from, r) = self.chat_type {
            Some((SpeechBubble::npc_new(&self.message, r, icon), from))
        } else {
            self.uid()
                .map(|from| (SpeechBubble::player_new(&self.message, icon), from))
        }
    }

    pub fn icon(&self) -> SpeechBubbleType {
        match &self.chat_type {
            ChatType::Online => SpeechBubbleType::None,
            ChatType::Offline => SpeechBubbleType::None,
            ChatType::CommandInfo => SpeechBubbleType::None,
            ChatType::CommandError => SpeechBubbleType::None,
            ChatType::Loot => SpeechBubbleType::None,
            ChatType::FactionMeta(_) => SpeechBubbleType::None,
            ChatType::GroupMeta(_) => SpeechBubbleType::None,
            ChatType::Kill => SpeechBubbleType::None,
            ChatType::Tell(_u, _) => SpeechBubbleType::Tell,
            ChatType::Say(_u) => SpeechBubbleType::Say,
            ChatType::Group(_u, _s) => SpeechBubbleType::Group,
            ChatType::Faction(_u, _s) => SpeechBubbleType::Faction,
            ChatType::Region(_u) => SpeechBubbleType::Region,
            ChatType::World(_u) => SpeechBubbleType::World,
            ChatType::Npc(_u, _r) => SpeechBubbleType::None,
            ChatType::Meta => SpeechBubbleType::None,
        }
    }

    pub fn uid(&self) -> Option<Uid> {
        match &self.chat_type {
            ChatType::Online => None,
            ChatType::Offline => None,
            ChatType::CommandInfo => None,
            ChatType::CommandError => None,
            ChatType::Loot => None,
            ChatType::FactionMeta(_) => None,
            ChatType::GroupMeta(_) => None,
            ChatType::Kill => None,
            ChatType::Tell(u, _t) => Some(*u),
            ChatType::Say(u) => Some(*u),
            ChatType::Group(u, _s) => Some(*u),
            ChatType::Faction(u, _s) => Some(*u),
            ChatType::Region(u) => Some(*u),
            ChatType::World(u) => Some(*u),
            ChatType::Npc(u, _r) => Some(*u),
            ChatType::Meta => None,
        }
    }
}

/// Player factions are used to coordinate pvp vs hostile factions or segment
/// chat from the world
///
/// Factions are currently just an associated String (the faction's name)
#[derive(Clone, Debug)]
pub struct Faction(pub String);
impl Component for Faction {
    type Storage = IdvStorage<Self>;
}
impl From<String> for Faction {
    fn from(s: String) -> Self { Faction(s) }
}

/// The contents of a speech bubble
pub enum SpeechBubbleMessage {
    /// This message was said by a player and needs no translation
    Plain(String),
    /// This message was said by an NPC. The fields are a i18n key and a random
    /// u16 index
    Localized(String, u16),
}

/// List of chat types for players and NPCs. Each one has its own icon.
///
/// This is a subset of `ChatType`, and a superset of `ChatMode`
pub enum SpeechBubbleType {
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
    pub icon: SpeechBubbleType,
    pub timeout: Instant,
}

impl SpeechBubble {
    /// Default duration in seconds of speech bubbles
    pub const DEFAULT_DURATION: f64 = 5.0;

    pub fn npc_new(i18n_key: &str, r: u16, icon: SpeechBubbleType) -> Self {
        let message = SpeechBubbleMessage::Localized(i18n_key.to_string(), r);
        let timeout = Instant::now() + Duration::from_secs_f64(SpeechBubble::DEFAULT_DURATION);
        Self {
            message,
            timeout,
            icon,
        }
    }

    pub fn player_new(message: &str, icon: SpeechBubbleType) -> Self {
        let message = SpeechBubbleMessage::Plain(message.to_string());
        let timeout = Instant::now() + Duration::from_secs_f64(SpeechBubble::DEFAULT_DURATION);
        Self {
            message,
            timeout,
            icon,
        }
    }

    pub fn message<F>(&self, i18n_variation: F) -> String
    where
        F: Fn(&str, u16) -> String,
    {
        match &self.message {
            SpeechBubbleMessage::Plain(m) => m.to_string(),
            SpeechBubbleMessage::Localized(k, i) => i18n_variation(&k, *i),
        }
    }
}
