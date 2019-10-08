use crossbeam_channel::Sender;
use crate::message::NetworkMessage;
use serde::Serialize;
use serde::de::DeserializeOwned;
use crate::{NetworkResult, Reliability};

pub struct MailSender<T> {
    id: u32,
    sender: Sender<NetworkMessage<T>>,
}

impl<T: Send + Serialize + DeserializeOwned> MailSender<T> {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn send(&self, data: T, reliability: Reliability) -> NetworkResult<()> {
        let (message, result_receiver) = NetworkMessage::new(data, reliability);
        self.sender.send(message)?;
        let result = result_receiver.recv()?;
        result
    }
}
