use crate::types::{Mid, Pid, Prio, Promises, Sid};
use serde::{Deserialize, Serialize};

// Used for Communication between Channel <----(TCP/UDP)----> Channel
#[derive(Serialize, Deserialize, Debug)]
pub enum Frame {
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
