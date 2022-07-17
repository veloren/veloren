pub extern crate common;

pub use common::comp::Health;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use common::{resources::GameMode, uid::Uid};

mod errors;

pub use errors::*;
pub use event::*;

/// The [`Action`] enum represents a push modification that will be made in the
/// ECS in the next tick Note that all actions when sent are async and will not
/// be executed in order like [`Retrieve`] that are sync. All actions sent will
/// be executed in the send order in the ticking before the rest of the logic
/// applies.
///
/// # Usage:
/// ```rust
/// # use veloren_plugin_api::*;
/// # pub fn emit_action(action: Action) { emit_actions(vec![action]) }
/// # pub fn emit_actions(_actions: Vec<Action>) {}
/// // Packing actions is better than sending multiple ones at the same time!
/// emit_actions(vec![
///     Action::KillEntity(Uid(1)),
///     Action::PlayerSendMessage(Uid(0), "This is a test message".to_owned()),
/// ]);
/// // You can also use this to only send one action
/// emit_action(Action::KillEntity(Uid(1)));
/// ```
#[derive(Deserialize, Serialize, Debug)]
pub enum Action {
    ServerClose,
    Print(String),
    PlayerSendMessage(Uid, String),
    KillEntity(Uid),
}

/// The [`Retrieve`] enum represents read of the ECS is sync and blocking.
/// This enum shouldn't be used by itself. You should always prefer `get`
/// methods on Plugin API Types For instance, prefer this method:
/// ```rust
/// # use veloren_plugin_api::*;
/// # let entityid = Player {id: Uid(0)};
/// # trait G { fn get_entity_health(&self) -> Option<i64>; }
/// # impl G for Player {fn get_entity_health(&self) -> Option<i64> {Some(1)}}
/// let life = entityid.get_entity_health().unwrap();
/// // Do something with life
/// ```
/// Over this one:
/// ```rust
/// # use common::comp::Body;
/// # use common::comp::body::humanoid;
/// # use veloren_plugin_api::*;
/// # let entityid = Uid(0);
/// # fn retrieve_action(r: &Retrieve) -> Result<RetrieveResult, RetrieveError> { Ok(RetrieveResult::GetEntityHealth(Health::new(Body::Humanoid(humanoid::Body::random()), 1))) }
/// let life = if let RetrieveResult::GetEntityHealth(e) =
///     retrieve_action(&Retrieve::GetEntityHealth(entityid)).unwrap()
/// {
///     e
/// } else {
///      unreachable!()
/// };
/// // Do something with life
/// ```
#[derive(Deserialize, Serialize, Debug)]
pub enum Retrieve {
    GetPlayerName(Uid),
    GetEntityHealth(Uid),
}

/// The [`RetrieveResult`] struct is generated while using the `retrieve_action`
/// function
///
/// You should always prefer using `get` methods available in Plugin API types.
///
/// Example:
/// ```rust
/// # use common::comp::Body;
/// # use common::comp::body::humanoid;
/// # use veloren_plugin_api::*;
/// # let entityid = Uid(0);
/// # fn retrieve_action(r: &Retrieve) -> Result<RetrieveResult, RetrieveError> { Ok(RetrieveResult::GetEntityHealth(Health::new(Body::Humanoid(humanoid::Body::random()), 1)))}
/// let life = if let RetrieveResult::GetEntityHealth(e) =
///     retrieve_action(&Retrieve::GetEntityHealth(entityid)).unwrap()
/// {
///     e
/// } else {
///      unreachable!()
/// };
/// // Do something with life
/// ```
#[derive(Serialize, Deserialize, Debug)]
pub enum RetrieveResult {
    GetPlayerName(String),
    GetEntityHealth(Health),
}

/// This trait is implement by all events and ensure type safety of FFI.
pub trait Event: Serialize + DeserializeOwned + Send + Sync {
    type Response: Serialize + DeserializeOwned + Send + Sync;

    fn get_event_name(&self) -> String;
}

/// This module contains all events from the api
pub mod event {
    use super::*;
    use serde::{Deserialize, Serialize};

    /// This event is called when a chat command is run.
    /// Your event should be named `on_command_<Your command>`
    ///
    /// If you return an Error the displayed message will be the error message
    /// in red You can return a Vec<String> that will be print to player
    /// chat as info
    ///
    /// # Example
    /// ```ignore
    /// #[event_handler]
    /// pub fn on_command_testplugin(command: ChatCommandEvent) -> Result<Vec<String>, String> {
    ///     Ok(vec![format!(
    ///         "Player of id {:?} named {} with {:?} sent command with args {:?}",
    ///         command.player.id,
    ///         command
    ///             .player
    ///             .get_player_name()
    ///             .expect("Can't get player name"),
    ///         command
    ///             .player
    ///             .get_entity_health()
    ///             .expect("Can't get player health"),
    ///         command.command_args
    ///     )])
    /// }
    /// ```
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

    /// This struct represent a player
    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub struct Player {
        pub id: Uid,
    }

    /// This event is called when a player connects.
    /// Your event should be named `on_join`
    ///
    /// You can either return `CloseConnection` or `None`
    /// If `CloseConnection` is returned the player will be kicked
    ///
    /// # Example
    /// ```ignore
    /// #[event_handler]
    /// pub fn on_join(command: PlayerJoinEvent) -> PlayerJoinResult {
    ///     PlayerJoinResult::CloseConnection
    /// }
    /// ```
    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub struct PlayerJoinEvent {
        pub player_name: String,
        pub player_id: [u8; 16],
    }

    impl Event for PlayerJoinEvent {
        type Response = PlayerJoinResult;

        fn get_event_name(&self) -> String { "on_join".to_owned() }
    }

    /// This is the return type of an `on_join` event. See [`PlayerJoinEvent`]
    ///
    /// Variants:
    ///  - `CloseConnection` will kick the player.
    ///  - `None` will let the player join the server.
    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    #[repr(u8)]
    pub enum PlayerJoinResult {
        Kick(String),
        None,
    }

    impl Default for PlayerJoinResult {
        fn default() -> Self { Self::None }
    }

    /// This event is called when the plugin is loaded
    /// Your event should be named `on_load`
    ///
    /// # Example
    /// ```ignore
    /// #[event_handler]
    /// pub fn on_load(load: PluginLoadEvent) {
    ///     match load.game_mode {
    ///         GameMode::Server => emit_action(Action::Print("Hello, server!".to_owned())),
    ///         GameMode::Client => emit_action(Action::Print("Hello, client!".to_owned())),
    ///         GameMode::Singleplayer => emit_action(Action::Print("Hello, singleplayer!".to_owned())),
    ///     }
    /// }
    /// ```
    #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
    pub struct PluginLoadEvent {
        pub game_mode: GameMode,
    }

    impl Event for PluginLoadEvent {
        type Response = ();

        fn get_event_name(&self) -> String { "on_load".to_owned() }
    }

    // impl Default for PlayerJoinResult {
    //     fn default() -> Self {
    //         Self::None
    //     }
    // }
}
