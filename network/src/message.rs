use serde::{de::DeserializeOwned, Serialize};
//use std::collections::VecDeque;
use crate::types::{Frame, Mid, Sid};
use std::{io, sync::Arc};

//Todo: Evaluate switching to VecDeque for quickly adding and removing data
// from front, back.
// - It would prob requiere custom bincode code but thats possible.
/// Support struct used for optimising sending the same Message to multiple
/// [`Stream`]
///
/// For an example usage see: [`send_raw`]
///
/// [`Stream`]: crate::api::Stream
/// [`send_raw`]: crate::api::Stream::send_raw
pub struct MessageBuffer {
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct OutgoingMessage {
    pub buffer: Arc<MessageBuffer>,
    pub cursor: u64,
    pub mid: Mid,
    pub sid: Sid,
}

#[derive(Debug)]
pub(crate) struct IncomingMessage {
    pub buffer: MessageBuffer,
    pub length: u64,
    pub mid: Mid,
    pub sid: Sid,
}

pub(crate) fn serialize<M: Serialize>(message: &M) -> MessageBuffer {
    //this will never fail: https://docs.rs/bincode/0.8.0/bincode/fn.serialize.html
    let writer = bincode::serialize(message).unwrap();
    MessageBuffer {
        data: lz4_compress::compress(&writer),
    }
}

//pub(crate) fn deserialize<M: DeserializeOwned>(buffer: MessageBuffer) ->
// std::Result<M, std::Box<bincode::error::bincode::ErrorKind>> {
pub(crate) fn deserialize<M: DeserializeOwned>(buffer: MessageBuffer) -> bincode::Result<M> {
    let span = lz4_compress::decompress(&buffer.data)
        .expect("lz4 decompression failed, failed to deserialze");
    //this might fail if you choose the wrong type for M. in that case probably X
    // got transfered while you assume Y. probably this means your application
    // logic is wrong. E.g. You expect a String, but just get a u8.
    bincode::deserialize(span.as_slice())
}

impl OutgoingMessage {
    pub(crate) const FRAME_DATA_SIZE: u64 = 1400;

    /// returns if msg is empty
    pub(crate) fn fill_next<E: Extend<(Sid, Frame)>>(
        &mut self,
        msg_sid: Sid,
        frames: &mut E,
    ) -> bool {
        let to_send = std::cmp::min(
            self.buffer.data[self.cursor as usize..].len() as u64,
            Self::FRAME_DATA_SIZE,
        );
        if to_send > 0 {
            if self.cursor == 0 {
                frames.extend(std::iter::once((msg_sid, Frame::DataHeader {
                    mid: self.mid,
                    sid: self.sid,
                    length: self.buffer.data.len() as u64,
                })));
            }
            frames.extend(std::iter::once((msg_sid, Frame::Data {
                mid: self.mid,
                start: self.cursor,
                data: self.buffer.data[self.cursor as usize..][..to_send as usize].to_vec(),
            })));
        };
        self.cursor += to_send;
        self.cursor >= self.buffer.data.len() as u64
    }
}

///wouldn't trust this aaaassss much, fine for tests
pub(crate) fn partial_eq_io_error(first: &io::Error, second: &io::Error) -> bool {
    if let Some(f) = first.raw_os_error() {
        if let Some(s) = second.raw_os_error() {
            f == s
        } else {
            false
        }
    } else {
        let fk = first.kind();
        fk == second.kind() && fk != io::ErrorKind::Other
    }
}

pub(crate) fn partial_eq_bincode(first: &bincode::ErrorKind, second: &bincode::ErrorKind) -> bool {
    match *first {
        bincode::ErrorKind::Io(ref f) => match *second {
            bincode::ErrorKind::Io(ref s) => partial_eq_io_error(f, s),
            _ => false,
        },
        bincode::ErrorKind::InvalidUtf8Encoding(f) => match *second {
            bincode::ErrorKind::InvalidUtf8Encoding(s) => f == s,
            _ => false,
        },
        bincode::ErrorKind::InvalidBoolEncoding(f) => match *second {
            bincode::ErrorKind::InvalidBoolEncoding(s) => f == s,
            _ => false,
        },
        bincode::ErrorKind::InvalidCharEncoding => match *second {
            bincode::ErrorKind::InvalidCharEncoding => true,
            _ => false,
        },
        bincode::ErrorKind::InvalidTagEncoding(f) => match *second {
            bincode::ErrorKind::InvalidTagEncoding(s) => f == s,
            _ => false,
        },
        bincode::ErrorKind::DeserializeAnyNotSupported => match *second {
            bincode::ErrorKind::DeserializeAnyNotSupported => true,
            _ => false,
        },
        bincode::ErrorKind::SizeLimit => match *second {
            bincode::ErrorKind::SizeLimit => true,
            _ => false,
        },
        bincode::ErrorKind::SequenceMustHaveLength => match *second {
            bincode::ErrorKind::SequenceMustHaveLength => true,
            _ => false,
        },
        bincode::ErrorKind::Custom(ref f) => match *second {
            bincode::ErrorKind::Custom(ref s) => f == s,
            _ => false,
        },
    }
}

impl std::fmt::Debug for MessageBuffer {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //TODO: small messages!
        let len = self.data.len();
        if len > 20 {
            write!(
                f,
                "MessageBuffer(len: {}, {}, {}, {}, {:X?}..{:X?})",
                len,
                u32::from_le_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]),
                u32::from_le_bytes([self.data[4], self.data[5], self.data[6], self.data[7]]),
                u32::from_le_bytes([self.data[8], self.data[9], self.data[10], self.data[11]]),
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
        assert_eq!(mb.data.len(), 9);
        assert_eq!(mb.data[0], 34);
        assert_eq!(mb.data[1], 3);
        assert_eq!(mb.data[6], b'a');
        assert_eq!(mb.data[7], b'b');
        assert_eq!(mb.data[8], b'c');
    }
}
