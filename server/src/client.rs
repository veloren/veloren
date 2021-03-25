use common_net::msg::{ClientType, ServerGeneral, ServerMsg};
use network::{Message, Participant, Stream, StreamError, StreamParams};
use serde::{de::DeserializeOwned, Serialize};
use specs::Component;
use specs_idvs::IdvStorage;
use std::sync::{atomic::AtomicBool, Mutex};

/// Client handles ALL network related information of everything that connects
/// to the server Client DOES NOT handle game states
/// Client DOES NOT handle network information that is only relevant to some
/// "things" connecting to the server (there is currently no such case). First a
/// Client connects to the game, when it registers, it gets the `Player`
/// component, when he enters the game he gets the `InGame` component.
pub struct Client {
    pub client_type: ClientType,
    pub participant: Option<Participant>,
    pub last_ping: Mutex<f64>,
    pub login_msg_sent: AtomicBool,
    pub terminate_msg_recv: AtomicBool,

    //TODO: improve network crate so that `send` is no longer `&mut self` and we can get rid of
    // this Mutex. This Mutex is just to please the compiler as we do not get into contention
    general_stream: Mutex<Stream>,
    ping_stream: Mutex<Stream>,
    register_stream: Mutex<Stream>,
    character_screen_stream: Mutex<Stream>,
    in_game_stream: Mutex<Stream>,
    terrain_stream: Mutex<Stream>,

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
    type Storage = IdvStorage<Self>;
}

impl Client {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        client_type: ClientType,
        participant: Participant,
        last_ping: f64,
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
            last_ping: Mutex::new(last_ping),
            login_msg_sent: AtomicBool::new(false),
            terminate_msg_recv: AtomicBool::new(false),
            general_stream: Mutex::new(general_stream),
            ping_stream: Mutex::new(ping_stream),
            register_stream: Mutex::new(register_stream),
            character_screen_stream: Mutex::new(character_screen_stream),
            in_game_stream: Mutex::new(in_game_stream),
            terrain_stream: Mutex::new(terrain_stream),
            general_stream_params,
            ping_stream_params,
            register_stream_params,
            character_screen_stream_params,
            in_game_stream_params,
            terrain_stream_params,
        }
    }

    pub(crate) fn send<M: Into<ServerMsg>>(&self, msg: M) -> Result<(), StreamError> {
        match msg.into() {
            ServerMsg::Info(m) => self.register_stream.lock().unwrap().send(m),
            ServerMsg::Init(m) => self.register_stream.lock().unwrap().send(m),
            ServerMsg::RegisterAnswer(m) => self.register_stream.lock().unwrap().send(m),
            ServerMsg::General(g) => {
                match g {
                    //Character Screen related
                    ServerGeneral::CharacterDataLoadError(_)
                    | ServerGeneral::CharacterListUpdate(_)
                    | ServerGeneral::CharacterActionError(_)
                    | ServerGeneral::CharacterCreated(_)
                    | ServerGeneral::CharacterSuccess => {
                        self.character_screen_stream.lock().unwrap().send(g)
                    },
                    //Ingame related
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
                    | ServerGeneral::FinishedTrade(_) => {
                        self.in_game_stream.lock().unwrap().send(g)
                    },
                    //Ingame related, terrain
                    ServerGeneral::TerrainChunkUpdate { .. }
                    | ServerGeneral::TerrainBlockUpdates(_) => {
                        self.terrain_stream.lock().unwrap().send(g)
                    },
                    // Always possible
                    ServerGeneral::PlayerListUpdate(_)
                    | ServerGeneral::ChatMsg(_)
                    | ServerGeneral::ChatMode(_)
                    | ServerGeneral::SetPlayerEntity(_)
                    | ServerGeneral::TimeOfDay(_)
                    | ServerGeneral::EntitySync(_)
                    | ServerGeneral::CompSync(_)
                    | ServerGeneral::CreateEntity(_)
                    | ServerGeneral::DeleteEntity(_)
                    | ServerGeneral::Disconnect(_)
                    | ServerGeneral::Notification(_) => self.general_stream.lock().unwrap().send(g),
                }
            },
            ServerMsg::Ping(m) => self.ping_stream.lock().unwrap().send(m),
        }
    }

    pub(crate) fn send_fallible<M: Into<ServerMsg>>(&self, msg: M) { let _ = self.send(msg); }

    pub(crate) fn send_prepared(&self, msg: &PreparedMsg) -> Result<(), StreamError> {
        match msg.stream_id {
            0 => self.register_stream.lock().unwrap().send_raw(&msg.message),
            1 => self
                .character_screen_stream
                .lock()
                .unwrap()
                .send_raw(&msg.message),
            2 => self.in_game_stream.lock().unwrap().send_raw(&msg.message),
            3 => self.general_stream.lock().unwrap().send_raw(&msg.message),
            4 => self.ping_stream.lock().unwrap().send_raw(&msg.message),
            5 => self.terrain_stream.lock().unwrap().send_raw(&msg.message),
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
                    //Character Screen related
                    ServerGeneral::CharacterDataLoadError(_)
                    | ServerGeneral::CharacterListUpdate(_)
                    | ServerGeneral::CharacterActionError(_)
                    | ServerGeneral::CharacterCreated(_)
                    | ServerGeneral::CharacterSuccess => {
                        PreparedMsg::new(1, &g, &self.character_screen_stream_params)
                    },
                    //Ingame related
                    ServerGeneral::GroupUpdate(_)
                    | ServerGeneral::Invite { .. }
                    | ServerGeneral::InvitePending(_)
                    | ServerGeneral::InviteComplete { .. }
                    | ServerGeneral::ExitInGameSuccess
                    | ServerGeneral::InventoryUpdate(_, _)
                    | ServerGeneral::SetViewDistance(_)
                    | ServerGeneral::Outcomes(_)
                    | ServerGeneral::Knockback(_)
                    | ServerGeneral::SiteEconomy(_)
                    | ServerGeneral::UpdatePendingTrade(_, _, _)
                    | ServerGeneral::FinishedTrade(_) => {
                        PreparedMsg::new(2, &g, &self.in_game_stream_params)
                    },
                    //Ingame related, terrain
                    ServerGeneral::TerrainChunkUpdate { .. }
                    | ServerGeneral::TerrainBlockUpdates(_) => {
                        PreparedMsg::new(5, &g, &self.terrain_stream_params)
                    },
                    // Always possible
                    ServerGeneral::PlayerListUpdate(_)
                    | ServerGeneral::ChatMsg(_)
                    | ServerGeneral::ChatMode(_)
                    | ServerGeneral::SetPlayerEntity(_)
                    | ServerGeneral::TimeOfDay(_)
                    | ServerGeneral::EntitySync(_)
                    | ServerGeneral::CompSync(_)
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

    pub(crate) fn recv<M: DeserializeOwned>(
        &self,
        stream_id: u8,
    ) -> Result<Option<M>, StreamError> {
        // TODO: are two systems using the same stream?? why is there contention here?
        match stream_id {
            0 => self.register_stream.lock().unwrap().try_recv(),
            1 => self.character_screen_stream.lock().unwrap().try_recv(),
            2 => self.in_game_stream.lock().unwrap().try_recv(),
            3 => self.general_stream.lock().unwrap().try_recv(),
            4 => self.ping_stream.lock().unwrap().try_recv(),
            5 => self.terrain_stream.lock().unwrap().try_recv(),
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
