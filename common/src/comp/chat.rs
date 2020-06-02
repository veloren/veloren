use crate::sync::Uid;
use specs::Component;
use specs_idvs::IDVStorage;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMsg {
    pub chat_type: ChatType,
    pub message: String,
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
