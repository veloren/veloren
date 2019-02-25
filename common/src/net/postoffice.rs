// Standard
use core::time::Duration;
use std::{
    collections::VecDeque,
    net::SocketAddr,
    thread,
};

// External
use mio::{net::TcpListener, Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel::{channel, Receiver, Sender};

// Crate
use super::{
    data::ControlMsg,
    error::{
        PostError,
        PostErrorInternal,
    },
    postbox::PostBox,
    PostRecv,
    PostSend,
};

// Constants
const CTRL_TOKEN: Token = Token(0); // Token for thread control messages
const DATA_TOKEN: Token = Token(1); // Token for thread data exchange
const CONN_TOKEN: Token = Token(2); // Token for TcpStream for the PostBox child thread

/// A high-level wrapper of [`TcpListener`](mio::net::TcpListener).
/// [`PostOffice`] listens for incoming connections in the background and wraps them into [`PostBox`]es, providing a simple non-blocking API for receiving them.
pub struct PostOffice<S, R>
where
    S: PostSend,
    R: PostRecv,
{
    handle: Option<thread::JoinHandle<()>>,
    ctrl: Sender<ControlMsg>,
    recv: Receiver<Result<PostBox<S, R>, PostErrorInternal>>,
    poll: Poll,
    err: Option<PostErrorInternal>,
}

impl<S, R> PostOffice<S, R>
where
    S: PostSend,
    R: PostRecv,
{
    /// Creates a new [`PostOffice`] listening on specified address
    pub fn new<A: Into<SocketAddr>>(addr: A) -> Result<Self, PostError> {
        let listener = TcpListener::bind(&addr.into())?;
        let (ctrl_tx, ctrl_rx) = channel();
        let (recv_tx, recv_rx) = channel();

        let thread_poll = Poll::new()?;
        let postbox_poll = Poll::new()?;
        thread_poll.register(&listener, CONN_TOKEN, Ready::readable(), PollOpt::edge())?;
        thread_poll.register(&ctrl_rx, CTRL_TOKEN, Ready::readable(), PollOpt::edge())?;
        postbox_poll.register(&recv_rx, DATA_TOKEN, Ready::readable(), PollOpt::edge())?;

        let handle = thread::Builder::new()
            .name("postoffice_worker".into())
            .spawn(move || postoffice_thread(listener, ctrl_rx, recv_tx, thread_poll))?;

        Ok(PostOffice {
            handle: Some(handle),
            ctrl: ctrl_tx,
            recv: recv_rx,
            poll: postbox_poll,
            err: None,
        })
    }

    /// Return an `Option<PostError>` indicating the current status of the `PostOffice`.
    pub fn status(&self) -> Option<PostError> {
        self.err.as_ref().map(|err| err.into())
    }

    /// Non-blocking method returning an iterator over new connections wrapped in [`PostBox`]es
    pub fn new_connections(
        &mut self,
    ) -> impl Iterator<Item = PostBox<S, R>> {
        let mut events = Events::with_capacity(256);
        let mut conns = VecDeque::new();

        // If an error occured, or previously occured, just give up
        if let Some(_) = self.err {
            return conns.into_iter();
        } else if let Err(err) = self.poll.poll(&mut events, Some(Duration::new(0, 0))) {
            self.err = Some(err.into());
            return conns.into_iter();
        }

        for event in events {
            match event.token() {
                // Ignore recv error
                DATA_TOKEN => match self.recv.try_recv() {
                    Ok(Ok(conn)) => conns.push_back(conn),
                    Err(err) => self.err = Some(err.into()),
                    Ok(Err(err)) => self.err = Some(err.into()),
                },
                _ => (),
            }
        }
        conns.into_iter()
    }
}

fn postoffice_thread<S, R>(
    listener: TcpListener,
    ctrl_rx: Receiver<ControlMsg>,
    recv_tx: Sender<Result<PostBox<S, R>, PostErrorInternal>>,
    poll: Poll,
) where
    S: PostSend,
    R: PostRecv,
{
    let mut events = Events::with_capacity(256);
    loop {
        poll.poll(&mut events, None).expect("Failed to execute recv_poll.poll() in PostOffce receiver thread, most likely fault of the OS.");
        for event in events.iter() {
            match event.token() {
                CTRL_TOKEN => match ctrl_rx.try_recv().unwrap() {
                    ControlMsg::Shutdown => return,
                },
                CONN_TOKEN => {
                    let (conn, _addr) = listener.accept().unwrap();
                    recv_tx.send(PostBox::from_tcpstream(conn)
                        // TODO: Is it okay to count a failure to create a postbox here as an 'internal error'?
                        .map_err(|_| PostErrorInternal::MioError)).unwrap();
                }
                _ => (),
            }
        }
    }
}

impl<S, R> Drop for PostOffice<S, R>
where
    S: PostSend,
    R: PostRecv,
{
    fn drop(&mut self) {
        self.ctrl.send(ControlMsg::Shutdown).unwrap_or(()); // If this fails the thread is dead already
        self.handle.take().map(|handle| handle.join());
    }
}
