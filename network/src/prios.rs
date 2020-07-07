//!Priorities are handled the following way.
//!Prios from 0-63 are allowed.
//!all 5 numbers the throughput is halved.
//!E.g. in the same time 100 prio0 messages are send, only 50 prio5, 25 prio10,
//! 12 prio15 or 6 prio20 messages are send. Note: TODO: prio0 will be send
//! immeadiatly when found!

use crate::{
    message::OutgoingMessage,
    metrics::NetworkMetrics,
    types::{Frame, Prio, Sid},
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use futures::channel::oneshot;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use tracing::*;

const PRIO_MAX: usize = 64;

#[derive(Default)]
struct PidSidInfo {
    len: u64,
    empty_notify: Option<oneshot::Sender<()>>,
}

pub(crate) struct PrioManager {
    points: [u32; PRIO_MAX],
    messages: [VecDeque<(Sid, OutgoingMessage)>; PRIO_MAX],
    messages_rx: Receiver<(Prio, Sid, OutgoingMessage)>,
    sid_owned: HashMap<Sid, PidSidInfo>,
    //you can register to be notified if a pid_sid combination is flushed completly here
    sid_flushed_rx: Receiver<(Sid, oneshot::Sender<()>)>,
    queued: HashSet<u8>,
    metrics: Arc<NetworkMetrics>,
    pid: String,
}

impl PrioManager {
    const FRAME_DATA_SIZE: u64 = 1400;
    const PRIOS: [u32; PRIO_MAX] = [
        100, 115, 132, 152, 174, 200, 230, 264, 303, 348, 400, 459, 528, 606, 696, 800, 919, 1056,
        1213, 1393, 1600, 1838, 2111, 2425, 2786, 3200, 3676, 4222, 4850, 5572, 6400, 7352, 8445,
        9701, 11143, 12800, 14703, 16890, 19401, 22286, 25600, 29407, 33779, 38802, 44572, 51200,
        58813, 67559, 77605, 89144, 102400, 117627, 135118, 155209, 178289, 204800, 235253, 270235,
        310419, 356578, 409600, 470507, 540470, 620838,
    ];

    #[allow(clippy::type_complexity)]
    pub fn new(
        metrics: Arc<NetworkMetrics>,
        pid: String,
    ) -> (
        Self,
        Sender<(Prio, Sid, OutgoingMessage)>,
        Sender<(Sid, oneshot::Sender<()>)>,
    ) {
        // (a2p_msg_s, a2p_msg_r)
        let (messages_tx, messages_rx) = unbounded();
        let (sid_flushed_tx, sid_flushed_rx) = unbounded();
        (
            Self {
                points: [0; PRIO_MAX],
                messages: [
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                    VecDeque::new(),
                ],
                messages_rx,
                queued: HashSet::new(), //TODO: optimize with u64 and 64 bits
                sid_flushed_rx,
                sid_owned: HashMap::new(),
                metrics,
                pid,
            },
            messages_tx,
            sid_flushed_tx,
        )
    }

    async fn tick(&mut self) {
        // Check Range
        let mut messages = 0;
        let mut closed = 0;
        for (prio, sid, msg) in self.messages_rx.try_iter() {
            debug_assert!(prio as usize <= PRIO_MAX);
            messages += 1;
            let sid_string = sid.to_string();
            self.metrics
                .message_out_total
                .with_label_values(&[&self.pid, &sid_string])
                .inc();
            self.metrics
                .message_out_throughput
                .with_label_values(&[&self.pid, &sid_string])
                .inc_by(msg.buffer.data.len() as i64);
            //trace!(?prio, ?sid_string, "tick");
            self.queued.insert(prio);
            self.messages[prio as usize].push_back((sid, msg));
            self.sid_owned.entry(sid).or_default().len += 1;
        }
        //this must be AFTER messages
        for (sid, return_sender) in self.sid_flushed_rx.try_iter() {
            closed += 1;
            if let Some(cnt) = self.sid_owned.get_mut(&sid) {
                // register sender
                cnt.empty_notify = Some(return_sender);
            } else {
                // return immediately
                return_sender.send(()).unwrap();
            }
        }
        if messages > 0 || closed > 0 {
            trace!(?messages, ?closed, "tick");
        }
    }

    //if None returned, we are empty!
    fn calc_next_prio(&self) -> Option<u8> {
        // compare all queued prios, max 64 operations
        let mut lowest = std::u32::MAX;
        let mut lowest_id = None;
        for &n in &self.queued {
            let n_points = self.points[n as usize];
            if n_points < lowest {
                lowest = n_points;
                lowest_id = Some(n)
            } else if n_points == lowest && lowest_id.is_some() && n < lowest_id.unwrap() {
                //on equial points lowest first!
                lowest_id = Some(n)
            }
        }
        lowest_id
        /*
        self.queued
            .iter()
            .min_by_key(|&n| self.points[*n as usize]).cloned()*/
    }

    /// returns if msg is empty
    fn tick_msg<E: Extend<(Sid, Frame)>>(
        msg: &mut OutgoingMessage,
        msg_sid: Sid,
        frames: &mut E,
    ) -> bool {
        let to_send = std::cmp::min(
            msg.buffer.data[msg.cursor as usize..].len() as u64,
            Self::FRAME_DATA_SIZE,
        );
        if to_send > 0 {
            if msg.cursor == 0 {
                frames.extend(std::iter::once((msg_sid, Frame::DataHeader {
                    mid: msg.mid,
                    sid: msg.sid,
                    length: msg.buffer.data.len() as u64,
                })));
            }
            frames.extend(std::iter::once((msg_sid, Frame::Data {
                mid: msg.mid,
                start: msg.cursor,
                data: msg.buffer.data[msg.cursor as usize..][..to_send as usize].to_vec(),
            })));
        };
        msg.cursor += to_send;
        msg.cursor >= msg.buffer.data.len() as u64
    }

    /// no_of_frames = frames.len()
    /// Your goal is to try to find a realistic no_of_frames!
    /// no_of_frames should be choosen so, that all Frames can be send out till
    /// the next tick!
    ///  - if no_of_frames is too high you will fill either the Socket buffer,
    ///    or your internal buffer. In that case you will increase latency for
    ///    high prio messages!
    ///  - if no_of_frames is too low you wont saturate your Socket fully, thus
    ///    have a lower bandwidth as possible
    pub async fn fill_frames<E: Extend<(Sid, Frame)>>(
        &mut self,
        no_of_frames: usize,
        frames: &mut E,
    ) {
        for v in self.messages.iter_mut() {
            v.reserve_exact(no_of_frames)
        }
        self.tick().await;
        for _ in 0..no_of_frames {
            match self.calc_next_prio() {
                Some(prio) => {
                    //let prio2 = self.calc_next_prio().unwrap();
                    //trace!(?prio, "handle next prio");
                    self.points[prio as usize] += Self::PRIOS[prio as usize];
                    //pop message from front of VecDeque, handle it and push it back, so that all
                    // => messages with same prio get a fair chance :)
                    //TODO: evalaute not poping every time
                    let (sid, mut msg) = self.messages[prio as usize].pop_front().unwrap();
                    if Self::tick_msg(&mut msg, sid, frames) {
                        //trace!(?m.mid, "finish message");
                        //check if prio is empty
                        if self.messages[prio as usize].is_empty() {
                            self.queued.remove(&prio);
                        }
                        //decrease pid_sid counter by 1 again
                        let cnt = self.sid_owned.get_mut(&sid).expect(
                            "the pid_sid_owned counter works wrong, more pid,sid removed than \
                             inserted",
                        );
                        cnt.len -= 1;
                        if cnt.len == 0 {
                            let cnt = self.sid_owned.remove(&sid).unwrap();
                            if let Some(empty_notify) = cnt.empty_notify {
                                empty_notify.send(()).unwrap();
                            }
                        }
                    } else {
                        trace!(?msg.mid, "repush message");
                        self.messages[prio as usize].push_front((sid, msg));
                    }
                },
                None => {
                    //QUEUE is empty, we are clearing the POINTS to not build up huge pipes of
                    // POINTS on a prio from the past
                    self.points = [0; PRIO_MAX];
                    break;
                },
            }
        }
    }
}

impl std::fmt::Debug for PrioManager {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut cnt = 0;
        for m in self.messages.iter() {
            cnt += m.len();
        }
        write!(f, "PrioManager(len: {}, queued: {:?})", cnt, &self.queued,)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        message::{MessageBuffer, OutgoingMessage},
        metrics::NetworkMetrics,
        prios::*,
        types::{Frame, Pid, Prio, Sid},
    };
    use crossbeam_channel::Sender;
    use futures::{channel::oneshot, executor::block_on};
    use std::{collections::VecDeque, sync::Arc};

    const SIZE: u64 = PrioManager::FRAME_DATA_SIZE;
    const USIZE: usize = PrioManager::FRAME_DATA_SIZE as usize;

    #[allow(clippy::type_complexity)]
    fn mock_new() -> (
        PrioManager,
        Sender<(Prio, Sid, OutgoingMessage)>,
        Sender<(Sid, oneshot::Sender<()>)>,
    ) {
        let pid = Pid::fake(1);
        PrioManager::new(
            Arc::new(NetworkMetrics::new(&pid).unwrap()),
            pid.to_string(),
        )
    }

    fn mock_out(prio: Prio, sid: u64) -> (Prio, Sid, OutgoingMessage) {
        let sid = Sid::new(sid);
        (prio, sid, OutgoingMessage {
            buffer: Arc::new(MessageBuffer {
                data: vec![48, 49, 50],
            }),
            cursor: 0,
            mid: 1,
            sid,
        })
    }

    fn mock_out_large(prio: Prio, sid: u64) -> (Prio, Sid, OutgoingMessage) {
        let sid = Sid::new(sid);
        let mut data = vec![48; USIZE];
        data.append(&mut vec![49; USIZE]);
        data.append(&mut vec![50; 20]);
        (prio, sid, OutgoingMessage {
            buffer: Arc::new(MessageBuffer { data }),
            cursor: 0,
            mid: 1,
            sid,
        })
    }

    fn assert_header(frames: &mut VecDeque<(Sid, Frame)>, f_sid: u64, f_length: u64) {
        let frame = frames
            .pop_front()
            .expect("frames vecdeque doesn't contain enough frames!")
            .1;
        if let Frame::DataHeader { mid, sid, length } = frame {
            assert_eq!(mid, 1);
            assert_eq!(sid, Sid::new(f_sid));
            assert_eq!(length, f_length);
        } else {
            panic!("wrong frame type!, expected DataHeader");
        }
    }

    fn assert_data(frames: &mut VecDeque<(Sid, Frame)>, f_start: u64, f_data: Vec<u8>) {
        let frame = frames
            .pop_front()
            .expect("frames vecdeque doesn't contain enough frames!")
            .1;
        if let Frame::Data { mid, start, data } = frame {
            assert_eq!(mid, 1);
            assert_eq!(start, f_start);
            assert_eq!(data, f_data);
        } else {
            panic!("wrong frame type!, expected Data");
        }
    }

    #[test]
    fn single_p16() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        msg_tx.send(mock_out(16, 1337)).unwrap();
        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(100, &mut frames));

        assert_header(&mut frames, 1337, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert!(frames.is_empty());
    }

    #[test]
    fn single_p16_p20() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        msg_tx.send(mock_out(16, 1337)).unwrap();
        msg_tx.send(mock_out(20, 42)).unwrap();
        let mut frames = VecDeque::new();

        block_on(mgr.fill_frames(100, &mut frames));
        assert_header(&mut frames, 1337, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert_header(&mut frames, 42, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert!(frames.is_empty());
    }

    #[test]
    fn single_p20_p16() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        msg_tx.send(mock_out(20, 42)).unwrap();
        msg_tx.send(mock_out(16, 1337)).unwrap();
        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(100, &mut frames));

        assert_header(&mut frames, 1337, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert_header(&mut frames, 42, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert!(frames.is_empty());
    }

    #[test]
    fn multiple_p16_p20() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        msg_tx.send(mock_out(20, 2)).unwrap();
        msg_tx.send(mock_out(16, 1)).unwrap();
        msg_tx.send(mock_out(16, 3)).unwrap();
        msg_tx.send(mock_out(16, 5)).unwrap();
        msg_tx.send(mock_out(20, 4)).unwrap();
        msg_tx.send(mock_out(20, 7)).unwrap();
        msg_tx.send(mock_out(16, 6)).unwrap();
        msg_tx.send(mock_out(20, 10)).unwrap();
        msg_tx.send(mock_out(16, 8)).unwrap();
        msg_tx.send(mock_out(20, 12)).unwrap();
        msg_tx.send(mock_out(16, 9)).unwrap();
        msg_tx.send(mock_out(16, 11)).unwrap();
        msg_tx.send(mock_out(20, 13)).unwrap();
        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(100, &mut frames));

        for i in 1..14 {
            assert_header(&mut frames, i, 3);
            assert_data(&mut frames, 0, vec![48, 49, 50]);
        }
        assert!(frames.is_empty());
    }

    #[test]
    fn multiple_fill_frames_p16_p20() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        msg_tx.send(mock_out(20, 2)).unwrap();
        msg_tx.send(mock_out(16, 1)).unwrap();
        msg_tx.send(mock_out(16, 3)).unwrap();
        msg_tx.send(mock_out(16, 5)).unwrap();
        msg_tx.send(mock_out(20, 4)).unwrap();
        msg_tx.send(mock_out(20, 7)).unwrap();
        msg_tx.send(mock_out(16, 6)).unwrap();
        msg_tx.send(mock_out(20, 10)).unwrap();
        msg_tx.send(mock_out(16, 8)).unwrap();
        msg_tx.send(mock_out(20, 12)).unwrap();
        msg_tx.send(mock_out(16, 9)).unwrap();
        msg_tx.send(mock_out(16, 11)).unwrap();
        msg_tx.send(mock_out(20, 13)).unwrap();

        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(3, &mut frames));
        for i in 1..4 {
            assert_header(&mut frames, i, 3);
            assert_data(&mut frames, 0, vec![48, 49, 50]);
        }
        assert!(frames.is_empty());
        block_on(mgr.fill_frames(11, &mut frames));
        for i in 4..14 {
            assert_header(&mut frames, i, 3);
            assert_data(&mut frames, 0, vec![48, 49, 50]);
        }
        assert!(frames.is_empty());
    }

    #[test]
    fn single_large_p16() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        msg_tx.send(mock_out_large(16, 1)).unwrap();
        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(100, &mut frames));

        assert_header(&mut frames, 1, SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![48; USIZE]);
        assert_data(&mut frames, SIZE, vec![49; USIZE]);
        assert_data(&mut frames, SIZE * 2, vec![50; 20]);
        assert!(frames.is_empty());
    }

    #[test]
    fn multiple_large_p16() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        msg_tx.send(mock_out_large(16, 1)).unwrap();
        msg_tx.send(mock_out_large(16, 2)).unwrap();
        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(100, &mut frames));

        assert_header(&mut frames, 1, SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![48; USIZE]);
        assert_data(&mut frames, SIZE, vec![49; USIZE]);
        assert_data(&mut frames, SIZE * 2, vec![50; 20]);
        assert_header(&mut frames, 2, SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![48; USIZE]);
        assert_data(&mut frames, SIZE, vec![49; USIZE]);
        assert_data(&mut frames, SIZE * 2, vec![50; 20]);
        assert!(frames.is_empty());
    }

    #[test]
    fn multiple_large_p16_sudden_p0() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        msg_tx.send(mock_out_large(16, 1)).unwrap();
        msg_tx.send(mock_out_large(16, 2)).unwrap();
        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(2, &mut frames));

        assert_header(&mut frames, 1, SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![48; USIZE]);
        assert_data(&mut frames, SIZE, vec![49; USIZE]);

        msg_tx.send(mock_out(0, 3)).unwrap();
        block_on(mgr.fill_frames(100, &mut frames));

        assert_header(&mut frames, 3, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);

        assert_data(&mut frames, SIZE * 2, vec![50; 20]);
        assert_header(&mut frames, 2, SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![48; USIZE]);
        assert_data(&mut frames, SIZE, vec![49; USIZE]);
        assert_data(&mut frames, SIZE * 2, vec![50; 20]);
        assert!(frames.is_empty());
    }

    #[test]
    fn single_p20_thousand_p16_at_once() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        for _ in 0..998 {
            msg_tx.send(mock_out(16, 2)).unwrap();
        }
        msg_tx.send(mock_out(20, 1)).unwrap();
        msg_tx.send(mock_out(16, 2)).unwrap();
        msg_tx.send(mock_out(16, 2)).unwrap();
        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(2000, &mut frames));

        assert_header(&mut frames, 2, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert_header(&mut frames, 1, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert_header(&mut frames, 2, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert_header(&mut frames, 2, 3);
        //unimportant
    }

    #[test]
    fn single_p20_thousand_p16_later() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        for _ in 0..998 {
            msg_tx.send(mock_out(16, 2)).unwrap();
        }
        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(2000, &mut frames));
        //^unimportant frames, gonna be dropped
        msg_tx.send(mock_out(20, 1)).unwrap();
        msg_tx.send(mock_out(16, 2)).unwrap();
        msg_tx.send(mock_out(16, 2)).unwrap();
        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(2000, &mut frames));

        //important in that test is, that after the first frames got cleared i reset
        // the Points even though 998 prio 16 messages have been send at this
        // point and 0 prio20 messages the next mesasge is a prio16 message
        // again, and only then prio20! we dont want to build dept over a idling
        // connection
        assert_header(&mut frames, 2, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert_header(&mut frames, 1, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert_header(&mut frames, 2, 3);
        //unimportant
    }

    #[test]
    fn gigantic_message() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        let mut data = vec![1; USIZE];
        data.extend_from_slice(&[2; USIZE]);
        data.extend_from_slice(&[3; USIZE]);
        data.extend_from_slice(&[4; USIZE]);
        data.extend_from_slice(&[5; USIZE]);
        let sid = Sid::new(2);
        msg_tx
            .send((16, sid, OutgoingMessage {
                buffer: Arc::new(MessageBuffer { data }),
                cursor: 0,
                mid: 1,
                sid,
            }))
            .unwrap();

        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(2000, &mut frames));

        assert_header(&mut frames, 2, 7000);
        assert_data(&mut frames, 0, vec![1; USIZE]);
        assert_data(&mut frames, 1400, vec![2; USIZE]);
        assert_data(&mut frames, 2800, vec![3; USIZE]);
        assert_data(&mut frames, 4200, vec![4; USIZE]);
        assert_data(&mut frames, 5600, vec![5; USIZE]);
    }

    #[test]
    fn gigantic_message_order() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        let mut data = vec![1; USIZE];
        data.extend_from_slice(&[2; USIZE]);
        data.extend_from_slice(&[3; USIZE]);
        data.extend_from_slice(&[4; USIZE]);
        data.extend_from_slice(&[5; USIZE]);
        let sid = Sid::new(2);
        msg_tx
            .send((16, sid, OutgoingMessage {
                buffer: Arc::new(MessageBuffer { data }),
                cursor: 0,
                mid: 1,
                sid,
            }))
            .unwrap();
        msg_tx.send(mock_out(16, 8)).unwrap();

        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(2000, &mut frames));

        assert_header(&mut frames, 2, 7000);
        assert_data(&mut frames, 0, vec![1; USIZE]);
        assert_data(&mut frames, 1400, vec![2; USIZE]);
        assert_data(&mut frames, 2800, vec![3; USIZE]);
        assert_data(&mut frames, 4200, vec![4; USIZE]);
        assert_data(&mut frames, 5600, vec![5; USIZE]);
        assert_header(&mut frames, 8, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
    }

    #[test]
    fn gigantic_message_order_other_prio() {
        let (mut mgr, msg_tx, _flush_tx) = mock_new();
        let mut data = vec![1; USIZE];
        data.extend_from_slice(&[2; USIZE]);
        data.extend_from_slice(&[3; USIZE]);
        data.extend_from_slice(&[4; USIZE]);
        data.extend_from_slice(&[5; USIZE]);
        let sid = Sid::new(2);
        msg_tx
            .send((16, sid, OutgoingMessage {
                buffer: Arc::new(MessageBuffer { data }),
                cursor: 0,
                mid: 1,
                sid,
            }))
            .unwrap();
        msg_tx.send(mock_out(20, 8)).unwrap();

        let mut frames = VecDeque::new();
        block_on(mgr.fill_frames(2000, &mut frames));

        assert_header(&mut frames, 2, 7000);
        assert_data(&mut frames, 0, vec![1; USIZE]);
        assert_header(&mut frames, 8, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert_data(&mut frames, 1400, vec![2; USIZE]);
        assert_data(&mut frames, 2800, vec![3; USIZE]);
        assert_data(&mut frames, 4200, vec![4; USIZE]);
        assert_data(&mut frames, 5600, vec![5; USIZE]);
    }
}
