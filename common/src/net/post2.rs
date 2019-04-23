use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream, SocketAddr, Shutdown},
    time::{Instant, Duration},
    marker::PhantomData,
    sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}},
    thread,
    collections::VecDeque,
    convert::TryFrom,
};
use serde::{Serialize, de::DeserializeOwned};

#[derive(Clone, Debug)]
pub enum Error {
    Io(Arc<io::Error>),
    Bincode(Arc<bincode::Error>),
    ChannelFailure,
    InvalidMessage,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(Arc::new(err))
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Error::Bincode(Arc::new(err))
    }
}

impl From<mpsc::TryRecvError> for Error {
    fn from(error: mpsc::TryRecvError) -> Self {
        Error::ChannelFailure
    }
}

pub trait PostMsg = Serialize + DeserializeOwned + 'static + Send;

const MAX_MSG_SIZE: usize = 1 << 20;

pub struct PostOffice<S: PostMsg, R: PostMsg> {
    listener: TcpListener,
    error: Option<Error>,
    phantom: PhantomData<(S, R)>,
}

impl<S: PostMsg, R: PostMsg> PostOffice<S, R> {
    pub fn bind<A: Into<SocketAddr>>(addr: A) -> Result<Self, Error> {
        let mut listener = TcpListener::bind(addr.into())?;
        listener.set_nonblocking(true)?;

        Ok(Self {
            listener,
            error: None,
            phantom: PhantomData,
        })
    }

    pub fn error(&self) -> Option<Error> {
        self.error.clone()
    }

    pub fn new_postboxes(&mut self) -> impl ExactSizeIterator<Item=PostBox<S, R>> {
        let mut new = Vec::new();

        if self.error.is_some() {
            return new.into_iter();
        }

        loop {
            match self.listener.accept() {
                Ok((stream, sock)) => new.push(PostBox::from_stream(stream).unwrap()),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    self.error = Some(e.into());
                    break;
                },
            }
        }

        new.into_iter()
    }
}

pub struct PostBox<S: PostMsg, R: PostMsg> {
    send_tx: mpsc::Sender<S>,
    recv_rx: mpsc::Receiver<Result<R, Error>>,
    worker: Option<thread::JoinHandle<()>>,
    running: Arc<AtomicBool>,
    error: Option<Error>,
}

impl<S: PostMsg, R: PostMsg> PostBox<S, R> {
    pub fn to<A: Into<SocketAddr>>(addr: A) -> Result<Self, Error> {
        Self::from_stream(TcpStream::connect(addr.into())?)
    }

    fn from_stream(stream: TcpStream) -> Result<Self, Error> {
        stream.set_nonblocking(true)?;

        let running = Arc::new(AtomicBool::new(true));
        let worker_running = running.clone();

        let (send_tx, send_rx) = mpsc::channel();
        let (recv_tx, recv_rx) = mpsc::channel();

        let worker = thread::spawn(move || Self::worker(stream, send_rx, recv_tx, worker_running));

        Ok(Self {
            send_tx,
            recv_rx,
            worker: Some(worker),
            running,
            error: None,
        })
    }

    pub fn error(&self) -> Option<Error> {
        self.error.clone()
    }

    pub fn send_message(&mut self, msg: S) {
        let _ = self.send_tx.send(msg);
    }

    pub fn next_message(&mut self) -> Option<R> {
        if self.error.is_some() {
            return None;
        }

        match self.recv_rx.recv().ok()? {
            Ok(msg) => Some(msg),
            Err(e) => {
                self.error = Some(e);
                None
            },
        }
    }

    pub fn new_messages(&mut self) -> impl ExactSizeIterator<Item=R> {
        let mut new = Vec::new();

        if self.error.is_some() {
            return new.into_iter();
        }

        loop {
            match self.recv_rx.try_recv() {
                Ok(Ok(msg)) => new.push(msg),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(e) => {
                    self.error = Some(e.into());
                    break;
                },
                Ok(Err(e)) => {
                    self.error = Some(e);
                    break;
                },
            }
        }

        new.into_iter()
    }

    fn worker(mut stream: TcpStream, send_rx: mpsc::Receiver<S>, recv_tx: mpsc::Sender<Result<R, Error>>, running: Arc<AtomicBool>) {
        let mut outgoing_chunks = VecDeque::new();
        let mut incoming_buf = Vec::new();

        'work: while running.load(Ordering::Relaxed) {
            // Get stream errors
            match stream.take_error() {
                Ok(Some(e)) | Err(e) => {
                    recv_tx.send(Err(e.into())).unwrap();
                    break 'work;
                },
                Ok(None) => {},
            }

            // Try getting messages from the send channel
            for _ in 0..100 {
                match send_rx.try_recv() {
                    Ok(send_msg) => {
                        // Serialize message
                        let mut msg_bytes = bincode::serialize(&send_msg).unwrap();

                        // Assemble into packet
                        let mut packet_bytes = msg_bytes
                            .len()
                            .to_le_bytes()
                            .as_ref()
                            .to_vec();
                        packet_bytes.append(&mut msg_bytes);

                        // Split packet into chunks
                        packet_bytes
                            .chunks(4096)
                            .map(|chunk| chunk.to_vec())
                            .for_each(|chunk| outgoing_chunks.push_back(chunk))
                    },
                    Err(mpsc::TryRecvError::Empty) => break,
                    // Worker error
                    Err(e) => {
                        let _ = recv_tx.send(Err(e.into()));
                        break 'work;
                    },
                }
            }

            // Try sending bytes through the TCP stream
            for _ in 0..100 {
                //println!("HERE! Outgoing len: {}", outgoing_chunks.len());
                match outgoing_chunks.pop_front() {
                    Some(chunk) => match stream.write_all(&chunk) {
                        Ok(()) => {},
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // Return chunk to the queue to try again later
                            outgoing_chunks.push_front(chunk);
                            break;
                        },
                        // Worker error
                        Err(e) => {
                            recv_tx.send(Err(e.into())).unwrap();
                            break 'work;
                        },
                    },
                    None => break,
                }
            }

            // Try receiving bytes from the TCP stream
            for _ in 0..100 {
                let mut buf = [0; 1024];

                match stream.read(&mut buf) {
                    Ok(n) => incoming_buf.extend_from_slice(&buf[0..n]),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    // Worker error
                    Err(e) => {
                        recv_tx.send(Err(e.into())).unwrap();
                        break 'work;
                    },
                }
            }

            // Try turning bytes into messages
            for _ in 0..100 {
                match incoming_buf.get(0..8) {
                    Some(len_bytes) => {
                        let len = usize::from_le_bytes(<[u8; 8]>::try_from(len_bytes).unwrap()); // Can't fail

                        if len > MAX_MSG_SIZE {
                            recv_tx.send(Err(Error::InvalidMessage)).unwrap();
                            break 'work;
                        } else if incoming_buf.len() >= len + 8 {
                            let deserialize_result = bincode::deserialize(&incoming_buf[8..len + 8]);

                            if let Err(e) = &deserialize_result {
                                println!("DESERIALIZE ERROR: {:?}", e);
                            }

                            recv_tx.send(deserialize_result.map_err(|e| e.into()));
                            incoming_buf = incoming_buf.split_off(len + 8);
                        } else {
                            break;
                        }
                    },
                    None => break,
                }
            }

            thread::sleep(Duration::from_millis(10));
        }

        stream.shutdown(Shutdown::Both);
    }
}

impl<S: PostMsg, R: PostMsg> Drop for PostBox<S, R> {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.worker.take().map(|handle| handle.join());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_postoffice<S: PostMsg, R: PostMsg>(id: u16) -> Result<(PostOffice<S, R>, SocketAddr), Error> {
        let sock = ([0; 4], 12345 + id).into();
        Ok((PostOffice::bind(sock)?, sock))
    }

    fn loop_for<F: FnMut()>(duration: Duration, mut f: F) {
        let start = Instant::now();
        while start.elapsed() < duration {
            f();
        }
    }

    #[test]
    fn connect() {
        let (mut postoffice, sock) = create_postoffice::<(), ()>(0).unwrap();

        let _client0 = PostBox::<(), ()>::to(sock).unwrap();
        let _client1 = PostBox::<(), ()>::to(sock).unwrap();
        let _client2 = PostBox::<(), ()>::to(sock).unwrap();

        let mut new_clients = 0;
        loop_for(Duration::from_millis(250), || {
            new_clients += postoffice.new_postboxes().count();
        });

        assert_eq!(new_clients, 3);
    }

    /*
    #[test]
    fn disconnect() {
        let (mut postoffice, sock) = create_postoffice::<(), ()>(1).unwrap();

        let mut client = PostBox::<i32, ()>::to(sock).unwrap();
        loop_for(Duration::from_millis(250), || ());
        let mut server = postoffice.new_postboxes().unwrap().next().unwrap();

        drop(client);
        loop_for(Duration::from_millis(300), || ());

        assert!(server.new_messages().is_err());
    }
    */

    #[test]
    fn send_recv() {
        let (mut postoffice, sock) = create_postoffice::<(), i32>(2).unwrap();
        let test_msgs = vec![1, 1337, 42, -48];

        let mut client = PostBox::<i32, ()>::to(sock).unwrap();
        loop_for(Duration::from_millis(250), || ());
        let mut server = postoffice.new_postboxes().next().unwrap();

        for msg in &test_msgs {
            client.send_message(msg.clone());
        }

        let mut recv_msgs = Vec::new();
        loop_for(Duration::from_millis(250), || server
                .new_messages()
                .for_each(|msg| recv_msgs.push(msg)));

        assert_eq!(test_msgs, recv_msgs);
    }

    #[test]
    fn send_recv_huge() {
        let (mut postoffice, sock) = create_postoffice::<(), Vec<i32>>(3).unwrap();
        let test_msgs: Vec<Vec<i32>> = (0..5).map(|i| (0..100000).map(|j| i * 2 + j).collect()).collect();

        let mut client = PostBox::<Vec<i32>, ()>::to(sock).unwrap();
        loop_for(Duration::from_millis(250), || ());
        let mut server = postoffice.new_postboxes().next().unwrap();

        for msg in &test_msgs {
            client.send_message(msg.clone());
        }

        let mut recv_msgs = Vec::new();
        loop_for(Duration::from_millis(3000), || server
                .new_messages()
                .for_each(|msg| recv_msgs.push(msg)));

        assert_eq!(test_msgs.len(), recv_msgs.len());
        assert!(test_msgs == recv_msgs);
    }

    #[test]
    fn send_recv_both() {
        let (mut postoffice, sock) = create_postoffice::<u32, u32>(4).unwrap();
        let test_msgs = vec![1, 1337, 42, -48];

        let mut client = PostBox::<u32, u32>::to(sock).unwrap();
        loop_for(Duration::from_millis(250), || ());
        let mut server = postoffice.new_postboxes().next().unwrap();

        let test_msgs = vec![
            (0xDEADBEAD, 0xBEEEEEEF),
            (0x1BADB002, 0xBAADF00D),
            (0xBAADA555, 0xC0DED00D),
            (0xCAFEBABE, 0xDEADC0DE),
        ];

        for (to, from) in test_msgs {
            client.send_message(to);
            server.send_message(from);

            loop_for(Duration::from_millis(250), || ());

            assert_eq!(client.new_messages().next().unwrap(), from);
            assert_eq!(server.new_messages().next().unwrap(), to);
        }
    }
}
