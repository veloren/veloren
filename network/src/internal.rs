use crate::{
    api::{Address, Promise},
    message::{InCommingMessage, OutGoingMessage},
};
use enumset::*;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, time::Instant};

pub(crate) const VELOREN_MAGIC_NUMBER: &str = "VELOREN";
pub const VELOREN_NETWORK_VERSION: [u32; 3] = [0, 1, 0];

pub(crate) trait Channel {
    /*
        uninitialized_dirty_speed_buffer: is just a already allocated buffer, that probably is already dirty because it's getting reused to save allocations, feel free to use it, but expect nothing
        aprox_time is the time taken when the events come in, you can reuse it for message timeouts, to not make any more syscalls
    */
    /// Execute when ready to read
    fn read(&mut self, uninitialized_dirty_speed_buffer: &mut [u8; 65000], aprox_time: Instant);
    /// Execute when ready to write
    fn write(&mut self, uninitialized_dirty_speed_buffer: &mut [u8; 65000], aprox_time: Instant);
    fn open_stream(&mut self, prio: u8, promises: EnumSet<Promise>) -> u32;
    fn close_stream(&mut self, sid: u32);
    fn handshake(&mut self);
    fn participant_id(&mut self, pid: uuid::Uuid);
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum Frame {
    Handshake {
        magic_number: String,
        version: [u32; 3],
    },
    ParticipantId {
        pid: uuid::Uuid,
    },
    OpenStream {
        sid: u32,
        prio: u8,
        promises: EnumSet<Promise>,
    },
    CloseStream {
        sid: u32,
    },
    DataHeader {
        id: u64,
        length: u64,
    },
    Data {
        id: u64,
        frame_no: u64,
        data: Vec<u8>,
    },
}

pub(crate) type TcpFrame = Frame;

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
pub(crate) struct Stream {
    sid: u32,
    prio: u8,
    promises: EnumSet<Promise>,
    to_send: VecDeque<OutGoingMessage>,
    to_receive: VecDeque<InCommingMessage>,
}

impl Stream {
    pub fn new(sid: u32, prio: u8, promises: EnumSet<Promise>) -> Self {
        Stream {
            sid,
            prio,
            promises,
            to_send: VecDeque::new(),
            to_receive: VecDeque::new(),
        }
    }

    pub fn sid(&self) -> u32 { self.sid }

    pub fn prio(&self) -> u8 { self.prio }

    pub fn promises(&self) -> EnumSet<Promise> { self.promises }
}
