use crate::{
    frame::OTFrame,
    message::OTMessage,
    metrics::{ProtocolMetricCache, RemoveReason},
    types::{Bandwidth, Mid, Prio, Promises, Sid, HIGHEST_PRIO},
};
use bytes::Bytes;
use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};

#[derive(Debug)]
struct StreamInfo {
    pub(crate) guaranteed_bandwidth: Bandwidth,
    pub(crate) prio: Prio,
    #[allow(dead_code)]
    pub(crate) promises: Promises,
    pub(crate) messages: VecDeque<OTMessage>,
}

/// Responsible for queueing messages.
/// every stream has a guaranteed bandwidth and a prio 0-7.
/// when `n` Bytes are available in the buffer, first the guaranteed bandwidth
/// is used. Then remaining bandwidth is used to fill up the prios.
#[derive(Debug)]
pub(crate) struct PrioManager {
    streams: HashMap<Sid, StreamInfo>,
    metrics: ProtocolMetricCache,
}

// Send everything ONCE, then keep it till it's confirmed

impl PrioManager {
    pub fn new(metrics: ProtocolMetricCache) -> Self {
        Self {
            streams: HashMap::new(),
            metrics,
        }
    }

    pub fn open_stream(
        &mut self,
        sid: Sid,
        prio: Prio,
        promises: Promises,
        guaranteed_bandwidth: Bandwidth,
    ) {
        self.streams.insert(sid, StreamInfo {
            guaranteed_bandwidth,
            prio,
            promises,
            messages: VecDeque::new(),
        });
    }

    pub fn try_close_stream(&mut self, sid: Sid) -> bool {
        if let Some(si) = self.streams.get(&sid) {
            if si.messages.is_empty() {
                self.streams.remove(&sid);
                return true;
            }
        }
        false
    }

    pub fn is_empty(&self) -> bool { self.streams.is_empty() }

    pub fn add(&mut self, buffer: Bytes, mid: Mid, sid: Sid) {
        self.streams
            .get_mut(&sid)
            .unwrap()
            .messages
            .push_back(OTMessage::new(buffer, mid, sid));
    }

    /// bandwidth might be extended, as for technical reasons
    /// guaranteed_bandwidth is used and frames are always 1400 bytes.
    pub fn grab(&mut self, bandwidth: Bandwidth, dt: Duration) -> (Vec<(Sid, OTFrame)>, Bandwidth) {
        let total_bytes = (bandwidth as f64 * dt.as_secs_f64()) as u64;
        let mut cur_bytes = 0u64;
        let mut frames = vec![];

        let mut prios = [0u64; (HIGHEST_PRIO + 1) as usize];
        let metrics = &mut self.metrics;

        let mut process_stream =
            |sid: &Sid, stream: &mut StreamInfo, mut bandwidth: i64, cur_bytes: &mut u64| {
                let mut finished = None;
                'outer: for (i, msg) in stream.messages.iter_mut().enumerate() {
                    while let Some(frame) = msg.next() {
                        let b = if let OTFrame::Data { data, .. } = &frame {
                            crate::frame::TCP_DATA_CNS + 1 + data.len()
                        } else {
                            crate::frame::TCP_DATA_HEADER_CNS + 1
                        } as u64;
                        bandwidth -= b as i64;
                        *cur_bytes += b;
                        frames.push((*sid, frame));
                        if bandwidth <= 0 {
                            break 'outer;
                        }
                    }
                    let (sid, bytes) = msg.get_sid_len();
                    metrics.smsg_ob(sid, RemoveReason::Finished, bytes);
                    finished = Some(i);
                }
                if let Some(i) = finished {
                    //cleanup
                    stream.messages.drain(..=i);
                }
            };

        // Add guaranteed bandwidth
        for (sid, stream) in self.streams.iter_mut() {
            prios[stream.prio as usize] += 1;
            let stream_byte_cnt = (stream.guaranteed_bandwidth as f64 * dt.as_secs_f64()) as u64;
            process_stream(sid, stream, stream_byte_cnt as i64, &mut cur_bytes);
        }

        if cur_bytes < total_bytes {
            // Add optional bandwidth
            for prio in 0..=HIGHEST_PRIO {
                if prios[prio as usize] == 0 {
                    continue;
                }
                let per_stream_bytes = ((total_bytes - cur_bytes) / prios[prio as usize]) as i64;
                for (sid, stream) in self.streams.iter_mut() {
                    if stream.prio != prio {
                        continue;
                    }
                    process_stream(sid, stream, per_stream_bytes, &mut cur_bytes);
                    if cur_bytes >= total_bytes {
                        break;
                    }
                }
            }
        }
        (frames, cur_bytes)
    }
}
