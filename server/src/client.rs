use crate::error::Error;
use common::msg::{
    ClientInGameMsg, ClientIngame, ClientMsg, ClientNotInGameMsg, ClientType, PingMsg,
    ServerInGameMsg, ServerInitMsg, ServerMsg, ServerNotInGameMsg,
};
use hashbrown::HashSet;
use network::{MessageBuffer, Participant, Stream};
use serde::{de::DeserializeOwned, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tracing::debug;
use vek::*;

pub struct Client {
    pub registered: bool,
    pub client_type: ClientType,
    pub in_game: Option<ClientIngame>,
    pub participant: Mutex<Option<Participant>>,
    pub singleton_stream: Stream,
    pub ping_stream: Stream,
    pub register_stream: Stream,
    pub in_game_stream: Stream,
    pub not_in_game_stream: Stream,
    pub network_error: AtomicBool,
    pub last_ping: f64,
    pub login_msg_sent: bool,
}

impl Component for Client {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

impl Client {
    fn internal_send<M: Serialize>(b: &AtomicBool, s: &mut Stream, msg: M) {
        if !b.load(Ordering::Relaxed) {
            if let Err(e) = s.send(msg) {
                debug!(?e, "got a network error with client");
                b.store(true, Ordering::Relaxed);
            }
        }
    }

    fn internal_send_raw(b: &AtomicBool, s: &mut Stream, msg: Arc<MessageBuffer>) {
        if !b.load(Ordering::Relaxed) {
            if let Err(e) = s.send_raw(msg) {
                debug!(?e, "got a network error with client");
                b.store(true, Ordering::Relaxed);
            }
        }
    }

    pub fn send_init(&mut self, msg: ServerInitMsg) {
        Self::internal_send(&self.network_error, &mut self.register_stream, msg);
    }

    pub fn send_msg(&mut self, msg: ServerMsg) {
        Self::internal_send(&self.network_error, &mut self.singleton_stream, msg);
    }

    pub fn send_in_game(&mut self, msg: ServerInGameMsg) {
        Self::internal_send(&self.network_error, &mut self.in_game_stream, msg);
    }

    pub fn send_not_in_game(&mut self, msg: ServerNotInGameMsg) {
        Self::internal_send(&self.network_error, &mut self.not_in_game_stream, msg);
    }

    pub fn send_ping(&mut self, msg: PingMsg) {
        Self::internal_send(&self.network_error, &mut self.ping_stream, msg);
    }

    pub fn send_msg_raw(&mut self, msg: Arc<MessageBuffer>) {
        Self::internal_send_raw(&self.network_error, &mut self.singleton_stream, msg);
    }

    pub async fn internal_recv<M: DeserializeOwned>(
        b: &AtomicBool,
        s: &mut Stream,
    ) -> Result<M, Error> {
        if !b.load(Ordering::Relaxed) {
            match s.recv().await {
                Ok(r) => Ok(r),
                Err(e) => {
                    debug!(?e, "got a network error with client while recv");
                    b.store(true, Ordering::Relaxed);
                    Err(Error::StreamErr(e))
                },
            }
        } else {
            Err(Error::StreamErr(network::StreamError::StreamClosed))
        }
    }

    pub async fn recv_msg(&mut self) -> Result<ClientMsg, Error> {
        Self::internal_recv(&self.network_error, &mut self.singleton_stream).await
    }

    pub async fn recv_in_game_msg(&mut self) -> Result<ClientInGameMsg, Error> {
        Self::internal_recv(&self.network_error, &mut self.in_game_stream).await
    }

    pub async fn recv_not_in_game_msg(&mut self) -> Result<ClientNotInGameMsg, Error> {
        Self::internal_recv(&self.network_error, &mut self.not_in_game_stream).await
    }

    pub async fn recv_ping_msg(&mut self) -> Result<PingMsg, Error> {
        Self::internal_recv(&self.network_error, &mut self.ping_stream).await
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
