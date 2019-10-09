use crate::{NetworkResult, NetworkError};
use crate::message::InternalNetworkMessage;
use crossbeam_channel::Receiver;
use serde::Serialize;
use serde::de::DeserializeOwned;
use crossbeam_channel::TryRecvError;

/// A mail represents an incoming message with the ID of the sender tied to it.
pub struct Mail<T> {
    id: u32,
    data: T
}

impl<T> Mail<T> {
    fn new(id: u32, data: T) -> Self {
        Self { id, data }
    }

    /// Peek at the ID of the sender.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Peek at the contained message.
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Open the mail, giving back ownership of the ID and message.
    pub fn open(self) -> (u32, T) {
        (self.id, self.data)
    }
}

/// A mailbox is where all mails received are stashed by the network engine.
pub struct MailBox<T> {
    message_queue: Receiver<InternalNetworkMessage<T>>,
}

impl<T: Send + Serialize + DeserializeOwned> MailBox<T> {
    /// Grab a mail. One mail will be return for every call to this method. There may be multiple mails available at any given time.
    /// You should call this in a loop until you get a None back to make sure you receive all available mails.
    pub fn receive(&self) -> NetworkResult<Option<Mail<T>>> {
        match self.message_queue.try_recv() {
            Ok(message) => {
                let (id, data) = message.deconstruct();
                Ok(Some(Mail::new(id, data)))
            }

            Err(error) => {
                match error {
                    TryRecvError::Empty => Ok(None),
                    TryRecvError::Disconnected => Err(NetworkError::EngineShutdown),
                }
            }
        }
    }
}
