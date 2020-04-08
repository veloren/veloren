/*
Priorities are handled the following way.
Prios from 0-63 are allowed.
all 5 numbers the throughput i halved.
E.g. in the same time 100 prio0 messages are send, only 50 prio5, 25 prio10, 12 prio15 or 6 prio20 messages are send.
Note: TODO: prio0 will be send immeadiatly when found!
*/

use crate::{
    message::OutGoingMessage,
    types::{Frame, Pid, Prio, Sid},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::mpsc::{channel, Receiver, Sender},
};

use tracing::*;

const PRIO_MAX: usize = 64;

pub(crate) struct PrioManager {
    points: [u32; PRIO_MAX],
    messages: [VecDeque<(Pid, Sid, OutGoingMessage)>; PRIO_MAX],
    messages_rx: Receiver<(Prio, Pid, Sid, OutGoingMessage)>,
    pid_sid_owned: HashMap<(Pid, Sid), u64>,
    queued: HashSet<u8>,
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

    pub fn new() -> (Self, Sender<(Prio, Pid, Sid, OutGoingMessage)>) {
        let (messages_tx, messages_rx) = channel();
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
                pid_sid_owned: HashMap::new(),
            },
            messages_tx,
        )
    }

    fn tick(&mut self) {
        // Check Range
        let mut times = 0;
        for (prio, pid, sid, msg) in self.messages_rx.try_iter() {
            debug_assert!(prio as usize <= PRIO_MAX);
            times += 1;
            //trace!(?prio, ?sid, ?pid, "tick");
            self.queued.insert(prio);
            self.messages[prio as usize].push_back((pid, sid, msg));
            if let Some(cnt) = self.pid_sid_owned.get_mut(&(pid, sid)) {
                *cnt += 1;
            } else {
                self.pid_sid_owned.insert((pid, sid), 1);
            }
        }
        if times > 0 {
            trace!(?times, "tick");
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
    }

    /// returns if msg is empty
    fn tick_msg<E: Extend<(Pid, Sid, Frame)>>(
        msg: &mut OutGoingMessage,
        msg_pid: Pid,
        msg_sid: Sid,
        frames: &mut E,
    ) -> bool {
        let to_send = std::cmp::min(
            msg.buffer.data.len() as u64 - msg.cursor,
            Self::FRAME_DATA_SIZE,
        );
        if to_send > 0 {
            if msg.cursor == 0 {
                frames.extend(std::iter::once((msg_pid, msg_sid, Frame::DataHeader {
                    mid: msg.mid,
                    sid: msg.sid,
                    length: msg.buffer.data.len() as u64,
                })));
            }
            frames.extend(std::iter::once((msg_pid, msg_sid, Frame::Data {
                id: msg.mid,
                start: msg.cursor,
                data: msg.buffer.data[msg.cursor as usize..(msg.cursor + to_send) as usize]
                    .to_vec(),
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
    pub fn fill_frames<E: Extend<(Pid, Sid, Frame)>>(
        &mut self,
        no_of_frames: usize,
        frames: &mut E,
    ) {
        self.tick();
        for _ in 0..no_of_frames {
            match self.calc_next_prio() {
                Some(prio) => {
                    //trace!(?prio, "handle next prio");
                    self.points[prio as usize] += Self::PRIOS[prio as usize];
                    //pop message from front of VecDeque, handle it and push it back, so that all
                    // => messages with same prio get a fair chance :)
                    //TODO: evalaute not poping every time
                    match self.messages[prio as usize].pop_front() {
                        Some((pid, sid, mut msg)) => {
                            if Self::tick_msg(&mut msg, pid, sid, frames) {
                                //debug!(?m.mid, "finish message");
                                //check if prio is empty
                                if self.messages[prio as usize].is_empty() {
                                    self.queued.remove(&prio);
                                }
                                //decrease pid_sid counter by 1 again
                                let cnt = self.pid_sid_owned.get_mut(&(pid, sid)).expect(
                                    "the pid_sid_owned counter works wrong, more pid,sid removed \
                                     than inserted",
                                );
                                *cnt -= 1;
                                if *cnt == 0 {
                                    self.pid_sid_owned.remove(&(pid, sid));
                                }
                            } else {
                                self.messages[prio as usize].push_back((pid, sid, msg));
                                //trace!(?m.mid, "repush message");
                            }
                        },
                        None => unreachable!("msg not in VecDeque, but queued"),
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

    /// if you want to make sure to empty the prio of a single pid and sid, use
    /// this
    pub(crate) fn contains_pid_sid(&self, pid: Pid, sid: Sid) -> bool {
        self.pid_sid_owned.contains_key(&(pid, sid))
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
        message::{MessageBuffer, OutGoingMessage},
        prios::*,
        types::{Frame, Pid, Prio, Sid},
    };
    use std::{collections::VecDeque, sync::Arc};

    const SIZE: u64 = PrioManager::FRAME_DATA_SIZE;
    const USIZE: usize = PrioManager::FRAME_DATA_SIZE as usize;

    fn mock_out(prio: Prio, sid: u64) -> (Prio, Pid, Sid, OutGoingMessage) {
        let sid = Sid::new(sid);
        (prio, Pid::fake(0), sid, OutGoingMessage {
            buffer: Arc::new(MessageBuffer {
                data: vec![48, 49, 50],
            }),
            cursor: 0,
            mid: 1,
            sid,
        })
    }

    fn mock_out_large(prio: Prio, sid: u64) -> (Prio, Pid, Sid, OutGoingMessage) {
        let sid = Sid::new(sid);
        let mut data = vec![48; USIZE];
        data.append(&mut vec![49; USIZE]);
        data.append(&mut vec![50; 20]);
        (prio, Pid::fake(0), sid, OutGoingMessage {
            buffer: Arc::new(MessageBuffer { data }),
            cursor: 0,
            mid: 1,
            sid,
        })
    }

    fn assert_header(frames: &mut VecDeque<(Pid, Sid, Frame)>, f_sid: u64, f_length: u64) {
        let frame = frames
            .pop_front()
            .expect("frames vecdeque doesn't contain enough frames!")
            .2;
        if let Frame::DataHeader { mid, sid, length } = frame {
            assert_eq!(mid, 1);
            assert_eq!(sid, Sid::new(f_sid));
            assert_eq!(length, f_length);
        } else {
            panic!("wrong frame type!, expected DataHeader");
        }
    }

    fn assert_data(frames: &mut VecDeque<(Pid, Sid, Frame)>, f_start: u64, f_data: Vec<u8>) {
        let frame = frames
            .pop_front()
            .expect("frames vecdeque doesn't contain enough frames!")
            .2;
        if let Frame::Data { id, start, data } = frame {
            assert_eq!(id, 1);
            assert_eq!(start, f_start);
            assert_eq!(data, f_data);
        } else {
            panic!("wrong frame type!, expected Data");
        }
    }

    fn assert_contains(mgr: &PrioManager, sid: u64) {
        assert!(mgr.contains_pid_sid(Pid::fake(0), Sid::new(sid)));
    }

    fn assert_no_contains(mgr: &PrioManager, sid: u64) {
        assert!(!mgr.contains_pid_sid(Pid::fake(0), Sid::new(sid)));
    }

    #[test]
    fn single_p16() {
        let (mut mgr, tx) = PrioManager::new();
        tx.send(mock_out(16, 1337)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(100, &mut frames);

        assert_header(&mut frames, 1337, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert!(frames.is_empty());
    }

    #[test]
    fn single_p16_p20() {
        let (mut mgr, tx) = PrioManager::new();
        tx.send(mock_out(16, 1337)).unwrap();
        tx.send(mock_out(20, 42)).unwrap();
        let mut frames = VecDeque::new();

        mgr.fill_frames(100, &mut frames);

        assert_no_contains(&mgr, 1337);
        assert_no_contains(&mgr, 42);
        assert_no_contains(&mgr, 666);

        assert_header(&mut frames, 1337, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert_header(&mut frames, 42, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert!(frames.is_empty());
    }

    #[test]
    fn single_p20_p16() {
        let (mut mgr, tx) = PrioManager::new();
        tx.send(mock_out(20, 42)).unwrap();
        tx.send(mock_out(16, 1337)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(100, &mut frames);

        assert_header(&mut frames, 1337, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert_header(&mut frames, 42, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert!(frames.is_empty());
    }

    #[test]
    fn multiple_p16_p20() {
        let (mut mgr, tx) = PrioManager::new();
        tx.send(mock_out(20, 2)).unwrap();
        tx.send(mock_out(16, 1)).unwrap();
        tx.send(mock_out(16, 3)).unwrap();
        tx.send(mock_out(16, 5)).unwrap();
        tx.send(mock_out(20, 4)).unwrap();
        tx.send(mock_out(20, 7)).unwrap();
        tx.send(mock_out(16, 6)).unwrap();
        tx.send(mock_out(20, 10)).unwrap();
        tx.send(mock_out(16, 8)).unwrap();
        tx.send(mock_out(20, 12)).unwrap();
        tx.send(mock_out(16, 9)).unwrap();
        tx.send(mock_out(16, 11)).unwrap();
        tx.send(mock_out(20, 13)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(100, &mut frames);

        for i in 1..14 {
            assert_header(&mut frames, i, 3);
            assert_data(&mut frames, 0, vec![48, 49, 50]);
        }
        assert!(frames.is_empty());
    }

    #[test]
    fn multiple_fill_frames_p16_p20() {
        let (mut mgr, tx) = PrioManager::new();
        tx.send(mock_out(20, 2)).unwrap();
        tx.send(mock_out(16, 1)).unwrap();
        tx.send(mock_out(16, 3)).unwrap();
        tx.send(mock_out(16, 5)).unwrap();
        tx.send(mock_out(20, 4)).unwrap();
        tx.send(mock_out(20, 7)).unwrap();
        tx.send(mock_out(16, 6)).unwrap();
        tx.send(mock_out(20, 10)).unwrap();
        tx.send(mock_out(16, 8)).unwrap();
        tx.send(mock_out(20, 12)).unwrap();
        tx.send(mock_out(16, 9)).unwrap();
        tx.send(mock_out(16, 11)).unwrap();
        tx.send(mock_out(20, 13)).unwrap();

        let mut frames = VecDeque::new();
        mgr.fill_frames(3, &mut frames);

        assert_no_contains(&mgr, 1);
        assert_no_contains(&mgr, 3);
        assert_contains(&mgr, 13);

        for i in 1..4 {
            assert_header(&mut frames, i, 3);
            assert_data(&mut frames, 0, vec![48, 49, 50]);
        }
        assert!(frames.is_empty());
        mgr.fill_frames(11, &mut frames);
        for i in 4..14 {
            assert_header(&mut frames, i, 3);
            assert_data(&mut frames, 0, vec![48, 49, 50]);
        }
        assert!(frames.is_empty());
    }

    #[test]
    fn single_large_p16() {
        let (mut mgr, tx) = PrioManager::new();
        tx.send(mock_out_large(16, 1)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(100, &mut frames);

        assert_header(&mut frames, 1, SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![48; USIZE]);
        assert_data(&mut frames, SIZE, vec![49; USIZE]);
        assert_data(&mut frames, SIZE * 2, vec![50; 20]);
        assert!(frames.is_empty());
    }

    #[test]
    fn multiple_large_p16() {
        let (mut mgr, tx) = PrioManager::new();
        tx.send(mock_out_large(16, 1)).unwrap();
        tx.send(mock_out_large(16, 2)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(100, &mut frames);

        assert_header(&mut frames, 1, SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![48; USIZE]);
        assert_header(&mut frames, 2, SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![48; USIZE]);
        assert_data(&mut frames, SIZE, vec![49; USIZE]);
        assert_data(&mut frames, SIZE, vec![49; USIZE]);
        assert_data(&mut frames, SIZE * 2, vec![50; 20]);
        assert_data(&mut frames, SIZE * 2, vec![50; 20]);
        assert!(frames.is_empty());
    }

    #[test]
    fn multiple_large_p16_sudden_p0() {
        let (mut mgr, tx) = PrioManager::new();
        tx.send(mock_out_large(16, 1)).unwrap();
        tx.send(mock_out_large(16, 2)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(3, &mut frames);

        assert_header(&mut frames, 1, SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![48; USIZE]);
        assert_header(&mut frames, 2, SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![48; USIZE]);
        assert_data(&mut frames, SIZE, vec![49; USIZE]);

        tx.send(mock_out(0, 3)).unwrap();
        mgr.fill_frames(100, &mut frames);

        assert_header(&mut frames, 3, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);

        assert_data(&mut frames, SIZE, vec![49; USIZE]);
        assert_data(&mut frames, SIZE * 2, vec![50; 20]);
        assert_data(&mut frames, SIZE * 2, vec![50; 20]);
        assert!(frames.is_empty());
    }

    #[test]
    fn single_p20_thousand_p16_at_once() {
        let (mut mgr, tx) = PrioManager::new();
        for _ in 0..998 {
            tx.send(mock_out(16, 2)).unwrap();
        }
        tx.send(mock_out(20, 1)).unwrap();
        tx.send(mock_out(16, 2)).unwrap();
        tx.send(mock_out(16, 2)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(2000, &mut frames);

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
        let (mut mgr, tx) = PrioManager::new();
        for _ in 0..998 {
            tx.send(mock_out(16, 2)).unwrap();
        }
        let mut frames = VecDeque::new();
        mgr.fill_frames(2000, &mut frames);
        //^unimportant frames, gonna be dropped
        tx.send(mock_out(20, 1)).unwrap();
        tx.send(mock_out(16, 2)).unwrap();
        tx.send(mock_out(16, 2)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(2000, &mut frames);

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
}
