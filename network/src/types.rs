use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::*;

pub type Sid = u64;
pub type Mid = u64;
pub type Cid = u64;
pub type Prio = u8;
pub type Promises = u8;

pub const PROMISES_NONE: Promises = 0;
pub const PROMISES_ORDERED: Promises = 1;
pub const PROMISES_CONSISTENCY: Promises = 2;
pub const PROMISES_GUARANTEED_DELIVERY: Promises = 4;
pub const PROMISES_COMPRESSED: Promises = 8;
pub const PROMISES_ENCRYPTED: Promises = 16;

pub(crate) const VELOREN_MAGIC_NUMBER: &str = "VELOREN";
pub const VELOREN_NETWORK_VERSION: [u32; 3] = [0, 2, 0];
pub(crate) const STREAM_ID_OFFSET1: Sid = 0;
pub(crate) const STREAM_ID_OFFSET2: Sid = u64::MAX / 2;

pub(crate) struct NetworkBuffer {
    pub(crate) data: Vec<u8>,
    pub(crate) read_idx: usize,
    pub(crate) write_idx: usize,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub struct Pid {
    internal: u128,
}

impl Pid {
    pub fn new() -> Self {
        Self {
            internal: rand::thread_rng().gen(),
        }
    }

    /// don't use fake! just for testing!
    /// This will panic if pid i greater than 7, as i do not want you to use
    /// this in production!
    pub fn fake(pid: u8) -> Self {
        assert!(pid < 8);
        Self {
            internal: pid as u128,
        }
    }
}

/// NetworkBuffer to use for streamed access
/// valid data is between read_idx and write_idx!
/// everything before read_idx is already processed and no longer important
/// everything after write_idx is either 0 or random data buffered
impl NetworkBuffer {
    pub(crate) fn new() -> Self {
        NetworkBuffer {
            data: vec![0; 2048],
            read_idx: 0,
            write_idx: 0,
        }
    }

    pub(crate) fn get_write_slice(&mut self, min_size: usize) -> &mut [u8] {
        if self.data.len() < self.write_idx + min_size {
            trace!(
                ?self,
                ?min_size,
                "need to resize because buffer is to small"
            );
            self.data.resize(self.write_idx + min_size, 0);
        }
        &mut self.data[self.write_idx..]
    }

    pub(crate) fn actually_written(&mut self, cnt: usize) { self.write_idx += cnt; }

    pub(crate) fn get_read_slice(&self) -> &[u8] { &self.data[self.read_idx..self.write_idx] }

    pub(crate) fn actually_read(&mut self, cnt: usize) {
        self.read_idx += cnt;
        if self.read_idx == self.write_idx {
            if self.read_idx > 10485760 {
                trace!(?self, "buffer empty, resetting indices");
            }
            self.read_idx = 0;
            self.write_idx = 0;
        }
        if self.write_idx > 10485760 {
            if self.write_idx - self.read_idx < 65536 {
                debug!(
                    ?self,
                    "This buffer is filled over 10 MB, but the actual data diff is less then \
                     65kB, which is a sign of stressing this connection much as always new data \
                     comes in - nevertheless, in order to handle this we will remove some data \
                     now so that this buffer doesn't grow endlessly"
                );
                let mut i2 = 0;
                for i in self.read_idx..self.write_idx {
                    self.data[i2] = self.data[i];
                    i2 += 1;
                }
                self.read_idx = 0;
                self.write_idx = i2;
            }
            if self.data.len() > 67108864 {
                warn!(
                    ?self,
                    "over 64Mbyte used, something seems fishy, len: {}",
                    self.data.len()
                );
            }
        }
    }
}

impl std::fmt::Debug for NetworkBuffer {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NetworkBuffer(len: {}, read: {}, write: {})",
            self.data.len(),
            self.read_idx,
            self.write_idx
        )
    }
}

impl std::fmt::Debug for Pid {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.internal)
    }
}
