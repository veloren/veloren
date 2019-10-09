use crate::{MailBox, MailSender};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
use std::net::SocketAddr;

/// The networking engine for the server.
/// This runs on a seperate thread in the background.
/// It can be stopped by calling the `close` method on it's mailbox.
pub struct NetworkServerEngine<T> {
    tmp: PhantomData<T>,
}

impl<T: Send + Serialize + DeserializeOwned> NetworkServerEngine<T> {
    /// Create a new NetworkServerEngine that will listen for new clients.
    /// The second argument is a callback which allows you to specify what to do with the MailSender for
    /// every client that connects. For example you could store it in a HashMap.
    pub fn new(addr: impl Into<SocketAddr>, on_connect: impl FnMut(MailSender<T>)) -> MailBox<T> {
        unimplemented!()
    }
}
