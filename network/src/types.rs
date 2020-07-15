use rand::Rng;

pub type Mid = u64;
pub type Cid = u64;
pub type Prio = u8;
/// use promises to modify the behavior of [`Streams`].
/// available promises are:
/// * [`PROMISES_NONE`]
/// * [`PROMISES_ORDERED`]
/// * [`PROMISES_CONSISTENCY`]
/// * [`PROMISES_GUARANTEED_DELIVERY`]
/// * [`PROMISES_COMPRESSED`]
/// * [`PROMISES_ENCRYPTED`]
///
/// [`Streams`]: crate::api::Stream
pub type Promises = u8;

/// use for no special promises on this [`Stream`](crate::api::Stream).
pub const PROMISES_NONE: Promises = 0;
/// this will guarantee that the order of messages which are send on one side,
/// is the same when received on the other.
pub const PROMISES_ORDERED: Promises = 1;
/// this will guarantee that messages received haven't been altered by errors,
/// like bit flips, this is done with a checksum.
pub const PROMISES_CONSISTENCY: Promises = 2;
/// this will guarantee that the other side will receive every message exactly
/// once no messages are droped
pub const PROMISES_GUARANTEED_DELIVERY: Promises = 4;
/// this will enable the internal compression on this
/// [`Stream`](crate::api::Stream)
pub const PROMISES_COMPRESSED: Promises = 8;
/// this will enable the internal encryption on this
/// [`Stream`](crate::api::Stream)
pub const PROMISES_ENCRYPTED: Promises = 16;

pub(crate) const VELOREN_MAGIC_NUMBER: [u8; 7] = [86, 69, 76, 79, 82, 69, 78]; //VELOREN
pub const VELOREN_NETWORK_VERSION: [u32; 3] = [0, 4, 0];
pub(crate) const STREAM_ID_OFFSET1: Sid = Sid::new(0);
pub(crate) const STREAM_ID_OFFSET2: Sid = Sid::new(u64::MAX / 2);

/// Support struct used for uniquely identifying [`Participant`] over the
/// [`Network`].
///
/// [`Participant`]: crate::api::Participant
/// [`Network`]: crate::api::Network
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Pid {
    internal: u128,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) struct Sid {
    internal: u64,
}

// Used for Communication between Channel <----(TCP/UDP)----> Channel
#[derive(Debug)]
pub(crate) enum Frame {
    Handshake {
        magic_number: [u8; 7],
        version: [u32; 3],
    },
    Init {
        pid: Pid,
        secret: u128,
    },
    Shutdown, /* Shutsdown this channel gracefully, if all channels are shut down, Participant
               * is deleted */
    OpenStream {
        sid: Sid,
        prio: Prio,
        promises: Promises,
    },
    CloseStream {
        sid: Sid,
    },
    DataHeader {
        mid: Mid,
        sid: Sid,
        length: u64,
    },
    Data {
        mid: Mid,
        start: u64,
        data: Vec<u8>,
    },
    /* WARNING: Sending RAW is only used for debug purposes in case someone write a new API
     * against veloren Server! */
    Raw(Vec<u8>),
}

impl Frame {
    #[cfg(feature = "metrics")]
    pub const FRAMES_LEN: u8 = 8;

    #[cfg(feature = "metrics")]
    pub const fn int_to_string(i: u8) -> &'static str {
        match i {
            0 => "Handshake",
            1 => "Init",
            2 => "Shutdown",
            3 => "OpenStream",
            4 => "CloseStream",
            5 => "DataHeader",
            6 => "Data",
            7 => "Raw",
            _ => "",
        }
    }

    #[cfg(feature = "metrics")]
    pub fn get_int(&self) -> u8 {
        match self {
            Frame::Handshake { .. } => 0,
            Frame::Init { .. } => 1,
            Frame::Shutdown => 2,
            Frame::OpenStream { .. } => 3,
            Frame::CloseStream { .. } => 4,
            Frame::DataHeader { .. } => 5,
            Frame::Data { .. } => 6,
            Frame::Raw(_) => 7,
        }
    }

    #[cfg(feature = "metrics")]
    pub fn get_string(&self) -> &str { Self::int_to_string(self.get_int()) }
}

impl Pid {
    /// create a new Pid with a random interior value
    ///
    /// # Example
    /// ```rust
    /// use veloren_network::{Network, Pid};
    ///
    /// let pid = Pid::new();
    /// let _ = Network::new(pid);
    /// ```
    pub fn new() -> Self {
        Self {
            internal: rand::thread_rng().gen(),
        }
    }

    /// don't use fake! just for testing!
    /// This will panic if pid i greater than 7, as I do not want you to use
    /// this in production!
    #[doc(hidden)]
    pub fn fake(pid: u8) -> Self {
        assert!(pid < 8);
        Self {
            internal: pid as u128,
        }
    }

    pub(crate) fn to_le_bytes(&self) -> [u8; 16] { self.internal.to_le_bytes() }

    pub(crate) fn from_le_bytes(bytes: [u8; 16]) -> Self {
        Self {
            internal: u128::from_le_bytes(bytes),
        }
    }
}

impl Sid {
    pub const fn new(internal: u64) -> Self { Self { internal } }

    pub(crate) fn to_le_bytes(&self) -> [u8; 8] { self.internal.to_le_bytes() }

    pub(crate) fn from_le_bytes(bytes: [u8; 8]) -> Self {
        Self {
            internal: u64::from_le_bytes(bytes),
        }
    }
}

impl std::fmt::Debug for Pid {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const BITS_PER_SIXLET: usize = 6;
        //only print last 6 chars of number as full u128 logs are unreadable
        const CHAR_COUNT: usize = 6;
        for i in 0..CHAR_COUNT {
            write!(
                f,
                "{}",
                sixlet_to_str((self.internal >> (i * BITS_PER_SIXLET)) & 0x3F)
            )?;
        }
        Ok(())
    }
}

impl Default for Pid {
    fn default() -> Self { Pid::new() }
}

impl std::fmt::Display for Pid {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
}

impl std::ops::AddAssign for Sid {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            internal: self.internal + other.internal,
        };
    }
}

impl std::fmt::Debug for Sid {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //only print last 6 chars of number as full u128 logs are unreadable
        write!(f, "{}", self.internal.rem_euclid(1000000))
    }
}

impl From<u64> for Sid {
    fn from(internal: u64) -> Self { Sid { internal } }
}

impl std::fmt::Display for Sid {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.internal)
    }
}

fn sixlet_to_str(sixlet: u128) -> char {
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"[sixlet as usize] as char
}

#[cfg(test)]
mod tests {
    use crate::types::*;

    #[test]
    fn frame_int2str() {
        assert_eq!(Frame::int_to_string(3), "OpenStream");
        assert_eq!(Frame::int_to_string(7), "Raw");
        assert_eq!(Frame::int_to_string(8), "");
    }

    #[test]
    fn frame_get_int() {
        assert_eq!(Frame::get_int(&Frame::Raw(b"Foo".to_vec())), 7);
        assert_eq!(Frame::get_int(&Frame::Shutdown), 2);
    }

    #[test]
    fn frame_creation() {
        Pid::new();
        assert_eq!(format!("{}", Pid::fake(2)), "CAAAAA");
    }

    #[test]
    fn test_sixlet_to_str() {
        assert_eq!(sixlet_to_str(0), 'A');
        assert_eq!(sixlet_to_str(29), 'd');
        assert_eq!(sixlet_to_str(63), '/');
    }
}
