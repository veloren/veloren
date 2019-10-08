use crossbeam_channel::Sender;
use crate::message::NetworkMessage;
use serde::Serialize;
use serde::de::DeserializeOwned;
use crate::NetworkResult;

pub struct MailSender<T> {
    sender: Sender<NetworkMessage<T>>,
}

impl<T: Send + Serialize + DeserializeOwned> MailSender<T> {
    pub fn send(&self, data: T) -> NetworkResult<()> {
        let (message, result_receiver) = NetworkMessage::new(data);
        self.sender.send(message)?;
        let result = result_receiver.recv()?;
        result
    }
}
