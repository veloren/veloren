use crossbeam_channel::Sender;
use crate::message::InternalNetworkMessage;
use serde::Serialize;
use serde::de::DeserializeOwned;
use crate::{NetworkResult, Reliability};

pub struct MailSender<T> {
    id: u32,
    sender: Sender<InternalNetworkMessage<T>>,
}

impl<T: Send + Serialize + DeserializeOwned> MailSender<T> {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn send(&self, data: T, reliability: Reliability) -> NetworkResult<()> {
        let (message, result_receiver) = InternalNetworkMessage::new(data, reliability, self.id);
        self.sender.send(message)?;
        let result = result_receiver.recv()?;
        result
    }
}
