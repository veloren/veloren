use crate::message::InternalNetworkMessage;
use crate::{FutureNetworkResult, Reliability};
use crossbeam_channel::Sender;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// A mail sender. This will send mail to the end it is connection to. This may be anything running a NetworkEngine.
pub struct MailSender<T> {
    id: u32,
    sender: Sender<InternalNetworkMessage<T>>,
}

impl<T: Send + Serialize + DeserializeOwned> MailSender<T> {
    /// Grab the ID of the receiver.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Send a mail. Here you will also specify the reliability guarantee of the mail. See the Reliability enum for details.
    /// This method doesn't block until the message is sent and therefore returns a eager future that will contain
    /// the result at some point in time. See the documentation for FutureNetworkResult for details.
    pub fn send(&self, data: T, reliability: Reliability) -> FutureNetworkResult<()> {
        let (message, result_receiver) = InternalNetworkMessage::new(data, reliability, self.id);
        match self.sender.send(message) {
            Err(error) => FutureNetworkResult::err_now(error),
            Ok(_) => FutureNetworkResult::new(result_receiver),
        }
    }
}
