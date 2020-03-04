use crate::{
    api::{Address, Promise},
    channel::Channel,
    message::{InCommingMessage, OutGoingMessage},
};
use enumset::EnumSet;
use futures;
use mio::{self, net::TcpListener, PollOpt, Ready};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::mpsc};
use uuid::Uuid;

//Participant Ids are randomly chosen
pub type Pid = Uuid;
//Stream Ids are unique per Participant* and are split in 2 ranges, one for
// every Network involved Every Channel gets a subrange during their handshake
// protocol from one of the 2 ranges
//*otherwise extra synchronization would be needed
pub type Sid = u32;
//Message Ids are unique per Stream* and are split in 2 ranges, one for every
// Channel involved
//*otherwise extra synchronization would be needed
pub type Mid = u64;

pub(crate) const VELOREN_MAGIC_NUMBER: &str = "VELOREN";
pub const VELOREN_NETWORK_VERSION: [u32; 3] = [0, 1, 0];

// Used for Communication between Controller <--> Worker
pub(crate) enum CtrlMsg {
    Shutdown,
    Register(TokenObjects, Ready, PollOpt),
    OpenStream {
        pid: Pid,
        prio: u8,
        promises: EnumSet<Promise>,
        msg_tx: futures::channel::mpsc::UnboundedSender<InCommingMessage>,
        return_sid: mpsc::Sender<Sid>,
    },
    CloseStream {
        pid: Pid,
        sid: Sid,
    },
    Send(OutGoingMessage),
}

pub(crate) enum RtrnMsg {
    Shutdown,
    ConnectedParticipant {
        pid: Pid,
    },
    OpendStream {
        pid: Pid,
        sid: Sid,
        prio: u8,
        msg_rx: futures::channel::mpsc::UnboundedReceiver<InCommingMessage>,
        promises: EnumSet<Promise>,
    },
    ClosedStream {
        pid: Pid,
        sid: Sid,
    },
}

#[derive(Debug)]
pub(crate) enum TokenObjects {
    TcpListener(TcpListener),
    Channel(Channel),
}

#[derive(Debug)]
pub(crate) struct IntStream {
    sid: Sid,
    prio: u8,
    promises: EnumSet<Promise>,
    msg_tx: futures::channel::mpsc::UnboundedSender<InCommingMessage>,
    pub to_send: VecDeque<OutGoingMessage>,
    pub to_receive: VecDeque<InCommingMessage>,
}

impl IntStream {
    pub fn new(
        sid: Sid,
        prio: u8,
        promises: EnumSet<Promise>,
        msg_tx: futures::channel::mpsc::UnboundedSender<InCommingMessage>,
    ) -> Self {
        IntStream {
            sid,
            prio,
            promises,
            msg_tx,
            to_send: VecDeque::new(),
            to_receive: VecDeque::new(),
        }
    }

    pub fn sid(&self) -> Sid { self.sid }

    pub fn prio(&self) -> u8 { self.prio }

    pub fn msg_tx(&self) -> futures::channel::mpsc::UnboundedSender<InCommingMessage> {
        self.msg_tx.clone()
    }

    pub fn promises(&self) -> EnumSet<Promise> { self.promises }
}

// Used for Communication between Channel <----(TCP/UDP)----> Channel
#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum Frame {
    Handshake {
        magic_number: String,
        version: [u32; 3],
    },
    Configure {
        //only one Participant will send this package and give the other a range to use
        stream_id_pool: tlid::Pool<tlid::Wrapping<Sid>>,
        msg_id_pool: tlid::Pool<tlid::Wrapping<Mid>>,
    },
    ParticipantId {
        pid: Pid,
    },
    Shutdown {/* Shutsdown this channel gracefully, if all channels are shut down, Participant is deleted */},
    OpenStream {
        sid: Sid,
        prio: u8,
        promises: EnumSet<Promise>,
    },
    CloseStream {
        sid: Sid,
    },
    DataHeader {
        mid: Mid,
        sid: Sid,
        length: u64,
    },
    Data {
        id: Mid,
        start: u64,
        data: Vec<u8>,
    },
    /* WARNING: Sending RAW is only used for debug purposes in case someone write a new API
     * against veloren Server! */
    Raw(Vec<u8>),
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
