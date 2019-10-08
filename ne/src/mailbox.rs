use crossbeam_channel::{Sender, Receiver};
use crate::message::NetworkMessage;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub struct Mailbox<T: Serialize + DeserializeOwned> {
    sender: Sender<NetworkMessage<T>>,
    receiver: Receiver<NetworkMessage<T>>,
}
