// Parent
use super::message::Message;

#[derive(Debug)]
pub enum Frame {
    Header { id: u64, length: u64 },
    Data { id: u64, frame_no: u64, data: Vec<u8> },
}

#[derive(Debug)]
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
    pub fn new<M: Message>(message: M, id: u64) -> PacketData {
        PacketData {
            bytes: message.to_bytes().unwrap(),
            id,
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
    pub fn new<M: Message>(message: M, id: u64) -> OutgoingPacket {
        OutgoingPacket {
            data: PacketData::new(message, id),
            pos: 0,
            headersend: false,
            dataframesno: 0,
            prio: 16,
        }
    }

    // maximal size of the frame (implementation aprox)
    pub fn generate_frame(&mut self, size: u64) -> Result<Frame, FrameError> {
        if !self.headersend {
            self.headersend = true;
            Ok(Frame::Header{
                id: self.data.id,
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
            //debug!("to_send {}" , to_send);
            let end_pos = self.pos + to_send;
            //debug!("daaaaa {:?}", self.data.bytes[self.pos as usize..end_pos as usize].to_vec());
            let frame = Frame::Data{
                id: self.data.id,
                frame_no: self.dataframesno,
                data: self.data.bytes[self.pos as usize..end_pos as usize].to_vec(),
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
            Frame::Header{id, length} => {
                IncommingPacket {
                    data: PacketData::new_size(length, id),
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
    pub fn load_data_frame(&mut self, data: Frame) -> bool {
        match data {
            Frame::Header{ .. } => {
                panic!("not implemented");
            },
            Frame::Data{ id, frame_no, data } => {
                if id != self.data.id {
                    panic!("id missmatch {} <> {}", id, self.data.id);
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
                //println!("pospos {} {} {}", self.pos , data.len(), self.data.bytes.len() as u64);
                return self.pos == self.data.bytes.len() as u64;
            }
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        return &self.data.bytes;
    }
}
