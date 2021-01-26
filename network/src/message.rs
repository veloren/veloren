use serde::{de::DeserializeOwned, Serialize};
//use std::collections::VecDeque;
#[cfg(feature = "compression")]
use crate::types::Promises;
use crate::{
    api::{Stream, StreamError},
    types::{Frame, Mid, Sid},
};
use std::{io, sync::Arc};
#[cfg(all(feature = "compression", debug_assertions))]
use tracing::warn;

/// Support struct used for optimising sending the same Message to multiple
/// [`Stream`]
///
/// For an example usage see: [`send_raw`]
///
/// [`Stream`]: crate::api::Stream
/// [`send_raw`]: crate::api::Stream::send_raw
pub struct Message {
    pub(crate) buffer: Arc<MessageBuffer>,
    #[cfg(feature = "compression")]
    pub(crate) compressed: bool,
}

//Todo: Evaluate switching to VecDeque for quickly adding and removing data
// from front, back.
// - It would prob require custom bincode code but thats possible.
pub(crate) struct MessageBuffer {
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct OutgoingMessage {
    pub buffer: Arc<MessageBuffer>,
    pub cursor: u64,
    pub mid: Mid,
    pub sid: Sid,
}

#[derive(Debug)]
pub(crate) struct IncomingMessage {
    pub buffer: MessageBuffer,
    pub length: u64,
    pub mid: Mid,
    pub sid: Sid,
}

impl Message {
    /// This serializes any message, according to the [`Streams`] [`Promises`].
    /// You can reuse this `Message` and send it via other [`Streams`], if the
    /// [`Promises`] match. E.g. Sending a `Message` via a compressed and
    /// uncompressed Stream is dangerous, unless the remote site knows about
    /// this.
    ///
    /// # Example
    /// for example coding, see [`send_raw`]
    ///
    /// [`send_raw`]: Stream::send_raw
    /// [`Participants`]: crate::api::Participant
    /// [`compress`]: lz_fear::raw::compress2
    /// [`Message::serialize`]: crate::message::Message::serialize
    ///
    /// [`Streams`]: crate::api::Stream
    pub fn serialize<M: Serialize + ?Sized>(message: &M, stream: &Stream) -> Self {
        //this will never fail: https://docs.rs/bincode/0.8.0/bincode/fn.serialize.html
        let serialized_data = bincode::serialize(message).unwrap();

        #[cfg(feature = "compression")]
        let compressed = stream.promises().contains(Promises::COMPRESSED);
        #[cfg(feature = "compression")]
        let data = if compressed {
            let mut compressed_data = Vec::with_capacity(serialized_data.len() / 4 + 10);
            let mut table = lz_fear::raw::U32Table::default();
            lz_fear::raw::compress2(&serialized_data, 0, &mut table, &mut compressed_data).unwrap();
            compressed_data
        } else {
            serialized_data
        };
        #[cfg(not(feature = "compression"))]
        let data = serialized_data;
        #[cfg(not(feature = "compression"))]
        let _stream = stream;

        Self {
            buffer: Arc::new(MessageBuffer { data }),
            #[cfg(feature = "compression")]
            compressed,
        }
    }

    /// deserialize this `Message`. This consumes the struct, as deserialization
    /// is only expected once. Use this when deserialize a [`recv_raw`]
    /// `Message`. If you are resending this message, deserialization might need
    /// to copy memory
    ///
    /// # Example
    /// ```
    /// # use veloren_network::{Network, ProtocolAddr, Pid};
    /// # use veloren_network::Promises;
    /// # use futures::executor::block_on;
    ///
    /// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// // Create a Network, listen on Port `2300` and wait for a Stream to be opened, then listen on it
    /// # let (network, f) = Network::new(Pid::new());
    /// # std::thread::spawn(f);
    /// # let (remote, fr) = Network::new(Pid::new());
    /// # std::thread::spawn(fr);
    /// # block_on(async {
    ///     # network.listen(ProtocolAddr::Tcp("127.0.0.1:2300".parse().unwrap())).await?;
    ///     # let remote_p = remote.connect(ProtocolAddr::Tcp("127.0.0.1:2300".parse().unwrap())).await?;
    ///     # let mut stream_p = remote_p.open(16, Promises::ORDERED | Promises::CONSISTENCY).await?;
    ///     # stream_p.send("Hello World");
    ///     # let participant_a = network.connected().await?;
    ///     let mut stream_a = participant_a.opened().await?;
    ///     //Recv  Message
    ///     let msg = stream_a.recv_raw().await?;
    ///     println!("Msg is {}", msg.deserialize::<String>()?);
    ///     # Ok(())
    /// # })
    /// # }
    /// ```
    ///
    /// [`recv_raw`]: crate::api::Stream::recv_raw
    pub fn deserialize<M: DeserializeOwned>(self) -> Result<M, StreamError> {
        #[cfg(not(feature = "compression"))]
        let uncompressed_data = match Arc::try_unwrap(self.buffer) {
            Ok(d) => d.data,
            Err(b) => b.data.clone(),
        };

        #[cfg(feature = "compression")]
        let uncompressed_data = if self.compressed {
            {
                let mut uncompressed_data = Vec::with_capacity(self.buffer.data.len() * 2);
                if let Err(e) = lz_fear::raw::decompress_raw(
                    &self.buffer.data,
                    &[0; 0],
                    &mut uncompressed_data,
                    usize::MAX,
                ) {
                    return Err(StreamError::Compression(e));
                }
                uncompressed_data
            }
        } else {
            match Arc::try_unwrap(self.buffer) {
                Ok(d) => d.data,
                Err(b) => b.data.clone(),
            }
        };

        match bincode::deserialize(uncompressed_data.as_slice()) {
            Ok(m) => Ok(m),
            Err(e) => Err(StreamError::Deserialize(e)),
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn verify(&self, stream: &Stream) {
        #[cfg(not(feature = "compression"))]
        let _stream = stream;
        #[cfg(feature = "compression")]
        if self.compressed != stream.promises().contains(Promises::COMPRESSED) {
            warn!(
                ?stream,
                "verify failed, msg is {} and it doesn't match with stream", self.compressed
            );
        }
    }
}

impl OutgoingMessage {
    pub(crate) const FRAME_DATA_SIZE: u64 = 1400;

    /// returns if msg is empty
    pub(crate) fn fill_next<E: Extend<(Sid, Frame)>>(
        &mut self,
        msg_sid: Sid,
        frames: &mut E,
    ) -> bool {
        let to_send = std::cmp::min(
            self.buffer.data[self.cursor as usize..].len() as u64,
            Self::FRAME_DATA_SIZE,
        );
        if to_send > 0 {
            if self.cursor == 0 {
                frames.extend(std::iter::once((msg_sid, Frame::DataHeader {
                    mid: self.mid,
                    sid: self.sid,
                    length: self.buffer.data.len() as u64,
                })));
            }
            frames.extend(std::iter::once((msg_sid, Frame::Data {
                mid: self.mid,
                start: self.cursor,
                data: self.buffer.data[self.cursor as usize..][..to_send as usize].to_vec(),
            })));
        };
        self.cursor += to_send;
        self.cursor >= self.buffer.data.len() as u64
    }
}

///wouldn't trust this aaaassss much, fine for tests
pub(crate) fn partial_eq_io_error(first: &io::Error, second: &io::Error) -> bool {
    if let Some(f) = first.raw_os_error() {
        if let Some(s) = second.raw_os_error() {
            f == s
        } else {
            false
        }
    } else {
        let fk = first.kind();
        fk == second.kind() && fk != io::ErrorKind::Other
    }
}

pub(crate) fn partial_eq_bincode(first: &bincode::ErrorKind, second: &bincode::ErrorKind) -> bool {
    use bincode::ErrorKind::*;
    match *first {
        Io(ref f) => matches!(*second, Io(ref s) if partial_eq_io_error(f, s)),
        InvalidUtf8Encoding(f) => matches!(*second, InvalidUtf8Encoding(s) if f == s),
        InvalidBoolEncoding(f) => matches!(*second, InvalidBoolEncoding(s) if f == s),
        InvalidCharEncoding => matches!(*second, InvalidCharEncoding),
        InvalidTagEncoding(f) => matches!(*second, InvalidTagEncoding(s) if f == s),
        DeserializeAnyNotSupported => matches!(*second, DeserializeAnyNotSupported),
        SizeLimit => matches!(*second, SizeLimit),
        SequenceMustHaveLength => matches!(*second, SequenceMustHaveLength),
        Custom(ref f) => matches!(*second, Custom(ref s) if f == s),
    }
}

impl std::fmt::Debug for MessageBuffer {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //TODO: small messages!
        let len = self.data.len();
        if len > 20 {
            write!(
                f,
                "MessageBuffer(len: {}, {}, {}, {}, {:X?}..{:X?})",
                len,
                u32::from_le_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]),
                u32::from_le_bytes([self.data[4], self.data[5], self.data[6], self.data[7]]),
                u32::from_le_bytes([self.data[8], self.data[9], self.data[10], self.data[11]]),
                &self.data[13..16],
                &self.data[len - 8..len]
            )
        } else {
            write!(f, "MessageBuffer(len: {}, {:?})", len, &self.data[..])
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{api::Stream, message::*};
    use futures::channel::mpsc;
    use std::sync::{atomic::AtomicBool, Arc};

    fn stub_stream(compressed: bool) -> Stream {
        use crate::{api::*, types::*};

        #[cfg(feature = "compression")]
        let promises = if compressed {
            Promises::COMPRESSED
        } else {
            Promises::empty()
        };

        #[cfg(not(feature = "compression"))]
        let promises = Promises::empty();

        let (a2b_msg_s, _a2b_msg_r) = crossbeam_channel::unbounded();
        let (_b2a_msg_recv_s, b2a_msg_recv_r) = mpsc::unbounded();
        let (a2b_close_stream_s, _a2b_close_stream_r) = mpsc::unbounded();

        Stream::new(
            Pid::fake(0),
            Sid::new(0),
            0u8,
            promises,
            Arc::new(AtomicBool::new(true)),
            a2b_msg_s,
            b2a_msg_recv_r,
            a2b_close_stream_s,
        )
    }

    #[test]
    fn serialize_test() {
        let msg = Message::serialize("abc", &stub_stream(false));
        assert_eq!(msg.buffer.data.len(), 11);
        assert_eq!(msg.buffer.data[0], 3);
        assert_eq!(msg.buffer.data[1..7], [0, 0, 0, 0, 0, 0]);
        assert_eq!(msg.buffer.data[8], b'a');
        assert_eq!(msg.buffer.data[9], b'b');
        assert_eq!(msg.buffer.data[10], b'c');
    }

    #[cfg(feature = "compression")]
    #[test]
    fn serialize_compress_small() {
        let msg = Message::serialize("abc", &stub_stream(true));
        assert_eq!(msg.buffer.data.len(), 12);
        assert_eq!(msg.buffer.data[0], 176);
        assert_eq!(msg.buffer.data[1], 3);
        assert_eq!(msg.buffer.data[2..8], [0, 0, 0, 0, 0, 0]);
        assert_eq!(msg.buffer.data[9], b'a');
        assert_eq!(msg.buffer.data[10], b'b');
        assert_eq!(msg.buffer.data[11], b'c');
    }

    #[cfg(feature = "compression")]
    #[test]
    fn serialize_compress_medium() {
        let msg = (
            "abccc",
            100u32,
            80u32,
            "DATA",
            4,
            0,
            0,
            0,
            "assets/data/plants/flowers/greenrose.ron",
        );
        let msg = Message::serialize(&msg, &stub_stream(true));
        assert_eq!(msg.buffer.data.len(), 79);
        assert_eq!(msg.buffer.data[0], 34);
        assert_eq!(msg.buffer.data[1], 5);
        assert_eq!(msg.buffer.data[2], 0);
        assert_eq!(msg.buffer.data[3], 1);
        assert_eq!(msg.buffer.data[20], 20);
        assert_eq!(msg.buffer.data[40], 115);
        assert_eq!(msg.buffer.data[60], 111);
    }

    #[cfg(feature = "compression")]
    #[test]
    fn serialize_compress_large() {
        use rand::{Rng, SeedableRng};
        let mut seed = [0u8; 32];
        seed[8] = 13;
        seed[9] = 37;
        let mut rnd = rand::rngs::StdRng::from_seed(seed);
        let mut msg = vec![0u8; 10000];
        for (i, s) in msg.iter_mut().enumerate() {
            match i.rem_euclid(32) {
                2 => *s = 128,
                3 => *s = 128 + 16,
                4 => *s = 150,
                11 => *s = 64,
                12 => *s = rnd.gen::<u8>() / 32,
                _ => {},
            }
        }
        let msg = Message::serialize(&msg, &stub_stream(true));
        assert_eq!(msg.buffer.data.len(), 1331);
    }
}
