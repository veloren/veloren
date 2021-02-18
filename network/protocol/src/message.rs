use crate::{
    frame::OTFrame,
    types::{Mid, Sid},
};
use bytes::{Bytes, BytesMut};

pub(crate) const ALLOC_BLOCK: usize = 16_777_216;

/// Contains a outgoing message for TCP protocol
/// All Chunks have the same size, except for the last chunk which can end
/// earlier. E.g.
/// ```ignore
/// msg = OTMessage::new();
/// msg.next();
/// msg.next();
/// ```
#[derive(Debug)]
pub(crate) struct OTMessage {
    data: Bytes,
    original_length: u64,
    send_header: bool,
    mid: Mid,
    sid: Sid,
    start: u64, /* remove */
}

#[derive(Debug)]
pub(crate) struct ITMessage {
    pub data: BytesMut,
    pub sid: Sid,
    pub length: u64,
}

impl OTMessage {
    pub(crate) const FRAME_DATA_SIZE: u64 = 1400;

    pub(crate) fn new(data: Bytes, mid: Mid, sid: Sid) -> Self {
        let original_length = data.len() as u64;
        Self {
            data,
            original_length,
            send_header: false,
            mid,
            sid,
            start: 0,
        }
    }

    fn get_header(&self) -> OTFrame {
        OTFrame::DataHeader {
            mid: self.mid,
            sid: self.sid,
            length: self.data.len() as u64,
        }
    }

    fn get_next_data(&mut self) -> OTFrame {
        let to_send = std::cmp::min(self.data.len(), Self::FRAME_DATA_SIZE as usize);
        let data = self.data.split_to(to_send);
        self.start += Self::FRAME_DATA_SIZE;

        OTFrame::Data {
            mid: self.mid,
            data,
        }
    }

    /// returns if something was added
    pub(crate) fn next(&mut self) -> Option<OTFrame> {
        if !self.send_header {
            self.send_header = true;
            Some(self.get_header())
        } else if !self.data.is_empty() {
            Some(self.get_next_data())
        } else {
            None
        }
    }

    pub(crate) fn get_sid_len(&self) -> (Sid, u64) { (self.sid, self.original_length) }
}

impl ITMessage {
    pub(crate) fn new(sid: Sid, length: u64, _allocator: &mut BytesMut) -> Self {
        //allocator.reserve(ALLOC_BLOCK);
        //TODO: grab mem from the allocatior, but this is only possible with unsafe
        Self {
            sid,
            length,
            data: BytesMut::with_capacity((length as usize).min(ALLOC_BLOCK /* anti-ddos */)),
        }
    }
}

/*
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
#[allow(dead_code)]
pub(crate) struct OUMessage {
    buffer: Arc<MessageBuffer>,
    send_index: u64, // 3 => 4200 (3*FRAME_DATA_SIZE)
    send_header: bool,
    mid: Mid,
    sid: Sid,
    max_index: u64, //speedup
    missing_header: bool,
    missing_indices: VecDeque<u64>,
}

#[allow(dead_code)]
impl OUMessage {
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
*/
