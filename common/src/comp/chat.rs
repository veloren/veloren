use crate::{
    comp::{group::Group, BuffKind},
    uid::Uid,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage};
use std::time::{Duration, Instant};

/// A player's current chat mode. These are chat types that can only be sent by
/// the player.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    Faction(String),
    /// Talk to every player on the server
    World,
}

impl Component for ChatMode {
    type Storage = DenseVecStorage<Self>;
}

impl ChatMode {
    /// Create a plain message from your current chat mode and uuid.
    pub fn to_plain_msg(
        &self,
        from: Uid,
        text: impl ToString,
        group: Option<Group>,
    ) -> Result<UnresolvedChatMsg, &'static str> {
        let chat_type = match self {
            ChatMode::Tell(to) => ChatType::Tell(from, *to),
            ChatMode::Say => ChatType::Say(from),
            ChatMode::Region => ChatType::Region(from),
            ChatMode::Group => ChatType::Group(
                from,
                group.ok_or(
                    "You tried sending a group message while not belonging to a group. Use /world \
                     or /region to change your chat mode.",
                )?,
            ),
            ChatMode::Faction(faction) => ChatType::Faction(from, faction.clone()),
            ChatMode::World => ChatType::World(from),
        };

        Ok(UnresolvedChatMsg {
            chat_type,
            content: Content::Plain(text.to_string()),
        })
    }
}

impl ChatMode {
    pub const fn default() -> Self { Self::World }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KillType {
    Buff(BuffKind),
    Melee,
    Projectile,
    Explosion,
    Energy,
    Other,
    // Projectile(String), TODO: add projectile name when available
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KillSource {
    Player(Uid, KillType),
    NonPlayer(String, KillType),
    NonExistent(KillType),
    Environment(String),
    FallDamage,
    Suicide,
    Other,
}

/// List of chat types. Each one is colored differently and has its own icon.
///
/// This is a superset of `SpeechBubbleType`, which is a superset of `ChatMode`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatType<G> {
    /// A player came online
    Online(Uid),
    /// A player went offline
    Offline(Uid),
    /// The result of chat commands
    CommandInfo,
    /// A chat command failed
    CommandError,
    /// Inform players that someone died (Source, Victim) Source may be None
    /// (ex: fall damage)
    Kill(KillSource, Uid),
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
    Npc(Uid),
    /// From NPCs but in the chat for clients in the near vicinity
    NpcSay(Uid),
    /// From NPCs but in the chat for a specific client. Shows a chat bubble.
    /// (from, to, localization variant)
    NpcTell(Uid, Uid),
    /// Anything else
    Meta,
}

impl<G> ChatType<G> {
    pub fn into_plain_msg(self, text: impl ToString) -> GenericChatMsg<G> {
        GenericChatMsg {
            chat_type: self,
            content: Content::Plain(text.to_string()),
        }
    }

    pub fn into_msg(self, content: Content) -> GenericChatMsg<G> {
        GenericChatMsg {
            chat_type: self,
            content,
        }
    }

    pub fn uid(&self) -> Option<Uid> {
        match self {
            ChatType::Online(_) => None,
            ChatType::Offline(_) => None,
            ChatType::CommandInfo => None,
            ChatType::CommandError => None,
            ChatType::FactionMeta(_) => None,
            ChatType::GroupMeta(_) => None,
            ChatType::Kill(_, _) => None,
            ChatType::Tell(u, _t) => Some(*u),
            ChatType::Say(u) => Some(*u),
            ChatType::Group(u, _s) => Some(*u),
            ChatType::Faction(u, _s) => Some(*u),
            ChatType::Region(u) => Some(*u),
            ChatType::World(u) => Some(*u),
            ChatType::Npc(u) => Some(*u),
            ChatType::NpcSay(u) => Some(*u),
            ChatType::NpcTell(u, _t) => Some(*u),
            ChatType::Meta => None,
        }
    }

    /// `None` means that the chat type is automated.
    pub fn is_private(&self) -> Option<bool> {
        match self {
            ChatType::Online(_)
            | ChatType::Offline(_)
            | ChatType::CommandInfo
            | ChatType::CommandError
            | ChatType::FactionMeta(_)
            | ChatType::GroupMeta(_)
            | ChatType::Npc(_)
            | ChatType::NpcSay(_)
            | ChatType::NpcTell(_, _)
            | ChatType::Meta
            | ChatType::Kill(_, _) => None,
            ChatType::Tell(_, _) | ChatType::Group(_, _) | ChatType::Faction(_, _) => Some(true),
            ChatType::Say(_) | ChatType::Region(_) | ChatType::World(_) => Some(false),
        }
    }
}

/// The content of a chat message.
// TODO: This could be generalised to *any* in-game text, not just chat messages (hence it not being
// called `ChatContent`). A few examples:
//
// - Signposts, both those appearing as overhead messages and those displayed 'in-world' on a shop
//   sign
// - UI elements
// - In-game notes/books (we could add a variant that allows structuring complex, novel textual
//   information as a syntax tree or some other intermediate format that can be localised by the
//   client)
// TODO: We probably want to have this type be able to represent similar things to
// `fluent::FluentValue`, such as numeric values, so that they can be properly localised in whatever
// manner is required.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Content {
    /// The content is a plaintext string that should be shown to the user
    /// verbatim.
    Plain(String),
    /// The content is a localizable message with the given arguments.
    Localized {
        /// i18n key
        key: String,
        /// Pseudorandom seed value that allows frontends to select a
        /// deterministic (but pseudorandom) localised output
        #[serde(default = "random_seed")]
        seed: u16,
        /// i18n arguments
        #[serde(default)]
        args: HashMap<String, LocalizationArg>,
    },
}

// TODO: Remove impl and make use of `Plain(...)` explicit (to discourage it)
impl From<String> for Content {
    fn from(text: String) -> Self { Self::Plain(text) }
}

// TODO: Remove impl and make use of `Plain(...)` explicit (to discourage it)
impl<'a> From<&'a str> for Content {
    fn from(text: &'a str) -> Self { Self::Plain(text.to_string()) }
}

/// A localisation argument for localised content (see [`Content::Localized`]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocalizationArg {
    /// The localisation argument is itself a section of content.
    ///
    /// Note that this allows [`Content`] to recursively refer to itself. It may
    /// be tempting to decide to parameterise everything, having dialogue
    /// generated with a compact tree. "It's simpler!", you might say. False.
    /// Over-parameterisation is an anti-pattern that hurts translators. Where
    /// possible, prefer fewer levels of nesting unless doing so would
    /// result in an intractably larger number of combinations. See [here](https://github.com/projectfluent/fluent/wiki/Good-Practices-for-Developers#prefer-wet-over-dry) for the
    /// guidance provided by the docs for `fluent`, the localisation library
    /// used by clients.
    Content(Content),
    /// The localisation argument is a natural number
    Nat(u64),
}

impl From<Content> for LocalizationArg {
    fn from(content: Content) -> Self { Self::Content(content) }
}

// TODO: Remove impl and make use of `Content(Plain(...))` explicit (to
// discourage it)
impl From<String> for LocalizationArg {
    fn from(text: String) -> Self { Self::Content(Content::Plain(text)) }
}

// TODO: Remove impl and make use of `Content(Plain(...))` explicit (to
// discourage it)
impl<'a> From<&'a str> for LocalizationArg {
    fn from(text: &'a str) -> Self { Self::Content(Content::Plain(text.to_string())) }
}

// TODO: Remove impl and make use of `Content(Plain(...))` explicit (to
// discourage it)
impl From<u64> for LocalizationArg {
    fn from(n: u64) -> Self { Self::Nat(n) }
}

fn random_seed() -> u16 { rand::random() }

impl Content {
    pub fn localized(key: impl ToString) -> Self {
        Self::Localized {
            key: key.to_string(),
            seed: random_seed(),
            args: HashMap::default(),
        }
    }

    pub fn localized_with_args<'a, A: Into<LocalizationArg>>(
        key: impl ToString,
        args: impl IntoIterator<Item = (&'a str, A)>,
    ) -> Self {
        Self::Localized {
            key: key.to_string(),
            seed: rand::random(),
            args: args
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.into()))
                .collect(),
        }
    }

    pub fn as_plain(&self) -> Option<&str> {
        match self {
            Self::Plain(text) => Some(text.as_str()),
            Self::Localized { .. } => None,
        }
    }
}

// Stores chat text, type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericChatMsg<G> {
    pub chat_type: ChatType<G>,
    content: Content,
}

pub type ChatMsg = GenericChatMsg<String>;
pub type UnresolvedChatMsg = GenericChatMsg<Group>;

impl<G> GenericChatMsg<G> {
    pub const NPC_DISTANCE: f32 = 100.0;
    pub const NPC_SAY_DISTANCE: f32 = 30.0;
    pub const REGION_DISTANCE: f32 = 1000.0;
    pub const SAY_DISTANCE: f32 = 100.0;

    pub fn npc(uid: Uid, content: Content) -> Self {
        let chat_type = ChatType::Npc(uid);
        Self { chat_type, content }
    }

    pub fn npc_say(uid: Uid, content: Content) -> Self {
        let chat_type = ChatType::NpcSay(uid);
        Self { chat_type, content }
    }

    pub fn npc_tell(from: Uid, to: Uid, content: Content) -> Self {
        let chat_type = ChatType::NpcTell(from, to);
        Self { chat_type, content }
    }

    pub fn map_group<T>(self, mut f: impl FnMut(G) -> T) -> GenericChatMsg<T> {
        let chat_type = match self.chat_type {
            ChatType::Online(a) => ChatType::Online(a),
            ChatType::Offline(a) => ChatType::Offline(a),
            ChatType::CommandInfo => ChatType::CommandInfo,
            ChatType::CommandError => ChatType::CommandError,
            ChatType::FactionMeta(a) => ChatType::FactionMeta(a),
            ChatType::GroupMeta(g) => ChatType::GroupMeta(f(g)),
            ChatType::Kill(a, b) => ChatType::Kill(a, b),
            ChatType::Tell(a, b) => ChatType::Tell(a, b),
            ChatType::Say(a) => ChatType::Say(a),
            ChatType::Group(a, g) => ChatType::Group(a, f(g)),
            ChatType::Faction(a, b) => ChatType::Faction(a, b),
            ChatType::Region(a) => ChatType::Region(a),
            ChatType::World(a) => ChatType::World(a),
            ChatType::Npc(a) => ChatType::Npc(a),
            ChatType::NpcSay(a) => ChatType::NpcSay(a),
            ChatType::NpcTell(a, b) => ChatType::NpcTell(a, b),
            ChatType::Meta => ChatType::Meta,
        };

        GenericChatMsg {
            chat_type,
            content: self.content,
        }
    }

    pub fn get_group(&self) -> Option<&G> {
        match &self.chat_type {
            ChatType::GroupMeta(g) => Some(g),
            ChatType::Group(_, g) => Some(g),
            _ => None,
        }
    }

    pub fn to_bubble(&self) -> Option<(SpeechBubble, Uid)> {
        self.uid()
            .map(|from| (SpeechBubble::new(self.content.clone(), self.icon()), from))
    }

    pub fn icon(&self) -> SpeechBubbleType {
        match &self.chat_type {
            ChatType::Online(_) => SpeechBubbleType::None,
            ChatType::Offline(_) => SpeechBubbleType::None,
            ChatType::CommandInfo => SpeechBubbleType::None,
            ChatType::CommandError => SpeechBubbleType::None,
            ChatType::FactionMeta(_) => SpeechBubbleType::None,
            ChatType::GroupMeta(_) => SpeechBubbleType::None,
            ChatType::Kill(_, _) => SpeechBubbleType::None,
            ChatType::Tell(_u, _) => SpeechBubbleType::Tell,
            ChatType::Say(_u) => SpeechBubbleType::Say,
            ChatType::Group(_u, _s) => SpeechBubbleType::Group,
            ChatType::Faction(_u, _s) => SpeechBubbleType::Faction,
            ChatType::Region(_u) => SpeechBubbleType::Region,
            ChatType::World(_u) => SpeechBubbleType::World,
            ChatType::Npc(_u) => SpeechBubbleType::None,
            ChatType::NpcSay(_u) => SpeechBubbleType::Say,
            ChatType::NpcTell(_f, _t) => SpeechBubbleType::Say,
            ChatType::Meta => SpeechBubbleType::None,
        }
    }

    pub fn uid(&self) -> Option<Uid> { self.chat_type.uid() }

    pub fn content(&self) -> &Content { &self.content }

    pub fn into_content(self) -> Content { self.content }

    pub fn set_content(&mut self, content: Content) { self.content = content; }
}

/// Player factions are used to coordinate pvp vs hostile factions or segment
/// chat from the world
///
/// Factions are currently just an associated String (the faction's name)
#[derive(Clone, Debug)]
pub struct Faction(pub String);
impl Component for Faction {
    type Storage = DenseVecStorage<Self>;
}
impl From<String> for Faction {
    fn from(s: String) -> Self { Faction(s) }
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
    pub content: Content,
    pub icon: SpeechBubbleType,
    pub timeout: Instant,
}

impl SpeechBubble {
    /// Default duration in seconds of speech bubbles
    pub const DEFAULT_DURATION: f64 = 5.0;

    pub fn new(content: Content, icon: SpeechBubbleType) -> Self {
        let timeout = Instant::now() + Duration::from_secs_f64(SpeechBubble::DEFAULT_DURATION);
        Self {
            content,
            icon,
            timeout,
        }
    }

    pub fn content(&self) -> &Content { &self.content }
}
