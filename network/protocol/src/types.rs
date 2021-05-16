use bitflags::bitflags;
use bytes::{Buf, BufMut, BytesMut};
use rand::Rng;

/// MessageID, unique ID per Message.
pub type Mid = u64;
/// ChannelID, unique ID per Channel (Protocol)
pub type Cid = u64;
/// Every Stream has a `Prio` and guaranteed [`Bandwidth`].
/// Every send, the guarantees part is used first.
/// If there is still bandwidth left, it will be shared by all Streams with the
/// same priority. Prio 0 will be send first, then 1, ... till the last prio 7
/// is send. Prio must be < 8!
///
/// [`Bandwidth`]: crate::Bandwidth
pub type Prio = u8;
/// guaranteed `Bandwidth`. See [`Prio`]
///
/// [`Prio`]: crate::Prio
pub type Bandwidth = u64;

bitflags! {
    /// use promises to modify the behavior of [`Streams`].
    /// see the consts in this `struct` for
    ///
    /// [`Streams`]: crate::api::Stream
    pub struct Promises: u8 {
        /// this will guarantee that the order of messages which are send on one side,
        /// is the same when received on the other.
        const ORDERED = 0b00000001;
        /// this will guarantee that messages received haven't been altered by errors,
        /// like bit flips, this is done with a checksum.
        const CONSISTENCY = 0b00000010;
        /// this will guarantee that the other side will receive every message exactly
        /// once no messages are dropped
        const GUARANTEED_DELIVERY = 0b00000100;
        /// this will enable the internal compression on this, only useable with #[cfg(feature = "compression")]
        /// [`Stream`](crate::api::Stream)
        const COMPRESSED = 0b00001000;
        /// this will enable the internal encryption on this
        /// [`Stream`](crate::api::Stream)
        const ENCRYPTED = 0b00010000;
    }
}

impl Promises {
    pub const fn to_le_bytes(self) -> [u8; 1] { self.bits.to_le_bytes() }
}

pub(crate) const VELOREN_MAGIC_NUMBER: [u8; 7] = *b"VELOREN";
/// When this semver differs, 2 Networks can't communicate.
pub const VELOREN_NETWORK_VERSION: [u32; 3] = [0, 6, 0];
pub(crate) const STREAM_ID_OFFSET1: Sid = Sid::new(0);
pub(crate) const STREAM_ID_OFFSET2: Sid = Sid::new(u64::MAX / 2);
/// Maximal possible Prio to choose (for performance reasons)
pub const HIGHEST_PRIO: u8 = 7;

/// Support struct used for uniquely identifying `Participant` over the
/// `Network`.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Pid {
    internal: u128,
}

/// Unique ID per Stream, in one Channel.
/// one side will always start with 0, while the other start with u64::MAX / 2.
/// number increases for each created Stream.
#[derive(PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct Sid {
    internal: u64,
}

impl Pid {
    /// create a new Pid with a random interior value
    ///
    /// # Example
    /// ```rust
    /// use veloren_network_protocol::Pid;
    ///
    /// let pid = Pid::new();
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
    pub fn fake(pid_offset: u8) -> Self {
        assert!(pid_offset < 8);
        let o = pid_offset as u128;
        const OFF: [u128; 5] = [
            0x40,
            0x40 * 0x40,
            0x40 * 0x40 * 0x40,
            0x40 * 0x40 * 0x40 * 0x40,
            0x40 * 0x40 * 0x40 * 0x40 * 0x40,
        ];
        Self {
            internal: o + o * OFF[0] + o * OFF[1] + o * OFF[2] + o * OFF[3] + o * OFF[4],
        }
    }

    #[inline]
    pub(crate) fn from_bytes(bytes: &mut BytesMut) -> Self {
        Self {
            internal: bytes.get_u128_le(),
        }
    }

    #[inline]
    pub(crate) fn to_bytes(self, bytes: &mut BytesMut) { bytes.put_u128_le(self.internal) }
}

impl Sid {
    pub const fn new(internal: u64) -> Self { Self { internal } }

    pub fn get_u64(&self) -> u64 { self.internal }

    #[inline]
    pub(crate) fn from_bytes(bytes: &mut BytesMut) -> Self {
        Self {
            internal: bytes.get_u64_le(),
        }
    }

    #[inline]
    pub(crate) fn to_bytes(self, bytes: &mut BytesMut) { bytes.put_u64_le(self.internal) }
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
    use super::*;

    #[test]
    fn frame_creation() {
        Pid::new();
        assert_eq!(format!("{}", Pid::fake(0)), "AAAAAA");
        assert_eq!(format!("{}", Pid::fake(1)), "BBBBBB");
        assert_eq!(format!("{}", Pid::fake(2)), "CCCCCC");
    }

    #[test]
    fn test_sixlet_to_str() {
        assert_eq!(sixlet_to_str(0), 'A');
        assert_eq!(sixlet_to_str(29), 'd');
        assert_eq!(sixlet_to_str(63), '/');
    }
}
