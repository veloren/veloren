use crate::{
    frame::Frame,
    types::{Mid, Sid},
};
use std::{collections::VecDeque, sync::Arc};

//Todo: Evaluate switching to VecDeque for quickly adding and removing data
// from front, back.
// - It would prob require custom bincode code but thats possible.
#[cfg_attr(test, derive(PartialEq))]
pub struct MessageBuffer {
    pub data: Vec<u8>,
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

/// Contains a outgoing message and store what was *send* and *confirmed*
/// All Chunks have the same size, except for the last chunk which can end
/// earlier. E.g.
/// ```ignore
/// msg = OutgoingMessage::new();
/// msg.next();
/// msg.next();
/// msg.confirm(1);
/// msg.confirm(2);
/// ```
#[derive(Debug)]
pub(crate) struct OutgoingMessage {
    buffer: Arc<MessageBuffer>,
    send_index: u64, // 3 => 4200 (3*FRAME_DATA_SIZE)
    send_header: bool,
    mid: Mid,
    sid: Sid,
    max_index: u64, //speedup
    missing_header: bool,
    missing_indices: VecDeque<u64>,
}

impl OutgoingMessage {
    pub(crate) const FRAME_DATA_SIZE: u64 = 1400;

    pub(crate) fn new(buffer: Arc<MessageBuffer>, mid: Mid, sid: Sid) -> Self {
        let max_index =
            (buffer.data.len() as u64 + Self::FRAME_DATA_SIZE - 1) / Self::FRAME_DATA_SIZE;
        Self {
            buffer,
            send_index: 0,
            send_header: false,
            mid,
            sid,
            max_index,
            missing_header: false,
            missing_indices: VecDeque::new(),
        }
    }

    /// all has been send once, but might been resend due to failures.
    #[allow(dead_code)]
    pub(crate) fn initial_sent(&self) -> bool { self.send_index == self.max_index }

    pub fn get_header(&self) -> Frame {
        Frame::DataHeader {
            mid: self.mid,
            sid: self.sid,
            length: self.buffer.data.len() as u64,
        }
    }

    pub fn get_data(&self, index: u64) -> Frame {
        let start = index * Self::FRAME_DATA_SIZE;
        let to_send = std::cmp::min(
            self.buffer.data[start as usize..].len() as u64,
            Self::FRAME_DATA_SIZE,
        );
        Frame::Data {
            mid: self.mid,
            start,
            data: self.buffer.data[start as usize..][..to_send as usize].to_vec(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_missing(&mut self, missing_header: bool, missing_indicies: VecDeque<u64>) {
        self.missing_header = missing_header;
        self.missing_indices = missing_indicies;
    }

    /// returns if something was added
    pub(crate) fn next(&mut self) -> Option<Frame> {
        if !self.send_header {
            self.send_header = true;
            Some(self.get_header())
        } else if self.send_index < self.max_index {
            self.send_index += 1;
            Some(self.get_data(self.send_index - 1))
        } else if self.missing_header {
            self.missing_header = false;
            Some(self.get_header())
        } else if let Some(index) = self.missing_indices.pop_front() {
            Some(self.get_data(index))
        } else {
            None
        }
    }

    pub(crate) fn get_sid_len(&self) -> (Sid, u64) { (self.sid, self.buffer.data.len() as u64) }
}
