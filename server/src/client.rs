use crate::error::Error;
use common::msg::{ClientInGame, ClientType, ServerGeneral, ServerMsg};
use hashbrown::HashSet;
use network::{Participant, Stream};
use serde::{de::DeserializeOwned, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use tracing::debug;
use vek::*;

pub struct Client {
    pub registered: bool,
    pub client_type: ClientType,
    pub in_game: Option<ClientInGame>,
    pub participant: Option<Participant>,
    pub general_stream: Stream,
    pub ping_stream: Stream,
    pub register_stream: Stream,
    pub character_screen_stream: Stream,
    pub in_game_stream: Stream,
    pub network_error: bool,
    pub last_ping: f64,
    pub login_msg_sent: bool,
}

impl Component for Client {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

impl Client {
    fn internal_send<M: Serialize>(err: &mut bool, s: &mut Stream, msg: M) {
        if !*err {
            if let Err(e) = s.send(msg) {
                debug!(?e, "got a network error with client");
                *err = true;
            }
        }
    }

    /*
    fn internal_send_raw(b: &AtomicBool, s: &mut Stream, msg: Arc<MessageBuffer>) {
        if !b.load(Ordering::Relaxed) {
            if let Err(e) = s.send_raw(msg) {
                debug!(?e, "got a network error with client");
                b.store(true, Ordering::Relaxed);
            }
        }
    }
     */

    pub fn send_msg<S>(&mut self, msg: S)
    where
        S: Into<ServerMsg>,
    {
        const ERR: &str =
            "Don't do that. Sending these messages is only done ONCE at connect and not by this fn";
        match msg.into() {
            ServerMsg::Info(_) => panic!(ERR),
            ServerMsg::Init(_) => panic!(ERR),
            ServerMsg::RegisterAnswer(msg) => {
                Self::internal_send(&mut self.network_error, &mut self.register_stream, &msg)
            },
            ServerMsg::General(msg) => {
                let stream = match &msg {
                    //Character Screen related
                    ServerGeneral::CharacterDataLoadError(_)
                    | ServerGeneral::CharacterListUpdate(_)
                    | ServerGeneral::CharacterActionError(_)
                    | ServerGeneral::CharacterSuccess => &mut self.character_screen_stream,
                    //Ingame related
                    ServerGeneral::GroupUpdate(_)
                    | ServerGeneral::GroupInvite { .. }
                    | ServerGeneral::InvitePending(_)
                    | ServerGeneral::InviteComplete { .. }
                    | ServerGeneral::ExitInGameSuccess
                    | ServerGeneral::InventoryUpdate(_, _)
                    | ServerGeneral::TerrainChunkUpdate { .. }
                    | ServerGeneral::TerrainBlockUpdates(_)
                    | ServerGeneral::SetViewDistance(_)
                    | ServerGeneral::Outcomes(_)
                    | ServerGeneral::Knockback(_) => &mut self.in_game_stream,
                    // Always possible
                    ServerGeneral::PlayerListUpdate(_)
                    | ServerGeneral::ChatMsg(_)
                    | ServerGeneral::SetPlayerEntity(_)
                    | ServerGeneral::TimeOfDay(_)
                    | ServerGeneral::EntitySync(_)
                    | ServerGeneral::CompSync(_)
                    | ServerGeneral::CreateEntity(_)
                    | ServerGeneral::DeleteEntity(_)
                    | ServerGeneral::Disconnect(_)
                    | ServerGeneral::Notification(_) => &mut self.general_stream,
                };
                Self::internal_send(&mut self.network_error, stream, &msg)
            },
            ServerMsg::Ping(msg) => {
                Self::internal_send(&mut self.network_error, &mut self.ping_stream, &msg)
            },
        };
    }

    pub async fn internal_recv<M: DeserializeOwned>(
        err: &mut bool,
        s: &mut Stream,
    ) -> Result<M, Error> {
        if !*err {
            match s.recv().await {
                Ok(r) => Ok(r),
                Err(e) => {
                    debug!(?e, "got a network error with client while recv");
                    *err = true;
                    Err(Error::StreamErr(e))
                },
            }
        } else {
            Err(Error::StreamErr(network::StreamError::StreamClosed))
        }
    }
}

// Distance from fuzzy_chunk before snapping to current chunk
pub const CHUNK_FUZZ: u32 = 2;
// Distance out of the range of a region before removing it from subscriptions
pub const REGION_FUZZ: u32 = 16;

#[derive(Clone, Debug)]
pub struct RegionSubscription {
    pub fuzzy_chunk: Vec2<i32>,
    pub regions: HashSet<Vec2<i32>>,
}

impl Component for RegionSubscription {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
