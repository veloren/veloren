use bincode;
use serde::{Deserialize, Serialize};
//use std::collections::VecDeque;
use std::sync::Arc;
pub trait Message<'a> = Serialize + Deserialize<'a>;

#[derive(Debug)]
pub(crate) struct MessageBuffer {
    // use VecDeque for msg storage, because it allows to quickly remove data from front.
    //however VecDeque needs custom bincode code, but it's possible
    data: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct OutGoingMessage {
    buffer: Arc<MessageBuffer>,
    cursor: u64,
}

#[derive(Debug)]
pub(crate) struct InCommingMessage {
    buffer: MessageBuffer,
    cursor: u64,
}

pub(crate) fn serialize<'a, M: Message<'a>>(message: &M) -> MessageBuffer {
    let mut writer = {
        let actual_size = bincode::serialized_size(message).unwrap();
        Vec::<u8>::with_capacity(actual_size as usize)
    };
    if let Err(e) = bincode::serialize_into(&mut writer, message) {
        println!("Oh nooo {}", e);
    };
    MessageBuffer { data: writer }
}

#[cfg(test)]
mod tests {
    use crate::message::*;

    #[test]
    fn serialize_test() {
        let msg = "abc";
        let mb = serialize(&msg);
        assert_eq!(mb.data.len(), 11);
        assert_eq!(mb.data[0], 3);
        assert_eq!(mb.data[1], 0);
        assert_eq!(mb.data[7], 0);
        assert_eq!(mb.data[8], 'a' as u8);
        assert_eq!(mb.data[8], 97);
        assert_eq!(mb.data[9], 'b' as u8);
        assert_eq!(mb.data[10], 'c' as u8);
    }
}
