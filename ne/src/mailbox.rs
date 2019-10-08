use crossbeam_channel::{Sender, Receiver};
use crate::message::NetworkMessage;
use serde::Serialize;
use serde::de::DeserializeOwned;
use crate::NetworkResult;

pub struct MailBox<T> {
    sender: Sender<NetworkMessage<T>>,
    receiver: Receiver<NetworkMessage<T>>,
}

impl<T: Send + Serialize + DeserializeOwned> MailBox<T> {
    pub fn send(&self, data: T) -> NetworkResult<()> {
        let (message, result_receiver) = NetworkMessage::new(data);
        self.sender.send(message)?;
        let result = result_receiver.recv()?;
        result
    }
}
