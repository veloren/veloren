pub extern crate common;

pub use common::comp::Health;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use common::{resources::GameMode, uid::Uid};

mod errors;

pub use errors::*;

#[derive(Deserialize, Serialize, Debug)]
pub enum Action {
    ServerClose,
    Print(String),
    PlayerSendMessage(Uid, String),
    KillEntity(Uid),
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Retrieve {
    GetPlayerName(Uid),
    GetEntityHealth(Uid),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RetrieveResult {
    GetPlayerName(String),
    GetEntityHealth(Health),
}

pub trait Event: Serialize + DeserializeOwned + Send + Sync {
    type Response: Serialize + DeserializeOwned + Send + Sync;
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
