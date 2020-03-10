use crate::{channel::ChannelProtocol, types::Frame};
use lazy_static::lazy_static; // 1.4.0
use mio_extras::channel::{Receiver, Sender};
use std::{
    collections::HashMap,
    sync::{Mutex, RwLock},
};
use tracing::*;

lazy_static! {
    pub(crate) static ref MPSC_REGISTRY: RwLock<HashMap<u64, Mutex<(Sender<Frame>, Receiver<Frame>)>>> =
        RwLock::new(HashMap::new());
}

pub(crate) struct MpscChannel {
    endpoint_sender: Sender<Frame>,
    endpoint_receiver: Receiver<Frame>,
}

impl MpscChannel {
    pub fn new(endpoint_sender: Sender<Frame>, endpoint_receiver: Receiver<Frame>) -> Self {
        Self {
            endpoint_sender,
            endpoint_receiver,
        }
    }
}

impl ChannelProtocol for MpscChannel {
    type Handle = Receiver<Frame>;

    /// Execute when ready to read
    fn read(&mut self) -> Vec<Frame> {
        let mut result = Vec::new();
        loop {
            match self.endpoint_receiver.try_recv() {
                Ok(frame) => {
                    trace!("incomming message");
                    result.push(frame);
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    debug!("read would block");
                    break;
                },
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    trace!(?self, "shutdown of mpsc channel detected");
                    result.push(Frame::Shutdown);
                    break;
                },
            };
        }
        result
    }

    fn write<I: std::iter::Iterator<Item = Frame>>(&mut self, frames: &mut I) {
        for frame in frames {
            match self.endpoint_sender.send(frame) {
                Ok(()) => {
                    trace!("sended");
                },
                Err(mio_extras::channel::SendError::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    debug!("write would block");
                    return;
                }
                Err(mio_extras::channel::SendError::Disconnected(frame)) => {
                    trace!(?frame, ?self, "shutdown of mpsc channel detected");
                    return;
                },
                Err(e) => {
                    panic!("{}", e);
                },
            };
        }
    }

    fn get_handle(&self) -> &Self::Handle { &self.endpoint_receiver }
}

impl std::fmt::Debug for MpscChannel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", "MPSC") }
}
