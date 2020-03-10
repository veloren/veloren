use crate::{
    channel::ChannelProtocol,
    types::{Frame, NetworkBuffer},
};
use bincode;
use mio::net::TcpStream;
use std::io::{Read, Write};
use tracing::*;

pub(crate) struct TcpChannel {
    endpoint: TcpStream,
    read_buffer: NetworkBuffer,
    write_buffer: NetworkBuffer,
}

impl TcpChannel {
    pub fn new(endpoint: TcpStream) -> Self {
        Self {
            endpoint,
            read_buffer: NetworkBuffer::new(),
            write_buffer: NetworkBuffer::new(),
        }
    }
}

impl ChannelProtocol for TcpChannel {
    type Handle = TcpStream;

    /// Execute when ready to read
    fn read(&mut self) -> Vec<Frame> {
        let mut result = Vec::new();
        loop {
            match self.endpoint.read(self.read_buffer.get_write_slice(2048)) {
                Ok(0) => {
                    //Shutdown
                    trace!(?self, "shutdown of tcp channel detected");
                    result.push(Frame::Shutdown);
                    break;
                },
                Ok(n) => {
                    self.read_buffer.actually_written(n);
                    trace!("incomming message with len: {}", n);
                    let slice = self.read_buffer.get_read_slice();
                    let mut cur = std::io::Cursor::new(slice);
                    let mut read_ok = 0;
                    while cur.position() < n as u64 {
                        let round_start = cur.position() as usize;
                        let r: Result<Frame, _> = bincode::deserialize_from(&mut cur);
                        match r {
                            Ok(frame) => {
                                result.push(frame);
                                read_ok = cur.position() as usize;
                            },
                            Err(e) => {
                                // Probably we have to wait for moare data!
                                let first_bytes_of_msg =
                                    &slice[round_start..std::cmp::min(n, round_start + 16)];
                                debug!(
                                    ?self,
                                    ?e,
                                    ?n,
                                    ?round_start,
                                    ?first_bytes_of_msg,
                                    "message cant be parsed, probably because we need to wait for \
                                     more data"
                                );
                                break;
                            },
                        }
                    }
                    self.read_buffer.actually_read(read_ok);
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    debug!("would block");
                    break;
                },
                Err(e) => panic!("{}", e),
            };
        }
        result
    }

    /// Execute when ready to write
    fn write<I: std::iter::Iterator<Item = Frame>>(&mut self, frames: &mut I) {
        loop {
            //serialize when len < MTU 1500, then write
            if self.write_buffer.get_read_slice().len() < 1500 {
                match frames.next() {
                    Some(frame) => {
                        if let Ok(size) = bincode::serialized_size(&frame) {
                            let slice = self.write_buffer.get_write_slice(size as usize);
                            if let Err(err) = bincode::serialize_into(slice, &frame) {
                                error!(
                                    ?err,
                                    "serialising frame was unsuccessful, this should never \
                                     happen! dropping frame!"
                                )
                            }
                            self.write_buffer.actually_written(size as usize); //I have to rely on those informations to be consistent!
                        } else {
                            error!(
                                "getting size of frame was unsuccessful, this should never \
                                 happen! dropping frame!"
                            )
                        };
                    },
                    None => break,
                }
            }

            match self.endpoint.write(self.write_buffer.get_read_slice()) {
                Ok(n) => {
                    self.write_buffer.actually_read(n);
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    debug!("can't send tcp yet, would block");
                    return;
                },
                Err(e) => panic!("{}", e),
            }
        }
    }

    fn get_handle(&self) -> &Self::Handle { &self.endpoint }
}

impl std::fmt::Debug for TcpChannel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.endpoint)
    }
}

impl std::fmt::Debug for NetworkBuffer {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NetworkBuffer(len: {}, read: {}, write: {})",
            self.data.len(),
            self.read_idx,
            self.write_idx
        )
    }
}
