use serde::{Serialize, de::DeserializeOwned, Deserialize};

#[derive(Deserialize,Serialize,Debug)]
pub enum Action {
    ServerClose,
    Print(String),
    PlayerSendMessage(usize,String),
    KillEntity(usize)
}

pub trait Event: Serialize + DeserializeOwned{
    type Response: Serialize + DeserializeOwned;
}

// TODO: Unify this with common/src/comp/uid.rs:Uid
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Uid(pub u64);

pub mod events {

    use crate::Uid;

    use super::Event;
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
        pub id: Uid,
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
    pub struct PluginLoadEvent;

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
