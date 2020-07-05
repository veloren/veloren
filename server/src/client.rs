use common::msg::{ClientState, RequestStateError, ServerMsg};
use hashbrown::HashSet;
use network::{Participant, Stream};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::sync::{Arc, Mutex};
use vek::*;

pub struct Client {
    pub client_state: ClientState,
    pub participant: Mutex<Option<Arc<Participant>>>,
    pub singleton_stream: Stream,
    pub last_ping: f64,
    pub login_msg_sent: bool,
}

impl Component for Client {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

impl Client {
    pub fn notify(&mut self, msg: ServerMsg) { let _ = self.singleton_stream.send(msg); }

    pub fn is_registered(&self) -> bool {
        match self.client_state {
            ClientState::Registered | ClientState::Spectator | ClientState::Character => true,
            _ => false,
        }
    }

    pub fn is_ingame(&self) -> bool {
        match self.client_state {
            ClientState::Spectator | ClientState::Character => true,
            _ => false,
        }
    }

    pub fn allow_state(&mut self, new_state: ClientState) {
        self.client_state = new_state;
        let _ = self
            .singleton_stream
            .send(ServerMsg::StateAnswer(Ok(new_state)));
    }

    pub fn error_state(&mut self, error: RequestStateError) {
        let _ = self
            .singleton_stream
            .send(ServerMsg::StateAnswer(Err((error, self.client_state))));
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
