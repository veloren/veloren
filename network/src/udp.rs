use crate::{channel::ChannelProtocol, types::Frame};
use bincode;
use mio::net::UdpSocket;
use tracing::*;

pub(crate) struct UdpChannel {
    endpoint: UdpSocket,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

impl UdpChannel {
    pub fn new(endpoint: UdpSocket) -> Self {
        Self {
            endpoint,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
        }
    }
}

impl ChannelProtocol for UdpChannel {
    type Handle = UdpSocket;

    /// Execute when ready to read
    fn read(&mut self) -> Vec<Frame> {
        let mut result = Vec::new();
        match self.endpoint.recv_from(self.read_buffer.as_mut_slice()) {
            Ok((n, remote)) => {
                trace!("incomming message with len: {}", n);
                let mut cur = std::io::Cursor::new(&self.read_buffer[..n]);
                while cur.position() < n as u64 {
                    let r: Result<Frame, _> = bincode::deserialize_from(&mut cur);
                    match r {
                        Ok(frame) => result.push(frame),
                        Err(e) => {
                            error!(
                                ?self,
                                ?e,
                                "failure parsing a message with len: {}, starting with: {:?}",
                                n,
                                &self.read_buffer[0..std::cmp::min(n, 10)]
                            );
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
    fn write(&mut self, frame: Frame) -> Result<(), ()> {
        if let Ok(mut data) = bincode::serialize(&frame) {
            let total = data.len();
            match self.endpoint.send(&data) {
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
                    return Err(());
                },
                Err(e) => {
                    panic!("{}", e);
                },
            };
        };
        Ok(())
    }

    fn get_handle(&self) -> &Self::Handle { &self.endpoint }
}

impl std::fmt::Debug for UdpChannel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.endpoint)
    }
}
