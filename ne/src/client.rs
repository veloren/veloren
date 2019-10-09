use crate::{MailBox, MailSender};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
use std::net::SocketAddr;

pub struct NetworkClientEngine<T> {
    tmp: PhantomData<T>,
}

impl<T: Send + Serialize + DeserializeOwned> NetworkClientEngine<T> {
    pub fn connect(addr: impl Into<SocketAddr>) -> (MailBox<T>, MailSender<T>) {
        unimplemented!()
    }
}
