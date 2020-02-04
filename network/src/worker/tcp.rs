use crate::{
    api::Promise,
    internal::{RemoteParticipant, VELOREN_MAGIC_NUMBER, VELOREN_NETWORK_VERSION},
    message::OutGoingMessage,
    worker::{
        channel::{Channel, ChannelState},
        types::{Pid, RtrnMsg, Stream, TcpFrame},
    },
};
use bincode;
use enumset::EnumSet;
use mio::{self, net::TcpStream};
use mio_extras::channel::Sender;
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::{Arc, RwLock},
    time::Instant,
};
use tracing::*;

#[derive(Debug)]
pub(crate) struct TcpChannel {
    state: ChannelState,
    pub tcpstream: TcpStream,
}

impl TcpChannel {
    pub fn new(
        tcpstream: TcpStream,
        local_pid: Pid,
        remotes: Arc<RwLock<HashMap<Pid, RemoteParticipant>>>,
    ) -> Self {
        TcpChannel {
            state: ChannelState::new(local_pid, remotes),
            tcpstream,
        }
    }
}

impl Channel for TcpChannel {
    fn read(
        &mut self,
        uninitialized_dirty_speed_buffer: &mut [u8; 65000],
        aprox_time: Instant,
        rtrn_tx: &Sender<RtrnMsg>,
    ) {
        let pid = self.state.remote_pid;
        let span = span!(Level::INFO, "channel", ?pid);
        let _enter = span.enter();
        match self.tcpstream.read(uninitialized_dirty_speed_buffer) {
            Ok(n) => {
                trace!("incomming message with len: {}", n);
                let mut cur = std::io::Cursor::new(&uninitialized_dirty_speed_buffer[..n]);
                while cur.position() < n as u64 {
                    let r: Result<TcpFrame, _> = bincode::deserialize_from(&mut cur);
                    match r {
                        Ok(frame) => self.state.handle(frame, rtrn_tx),
                        Err(e) => {
                            error!(
                                ?self,
                                ?e,
                                "failure parsing a message with len: {}, starting with: {:?}",
                                n,
                                &uninitialized_dirty_speed_buffer[0..std::cmp::min(n, 10)]
                            );
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
    }

    fn write(&mut self, uninitialized_dirty_speed_buffer: &mut [u8; 65000], aprox_time: Instant) {
        let pid = self.state.remote_pid;
        let span = span!(Level::INFO, "channel", ?pid);
        let _enter = span.enter();
        loop {
            while let Some(elem) = self.state.send_queue.pop_front() {
                if let Ok(mut data) = bincode::serialize(&elem) {
                    let total = data.len();
                    match self.tcpstream.write(&data) {
                        Ok(n) if n == total => {},
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
            // run streams
            self.state.tick_streams();
            if self.state.send_queue.is_empty() {
                break;
            }
        }
    }

    fn open_stream(&mut self, prio: u8, promises: EnumSet<Promise>) -> u32 {
        // validate promises
        if let Some(stream_id_pool) = &mut self.state.stream_id_pool {
            let sid = stream_id_pool.next();
            let stream = Stream::new(sid, prio, promises.clone());
            self.state.streams.push(stream);
            self.state.send_queue.push_back(TcpFrame::OpenStream {
                sid,
                prio,
                promises,
            });
            return sid;
        }
        error!("fix me");
        return 0;
        //TODO: fix me
    }

    fn close_stream(&mut self, sid: u32) {
        self.state.streams.retain(|stream| stream.sid() != sid);
        self.state
            .send_queue
            .push_back(TcpFrame::CloseStream { sid });
    }

    fn handshake(&mut self) {
        self.state.send_queue.push_back(TcpFrame::Handshake {
            magic_number: VELOREN_MAGIC_NUMBER.to_string(),
            version: VELOREN_NETWORK_VERSION,
        });
        self.state.send_handshake = true;
    }

    fn shutdown(&mut self) {
        self.state.send_queue.push_back(TcpFrame::Shutdown {});
        self.state.send_shutdown = true;
    }

    fn send(&mut self, outgoing: OutGoingMessage) {
        //TODO: fix me
        for s in self.state.streams.iter_mut() {
            s.to_send.push_back(outgoing);
            break;
        }
    }
}
