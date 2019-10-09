use crate::{NetworkResult, NetworkError};
use crate::message::InternalNetworkMessage;
use crossbeam_channel::Receiver;
use serde::Serialize;
use serde::de::DeserializeOwned;
use crossbeam_channel::TryRecvError;

pub struct Mail<T> {
    id: u32,
    data: T
}

impl<T> Mail<T> {
    fn new(id: u32, data: T) -> Self {
        Self { id, data }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn open(self) -> (u32, T) {
        (self.id, self.data)
    }
}

pub struct MailBox<T> {
    message_queue: Receiver<InternalNetworkMessage<T>>,
}

impl<T: Send + Serialize + DeserializeOwned> MailBox<T> {
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
