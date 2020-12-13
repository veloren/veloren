use serde::{Serialize, de::DeserializeOwned, Deserialize};
use common::{sync, resources};

#[derive(Deserialize,Serialize,Debug)]
pub enum Action {
    ServerClose,
    Print(String),
    PlayerSendMessage(usize,String),
    KillEntity(usize)
}

pub trait Event: Serialize + DeserializeOwned + Send + Sync{
    type Response: Serialize + DeserializeOwned + Send + Sync;
}

pub use resources::GameMode;

pub mod event {
    use super::*;
    use serde::{Serialize,Deserialize};

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
        pub id: sync::Uid,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub struct PlayerJoinEvent {
        pub player_name: String,
        pub player_id: usize
    }

    impl Event for PlayerJoinEvent {
        type Response = PlayerJoinResult;
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub enum PlayerJoinResult {
        CloseConnection,
        None
    }

    impl Default for PlayerJoinResult {
        fn default() -> Self {
            Self::None
        }
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
