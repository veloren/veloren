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

pub mod events {
    use super::Event;
    use serde::{Serialize,Deserialize};

    #[derive(Serialize, Deserialize, Debug)]
    pub struct PlayerJoinEvent {
        pub player_name: String,
        pub player_id: usize
    }

    impl Event for PlayerJoinEvent {
        type Response = PlayerJoinResult;
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub enum PlayerJoinResult {
        CloseConnection,
        None
    }

    impl Default for PlayerJoinResult {
        fn default() -> Self {
            Self::None
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
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
