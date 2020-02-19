use crate::{
    api::Promise,
    message::{InCommingMessage, OutGoingMessage},
    worker::{Channel, MpscChannel, TcpChannel, UdpChannel},
};
use enumset::EnumSet;
use mio::{self, net::TcpListener, PollOpt, Ready};
use mio_extras::channel::Sender;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

pub type Pid = Uuid;
pub type Sid = u32;
pub type Mid = u64;

// Used for Communication between Controller <--> Worker
pub(crate) enum CtrlMsg {
    Shutdown,
    Register(TokenObjects, Ready, PollOpt),
    OpenStream {
        pid: Pid,
        prio: u8,
        promises: EnumSet<Promise>,
    },
    CloseStream {
        pid: Pid,
        sid: Sid,
    },
    Send(OutGoingMessage),
}

pub(crate) enum RtrnMsg {
    Shutdown,
    OpendStream {
        pid: Pid,
        prio: u8,
        promises: EnumSet<Promise>,
    },
    ClosedStream {
        pid: Pid,
        sid: Sid,
    },
    Receive(InCommingMessage),
}

pub(crate) enum TokenObjects {
    TcpListener(TcpListener),
    TcpChannel(Channel<TcpChannel>, Option<Sender<Pid>>),
    UdpChannel(Channel<UdpChannel>, Option<Sender<Pid>>),
    MpscChannel(Channel<MpscChannel>, Option<Sender<Pid>>),
}

impl std::fmt::Debug for TokenObjects {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenObjects::TcpListener(l) => write!(f, "{:?}", l),
            TokenObjects::TcpChannel(c, _) => write!(f, "{:?}", c),
            TokenObjects::UdpChannel(c, _) => write!(f, "{:?}", c),
            TokenObjects::MpscChannel(c, _) => unimplemented!("MPSC"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Stream {
    sid: Sid,
    prio: u8,
    promises: EnumSet<Promise>,
    pub to_send: VecDeque<OutGoingMessage>,
    pub to_receive: VecDeque<InCommingMessage>,
}

impl Stream {
    pub fn new(sid: Sid, prio: u8, promises: EnumSet<Promise>) -> Self {
        Stream {
            sid,
            prio,
            promises,
            to_send: VecDeque::new(),
            to_receive: VecDeque::new(),
        }
    }

    pub fn sid(&self) -> Sid { self.sid }

    pub fn prio(&self) -> u8 { self.prio }

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
