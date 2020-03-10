use crate::{
    api::Promise,
    channel::Channel,
    message::{InCommingMessage, OutGoingMessage},
};
use enumset::EnumSet;
use futures;
use mio::{self, net::TcpListener, PollOpt, Ready};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::*;
use uuid::Uuid;

//Participant Ids are randomly chosen
pub type Pid = Uuid;
//Stream Ids are unique per Participant* and are split in 2 ranges, one for
// every Network involved Every Channel gets a subrange during their handshake
// protocol from one of the 2 ranges
//*otherwise extra synchronization would be needed
pub type Sid = u64;
//Message Ids are unique per Stream* and are split in 2 ranges, one for every
// Channel involved
//*otherwise extra synchronization would be needed
pub type Mid = u64;

pub(crate) const VELOREN_MAGIC_NUMBER: &str = "VELOREN";
pub const VELOREN_NETWORK_VERSION: [u32; 3] = [0, 2, 0];
pub const DEFAULT_SID_SIZE: u64 = 1 << 48;

// Used for Communication between Controller <--> Worker
pub(crate) enum CtrlMsg {
    Shutdown,
    Register(TokenObjects, Ready, PollOpt),
    OpenStream {
        pid: Pid,
        sid: Sid,
        prio: u8,
        promises: EnumSet<Promise>,
        msg_tx: futures::channel::mpsc::UnboundedSender<InCommingMessage>,
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
        controller_sids: tlid::Pool<tlid::Wrapping<Sid>>,
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
    pub mid_pool: tlid::Pool<tlid::Wrapping<Mid>>,
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
            mid_pool: tlid::Pool::new_full(),
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
        sender_controller_sids: tlid::RemoveAllocation<Sid>,
        sender_worker_sids: tlid::RemoveAllocation<Sid>,
        receiver_controller_sids: tlid::Pool<tlid::Wrapping<Sid>>,
        receiver_worker_sids: tlid::Pool<tlid::Wrapping<Sid>>,
    },
    ParticipantId {
        pid: Pid,
    },
    Shutdown, /* Shutsdown this channel gracefully, if all channels are shut down, Participant
               * is deleted */
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

pub(crate) struct NetworkBuffer {
    pub(crate) data: Vec<u8>,
    pub(crate) read_idx: usize,
    pub(crate) write_idx: usize,
}

/// NetworkBuffer to use for streamed access
/// valid data is between read_idx and write_idx!
/// everything before read_idx is already processed and no longer important
/// everything after write_idx is either 0 or random data buffered
impl NetworkBuffer {
    pub(crate) fn new() -> Self {
        NetworkBuffer {
            data: vec![0; 2048],
            read_idx: 0,
            write_idx: 0,
        }
    }

    pub(crate) fn get_write_slice(&mut self, min_size: usize) -> &mut [u8] {
        if self.data.len() < self.write_idx + min_size {
            trace!(
                ?self,
                ?min_size,
                "need to resize because buffer is to small"
            );
            self.data.resize(self.write_idx + min_size, 0);
        }
        &mut self.data[self.write_idx..]
    }

    pub(crate) fn actually_written(&mut self, cnt: usize) { self.write_idx += cnt; }

    pub(crate) fn get_read_slice(&self) -> &[u8] { &self.data[self.read_idx..self.write_idx] }

    pub(crate) fn actually_read(&mut self, cnt: usize) {
        self.read_idx += cnt;
        if self.read_idx == self.write_idx {
            if self.read_idx > 10485760 {
                trace!(?self, "buffer empty, resetting indices");
            }
            self.read_idx = 0;
            self.write_idx = 0;
        }
        if self.write_idx > 10485760 {
            if self.write_idx - self.read_idx < 65536 {
                debug!(
                    ?self,
                    "This buffer is filled over 10 MB, but the actual data diff is less then \
                     65kB, which is a sign of stressing this connection much as always new data \
                     comes in - nevertheless, in order to handle this we will remove some data \
                     now so that this buffer doesn't grow endlessly"
                );
                let mut i2 = 0;
                for i in self.read_idx..self.write_idx {
                    self.data[i2] = self.data[i];
                    i2 += 1;
                }
                self.read_idx = 0;
                self.write_idx = i2;
            }
            if self.data.len() > 67108864 {
                warn!(
                    ?self,
                    "over 64Mbyte used, something seems fishy, len: {}",
                    self.data.len()
                );
            }
        }
    }
}

fn chose_protocol(
    available_protocols: u8, /* 1 = TCP, 2= UDP, 4 = MPSC */
    promises: u8,            /*  */
) -> u8 /*1,2 or 4*/ {
    if available_protocols & (1 << 3) != 0 {
        4
    } else if available_protocols & (1 << 1) != 0 {
        1
    } else {
        2
    }
}
