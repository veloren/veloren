/*

This will become a single class,
it contains a list of all open Channels and all Participants and all streams.
Important, we need to change stream ids to be unique per participant
and msg ids need to be unique per participant too. The other way would be to always send sid with Data Frame but this is to much overhead.

We need a external (like timer like) Future that opens a thread in threadpool, and is Ready once serialized

We should focus in this implementation on the routing side, Prio and choosing the correct Protocol.
A Message should be delivered over 2 Channels, e.g. Create Info via TCP and data via UDP. keep in mind that UDP might be read before TCP is read...

maybe even a future that builds together a message from incremental steps.

Or a future that sends a message, however on each seend prio needs to be considered, maybe overkill.


it should be quite easy as all is in one thread now, but i am still not sure if its in the same as the network, or if we still have a sperate one,
probably start with a seperate thread for now.

Focus on the routing for now, and ignoring protocols and details...
*/

/*
Priorities are handled the following way.
Prios from 0-63 are allowed.
all 5 numbers the throughput i halved.
E.g. in the same time 100 prio0 messages are send, only 50 prio5, 25 prio10, 12 prio15 or 6 prio20 messages are send.
Node: TODO: prio0 will be send immeadiatly when found!
*/

/*
algo:
let past = [u64, 100] = [0,0,0,0..]
send_prio0()
past[0] += 100;
#check_next_prio
if past[0] - past[1] > prio_numbers[1] {
    sendprio1();
    past[1] += 100;
    if past[0] - past[2] > prio_numbers[2] {
        sendprio2();
        past[2] += 100;
    }
}


*/

use crate::{message::OutGoingMessage, types::Frame};
use std::{
    collections::{HashSet, VecDeque},
    sync::mpsc::{channel, Receiver, Sender},
};

const PRIO_MAX: usize = 64;

struct PrioManager {
    points: [u32; PRIO_MAX],
    messages: [VecDeque<OutGoingMessage>; PRIO_MAX],
    messages_tx: Sender<(u8, OutGoingMessage)>,
    messages_rx: Receiver<(u8, OutGoingMessage)>,
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

    pub fn new() -> Self {
        let (messages_tx, messages_rx) = channel();
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
            messages_tx,
            messages_rx,
            queued: HashSet::new(), //TODO: optimize with u64 and 64 bits
        }
    }

    fn tick(&mut self) {
        // Check Range
        for (prio, msg) in self.messages_rx.try_iter() {
            debug_assert!(prio as usize <= PRIO_MAX);
            println!("tick {}", prio);
            self.queued.insert(prio);
            self.messages[prio as usize].push_back(msg);
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
    fn tick_msg<E: Extend<Frame>>(msg: &mut OutGoingMessage, frames: &mut E) -> bool {
        let to_send = std::cmp::min(
            msg.buffer.data.len() as u64 - msg.cursor,
            Self::FRAME_DATA_SIZE,
        );
        if to_send > 0 {
            if msg.cursor == 0 {
                //TODO: OutGoingMessage MUST HAVE A MID AT THIS POINT ALREADY! AS I HAVE NO
                // IDEA OF STREAMS HERE!
                debug_assert!(msg.mid.is_some());
                frames.extend(std::iter::once(Frame::DataHeader {
                    mid: msg
                        .mid
                        .expect("read comment 3 lines above this error message 41231255661"),
                    sid: msg.sid,
                    length: msg.buffer.data.len() as u64,
                }));
            }
            frames.extend(std::iter::once(Frame::Data {
                id: msg.mid.unwrap(),
                start: msg.cursor,
                data: msg.buffer.data[msg.cursor as usize..(msg.cursor + to_send) as usize]
                    .to_vec(),
            }));
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
    pub fn fill_frames<E: Extend<Frame>>(&mut self, no_of_frames: usize, frames: &mut E) {
        self.tick();
        for _ in 0..no_of_frames {
            match self.calc_next_prio() {
                Some(prio) => {
                    println!("dasd {}", prio);
                    self.points[prio as usize] += Self::PRIOS[prio as usize];
                    //pop message from front of VecDeque, handle it and push it back, so that all
                    // => messages with same prio get a fair chance :)
                    //TODO: evalaute not poping every time
                    match self.messages[prio as usize].pop_front() {
                        Some(mut msg) => {
                            if Self::tick_msg(&mut msg, frames) {
                                //debug!(?m.mid, "finish message");
                                //check if prio is empty
                                if self.messages[prio as usize].is_empty() {
                                    self.queued.remove(&prio);
                                }
                            } else {
                                self.messages[prio as usize].push_back(msg);
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

    pub fn get_tx(&self) -> &Sender<(u8, OutGoingMessage)> { &self.messages_tx }
}

#[cfg(test)]
mod tests {
    use crate::{
        message::{MessageBuffer, OutGoingMessage},
        prios::*,
        types::{Frame, Mid, Sid},
    };
    use std::{collections::VecDeque, sync::Arc};

    fn mock_out(prio: u8, sid: Sid) -> (u8, OutGoingMessage) {
        (prio, OutGoingMessage {
            buffer: Arc::new(MessageBuffer {
                data: vec![48, 49, 50],
            }),
            cursor: 0,
            mid: Some(1),
            sid,
        })
    }

    fn mock_out_large(prio: u8, sid: Sid) -> (u8, OutGoingMessage) {
        const MSG_SIZE: usize = PrioManager::FRAME_DATA_SIZE as usize;
        let mut data = vec![48; MSG_SIZE];
        data.append(&mut vec![49; MSG_SIZE]);
        data.append(&mut vec![50; 20]);
        (prio, OutGoingMessage {
            buffer: Arc::new(MessageBuffer { data }),
            cursor: 0,
            mid: Some(1),
            sid,
        })
    }

    fn assert_header(frames: &mut VecDeque<Frame>, f_sid: Sid, f_length: u64) {
        let frame = frames
            .pop_front()
            .expect("frames vecdeque doesn't contain enough frames!");
        if let Frame::DataHeader { mid, sid, length } = frame {
            assert_eq!(mid, 1);
            assert_eq!(sid, f_sid);
            assert_eq!(length, f_length);
        } else {
            panic!("wrong frame type!, expected DataHeader");
        }
    }

    fn assert_data(frames: &mut VecDeque<Frame>, f_start: u64, f_data: Vec<u8>) {
        let frame = frames
            .pop_front()
            .expect("frames vecdeque doesn't contain enough frames!");
        if let Frame::Data { id, start, data } = frame {
            assert_eq!(id, 1);
            assert_eq!(start, f_start);
            assert_eq!(data, f_data);
        } else {
            panic!("wrong frame type!, expected Data");
        }
    }

    #[test]
    fn single_p16() {
        let mut mgr = PrioManager::new();
        mgr.get_tx().send(mock_out(16, 1337)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(100, &mut frames);

        assert_header(&mut frames, 1337, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert!(frames.is_empty());
    }

    #[test]
    fn single_p16_p20() {
        let mut mgr = PrioManager::new();
        mgr.get_tx().send(mock_out(16, 1337)).unwrap();
        mgr.get_tx().send(mock_out(20, 42)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(100, &mut frames);

        assert_header(&mut frames, 1337, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert_header(&mut frames, 42, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);
        assert!(frames.is_empty());
    }

    #[test]
    fn single_p20_p16() {
        let mut mgr = PrioManager::new();
        mgr.get_tx().send(mock_out(20, 42)).unwrap();
        mgr.get_tx().send(mock_out(16, 1337)).unwrap();
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
        let mut mgr = PrioManager::new();
        mgr.get_tx().send(mock_out(20, 2)).unwrap();
        mgr.get_tx().send(mock_out(16, 1)).unwrap();
        mgr.get_tx().send(mock_out(16, 3)).unwrap();
        mgr.get_tx().send(mock_out(16, 5)).unwrap();
        mgr.get_tx().send(mock_out(20, 4)).unwrap();
        mgr.get_tx().send(mock_out(20, 7)).unwrap();
        mgr.get_tx().send(mock_out(16, 6)).unwrap();
        mgr.get_tx().send(mock_out(20, 10)).unwrap();
        mgr.get_tx().send(mock_out(16, 8)).unwrap();
        mgr.get_tx().send(mock_out(20, 12)).unwrap();
        mgr.get_tx().send(mock_out(16, 9)).unwrap();
        mgr.get_tx().send(mock_out(16, 11)).unwrap();
        mgr.get_tx().send(mock_out(20, 13)).unwrap();
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
        let mut mgr = PrioManager::new();
        mgr.get_tx().send(mock_out(20, 2)).unwrap();
        mgr.get_tx().send(mock_out(16, 1)).unwrap();
        mgr.get_tx().send(mock_out(16, 3)).unwrap();
        mgr.get_tx().send(mock_out(16, 5)).unwrap();
        mgr.get_tx().send(mock_out(20, 4)).unwrap();
        mgr.get_tx().send(mock_out(20, 7)).unwrap();
        mgr.get_tx().send(mock_out(16, 6)).unwrap();
        mgr.get_tx().send(mock_out(20, 10)).unwrap();
        mgr.get_tx().send(mock_out(16, 8)).unwrap();
        mgr.get_tx().send(mock_out(20, 12)).unwrap();
        mgr.get_tx().send(mock_out(16, 9)).unwrap();
        mgr.get_tx().send(mock_out(16, 11)).unwrap();
        mgr.get_tx().send(mock_out(20, 13)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(3, &mut frames);
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
        let mut mgr = PrioManager::new();
        mgr.get_tx().send(mock_out_large(16, 1)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(100, &mut frames);

        assert_header(&mut frames, 1, PrioManager::FRAME_DATA_SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![
            48;
            PrioManager::FRAME_DATA_SIZE as usize
        ]);
        assert_data(&mut frames, PrioManager::FRAME_DATA_SIZE, vec![
            49;
            PrioManager::FRAME_DATA_SIZE
                as usize
        ]);
        assert_data(&mut frames, PrioManager::FRAME_DATA_SIZE * 2, vec![50; 20]);
        assert!(frames.is_empty());
    }

    #[test]
    fn multiple_large_p16() {
        let mut mgr = PrioManager::new();
        mgr.get_tx().send(mock_out_large(16, 1)).unwrap();
        mgr.get_tx().send(mock_out_large(16, 2)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(100, &mut frames);

        assert_header(&mut frames, 1, PrioManager::FRAME_DATA_SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![
            48;
            PrioManager::FRAME_DATA_SIZE as usize
        ]);
        assert_header(&mut frames, 2, PrioManager::FRAME_DATA_SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![
            48;
            PrioManager::FRAME_DATA_SIZE as usize
        ]);
        assert_data(&mut frames, PrioManager::FRAME_DATA_SIZE, vec![
            49;
            PrioManager::FRAME_DATA_SIZE
                as usize
        ]);
        assert_data(&mut frames, PrioManager::FRAME_DATA_SIZE, vec![
            49;
            PrioManager::FRAME_DATA_SIZE
                as usize
        ]);
        assert_data(&mut frames, PrioManager::FRAME_DATA_SIZE * 2, vec![50; 20]);
        assert_data(&mut frames, PrioManager::FRAME_DATA_SIZE * 2, vec![50; 20]);
        assert!(frames.is_empty());
    }

    #[test]
    fn multiple_large_p16_sudden_p0() {
        let mut mgr = PrioManager::new();
        mgr.get_tx().send(mock_out_large(16, 1)).unwrap();
        mgr.get_tx().send(mock_out_large(16, 2)).unwrap();
        let mut frames = VecDeque::new();
        mgr.fill_frames(3, &mut frames);

        assert_header(&mut frames, 1, PrioManager::FRAME_DATA_SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![
            48;
            PrioManager::FRAME_DATA_SIZE as usize
        ]);
        assert_header(&mut frames, 2, PrioManager::FRAME_DATA_SIZE * 2 + 20);
        assert_data(&mut frames, 0, vec![
            48;
            PrioManager::FRAME_DATA_SIZE as usize
        ]);
        assert_data(&mut frames, PrioManager::FRAME_DATA_SIZE, vec![
            49;
            PrioManager::FRAME_DATA_SIZE
                as usize
        ]);

        mgr.get_tx().send(mock_out(0, 3)).unwrap();
        mgr.fill_frames(100, &mut frames);

        assert_header(&mut frames, 3, 3);
        assert_data(&mut frames, 0, vec![48, 49, 50]);

        assert_data(&mut frames, PrioManager::FRAME_DATA_SIZE, vec![
            49;
            PrioManager::FRAME_DATA_SIZE
                as usize
        ]);
        assert_data(&mut frames, PrioManager::FRAME_DATA_SIZE * 2, vec![50; 20]);
        assert_data(&mut frames, PrioManager::FRAME_DATA_SIZE * 2, vec![50; 20]);
        assert!(frames.is_empty());
    }

    #[test]
    fn single_p20_thousand_p16_at_once() {
        let mut mgr = PrioManager::new();
        for _ in 0..998 {
            mgr.get_tx().send(mock_out(16, 2)).unwrap();
        }
        mgr.get_tx().send(mock_out(20, 1)).unwrap();
        mgr.get_tx().send(mock_out(16, 2)).unwrap();
        mgr.get_tx().send(mock_out(16, 2)).unwrap();
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
        let mut mgr = PrioManager::new();
        for _ in 0..998 {
            mgr.get_tx().send(mock_out(16, 2)).unwrap();
        }
        let mut frames = VecDeque::new();
        mgr.fill_frames(2000, &mut frames);
        //^unimportant frames, gonna be dropped
        mgr.get_tx().send(mock_out(20, 1)).unwrap();
        mgr.get_tx().send(mock_out(16, 2)).unwrap();
        mgr.get_tx().send(mock_out(16, 2)).unwrap();
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
