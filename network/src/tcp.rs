use crate::{channel::ChannelProtocol, types::Frame};
use bincode;
use mio::net::TcpStream;
use std::{
    io::{Read, Write},
    ops::Range,
};
use tracing::*;

pub(crate) struct TcpChannel {
    endpoint: TcpStream,
    //these buffers only ever contain 1 FRAME !
    read_buffer: NetworkBuffer,
    write_buffer: NetworkBuffer,
    need_to_send_till: usize,
}

struct NetworkBuffer {
    data: Vec<u8>,
    read_idx: usize,
    write_idx: usize,
}

impl TcpChannel {
    pub fn new(endpoint: TcpStream) -> Self {
        Self {
            endpoint,
            read_buffer: NetworkBuffer::new(),
            write_buffer: NetworkBuffer::new(),
            need_to_send_till: 0,
        }
    }
}

/// NetworkBuffer to use for streamed access
/// valid data is between read_idx and write_idx!
/// everything before read_idx is already processed and no longer important
/// everything after write_idx is either 0 or random data buffered
impl NetworkBuffer {
    fn new() -> Self {
        NetworkBuffer {
            data: vec![0; 2048],
            read_idx: 0,
            write_idx: 0,
        }
    }

    fn get_write_slice(&mut self, min_size: usize) -> &mut [u8] {
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

    fn actually_written(&mut self, cnt: usize) { self.write_idx += cnt; }

    fn get_read_slice(&self) -> &[u8] {
        trace!(?self, "get_read_slice");
        &self.data[self.read_idx..self.write_idx]
    }

    fn actually_read(&mut self, cnt: usize) {
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

fn move_in_vec(vec: &mut Vec<u8>, src: Range<usize>, dest: Range<usize>) {
    debug_assert_eq!(src.end - src.start, dest.end - dest.start);
    let mut i2 = dest.start;
    for i in src {
        vec[i2] = vec[i];
        i2 += 1;
    }
}

impl ChannelProtocol for TcpChannel {
    type Handle = TcpStream;

    /// Execute when ready to read
    fn read(&mut self) -> Vec<Frame> {
        let mut result = Vec::new();
        loop {
            match self.endpoint.read(self.read_buffer.get_write_slice(2048)) {
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
    fn write(&mut self, frame: Frame) -> Result<(), ()> {
        if let Ok(mut size) = bincode::serialized_size(&frame) {
            let slice = self.write_buffer.get_write_slice(size as usize);
            if let Err(e) = bincode::serialize_into(slice, &frame) {
                error!(
                    "serialising frame was unsuccessful, this should never happen! dropping frame!"
                )
            }
            self.write_buffer.actually_written(size as usize); //I have to rely on those informations to be consistent!
        } else {
            error!(
                "getting size of frame was unsuccessful, this should never happen! dropping frame!"
            )
        };
        match self.endpoint.write(self.write_buffer.get_read_slice()) {
            Ok(n) => {
                self.write_buffer.actually_read(n);
                Ok(())
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                debug!("can't send tcp yet, would block");
                Err(())
            },
            Err(e) => panic!("{}", e),
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
