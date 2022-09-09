use crate::api::{StreamError, StreamParams};
use bytes::Bytes;
#[cfg(feature = "compression")]
use network_protocol::Promises;
use serde::{de::DeserializeOwned, Serialize};
use std::io;
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
    pub(crate) data: Bytes,
    #[cfg(feature = "compression")]
    pub(crate) compressed: bool,
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
    /// [`send_raw`]: crate::api::Stream::send_raw
    /// [`Participants`]: crate::api::Participant
    /// [`compress`]: lz_fear::raw::compress2
    /// [`Message::serialize`]: crate::message::Message::serialize
    ///
    /// [`Streams`]: crate::api::Stream
    pub fn serialize<M: Serialize + ?Sized>(message: &M, stream_params: StreamParams) -> Self {
        //this will never fail: https://docs.rs/bincode/0.8.0/bincode/fn.serialize.html
        let serialized_data = bincode::serialize(message).unwrap();

        #[cfg(feature = "compression")]
        let compressed = stream_params.promises.contains(Promises::COMPRESSED);
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
        let _stream_params = stream_params;

        Self {
            data: Bytes::from(data),
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
    /// # use veloren_network::{Network, ListenAddr, ConnectAddr, Pid};
    /// # use veloren_network::Promises;
    /// # use tokio::runtime::Runtime;
    /// # use std::sync::Arc;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create a Network, listen on Port `2300` and wait for a Stream to be opened, then listen on it
    /// # let runtime = Runtime::new().unwrap();
    /// # let mut network = Network::new(Pid::new(), &runtime);
    /// # let remote = Network::new(Pid::new(), &runtime);
    /// # runtime.block_on(async {
    ///     # network.listen(ListenAddr::Tcp("127.0.0.1:2300".parse().unwrap())).await?;
    ///     # let remote_p = remote.connect(ConnectAddr::Tcp("127.0.0.1:2300".parse().unwrap())).await?;
    ///     # let mut stream_p = remote_p.open(4, Promises::ORDERED | Promises::CONSISTENCY, 0).await?;
    ///     # stream_p.send("Hello World");
    ///     # let mut participant_a = network.connected().await?;
    ///     let mut stream_a = participant_a.opened().await?;
    ///     //Recv  Message
    ///     let msg = stream_a.recv_raw().await?;
    ///     println!("Msg is {}", msg.deserialize::<String>()?);
    ///     drop(network);
    ///     # drop(remote);
    ///     # Ok(())
    /// # })
    /// # }
    /// ```
    ///
    /// [`recv_raw`]: crate::api::Stream::recv_raw
    pub fn deserialize<M: DeserializeOwned>(self) -> Result<M, StreamError> {
        #[cfg(not(feature = "compression"))]
        let uncompressed_data = self.data;

        #[cfg(feature = "compression")]
        let uncompressed_data = if self.compressed {
            {
                let mut uncompressed_data = Vec::with_capacity(self.data.len() * 2);
                if let Err(e) = lz_fear::raw::decompress_raw(
                    &self.data,
                    &[0; 0],
                    &mut uncompressed_data,
                    usize::MAX,
                ) {
                    return Err(StreamError::Compression(e));
                }
                Bytes::from(uncompressed_data)
            }
        } else {
            self.data
        };

        match bincode::deserialize(&uncompressed_data) {
            Ok(m) => Ok(m),
            Err(e) => Err(StreamError::Deserialize(e)),
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn verify(&self, params: StreamParams) {
        #[cfg(not(feature = "compression"))]
        let _params = params;
        #[cfg(feature = "compression")]
        if self.compressed != params.promises.contains(Promises::COMPRESSED) {
            warn!(
                ?params,
                "verify failed, msg is {} and it doesn't match with stream", self.compressed
            );
        }
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

#[cfg(test)]
mod tests {
    use crate::{api::StreamParams, message::*};

    fn stub_stream(compressed: bool) -> StreamParams {
        #[cfg(feature = "compression")]
        let promises = if compressed {
            Promises::COMPRESSED
        } else {
            Promises::empty()
        };

        #[cfg(not(feature = "compression"))]
        let promises = Promises::empty();

        StreamParams { promises }
    }

    #[test]
    fn serialize_test() {
        let msg = Message::serialize("abc", stub_stream(false));
        assert_eq!(msg.data.len(), 11);
        assert_eq!(msg.data[0], 3);
        assert_eq!(msg.data[1..7], [0, 0, 0, 0, 0, 0]);
        assert_eq!(msg.data[8], b'a');
        assert_eq!(msg.data[9], b'b');
        assert_eq!(msg.data[10], b'c');
    }

    #[cfg(feature = "compression")]
    #[test]
    fn serialize_compress_small() {
        let msg = Message::serialize("abc", stub_stream(true));
        assert_eq!(msg.data.len(), 12);
        assert_eq!(msg.data[0], 176);
        assert_eq!(msg.data[1], 3);
        assert_eq!(msg.data[2..8], [0, 0, 0, 0, 0, 0]);
        assert_eq!(msg.data[9], b'a');
        assert_eq!(msg.data[10], b'b');
        assert_eq!(msg.data[11], b'c');
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
        let msg = Message::serialize(&msg, stub_stream(true));
        assert_eq!(msg.data.len(), 79);
        assert_eq!(msg.data[0], 34);
        assert_eq!(msg.data[1], 5);
        assert_eq!(msg.data[2], 0);
        assert_eq!(msg.data[3], 1);
        assert_eq!(msg.data[20], 20);
        assert_eq!(msg.data[40], 115);
        assert_eq!(msg.data[60], 111);
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
        let msg = Message::serialize(&msg, stub_stream(true));
        assert_eq!(msg.data.len(), 1331);
    }
}
