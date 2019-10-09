use crate::NetworkResult;
use crate::message::InternalNetworkMessage;
use crossbeam_channel::Receiver;
use serde::Serialize;
use serde::de::DeserializeOwned;

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
    pub fn receive(&self) -> NetworkResult<Mail<T>> {
        let message = self.message_queue.try_recv()?;
        let (id, data) = message.deconstruct();
        Ok(Mail::new(id, data))
    }
}
