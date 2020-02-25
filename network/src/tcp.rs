use crate::{channel::ChannelProtocol, types::Frame};
use bincode;
use mio::net::TcpStream;
use std::io::{Read, Write};
use tracing::*;

pub(crate) struct TcpChannel {
    endpoint: TcpStream,
    //these buffers only ever contain 1 FRAME !
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

impl TcpChannel {
    pub fn new(endpoint: TcpStream) -> Self {
        let mut b = vec![0; 1600000];
        Self {
            endpoint,
            read_buffer: b.clone(),
            write_buffer: b,
        }
    }
}

impl ChannelProtocol for TcpChannel {
    type Handle = TcpStream;

    /// Execute when ready to read
    fn read(&mut self) -> Vec<Frame> {
        let mut result = Vec::new();
        match self.endpoint.read(self.read_buffer.as_mut_slice()) {
            Ok(n) => {
                trace!("incomming message with len: {}", n);
                let mut cur = std::io::Cursor::new(&self.read_buffer[..n]);
                while cur.position() < n as u64 {
                    let round_start = cur.position();
                    let r: Result<Frame, _> = bincode::deserialize_from(&mut cur);
                    match r {
                        Ok(frame) => result.push(frame),
                        Err(e) => {
                            let newlen = self.read_buffer.len() * 2;
                            let debug_part = &self.read_buffer[(round_start as usize)
                                ..std::cmp::min(n as usize, (round_start + 10) as usize)];
                            warn!(
                                ?self,
                                ?e,
                                ?round_start,
                                "message cant be parsed, probably because buffer isn't large \
                                 enough, starting with: {:?}, increase to {}",
                                debug_part,
                                newlen
                            );
                            error!(
                                "please please please find a solution, either we need to keep the \
                                 buffer hight 1500 and hope for the other part to coorporate or \
                                 we need a way to keep some data in read_buffer till next call or \
                                 have a loop around it ... etc... which is error prone, so i dont \
                                 want to do it!"
                            );
                            if newlen > 204800000 {
                                error!(
                                    "something is seriossly broken with our messages, skipp the \
                                     resize"
                                );
                            } else {
                                self.read_buffer.resize(newlen as usize, 0);
                            }
                            break;
                        },
                    }
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                debug!("would block");
            },
            Err(e) => {
                panic!("{}", e);
            },
        };
        result
    }

    /// Execute when ready to write
    fn write(&mut self, frame: Frame) {
        if let Ok(mut data) = bincode::serialize(&frame) {
            let total = data.len();
            match self.endpoint.write(&data) {
                Ok(n) if n == total => {
                    trace!("send {} bytes", n);
                },
                Ok(n) => {
                    error!("could only send part");
                    //let data = data.drain(n..).collect(); //TODO:
                    // validate n.. is correct
                    // to_send.push_front(data);
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    debug!("would block");
                    return;
                },
                Err(e) => {
                    panic!("{}", e);
                },
            };
        };
    }

    fn get_handle(&self) -> &Self::Handle { &self.endpoint }
}

impl std::fmt::Debug for TcpChannel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.endpoint)
    }
}
