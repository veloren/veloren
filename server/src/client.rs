use common_net::msg::{ClientType, ServerGeneral, ServerMsg};
use network::{Message, Participant, Stream, StreamError, StreamParams};
use serde::{de::DeserializeOwned, Serialize};
use specs::Component;
use std::sync::atomic::AtomicBool;

/// Client handles ALL network related information of everything that connects
/// to the server Client DOES NOT handle game states
/// Client DOES NOT handle network information that is only relevant to some
/// "things" connecting to the server (there is currently no such case). First a
/// Client connects to the game, when it registers, it gets the `Player`
/// component, when he enters the game he gets the `InGame` component.
pub struct Client {
    pub client_type: ClientType,
    pub participant: Option<Participant>,
    pub last_ping: f64,
    pub login_msg_sent: AtomicBool,
    pub locale: Option<String>,

    //TODO: Consider splitting each of these out into their own components so all the message
    //processing systems can run in parallel with each other (though it may turn out not to
    //matter that much).
    general_stream: Stream,
    ping_stream: Stream,
    register_stream: Stream,
    character_screen_stream: Stream,
    in_game_stream: Stream,
    terrain_stream: Stream,

    general_stream_params: StreamParams,
    ping_stream_params: StreamParams,
    register_stream_params: StreamParams,
    character_screen_stream_params: StreamParams,
    in_game_stream_params: StreamParams,
    terrain_stream_params: StreamParams,
}

pub struct PreparedMsg {
    stream_id: u8,
    message: Message,
}

impl Component for Client {
    type Storage = specs::DenseVecStorage<Self>;
}

impl Client {
    pub(crate) fn new(
        client_type: ClientType,
        participant: Participant,
        last_ping: f64,
        locale: Option<String>,
        general_stream: Stream,
        ping_stream: Stream,
        register_stream: Stream,
        character_screen_stream: Stream,
        in_game_stream: Stream,
        terrain_stream: Stream,
    ) -> Self {
        let general_stream_params = general_stream.params();
        let ping_stream_params = ping_stream.params();
        let register_stream_params = register_stream.params();
        let character_screen_stream_params = character_screen_stream.params();
        let in_game_stream_params = in_game_stream.params();
        let terrain_stream_params = terrain_stream.params();
        Client {
            client_type,
            participant: Some(participant),
            last_ping,
            locale,
            login_msg_sent: AtomicBool::new(false),
            general_stream,
            ping_stream,
            register_stream,
            character_screen_stream,
            in_game_stream,
            terrain_stream,
            general_stream_params,
            ping_stream_params,
            register_stream_params,
            character_screen_stream_params,
            in_game_stream_params,
            terrain_stream_params,
        }
    }

    pub(crate) fn send<M: Into<ServerMsg>>(&self, msg: M) -> Result<(), StreamError> {
        // TODO: hack to avoid locking stream mutex while serializing the message,
        // remove this when the mutexes on the Streams are removed
        let prepared = self.prepare(msg);
        self.send_prepared(&prepared)
        /*match msg.into() {
            ServerMsg::Info(m) => self.register_stream.lock().unwrap().send(m),
            ServerMsg::Init(m) => self.register_stream.lock().unwrap().send(m),
            ServerMsg::RegisterAnswer(m) => self.register_stream.lock().unwrap().send(m),
            ServerMsg::General(g) => {
                match g {
                    //Character Screen related
                    ServerGeneral::CharacterDataLoadResult(_)
                    | ServerGeneral::CharacterListUpdate(_)
                    | ServerGeneral::CharacterActionError(_)
                    | ServerGeneral::CharacterCreated(_)
                    | ServerGeneral::CharacterEdited(_)
                    | ServerGeneral::CharacterSuccess => {
                        self.character_screen_stream.lock().unwrap().send(g)
                    },
                    //In-game related
                    ServerGeneral::GroupUpdate(_)
                    | ServerGeneral::Invite { .. }
                    | ServerGeneral::InvitePending(_)
                    | ServerGeneral::InviteComplete { .. }
                    | ServerGeneral::ExitInGameSuccess
                    | ServerGeneral::InventoryUpdate(_, _)
                    | ServerGeneral::SetViewDistance(_)
                    | ServerGeneral::SiteEconomy(_)
                    | ServerGeneral::Outcomes(_)
                    | ServerGeneral::Knockback(_)
                    | ServerGeneral::UpdatePendingTrade(_, _, _)
                    | ServerGeneral::FinishedTrade(_)
                    | ServerGeneral::WeatherUpdate(_) => {
                        self.in_game_stream.lock().unwrap().send(g)
                    },
                    //Ingame related, terrain
                    ServerGeneral::TerrainChunkUpdate { .. }
                    | ServerGeneral::LodZoneUpdate { .. }
                    | ServerGeneral::TerrainBlockUpdates(_) => {
                        self.terrain_stream.lock().unwrap().send(g)
                    },
                    // Always possible
                    ServerGeneral::PlayerListUpdate(_)
                    | ServerGeneral::ChatMsg(_)
                    | ServerGeneral::ChatMode(_)
                    | ServerGeneral::SetPlayerEntity(_)
                    | ServerGeneral::TimeOfDay(_, _)
                    | ServerGeneral::EntitySync(_)
                    | ServerGeneral::CompSync(_)
                    | ServerGeneral::CreateEntity(_)
                    | ServerGeneral::DeleteEntity(_)
                    | ServerGeneral::Disconnect(_)
                    | ServerGeneral::Notification(_) => self.general_stream.lock().unwrap().send(g),
                }
            },
            ServerMsg::Ping(m) => self.ping_stream.lock().unwrap().send(m),
        }*/
    }

    /// Like `send` but any errors are explicitly ignored.
    pub(crate) fn send_fallible<M: Into<ServerMsg>>(&self, msg: M) { let _ = self.send(msg); }

    pub(crate) fn send_prepared(&self, msg: &PreparedMsg) -> Result<(), StreamError> {
        match msg.stream_id {
            0 => self.register_stream.send_raw(&msg.message),
            1 => self.character_screen_stream.send_raw(&msg.message),
            2 => self.in_game_stream.send_raw(&msg.message),
            3 => self.general_stream.send_raw(&msg.message),
            4 => self.ping_stream.send_raw(&msg.message),
            5 => self.terrain_stream.send_raw(&msg.message),
            _ => unreachable!("invalid stream id"),
        }
    }

    pub(crate) fn prepare<M: Into<ServerMsg>>(&self, msg: M) -> PreparedMsg {
        match msg.into() {
            ServerMsg::Info(m) => PreparedMsg::new(0, &m, &self.register_stream_params),
            ServerMsg::Init(m) => PreparedMsg::new(0, &m, &self.register_stream_params),
            ServerMsg::RegisterAnswer(m) => PreparedMsg::new(0, &m, &self.register_stream_params),
            ServerMsg::General(g) => {
                match g {
                    // Character Screen related
                    ServerGeneral::CharacterDataLoadResult(_)
                    | ServerGeneral::CharacterListUpdate(_)
                    | ServerGeneral::CharacterActionError(_)
                    | ServerGeneral::CharacterCreated(_)
                    | ServerGeneral::CharacterEdited(_)
                    | ServerGeneral::CharacterSuccess
                    | ServerGeneral::SpectatorSuccess(_) => {
                        PreparedMsg::new(1, &g, &self.character_screen_stream_params)
                    },
                    // In-game related
                    ServerGeneral::GroupUpdate(_)
                    | ServerGeneral::Invite { .. }
                    | ServerGeneral::InvitePending(_)
                    | ServerGeneral::InviteComplete { .. }
                    | ServerGeneral::ExitInGameSuccess
                    | ServerGeneral::InventoryUpdate(_, _)
                    | ServerGeneral::GroupInventoryUpdate(_, _)
                    | ServerGeneral::SetViewDistance(_)
                    | ServerGeneral::Outcomes(_)
                    | ServerGeneral::Knockback(_)
                    | ServerGeneral::SiteEconomy(_)
                    | ServerGeneral::UpdatePendingTrade(_, _, _)
                    | ServerGeneral::FinishedTrade(_)
                    | ServerGeneral::MapMarker(_)
                    | ServerGeneral::WeatherUpdate(_)
                    | ServerGeneral::LocalWindUpdate(_)
                    | ServerGeneral::SpectatePosition(_) => {
                        PreparedMsg::new(2, &g, &self.in_game_stream_params)
                    },
                    // Terrain
                    ServerGeneral::TerrainChunkUpdate { .. }
                    | ServerGeneral::LodZoneUpdate { .. }
                    | ServerGeneral::TerrainBlockUpdates(_) => {
                        PreparedMsg::new(5, &g, &self.terrain_stream_params)
                    },
                    // Always possible
                    ServerGeneral::PlayerListUpdate(_)
                    | ServerGeneral::ChatMsg(_)
                    | ServerGeneral::ChatMode(_)
                    | ServerGeneral::SetPlayerEntity(_)
                    | ServerGeneral::TimeOfDay(_, _, _, _)
                    | ServerGeneral::EntitySync(_)
                    | ServerGeneral::CompSync(_, _)
                    | ServerGeneral::CreateEntity(_)
                    | ServerGeneral::DeleteEntity(_)
                    | ServerGeneral::Disconnect(_)
                    | ServerGeneral::Notification(_) => {
                        PreparedMsg::new(3, &g, &self.general_stream_params)
                    },
                }
            },
            ServerMsg::Ping(m) => PreparedMsg::new(4, &m, &self.ping_stream_params),
        }
    }

    pub(crate) fn terrain_params(&self) -> StreamParams { self.terrain_stream_params.clone() }

    /// Only used for Serialize Chunks in a SlowJob.
    /// TODO: find a more elegant version for this invariant
    pub(crate) fn prepare_chunk_update_msg(
        terrain_chunk_update: ServerGeneral,
        params: &StreamParams,
    ) -> PreparedMsg {
        if !matches!(
            terrain_chunk_update,
            ServerGeneral::TerrainChunkUpdate { .. }
        ) {
            unreachable!("You must not call this function without a terrain chunk update!")
        }
        PreparedMsg::new(5, &terrain_chunk_update, params)
    }

    pub(crate) fn recv<M: DeserializeOwned>(
        &mut self,
        stream_id: u8,
    ) -> Result<Option<M>, StreamError> {
        // TODO: are two systems using the same stream?? why is there contention here?
        match stream_id {
            0 => self.register_stream.try_recv(),
            1 => self.character_screen_stream.try_recv(),
            2 => self.in_game_stream.try_recv(),
            3 => self.general_stream.try_recv(),
            4 => self.ping_stream.try_recv(),
            5 => self.terrain_stream.try_recv(),
            _ => unreachable!("invalid stream id"),
        }
    }
}

impl PreparedMsg {
    fn new<M: Serialize + ?Sized>(id: u8, msg: &M, stream_params: &StreamParams) -> PreparedMsg {
        Self {
            stream_id: id,
            message: Message::serialize(&msg, stream_params.clone()),
        }
    }
}
