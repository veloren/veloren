use net::message::Message;

pub enum Frame {
    Header { uid: u64, length: u64 },
    Data { uid: u64, frame_no: u64, data: Vec<u8> },
}

pub enum FrameError {
    SendDone,
}

//TODO: enhance this PacketData / OutgoingPacket structure, so that only one byte stream is keept for broadcast
pub struct PacketData {
    bytes: Vec<u8>,
    id: u64,
}

pub struct OutgoingPacket {
    data: PacketData,
    pos: u64,
    headersend: bool,
    dataframesno: u64,
    prio: u8,
}

pub struct IncommingPacket {
    data: PacketData,
    pos: u64,
    dataframesno: u64,
}

impl PacketData {
    pub fn new<M: Message>(message: M) -> PacketData {
        println!("{:?}", message.serialize().unwrap());
        PacketData {
            bytes: message.serialize().unwrap(),
            id: 0,
        }
    }

    pub fn new_size(size: u64, id: u64) -> PacketData {
        PacketData {
            bytes: vec![0; size as usize],
            id,
        }
    }
}

impl OutgoingPacket {
    pub fn new<M: Message>(message: M) -> OutgoingPacket {
        OutgoingPacket {
            data: PacketData::new(message),
            pos: 0,
            headersend: false,
            dataframesno: 0,
            prio: 16,
        }
    }

    // maximal size of the frame (implementation aprox)
    pub fn generateFrame(&mut self, size: u64) -> Result<Frame, FrameError> {
        if !self.headersend {
            self.headersend = true;
            Ok(Frame::Header{
                uid: self.data.id,
                length: self.data.bytes.len() as u64,
            })
        } else {
            let remaining = self.data.bytes.len() as u64 - self.pos;
            if remaining == 0 {
                return Err(FrameError::SendDone)
            }
            let to_send;
            if size >= remaining {
                to_send = remaining;
            } else {
                to_send = size;
            }
            let frame = Frame::Data{
                uid: self.data.id,
                frame_no: self.dataframesno,
                data: self.data.bytes[to_send as usize..].to_vec(),
            };
            self.pos += to_send as u64;
            self.dataframesno += 1;
            return Ok(frame);
        }
    }
}

impl IncommingPacket {
    pub fn new(header: Frame) -> IncommingPacket {
        match header {
            Frame::Header{uid, length} => {
                IncommingPacket {
                    data: PacketData::new_size(uid, length),
                    pos: 0,
                    dataframesno: 0,
                }
            },
            Frame::Data{ .. } => {
                panic!("not implemented");
            }
        }

    }

    // returns finished
    pub fn loadDataFrame(&mut self, data: Frame) -> bool {
        match data {
            Frame::Header{ .. } => {
                panic!("not implemented");
            },
            Frame::Data{ uid, frame_no, data } => {
                if uid != self.data.id {
                    panic!("uid missmatch {} <> {}", uid, self.data.id);
                }
                if frame_no != self.dataframesno {
                    panic!("bufferin for frames not yet implemented");
                }
                // copy data starting from self.pos
                //TODO: check size of send with reserved
                for (dst, src) in self.data.bytes[self.pos as usize..].iter_mut().zip(&data) {
                    *dst = *src;
                }
                self.pos += data.len() as u64;
                self.dataframesno += 1;
                return self.pos == self.data.bytes.len() as u64;
            }
        }
    }
}
