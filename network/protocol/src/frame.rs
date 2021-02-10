use crate::types::{Mid, Pid, Prio, Promises, Sid};
use bytes::{Buf, BufMut, BytesMut};

// const FRAME_RESERVED_1: u8 = 0;
const FRAME_HANDSHAKE: u8 = 1;
const FRAME_INIT: u8 = 2;
const FRAME_SHUTDOWN: u8 = 3;
const FRAME_OPEN_STREAM: u8 = 4;
const FRAME_CLOSE_STREAM: u8 = 5;
const FRAME_DATA_HEADER: u8 = 6;
const FRAME_DATA: u8 = 7;
const FRAME_RAW: u8 = 8;
//const FRAME_RESERVED_2: u8 = 10;
//const FRAME_RESERVED_3: u8 = 13;

/// Used for Communication between Channel <----(TCP/UDP)----> Channel
#[derive(Debug, PartialEq, Clone)]
pub /* should be crate only */ enum InitFrame {
    Handshake {
        magic_number: [u8; 7],
        version: [u32; 3],
    },
    Init {
        pid: Pid,
        secret: u128,
    },
    /* WARNING: Sending RAW is only used for debug purposes in case someone write a new API
     * against veloren Server! */
    Raw(Vec<u8>),
}

/// Used for Communication between Channel <----(TCP/UDP)----> Channel
#[derive(Debug, PartialEq, Clone)]
pub enum Frame {
    Shutdown, /* Shutdown this channel gracefully, if all channels are shutdown (gracefully),
               * Participant is deleted */
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
}

impl InitFrame {
    // Size WITHOUT the 1rst indicating byte
    pub(crate) const HANDSHAKE_CNS: usize = 19;
    pub(crate) const INIT_CNS: usize = 32;
    /// const part of the RAW frame, actual size is variable
    pub(crate) const RAW_CNS: usize = 2;

    //provide an appropriate buffer size. > 1500
    pub(crate) fn to_bytes(self, bytes: &mut BytesMut) {
        match self {
            InitFrame::Handshake {
                magic_number,
                version,
            } => {
                bytes.put_u8(FRAME_HANDSHAKE);
                bytes.put_slice(&magic_number);
                bytes.put_u32_le(version[0]);
                bytes.put_u32_le(version[1]);
                bytes.put_u32_le(version[2]);
            },
            InitFrame::Init { pid, secret } => {
                bytes.put_u8(FRAME_INIT);
                pid.to_bytes(bytes);
                bytes.put_u128_le(secret);
            },
            InitFrame::Raw(data) => {
                bytes.put_u8(FRAME_RAW);
                bytes.put_u16_le(data.len() as u16);
                bytes.put_slice(&data);
            },
        }
    }

    pub(crate) fn to_frame(bytes: &mut BytesMut) -> Option<Self> {
        let frame_no = match bytes.get(0) {
            Some(&f) => f,
            None => return None,
        };
        let frame = match frame_no {
            FRAME_HANDSHAKE => {
                if bytes.len() < Self::HANDSHAKE_CNS + 1 {
                    return None;
                }
                bytes.advance(1);
                let mut magic_number_bytes = bytes.copy_to_bytes(7);
                let mut magic_number = [0u8; 7];
                magic_number_bytes.copy_to_slice(&mut magic_number);
                InitFrame::Handshake {
                    magic_number,
                    version: [bytes.get_u32_le(), bytes.get_u32_le(), bytes.get_u32_le()],
                }
            },
            FRAME_INIT => {
                if bytes.len() < Self::INIT_CNS + 1 {
                    return None;
                }
                bytes.advance(1);
                InitFrame::Init {
                    pid: Pid::from_bytes(bytes),
                    secret: bytes.get_u128_le(),
                }
            },
            FRAME_RAW => {
                if bytes.len() < Self::RAW_CNS + 1 {
                    return None;
                }
                bytes.advance(1);
                let length = bytes.get_u16_le() as usize;
                // lower length is allowed
                let max_length = length.min(bytes.len());
                println!("dasdasd {:?}", length);
                println!("aaaaa {:?}", max_length);
                let mut data = vec![0; max_length];
                data.copy_from_slice(&bytes[..max_length]);
                InitFrame::Raw(data)
            },
            _ => InitFrame::Raw(bytes.to_vec()),
        };
        Some(frame)
    }
}

impl Frame {
    pub(crate) const CLOSE_STREAM_CNS: usize = 8;
    /// const part of the DATA frame, actual size is variable
    pub(crate) const DATA_CNS: usize = 18;
    pub(crate) const DATA_HEADER_CNS: usize = 24;
    pub(crate) const OPEN_STREAM_CNS: usize = 10;
    // Size WITHOUT the 1rst indicating byte
    pub(crate) const SHUTDOWN_CNS: usize = 0;

    //provide an appropriate buffer size. > 1500
    pub fn to_bytes(self, bytes: &mut BytesMut) -> u64 {
        match self {
            Frame::Shutdown => {
                bytes.put_u8(FRAME_SHUTDOWN);
                0
            },
            Frame::OpenStream {
                sid,
                prio,
                promises,
            } => {
                bytes.put_u8(FRAME_OPEN_STREAM);
                bytes.put_slice(&sid.to_le_bytes());
                bytes.put_u8(prio);
                bytes.put_u8(promises.to_le_bytes()[0]);
                0
            },
            Frame::CloseStream { sid } => {
                bytes.put_u8(FRAME_CLOSE_STREAM);
                bytes.put_slice(&sid.to_le_bytes());
                0
            },
            Frame::DataHeader { mid, sid, length } => {
                bytes.put_u8(FRAME_DATA_HEADER);
                bytes.put_u64_le(mid);
                bytes.put_slice(&sid.to_le_bytes());
                bytes.put_u64_le(length);
                0
            },
            Frame::Data { mid, start, data } => {
                bytes.put_u8(FRAME_DATA);
                bytes.put_u64_le(mid);
                bytes.put_u64_le(start);
                bytes.put_u16_le(data.len() as u16);
                bytes.put_slice(&data);
                data.len() as u64
            },
        }
    }

    pub(crate) fn to_frame(bytes: &mut BytesMut) -> Option<Self> {
        let frame_no = match bytes.first() {
            Some(&f) => f,
            None => return None,
        };
        let size = match frame_no {
            FRAME_SHUTDOWN => Self::SHUTDOWN_CNS,
            FRAME_OPEN_STREAM => Self::OPEN_STREAM_CNS,
            FRAME_CLOSE_STREAM => Self::CLOSE_STREAM_CNS,
            FRAME_DATA_HEADER => Self::DATA_HEADER_CNS,
            FRAME_DATA => {
                if bytes.len() < 17 + 1 + 1 {
                    return None;
                }
                u16::from_le_bytes([bytes[16 + 1], bytes[17 + 1]]) as usize + Self::DATA_CNS
            },
            _ => return None,
        };

        if bytes.len() < size + 1 {
            return None;
        }

        let frame = match frame_no {
            FRAME_SHUTDOWN => {
                let _ = bytes.split_to(size + 1);
                Frame::Shutdown
            },
            FRAME_OPEN_STREAM => {
                let mut bytes = bytes.split_to(size + 1);
                bytes.advance(1);
                Frame::OpenStream {
                    sid: Sid::new(bytes.get_u64_le()),
                    prio: bytes.get_u8(),
                    promises: Promises::from_bits_truncate(bytes.get_u8()),
                }
            },
            FRAME_CLOSE_STREAM => {
                let mut bytes = bytes.split_to(size + 1);
                bytes.advance(1);
                Frame::CloseStream {
                    sid: Sid::new(bytes.get_u64_le()),
                }
            },
            FRAME_DATA_HEADER => {
                let mut bytes = bytes.split_to(size + 1);
                bytes.advance(1);
                Frame::DataHeader {
                    mid: bytes.get_u64_le(),
                    sid: Sid::new(bytes.get_u64_le()),
                    length: bytes.get_u64_le(),
                }
            },
            FRAME_DATA => {
                let mut info = bytes.split_to(Self::DATA_CNS + 1);
                info.advance(1);
                let mid = info.get_u64_le();
                let start = info.get_u64_le();
                let length = info.get_u16_le();
                debug_assert_eq!(length as usize, size - Self::DATA_CNS);
                let data = bytes.split_to(length as usize);
                let data = data.to_vec();
                Frame::Data { mid, start, data }
            },
            _ => unreachable!("Frame::to_frame should be handled before!"),
        };
        Some(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{VELOREN_MAGIC_NUMBER, VELOREN_NETWORK_VERSION};

    fn get_initframes() -> Vec<InitFrame> {
        vec![
            InitFrame::Handshake {
                magic_number: VELOREN_MAGIC_NUMBER,
                version: VELOREN_NETWORK_VERSION,
            },
            InitFrame::Init {
                pid: Pid::fake(0),
                secret: 0u128,
            },
            InitFrame::Raw(vec![1, 2, 3]),
        ]
    }

    fn get_frames() -> Vec<Frame> {
        vec![
            Frame::OpenStream {
                sid: Sid::new(1337),
                prio: 14,
                promises: Promises::GUARANTEED_DELIVERY,
            },
            Frame::DataHeader {
                sid: Sid::new(1337),
                mid: 0,
                length: 36,
            },
            Frame::Data {
                mid: 0,
                start: 0,
                data: vec![77u8; 20],
            },
            Frame::Data {
                mid: 0,
                start: 20,
                data: vec![42u8; 16],
            },
            Frame::CloseStream {
                sid: Sid::new(1337),
            },
            Frame::Shutdown,
        ]
    }

    #[test]
    fn initframe_individual() {
        let dupl = |frame: InitFrame| {
            let mut buffer = BytesMut::with_capacity(1500);
            InitFrame::to_bytes(frame.clone(), &mut buffer);
            InitFrame::to_frame(&mut buffer)
        };

        for frame in get_initframes() {
            println!("initframe: {:?}", &frame);
            assert_eq!(Some(frame.clone()), dupl(frame));
        }
    }

    #[test]
    fn initframe_multiple() {
        let mut buffer = BytesMut::with_capacity(3000);

        let mut frames = get_initframes();
        // to string
        for f in &frames {
            InitFrame::to_bytes(f.clone(), &mut buffer);
        }

        // from string
        let mut framesd = frames
            .iter()
            .map(|&_| InitFrame::to_frame(&mut buffer))
            .collect::<Vec<_>>();

        // compare
        for (f, fd) in frames.drain(..).zip(framesd.drain(..)) {
            println!("initframe: {:?}", &f);
            assert_eq!(Some(f), fd);
        }
    }

    #[test]
    fn frame_individual() {
        let dupl = |frame: Frame| {
            let mut buffer = BytesMut::with_capacity(1500);
            Frame::to_bytes(frame.clone(), &mut buffer);
            Frame::to_frame(&mut buffer)
        };

        for frame in get_frames() {
            println!("frame: {:?}", &frame);
            assert_eq!(Some(frame.clone()), dupl(frame));
        }
    }

    #[test]
    fn frame_multiple() {
        let mut buffer = BytesMut::with_capacity(3000);

        let mut frames = get_frames();
        // to string
        for f in &frames {
            Frame::to_bytes(f.clone(), &mut buffer);
        }

        // from string
        let mut framesd = frames
            .iter()
            .map(|&_| Frame::to_frame(&mut buffer))
            .collect::<Vec<_>>();

        // compare
        for (f, fd) in frames.drain(..).zip(framesd.drain(..)) {
            println!("frame: {:?}", &f);
            assert_eq!(Some(f), fd);
        }
    }

    #[test]
    fn frame_exact_size() {
        const SIZE: usize = Frame::CLOSE_STREAM_CNS+1/*first byte*/;
        let mut buffer = BytesMut::with_capacity(SIZE);

        let frame1 = Frame::CloseStream { sid: Sid::new(2) };
        Frame::to_bytes(frame1.clone(), &mut buffer);
        assert_eq!(buffer.len(), SIZE);
        let mut deque = buffer.iter().map(|b| *b).collect();
        let frame2 = Frame::to_frame(&mut deque);
        assert_eq!(Some(frame1), frame2);
    }

    #[test]
    fn initframe_too_short_buffer() {
        let mut buffer = BytesMut::with_capacity(10);

        let frame1 = InitFrame::Handshake {
            magic_number: VELOREN_MAGIC_NUMBER,
            version: VELOREN_NETWORK_VERSION,
        };
        InitFrame::to_bytes(frame1.clone(), &mut buffer);
    }

    #[test]
    fn initframe_too_less_data() {
        let mut buffer = BytesMut::with_capacity(20);

        let frame1 = InitFrame::Handshake {
            magic_number: VELOREN_MAGIC_NUMBER,
            version: VELOREN_NETWORK_VERSION,
        };
        let _ = InitFrame::to_bytes(frame1.clone(), &mut buffer);
        buffer.truncate(6); // simulate partial retrieve
        let frame1d = InitFrame::to_frame(&mut buffer);
        assert_eq!(frame1d, None);
    }

    #[test]
    fn initframe_rubish() {
        let mut buffer = BytesMut::from(&b"dtrgwcser"[..]);
        assert_eq!(
            InitFrame::to_frame(&mut buffer),
            Some(InitFrame::Raw(b"dtrgwcser".to_vec()))
        );
    }

    #[test]
    fn initframe_attack_too_much_length() {
        let mut buffer = BytesMut::with_capacity(50);

        let frame1 = InitFrame::Raw(b"foobar".to_vec());
        let _ = InitFrame::to_bytes(frame1.clone(), &mut buffer);
        buffer[1] = 255;
        let framed = InitFrame::to_frame(&mut buffer);
        assert_eq!(framed, Some(frame1));
    }

    #[test]
    fn initframe_attack_too_low_length() {
        let mut buffer = BytesMut::with_capacity(50);

        let frame1 = InitFrame::Raw(b"foobar".to_vec());
        let _ = InitFrame::to_bytes(frame1.clone(), &mut buffer);
        buffer[1] = 3;
        let framed = InitFrame::to_frame(&mut buffer);
        // we accept a different frame here, as it's RAW and debug only!
        assert_eq!(framed, Some(InitFrame::Raw(b"foo".to_vec())));
    }

    #[test]
    fn frame_too_short_buffer() {
        let mut buffer = BytesMut::with_capacity(10);

        let frame1 = Frame::OpenStream {
            sid: Sid::new(88),
            promises: Promises::ENCRYPTED,
            prio: 88,
        };
        Frame::to_bytes(frame1.clone(), &mut buffer);
    }

    #[test]
    fn frame_too_less_data() {
        let mut buffer = BytesMut::with_capacity(20);

        let frame1 = Frame::OpenStream {
            sid: Sid::new(88),
            promises: Promises::ENCRYPTED,
            prio: 88,
        };
        Frame::to_bytes(frame1.clone(), &mut buffer);
        buffer.truncate(6); // simulate partial retrieve
        let frame1d = Frame::to_frame(&mut buffer);
        assert_eq!(frame1d, None);
    }

    #[test]
    fn frame_rubish() {
        let mut buffer = BytesMut::from(&b"dtrgwcser"[..]);
        assert_eq!(Frame::to_frame(&mut buffer), None);
    }

    #[test]
    fn frame_attack_too_much_length() {
        let mut buffer = BytesMut::with_capacity(50);

        let frame1 = Frame::Data {
            mid: 7u64,
            start: 1u64,
            data: b"foobar".to_vec(),
        };

        Frame::to_bytes(frame1.clone(), &mut buffer);
        buffer[17] = 255;
        let framed = Frame::to_frame(&mut buffer);
        assert_eq!(framed, None);
    }

    #[test]
    fn frame_attack_too_low_length() {
        let mut buffer = BytesMut::with_capacity(50);

        let frame1 = Frame::Data {
            mid: 7u64,
            start: 1u64,
            data: b"foobar".to_vec(),
        };

        Frame::to_bytes(frame1.clone(), &mut buffer);
        buffer[17] = 3;
        let framed = Frame::to_frame(&mut buffer);
        assert_eq!(
            framed,
            Some(Frame::Data {
                mid: 7u64,
                start: 1u64,
                data: b"foo".to_vec(),
            })
        );
        //next = Invalid => Empty
        let framed = Frame::to_frame(&mut buffer);
        assert_eq!(framed, None);
    }
}
