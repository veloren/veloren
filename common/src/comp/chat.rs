use specs::{Component, Entity};
use specs_idvs::IDVStorage;

/// Limit chat to a subset of players
pub enum ChatMode {
    /// Private message to another player (by entity)
    Tell(Entity),
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
