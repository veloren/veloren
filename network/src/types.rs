use rand::Rng;
use serde::{Deserialize, Serialize};

pub type Mid = u64;
pub type Cid = u64;
pub type Prio = u8;
pub type Promises = u8;

pub const PROMISES_NONE: Promises = 0;
pub const PROMISES_ORDERED: Promises = 1;
pub const PROMISES_CONSISTENCY: Promises = 2;
pub const PROMISES_GUARANTEED_DELIVERY: Promises = 4;
pub const PROMISES_COMPRESSED: Promises = 8;
pub const PROMISES_ENCRYPTED: Promises = 16;

pub(crate) const VELOREN_MAGIC_NUMBER: &str = "VELOREN";
pub const VELOREN_NETWORK_VERSION: [u32; 3] = [0, 2, 0];
pub(crate) const STREAM_ID_OFFSET1: Sid = Sid::new(0);
pub(crate) const STREAM_ID_OFFSET2: Sid = Sid::new(u64::MAX / 2);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub struct Pid {
    internal: u128,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct Sid {
    internal: u64,
}

// Used for Communication between Channel <----(TCP/UDP)----> Channel
#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum Frame {
    Handshake {
        magic_number: String,
        version: [u32; 3],
    },
    ParticipantId {
        pid: Pid,
    },
    Shutdown, /* Shutsdown this channel gracefully, if all channels are shut down, Participant
               * is deleted */
    OpenStream {
        sid: Sid,
        prio: Prio,
        promises: Promises,
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

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum Requestor {
    User,
    Api,
    Scheduler,
    Remote,
}

impl Pid {
    pub fn new() -> Self {
        Self {
            internal: rand::thread_rng().gen(),
        }
    }

    /// don't use fake! just for testing!
    /// This will panic if pid i greater than 7, as i do not want you to use
    /// this in production!
    pub fn fake(pid: u8) -> Self {
        assert!(pid < 8);
        Self {
            internal: pid as u128,
        }
    }
}

impl Sid {
    pub const fn new(internal: u64) -> Self { Self { internal } }
}

impl std::fmt::Debug for Pid {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //only print last 6 chars of number as full u128 logs are unreadable
        write!(f, "{}", self.internal.rem_euclid(100000))
    }
}

impl std::ops::AddAssign for Sid {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            internal: self.internal + other.internal,
        };
    }
}

impl std::fmt::Debug for Sid {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //only print last 6 chars of number as full u128 logs are unreadable
        write!(f, "{}", self.internal.rem_euclid(1000000))
    }
}

impl From<u64> for Sid {
    fn from(internal: u64) -> Self { Sid { internal } }
}
