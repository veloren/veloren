use bincode;
use serde::{de::DeserializeOwned, Serialize};
//use std::collections::VecDeque;
use crate::types::{Mid, Sid};
use byteorder::{NetworkEndian, ReadBytesExt};
use std::sync::Arc;
use tracing::*;

pub(crate) struct MessageBuffer {
    // use VecDeque for msg storage, because it allows to quickly remove data from front.
    //however VecDeque needs custom bincode code, but it's possible
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct OutGoingMessage {
    pub buffer: Arc<MessageBuffer>,
    pub cursor: u64,
    pub mid: Mid,
    pub sid: Sid,
}

#[derive(Debug)]
pub(crate) struct InCommingMessage {
    pub buffer: MessageBuffer,
    pub length: u64,
    pub mid: Mid,
    pub sid: Sid,
}

pub(crate) fn serialize<M: Serialize>(message: &M) -> MessageBuffer {
    let mut writer = {
        let actual_size = bincode::serialized_size(message).unwrap();
        Vec::<u8>::with_capacity(actual_size as usize)
    };
    if let Err(e) = bincode::serialize_into(&mut writer, message) {
        error!("Oh nooo {}", e);
    };
    MessageBuffer { data: writer }
}

pub(crate) fn deserialize<M: DeserializeOwned>(buffer: MessageBuffer) -> M {
    let span = buffer.data;
    let decoded: M = bincode::deserialize(span.as_slice()).unwrap();
    decoded
}

impl std::fmt::Debug for MessageBuffer {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //TODO: small messages!
        let len = self.data.len();
        if len > 20 {
            let n1 = (&self.data[0..4]).read_u32::<NetworkEndian>().unwrap();
            let n2 = (&self.data[4..8]).read_u32::<NetworkEndian>().unwrap();
            let n3 = (&self.data[8..12]).read_u32::<NetworkEndian>().unwrap();
            write!(
                f,
                "MessageBuffer(len: {}, {}, {}, {}, {:?}..{:?})",
                len,
                n1,
                n2,
                n3,
                &self.data[13..16],
                &self.data[len - 8..len]
            )
        } else {
            write!(f, "MessageBuffer(len: {}, {:?})", len, &self.data[..])
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::message::*;

    #[test]
    fn serialize_test() {
        let msg = "abc";
        let mb = serialize(&msg);
        assert_eq!(mb.data.len(), 11);
        assert_eq!(mb.data[0], 3);
        assert_eq!(mb.data[1], 0);
        assert_eq!(mb.data[7], 0);
        assert_eq!(mb.data[8], 'a' as u8);
        assert_eq!(mb.data[8], 97);
        assert_eq!(mb.data[9], 'b' as u8);
        assert_eq!(mb.data[10], 'c' as u8);
    }
}
