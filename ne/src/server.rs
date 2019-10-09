use crate::{MailBox, MailSender};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
use std::net::SocketAddr;

pub struct NetworkServerEngine<T> {
    tmp: PhantomData<T>,
}

impl<T: Send + Serialize + DeserializeOwned> NetworkServerEngine<T> {
    pub fn new(addr: impl Into<SocketAddr>, on_connect: impl FnMut(MailSender<T>)) -> MailBox<T> {
        unimplemented!()
    }
}
