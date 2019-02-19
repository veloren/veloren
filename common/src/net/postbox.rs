// Standard
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::io::ErrorKind;
use std::io::Read;
use std::net::SocketAddr;
use std::thread;

// External
use bincode;
use mio::{net::TcpStream, Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel::{channel, Receiver, Sender};

// Crate
use super::data::ControlMsg;
use super::error::PostError;
use super::{PostRecv, PostSend};

// Constants
const CTRL_TOKEN: Token = Token(0); // Token for thread control messages
const DATA_TOKEN: Token = Token(1); // Token for thread data exchange
const CONN_TOKEN: Token = Token(2); // Token for TcpStream for the PostBox child thread
const MESSAGE_SIZE_CAP: u64 = 1 << 20; // Maximum accepted length of a packet

/// A high-level wrapper of [`TcpStream`](mio::net::TcpStream).
/// [`PostBox`] takes care of serializing sent packets and deserializing received packets in the background, providing a simple API for sending and receiving objects over network.
pub struct PostBox<S, R>
where
    S: PostSend,
    R: PostRecv,
{
    handle: Option<thread::JoinHandle<()>>,
    ctrl: Sender<ControlMsg>,
    recv: Receiver<Result<R, PostError>>,
    send: Sender<S>,
    poll: Poll,
}

impl<S, R> PostBox<S, R>
where
    S: PostSend,
    R: PostRecv,
{
    /// Creates a new [`PostBox`] connected to specified address, meant to be used by the client
    pub fn to_server(addr: &SocketAddr) -> Result<PostBox<S, R>, PostError> {
        let connection = TcpStream::connect(addr)?;
        Self::from_tcpstream(connection)
    }

    /// Creates a new [`PostBox`] from an existing connection, meant to be used by [`PostOffice`](super::PostOffice) on the server
    pub fn from_tcpstream(connection: TcpStream) -> Result<PostBox<S, R>, PostError> {
        let (ctrl_tx, ctrl_rx) = channel::<ControlMsg>(); // Control messages
        let (send_tx, send_rx) = channel::<S>(); // main thread -[data to be serialized and sent]> worker thread
        let (recv_tx, recv_rx) = channel::<Result<R, PostError>>(); // main thread <[received and deserialized data]- worker thread
        let thread_poll = Poll::new().unwrap();
        let postbox_poll = Poll::new().unwrap();
        thread_poll
            .register(&connection, CONN_TOKEN, Ready::readable(), PollOpt::edge())
            .unwrap();
        thread_poll
            .register(&ctrl_rx, CTRL_TOKEN, Ready::readable(), PollOpt::edge())
            .unwrap();
        thread_poll
            .register(&send_rx, DATA_TOKEN, Ready::readable(), PollOpt::edge())
            .unwrap();
        postbox_poll
            .register(&recv_rx, DATA_TOKEN, Ready::readable(), PollOpt::edge())
            .unwrap();
        let handle = thread::Builder::new()
            .name("postbox_worker".into())
            .spawn(move || postbox_thread(connection, ctrl_rx, send_rx, recv_tx, thread_poll))?;
        Ok(PostBox {
            handle: Some(handle),
            ctrl: ctrl_tx,
            recv: recv_rx,
            send: send_tx,
            poll: postbox_poll,
        })
    }

    /// Non-blocking sender method
    pub fn send(&self, data: S) {
        self.send.send(data).unwrap_or(());
    }

    /// Non-blocking receiver method returning an iterator over already received and deserialized objects
    /// # Errors
    /// If the other side disconnects PostBox won't realize that until you try to send something
    pub fn recv_iter(&self) -> Result<impl Iterator<Item = Result<R, PostError>>, PostError> {
        let mut events = Events::with_capacity(4096);
        self.poll
            .poll(&mut events, Some(core::time::Duration::new(0, 0)))?;
        let mut data: VecDeque<Result<R, PostError>> = VecDeque::new();
        for event in events {
            match event.token() {
                DATA_TOKEN => {
                    data.push_back(self.recv.try_recv()?);
                }
                _ => (),
            }
        }
        Ok(data.into_iter())
    }
}

fn postbox_thread<S, R>(
    mut connection: TcpStream,
    ctrl_rx: Receiver<ControlMsg>,
    send_rx: Receiver<S>,
    recv_tx: Sender<Result<R, PostError>>,
    poll: Poll,
) where
    S: PostSend,
    R: PostRecv,
{
    let mut events = Events::with_capacity(64);
    // Receiving related variables
    let mut recv_buff = Vec::new();
    let mut recv_nextlen: u64 = 0;
    loop {
        let mut disconnected = false;
        poll.poll(&mut events, None)
            .expect("Failed to execute poll(), most likely fault of the OS");
        for event in events.iter() {
            match event.token() {
                CTRL_TOKEN => match ctrl_rx.try_recv().unwrap() {
                    ControlMsg::Shutdown => return,
                },
                CONN_TOKEN => match connection.read_to_end(&mut recv_buff) {
                    Ok(_) => {}
                    // Returned when all the data has been read
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => {
                        recv_tx.send(Err(e.into())).unwrap();
                    }
                },
                DATA_TOKEN => {
                    let mut packet = bincode::serialize(&send_rx.try_recv().unwrap()).unwrap();
                    packet.splice(0..0, (packet.len() as u64).to_be_bytes().iter().cloned());
                    match connection.write_bufs(&[packet.as_slice().into()]) {
                        Ok(_) => {}
                        Err(e) => {
                            recv_tx.send(Err(e.into())).unwrap();
                        }
                    };
                }
                _ => {}
            }
        }
        loop {
            if recv_nextlen == 0 && recv_buff.len() >= 8 {
                recv_nextlen = u64::from_be_bytes(
                    <[u8; 8]>::try_from(recv_buff.drain(0..8).collect::<Vec<u8>>().as_slice())
                        .unwrap(),
                );
                if recv_nextlen > MESSAGE_SIZE_CAP {
                    recv_tx.send(Err(PostError::MsgSizeLimitExceeded)).unwrap();
                    connection.shutdown(std::net::Shutdown::Both).unwrap();
                    recv_buff.drain(..);
                    recv_nextlen = 0;
                    break;
                }
            }
            if recv_buff.len() as u64 >= recv_nextlen && recv_nextlen != 0 {
                match bincode::deserialize(recv_buff
                        .drain(
                            0..usize::try_from(recv_nextlen)
                                .expect("Message size was larger than usize (insane message size and 32 bit OS)"),
                        )
                        .collect::<Vec<u8>>()
                        .as_slice()) {
                            Ok(ok) => {
                                recv_tx
                                    .send(Ok(ok))
                                    .unwrap();
                                recv_nextlen = 0;
                            }
                            Err(e) => {
                                recv_tx.send(Err(e.into())).unwrap();
                                recv_nextlen = 0;
                                continue
                            }
                        }
            } else {
                break;
            }
        }
        match connection.take_error().unwrap() {
            Some(e) => {
                if e.kind() == ErrorKind::BrokenPipe {
                    disconnected = true;
                }
                recv_tx.send(Err(e.into())).unwrap();
            }
            None => {}
        }
        if disconnected == true {
            break;
        }
    }

    // Loop after disconnected
    loop {
        poll.poll(&mut events, None)
            .expect("Failed to execute poll(), most likely fault of the OS");
        for event in events.iter() {
            match event.token() {
                CTRL_TOKEN => match ctrl_rx.try_recv().unwrap() {
                    ControlMsg::Shutdown => return,
                },
                _ => {}
            }
        }
    }
}

impl<S, R> Drop for PostBox<S, R>
where
    S: PostSend,
    R: PostRecv,
{
    fn drop(&mut self) {
        self.ctrl.send(ControlMsg::Shutdown).unwrap_or(());
        self.handle.take().map(|handle| handle.join());
    }
}
