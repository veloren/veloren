use crate::internal::Channel;
use mio::{self, net::TcpStream};
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};
use tracing::{debug, error, info, span, trace, warn, Level};

#[derive(Debug)]
pub(crate) struct TcpChannel {
    pub stream: TcpStream,
    pub to_send: RwLock<VecDeque<Vec<u8>>>,
    pub to_receive: RwLock<VecDeque<Vec<u8>>>,
}

impl TcpChannel {
    pub fn new(stream: TcpStream) -> Self {
        TcpChannel {
            stream,
            to_send: RwLock::new(VecDeque::new()),
            to_receive: RwLock::new(VecDeque::new()),
        }
    }
}

impl Channel for TcpChannel {
    fn get_preferred_queue_size() -> usize {
        1400 /*TCP MTU is often 1500, minus some headers*/
        //TODO: get this from the underlying network interface
    }

    fn get_preferred_buffer_len() -> usize {
        5
        // = 1400*5 = 7000bytes => 0.0056s of buffer on 10Mbit/s network
    }

    fn queue(&self, msg: Vec<u8>) {}

    fn recv(&self) -> Option<Vec<u8>> { None }
}
