use crate::api::Address;

pub(crate) trait Channel {
    fn get_preferred_queue_size() -> usize;
    fn get_preferred_buffer_len() -> usize;
    fn queue(&self, msg: Vec<u8>);
    fn recv(&self) -> Option<Vec<u8>>;
}

#[derive(Debug)]
pub(crate) enum TcpFrame {
    Header {
        id: u64,
        length: u64,
    },
    Data {
        id: u64,
        frame_no: u64,
        data: Vec<u8>,
    },
}

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
