use crate::{
    api::Promise,
    message::{InCommingMessage, OutGoingMessage},
    worker::Channel,
};
use enumset::EnumSet;
use mio::{self, net::TcpListener, PollOpt, Ready};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
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

// Used for Communication between Controller <--> Worker
pub(crate) enum CtrlMsg {
    Shutdown,
    Register(TokenObjects, Ready, PollOpt),
    OpenStream {
        pid: Pid,
        prio: u8,
        promises: EnumSet<Promise>,
        return_sid: std::sync::mpsc::Sender<Sid>,
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

#[derive(Debug)]
pub(crate) enum TokenObjects {
    TcpListener(TcpListener),
    Channel(Channel),
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
