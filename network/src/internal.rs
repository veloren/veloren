use crate::{
    api::Address,
    worker::types::{Mid, Sid},
};

pub(crate) const VELOREN_MAGIC_NUMBER: &str = "VELOREN";
pub const VELOREN_NETWORK_VERSION: [u32; 3] = [0, 1, 0];

pub(crate) enum Protocol {
    Tcp,
    Udp,
}

impl Address {
    pub(crate) fn get_protocol(&self) -> Protocol {
        match self {
            Address::Tcp(_) => Protocol::Tcp,
            Address::Udp(_) => Protocol::Udp,
        }
    }
}

#[derive(Debug)]
pub struct RemoteParticipant {
    pub stream_id_pool: tlid::Pool<tlid::Wrapping<Sid>>,
    pub msg_id_pool: tlid::Pool<tlid::Wrapping<Mid>>,
}

impl RemoteParticipant {
    pub(crate) fn new() -> Self {
        Self {
            stream_id_pool: tlid::Pool::new_full(),
            msg_id_pool: tlid::Pool::new_full(),
        }
    }
}
