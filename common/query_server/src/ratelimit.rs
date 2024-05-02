use std::{
    collections::HashMap,
    net::IpAddr,
    time::{Duration, Instant},
};

const SHIFT_EVERY: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReducedIpAddr {
    V4(u32),
    V6(u64),
}

/// Per-IP state, divided into 4 segments of [`SHIFT_EVERY`] each (one minute at
/// the time of writing).
pub struct IpState([u16; 4]);

pub struct RateLimiter {
    states: HashMap<ReducedIpAddr, IpState>,
    last_shift: Instant,
    /// Maximum amount requests that can be done in `4 * SHIFT_EVERY`
    limit: u16,
}

impl RateLimiter {
    pub fn new(limit: u16) -> Self {
        Self {
            states: Default::default(),
            last_shift: Instant::now(),
            limit,
        }
    }

    pub fn maintain(&mut self, now: Instant) {
        if now.duration_since(self.last_shift) > SHIFT_EVERY {
            // Remove empty states
            self.states.retain(|_, state| {
                state.shift();
                !state.is_empty()
            });
            self.last_shift = now;
        }
    }

    pub fn can_request(&mut self, ip: ReducedIpAddr) -> bool {
        if let Some(state) = self.states.get_mut(&ip) {
            state.0[0] = state.0[0].saturating_add(1);

            state.total() < self.limit
        } else {
            self.states.insert(ip, IpState::default());
            true
        }
    }
}

impl IpState {
    fn shift(&mut self) {
        self.0.rotate_right(1);
        self.0[0] = 0;
    }

    fn is_empty(&self) -> bool { self.0.iter().all(|&freq| freq == 0) }

    fn total(&self) -> u16 { self.0.iter().fold(0, |total, &v| total.saturating_add(v)) }
}

impl Default for IpState {
    fn default() -> Self { Self([1, 0, 0, 0]) }
}

impl From<IpAddr> for ReducedIpAddr {
    fn from(value: IpAddr) -> Self {
        match value {
            IpAddr::V4(v4) => Self::V4(u32::from_be_bytes(v4.octets())),
            IpAddr::V6(v6) => {
                let bytes = v6.octets();
                Self::V6(u64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]))
            },
        }
    }
}
