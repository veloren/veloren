use crate::{Reliability, NetworkResult};
use crate::message::InternalNetworkMessage;
use crossbeam_channel::Receiver;

pub struct MailBox<T> {
    messages: Receiver<InternalNetworkMessage<T>>,
}
