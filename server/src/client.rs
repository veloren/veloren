use crate::error::Error;
use common::msg::{ClientMsg, ClientState, RequestStateError, ServerMsg};
use hashbrown::HashSet;
use network::{Participant, Stream};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tracing::debug;
use vek::*;

pub struct Client {
    pub client_state: ClientState,
    pub participant: Mutex<Option<Participant>>,
    pub singleton_stream: Stream,
    pub network_error: AtomicBool,
    pub last_ping: f64,
    pub login_msg_sent: bool,
}

impl Component for Client {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

impl Client {
    pub fn notify(&mut self, msg: ServerMsg) {
        if !self.network_error.load(Ordering::Relaxed) {
            if let Err(e) = self.singleton_stream.send(msg) {
                debug!(?e, "got a network error with client");
                self.network_error.store(true, Ordering::Relaxed);
            }
        }
    }

    pub async fn recv(&mut self) -> Result<ClientMsg, Error> {
        if !self.network_error.load(Ordering::Relaxed) {
            match self.singleton_stream.recv().await {
                Ok(r) => Ok(r),
                Err(e) => {
                    debug!(?e, "got a network error with client while recv");
                    self.network_error.store(true, Ordering::Relaxed);
                    Err(Error::StreamErr(e))
                },
            }
        } else {
            Err(Error::StreamErr(network::StreamError::StreamClosed))
        }
    }

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
        let _ = self.notify(ServerMsg::StateAnswer(Err((error, self.client_state))));
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
