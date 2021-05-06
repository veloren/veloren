use crate::comp::Pos;
use serde::{Deserialize, Serialize};
use specs::Entity;

/// A resource that stores the time of day.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
pub struct TimeOfDay(pub f64);

/// A resource that stores the tick (i.e: physics) time.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Time(pub f64);

/// A resource that stores the time since the previous tick.
#[derive(Default)]
pub struct DeltaTime(pub f32);

#[derive(Default)]
pub struct EntitiesDiedLastTick(pub Vec<(Entity, Pos)>);

/// A resource that indicates what mode the local game is being played in.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GameMode {
    /// The game is being played in server mode (i.e: the code is running
    /// server-side)
    Server,
    /// The game is being played in client mode (i.e: the code is running
    /// client-side)
    Client,
    /// The game is being played in singleplayer mode (i.e: both client and
    /// server at once)
    // To be used later when we no longer start up an entirely new server for singleplayer
    Singleplayer,
}

/// A resource that stores the player's entity (on the client), and None on the
/// server
#[derive(Copy, Clone, Default, Debug)]
pub struct PlayerEntity(pub Option<Entity>);

#[derive(Copy, Clone, Debug)]
pub struct PlayerPhysicsSetting {
    /// true if the client wants server-authoratative physics (e.g. to use
    /// airships properly)
    pub client_optin: bool,
    /// true if the server is forcing server-authoratative physics (e.g. as
    /// punishment for wallhacking)
    pub server_force: bool,
}

impl Default for PlayerPhysicsSetting {
    fn default() -> PlayerPhysicsSetting {
        PlayerPhysicsSetting {
            client_optin: false,
            server_force: false,
        }
    }
}

impl PlayerPhysicsSetting {
    pub fn server_authoritative(&self) -> bool { self.client_optin || self.server_force }

    pub fn client_authoritative(&self) -> bool { !self.server_authoritative() }
}

/// List of which players are using client-authoratative vs server-authoratative
/// physics, as a stop-gap until we can use server-authoratative physics for
/// everyone
#[derive(Clone, Default, Debug)]
pub struct PlayerPhysicsSettings {
    pub settings: hashbrown::HashMap<uuid::Uuid, PlayerPhysicsSetting>,
}
