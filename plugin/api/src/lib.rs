
use std::fmt;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub enum Action {
    ServerClose,
    Print(String),
    PlayerSendMessage(Uid, String),
    KillEntity(Uid),
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Retreive {
    GetEntityName(Uid),
}

pub trait Event: Serialize + DeserializeOwned + Send + Sync {
    type Response: Serialize + DeserializeOwned + Send + Sync;
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Uid(pub u64);

impl Into<u64> for Uid {
    fn into(self) -> u64 { self.0 }
}

impl From<u64> for Uid {
    fn from(uid: u64) -> Self { Self(uid) }
}

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}

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

pub mod event {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub struct ChatCommandEvent {
        pub command: String,
        pub command_args: Vec<String>,
        pub player: Player,
    }

    impl Event for ChatCommandEvent {
        type Response = Result<Vec<String>, String>;
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub struct Player {
        pub id: Uid,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub struct PlayerJoinEvent {
        pub player_name: String,
        pub player_id: Uid,
    }

    impl Event for PlayerJoinEvent {
        type Response = PlayerJoinResult;
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub enum PlayerJoinResult {
        CloseConnection,
        None,
    }

    impl Default for PlayerJoinResult {
        fn default() -> Self { Self::None }
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub struct PluginLoadEvent {
        pub game_mode: GameMode,
    }

    impl Event for PluginLoadEvent {
        type Response = ();
    }

    // #[derive(Serialize, Deserialize, Debug)]
    // pub struct EmptyResult;

    // impl Default for PlayerJoinResult {
    //     fn default() -> Self {
    //         Self::None
    //     }
    // }
}
