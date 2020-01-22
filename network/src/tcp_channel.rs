use crate::{
    api::Promise,
    internal::{Channel, Stream, TcpFrame, VELOREN_MAGIC_NUMBER, VELOREN_NETWORK_VERSION},
};
use bincode;
use enumset::EnumSet;
use mio::{self, net::TcpStream};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    time::Instant,
};
use tracing::*;

#[derive(Debug)]
pub(crate) struct TcpChannel {
    stream_id_pool: tlid::Pool<tlid::Wrapping<u32>>, //TODO: stream_id unique per participant
    msg_id_pool: tlid::Pool<tlid::Wrapping<u64>>,    //TODO: msg_id unique per participant
    participant_id: Option<uuid::Uuid>,
    pub tcpstream: TcpStream,
    pub streams: Vec<Stream>,
    pub send_queue: VecDeque<TcpFrame>,
    pub recv_queue: VecDeque<TcpFrame>,
}

impl TcpChannel {
    pub fn new(tcpstream: TcpStream) -> Self {
        TcpChannel {
            stream_id_pool: tlid::Pool::new_full(),
            msg_id_pool: tlid::Pool::new_full(),
            participant_id: None,
            tcpstream,
            streams: Vec::new(),
            send_queue: VecDeque::new(),
            recv_queue: VecDeque::new(),
        }
    }

    fn handle_frame(&mut self, frame: TcpFrame) {
        match frame {
            TcpFrame::Handshake {
                magic_number,
                version,
            } => {
                if magic_number != VELOREN_MAGIC_NUMBER {
                    error!("tcp connection with invalid handshake, closing connection");
                    #[cfg(debug_assertions)]
                    {
                        debug!("sending client instructions before killing");
                        let _ = self.tcpstream.write(
                            "Handshake does not contain the magic number requiered by veloren \
                             server.\nWe are not sure if you are a valid veloren client.\nClosing \
                             the connection"
                                .as_bytes(),
                        );
                    }
                }
                if version != VELOREN_NETWORK_VERSION {
                    error!("tcp connection with wrong network version");
                    #[cfg(debug_assertions)]
                    {
                        debug!("sending client instructions before killing");
                        let _ = self.tcpstream.write(
                            format!(
                                "Handshake does not contain a correct magic number, but invalid \
                                 version.\nWe don't know how to communicate with you.\nOur \
                                 Version: {:?}\nYour Version: {:?}\nClosing the connection",
                                VELOREN_NETWORK_VERSION, version,
                            )
                            .as_bytes(),
                        );
                    }
                }
                info!(?self, "handshake completed");
            },
            TcpFrame::ParticipantId { pid } => {
                self.participant_id = Some(pid);
                info!("Participant: {} send their ID", pid);
            },
            TcpFrame::OpenStream {
                sid,
                prio,
                promises,
            } => {
                if let Some(pid) = self.participant_id {
                    let sid = self.stream_id_pool.next();
                    let stream = Stream::new(sid, prio, promises.clone());
                    self.streams.push(stream);
                    info!("Participant: {} opened a stream", pid);
                }
            },
            TcpFrame::CloseStream { sid } => {
                if let Some(pid) = self.participant_id {
                    self.streams.retain(|stream| stream.sid() != sid);
                    info!("Participant: {} closed a stream", pid);
                }
            },
            TcpFrame::DataHeader { id, length } => {
                info!("Data Header {}", id);
            },
            TcpFrame::Data { id, frame_no, data } => {
                info!("Data Package {}", id);
            },
        }
    }
}

impl Channel for TcpChannel {
    fn read(&mut self, uninitialized_dirty_speed_buffer: &mut [u8; 65000], aprox_time: Instant) {
        match self.tcpstream.read(uninitialized_dirty_speed_buffer) {
            Ok(n) => {
                trace!("incomming message with len: {}", n);
                let mut cur = std::io::Cursor::new(&uninitialized_dirty_speed_buffer[..n]);
                while cur.position() < n as u64 {
                    let r: Result<TcpFrame, _> = bincode::deserialize_from(&mut cur);
                    match r {
                        Ok(frame) => self.handle_frame(frame),
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
        while let Some(elem) = self.send_queue.pop_front() {
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
                    },
                    Err(e) => {
                        panic!("{}", e);
                    },
                };
            };
        }
    }

    fn open_stream(&mut self, prio: u8, promises: EnumSet<Promise>) -> u32 {
        // validate promises
        let sid = self.stream_id_pool.next();
        let stream = Stream::new(sid, prio, promises.clone());
        self.streams.push(stream);
        self.send_queue.push_back(TcpFrame::OpenStream {
            sid,
            prio,
            promises,
        });
        sid
    }

    fn close_stream(&mut self, sid: u32) {
        self.streams.retain(|stream| stream.sid() != sid);
        self.send_queue.push_back(TcpFrame::CloseStream { sid });
    }

    fn handshake(&mut self) {
        self.send_queue.push_back(TcpFrame::Handshake {
            magic_number: VELOREN_MAGIC_NUMBER.to_string(),
            version: VELOREN_NETWORK_VERSION,
        });
    }

    fn participant_id(&mut self, pid: uuid::Uuid) {
        self.send_queue.push_back(TcpFrame::ParticipantId { pid });
    }
}
