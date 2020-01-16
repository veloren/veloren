use common::{
    msg::{ClientMsg, ClientState, RequestStateError, ServerMsg},
    net::PostBox,
};
use hashbrown::HashSet;
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use vek::*;

pub struct Client {
    pub client_state: ClientState,
    pub postbox: PostBox<ServerMsg, ClientMsg>,
    pub last_ping: f64,
    pub login_msg_sent: bool,
}

impl Component for Client {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

impl Client {
    pub fn notify(&mut self, msg: ServerMsg) {
        self.postbox.send_message(msg);
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
        self.postbox
            .send_message(ServerMsg::StateAnswer(Ok(new_state)));
    }
    pub fn error_state(&mut self, error: RequestStateError) {
        self.postbox
            .send_message(ServerMsg::StateAnswer(Err((error, self.client_state))));
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
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
