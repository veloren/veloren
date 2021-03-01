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

    fn get_event_name(&self) -> String;
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

        fn get_event_name(&self) -> String { format!("on_command_{}", self.command) }
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub struct Player {
        pub id: Uid,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub struct PlayerJoinEvent {
        pub player_name: String,
        pub player_id: [u8; 16],
    }

    impl Event for PlayerJoinEvent {
        type Response = PlayerJoinResult;

        fn get_event_name(&self) -> String { "on_join".to_owned() }
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    #[repr(u8)]
    pub enum PlayerJoinResult {
        Kick(String),
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

        fn get_event_name(&self) -> String { "on_load".to_owned() }
    }

    // #[derive(Serialize, Deserialize, Debug)]
    // pub struct EmptyResult;

    // impl Default for PlayerJoinResult {
    //     fn default() -> Self {
    //         Self::None
    //     }
    // }
}
