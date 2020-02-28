use crate::{channel::ChannelProtocol, types::Frame};
use mio_extras::channel::{Receiver, Sender};
use tracing::*;

pub(crate) struct MpscChannel {
    endpoint_sender: Sender<Frame>,
    endpoint_receiver: Receiver<Frame>,
}

impl MpscChannel {}

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
                    debug!("would block");
                    break;
                },
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    panic!("disconnected");
                },
            };
        }
        result
    }

    /// Execute when ready to write
    fn write(&mut self, frame: Frame) -> Result<(), ()> {
        match self.endpoint_sender.send(frame) {
            Ok(n) => {
                trace!("semded");
            },
            Err(mio_extras::channel::SendError::Io(e))
                if e.kind() == std::io::ErrorKind::WouldBlock =>
            {
                debug!("would block");
                return Err(());
            }
            Err(e) => {
                panic!("{}", e);
            },
        };
        Ok(())
    }

    fn get_handle(&self) -> &Self::Handle { &self.endpoint_receiver }
}

impl std::fmt::Debug for MpscChannel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", "MPSC") }
}
