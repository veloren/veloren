use crate::types::{Mid, Pid, Prio, Promises, Sid};
use std::{collections::VecDeque, convert::TryFrom};

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
    pub(crate) fn to_bytes(self, bytes: &mut [u8]) -> usize {
        match self {
            InitFrame::Handshake {
                magic_number,
                version,
            } => {
                let x = FRAME_HANDSHAKE.to_be_bytes();
                bytes[0] = x[0];
                bytes[1..8].copy_from_slice(&magic_number);
                bytes[8..12].copy_from_slice(&version[0].to_le_bytes());
                bytes[12..16].copy_from_slice(&version[1].to_le_bytes());
                bytes[16..Self::HANDSHAKE_CNS + 1].copy_from_slice(&version[2].to_le_bytes());
                Self::HANDSHAKE_CNS + 1
            },
            InitFrame::Init { pid, secret } => {
                bytes[0] = FRAME_INIT.to_be_bytes()[0];
                bytes[1..17].copy_from_slice(&pid.to_le_bytes());
                bytes[17..Self::INIT_CNS + 1].copy_from_slice(&secret.to_le_bytes());
                Self::INIT_CNS + 1
            },
            InitFrame::Raw(data) => {
                bytes[0] = FRAME_RAW.to_be_bytes()[0];
                bytes[1..3].copy_from_slice(&(data.len() as u16).to_le_bytes());
                bytes[Self::RAW_CNS + 1..(data.len() + Self::RAW_CNS + 1)]
                    .clone_from_slice(&data[..]);
                Self::RAW_CNS + 1 + data.len()
            },
        }
    }

    pub(crate) fn to_frame(bytes: Vec<u8>) -> Option<Self> {
        let frame_no = match bytes.get(0) {
            Some(&f) => f,
            None => return None,
        };
        let frame = match frame_no {
            FRAME_HANDSHAKE => {
                if bytes.len() < Self::HANDSHAKE_CNS + 1 {
                    return None;
                }
                InitFrame::gen_handshake(
                    *<&[u8; Self::HANDSHAKE_CNS]>::try_from(&bytes[1..Self::HANDSHAKE_CNS + 1])
                        .unwrap(),
                )
            },
            FRAME_INIT => {
                if bytes.len() < Self::INIT_CNS + 1 {
                    return None;
                }
                InitFrame::gen_init(
                    *<&[u8; Self::INIT_CNS]>::try_from(&bytes[1..Self::INIT_CNS + 1]).unwrap(),
                )
            },
            FRAME_RAW => {
                if bytes.len() < Self::RAW_CNS + 1 {
                    return None;
                }
                let length = InitFrame::gen_raw(
                    *<&[u8; Self::RAW_CNS]>::try_from(&bytes[1..Self::RAW_CNS + 1]).unwrap(),
                );
                let mut data = vec![0; length as usize];
                let slice = &bytes[Self::RAW_CNS + 1..];
                if slice.len() != length as usize {
                    return None;
                }
                data.copy_from_slice(&bytes[Self::RAW_CNS + 1..]);
                InitFrame::Raw(data)
            },
            _ => InitFrame::Raw(bytes),
        };
        Some(frame)
    }

    fn gen_handshake(buf: [u8; Self::HANDSHAKE_CNS]) -> Self {
        let magic_number = *<&[u8; 7]>::try_from(&buf[0..7]).unwrap();
        InitFrame::Handshake {
            magic_number,
            version: [
                u32::from_le_bytes(*<&[u8; 4]>::try_from(&buf[7..11]).unwrap()),
                u32::from_le_bytes(*<&[u8; 4]>::try_from(&buf[11..15]).unwrap()),
                u32::from_le_bytes(*<&[u8; 4]>::try_from(&buf[15..Self::HANDSHAKE_CNS]).unwrap()),
            ],
        }
    }

    fn gen_init(buf: [u8; Self::INIT_CNS]) -> Self {
        InitFrame::Init {
            pid: Pid::from_le_bytes(*<&[u8; 16]>::try_from(&buf[0..16]).unwrap()),
            secret: u128::from_le_bytes(*<&[u8; 16]>::try_from(&buf[16..Self::INIT_CNS]).unwrap()),
        }
    }

    fn gen_raw(buf: [u8; Self::RAW_CNS]) -> u16 {
        u16::from_le_bytes(*<&[u8; 2]>::try_from(&buf[0..Self::RAW_CNS]).unwrap())
    }
}

impl Frame {
    pub(crate) const CLOSE_STREAM_CNS: usize = 8;
    /// const part of the DATA frame, actual size is variable
    pub(crate) const DATA_CNS: usize = 18;
    pub(crate) const DATA_HEADER_CNS: usize = 24;
    #[cfg(feature = "metrics")]
    pub const FRAMES_LEN: u8 = 5;
    pub(crate) const OPEN_STREAM_CNS: usize = 10;
    // Size WITHOUT the 1rst indicating byte
    pub(crate) const SHUTDOWN_CNS: usize = 0;

    #[cfg(feature = "metrics")]
    pub const fn int_to_string(i: u8) -> &'static str {
        match i {
            0 => "Shutdown",
            1 => "OpenStream",
            2 => "CloseStream",
            3 => "DataHeader",
            4 => "Data",
            _ => "",
        }
    }

    #[cfg(feature = "metrics")]
    pub fn get_int(&self) -> u8 {
        match self {
            Frame::Shutdown => 0,
            Frame::OpenStream { .. } => 1,
            Frame::CloseStream { .. } => 2,
            Frame::DataHeader { .. } => 3,
            Frame::Data { .. } => 4,
        }
    }

    #[cfg(feature = "metrics")]
    pub fn get_string(&self) -> &str { Self::int_to_string(self.get_int()) }

    //provide an appropriate buffer size. > 1500
    pub fn to_bytes(self, bytes: &mut [u8]) -> (/* buf */ usize, /* actual data */ u64) {
        match self {
            Frame::Shutdown => {
                bytes[Self::SHUTDOWN_CNS] = FRAME_SHUTDOWN.to_be_bytes()[0];
                (Self::SHUTDOWN_CNS + 1, 0)
            },
            Frame::OpenStream {
                sid,
                prio,
                promises,
            } => {
                bytes[0] = FRAME_OPEN_STREAM.to_be_bytes()[0];
                bytes[1..9].copy_from_slice(&sid.to_le_bytes());
                bytes[9] = prio.to_le_bytes()[0];
                bytes[Self::OPEN_STREAM_CNS] = promises.to_le_bytes()[0];
                (Self::OPEN_STREAM_CNS + 1, 0)
            },
            Frame::CloseStream { sid } => {
                bytes[0] = FRAME_CLOSE_STREAM.to_be_bytes()[0];
                bytes[1..Self::CLOSE_STREAM_CNS + 1].copy_from_slice(&sid.to_le_bytes());
                (Self::CLOSE_STREAM_CNS + 1, 0)
            },
            Frame::DataHeader { mid, sid, length } => {
                bytes[0] = FRAME_DATA_HEADER.to_be_bytes()[0];
                bytes[1..9].copy_from_slice(&mid.to_le_bytes());
                bytes[9..17].copy_from_slice(&sid.to_le_bytes());
                bytes[17..Self::DATA_HEADER_CNS + 1].copy_from_slice(&length.to_le_bytes());
                (Self::DATA_HEADER_CNS + 1, 0)
            },
            Frame::Data { mid, start, data } => {
                bytes[0] = FRAME_DATA.to_be_bytes()[0];
                bytes[1..9].copy_from_slice(&mid.to_le_bytes());
                bytes[9..17].copy_from_slice(&start.to_le_bytes());
                bytes[17..Self::DATA_CNS + 1].copy_from_slice(&(data.len() as u16).to_le_bytes());
                bytes[Self::DATA_CNS + 1..(data.len() + Self::DATA_CNS + 1)]
                    .clone_from_slice(&data[..]);
                (Self::DATA_CNS + 1 + data.len(), data.len() as u64)
            },
        }
    }

    pub(crate) fn to_frame(bytes: &mut VecDeque<u8>) -> Option<Self> {
        let frame_no = match bytes.get(0) {
            Some(&f) => f,
            None => return None,
        };
        let size = match frame_no {
            FRAME_SHUTDOWN => Self::SHUTDOWN_CNS,
            FRAME_OPEN_STREAM => Self::OPEN_STREAM_CNS,
            FRAME_CLOSE_STREAM => Self::CLOSE_STREAM_CNS,
            FRAME_DATA_HEADER => Self::DATA_HEADER_CNS,
            FRAME_DATA => {
                u16::from_le_bytes([bytes[16 + 1], bytes[17 + 1]]) as usize + Self::DATA_CNS
            },
            _ => return None,
        };

        if bytes.len() < size + 1 {
            return None;
        }

        let frame = match frame_no {
            FRAME_SHUTDOWN => {
                let _ = bytes.drain(..size + 1);
                Frame::Shutdown
            },
            FRAME_OPEN_STREAM => {
                let bytes = bytes.drain(..size + 1).skip(1).collect::<Vec<u8>>();
                Frame::gen_open_stream(<[u8; 10]>::try_from(bytes).unwrap())
            },
            FRAME_CLOSE_STREAM => {
                let bytes = bytes.drain(..size + 1).skip(1).collect::<Vec<u8>>();
                Frame::gen_close_stream(<[u8; 8]>::try_from(bytes).unwrap())
            },
            FRAME_DATA_HEADER => {
                let bytes = bytes.drain(..size + 1).skip(1).collect::<Vec<u8>>();
                Frame::gen_data_header(<[u8; 24]>::try_from(bytes).unwrap())
            },
            FRAME_DATA => {
                let info = bytes
                    .drain(..Self::DATA_CNS + 1)
                    .skip(1)
                    .collect::<Vec<u8>>();
                let (mid, start, length) = Frame::gen_data(<[u8; 18]>::try_from(info).unwrap());
                debug_assert_eq!(length as usize, size - Self::DATA_CNS);
                let data = bytes.drain(..length as usize).collect::<Vec<u8>>();
                Frame::Data { mid, start, data }
            },
            _ => unreachable!("Frame::to_frame should be handled before!"),
        };
        Some(frame)
    }

    fn gen_open_stream(buf: [u8; Self::OPEN_STREAM_CNS]) -> Self {
        Frame::OpenStream {
            sid: Sid::from_le_bytes(*<&[u8; 8]>::try_from(&buf[0..8]).unwrap()),
            prio: buf[8],
            promises: Promises::from_bits_truncate(buf[Self::OPEN_STREAM_CNS - 1]),
        }
    }

    fn gen_close_stream(buf: [u8; Self::CLOSE_STREAM_CNS]) -> Self {
        Frame::CloseStream {
            sid: Sid::from_le_bytes(
                *<&[u8; 8]>::try_from(&buf[0..Self::CLOSE_STREAM_CNS]).unwrap(),
            ),
        }
    }

    fn gen_data_header(buf: [u8; Self::DATA_HEADER_CNS]) -> Self {
        Frame::DataHeader {
            mid: Mid::from_le_bytes(*<&[u8; 8]>::try_from(&buf[0..8]).unwrap()),
            sid: Sid::from_le_bytes(*<&[u8; 8]>::try_from(&buf[8..16]).unwrap()),
            length: u64::from_le_bytes(
                *<&[u8; 8]>::try_from(&buf[16..Self::DATA_HEADER_CNS]).unwrap(),
            ),
        }
    }

    fn gen_data(buf: [u8; Self::DATA_CNS]) -> (Mid, u64, u16) {
        let mid = Mid::from_le_bytes(*<&[u8; 8]>::try_from(&buf[0..8]).unwrap());
        let start = u64::from_le_bytes(*<&[u8; 8]>::try_from(&buf[8..16]).unwrap());
        let length = u16::from_le_bytes(*<&[u8; 2]>::try_from(&buf[16..Self::DATA_CNS]).unwrap());
        (mid, start, length)
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
            let mut buffer = vec![0u8; 1500];
            let size = InitFrame::to_bytes(frame.clone(), &mut buffer);
            buffer.truncate(size);
            InitFrame::to_frame(buffer)
        };

        for frame in get_initframes() {
            println!("initframe: {:?}", &frame);
            assert_eq!(Some(frame.clone()), dupl(frame));
        }
    }

    #[test]
    fn initframe_multiple() {
        let mut buffer = vec![0u8; 3000];

        let mut frames = get_initframes();
        let mut last = 0;
        // to string
        let sizes = frames
            .iter()
            .map(|f| {
                let s = InitFrame::to_bytes(f.clone(), &mut buffer[last..]);
                last += s;
                s
            })
            .collect::<Vec<_>>();

        // from string
        let mut last = 0;
        let mut framesd = sizes
            .iter()
            .map(|&s| {
                let f = InitFrame::to_frame(buffer[last..last + s].to_vec());
                last += s;
                f
            })
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
            let mut buffer = vec![0u8; 1500];
            let (size, _) = Frame::to_bytes(frame.clone(), &mut buffer);
            let mut deque = buffer[..size].iter().map(|b| *b).collect();
            Frame::to_frame(&mut deque)
        };

        for frame in get_frames() {
            println!("frame: {:?}", &frame);
            assert_eq!(Some(frame.clone()), dupl(frame));
        }
    }

    #[test]
    fn frame_multiple() {
        let mut buffer = vec![0u8; 3000];

        let mut frames = get_frames();
        let mut last = 0;
        // to string
        let sizes = frames
            .iter()
            .map(|f| {
                let s = Frame::to_bytes(f.clone(), &mut buffer[last..]).0;
                last += s;
                s
            })
            .collect::<Vec<_>>();

        assert_eq!(sizes[0], 1 + Frame::OPEN_STREAM_CNS);
        assert_eq!(sizes[1], 1 + Frame::DATA_HEADER_CNS);
        assert_eq!(sizes[2], 1 + Frame::DATA_CNS + 20);
        assert_eq!(sizes[3], 1 + Frame::DATA_CNS + 16);
        assert_eq!(sizes[4], 1 + Frame::CLOSE_STREAM_CNS);
        assert_eq!(sizes[5], 1 + Frame::SHUTDOWN_CNS);

        let mut buffer = buffer.drain(..).collect::<VecDeque<_>>();

        // from string
        let mut framesd = sizes
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
        let mut buffer = vec![0u8; Frame::CLOSE_STREAM_CNS+1/*first byte*/];

        let frame1 = Frame::CloseStream {
            sid: Sid::new(1337),
        };
        let _ = Frame::to_bytes(frame1.clone(), &mut buffer);
        let mut deque = buffer.iter().map(|b| *b).collect();
        let frame2 = Frame::to_frame(&mut deque);
        assert_eq!(Some(frame1), frame2);
    }

    #[test]
    #[should_panic]
    fn initframe_too_short_buffer() {
        let mut buffer = vec![0u8; 10];

        let frame1 = InitFrame::Handshake {
            magic_number: VELOREN_MAGIC_NUMBER,
            version: VELOREN_NETWORK_VERSION,
        };
        let _ = InitFrame::to_bytes(frame1.clone(), &mut buffer);
    }

    #[test]
    fn initframe_too_less_data() {
        let mut buffer = vec![0u8; 20];

        let frame1 = InitFrame::Handshake {
            magic_number: VELOREN_MAGIC_NUMBER,
            version: VELOREN_NETWORK_VERSION,
        };
        let _ = InitFrame::to_bytes(frame1.clone(), &mut buffer);
        buffer.truncate(6); // simulate partial retrieve
        let frame1d = InitFrame::to_frame(buffer[..6].to_vec());
        assert_eq!(frame1d, None);
    }

    #[test]
    fn initframe_rubish() {
        let buffer = b"dtrgwcser".to_vec();
        assert_eq!(
            InitFrame::to_frame(buffer),
            Some(InitFrame::Raw(b"dtrgwcser".to_vec()))
        );
    }

    #[test]
    fn initframe_attack_too_much_length() {
        let mut buffer = vec![0u8; 50];

        let frame1 = InitFrame::Raw(b"foobar".to_vec());
        let _ = InitFrame::to_bytes(frame1.clone(), &mut buffer);
        buffer[2] = 255;
        let framed = InitFrame::to_frame(buffer);
        assert_eq!(framed, None);
    }

    #[test]
    fn initframe_attack_too_low_length() {
        let mut buffer = vec![0u8; 50];

        let frame1 = InitFrame::Raw(b"foobar".to_vec());
        let _ = InitFrame::to_bytes(frame1.clone(), &mut buffer);
        buffer[2] = 3;
        let framed = InitFrame::to_frame(buffer);
        assert_eq!(framed, None);
    }

    #[test]
    #[should_panic]
    fn frame_too_short_buffer() {
        let mut buffer = vec![0u8; 10];

        let frame1 = Frame::OpenStream {
            sid: Sid::new(88),
            promises: Promises::ENCRYPTED,
            prio: 88,
        };
        let _ = Frame::to_bytes(frame1.clone(), &mut buffer);
    }

    #[test]
    fn frame_too_less_data() {
        let mut buffer = vec![0u8; 20];

        let frame1 = Frame::OpenStream {
            sid: Sid::new(88),
            promises: Promises::ENCRYPTED,
            prio: 88,
        };
        let _ = Frame::to_bytes(frame1.clone(), &mut buffer);
        buffer.truncate(6); // simulate partial retrieve
        let mut buffer = buffer.drain(..6).collect::<VecDeque<_>>();
        let frame1d = Frame::to_frame(&mut buffer);
        assert_eq!(frame1d, None);
    }

    #[test]
    fn frame_rubish() {
        let mut buffer = b"dtrgwcser".iter().map(|u| *u).collect::<VecDeque<_>>();
        assert_eq!(Frame::to_frame(&mut buffer), None);
    }

    #[test]
    fn frame_attack_too_much_length() {
        let mut buffer = vec![0u8; 50];

        let frame1 = Frame::Data {
            mid: 7u64,
            start: 1u64,
            data: b"foobar".to_vec(),
        };

        let _ = Frame::to_bytes(frame1.clone(), &mut buffer);
        buffer[17] = 255;
        let mut buffer = buffer.drain(..).collect::<VecDeque<_>>();
        let framed = Frame::to_frame(&mut buffer);
        assert_eq!(framed, None);
    }

    #[test]
    fn frame_attack_too_low_length() {
        let mut buffer = vec![0u8; 50];

        let frame1 = Frame::Data {
            mid: 7u64,
            start: 1u64,
            data: b"foobar".to_vec(),
        };

        let _ = Frame::to_bytes(frame1.clone(), &mut buffer);
        buffer[17] = 3;
        let mut buffer = buffer.drain(..).collect::<VecDeque<_>>();
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

    #[test]
    fn frame_int2str() {
        assert_eq!(Frame::int_to_string(0), "Shutdown");
        assert_eq!(Frame::int_to_string(1), "OpenStream");
        assert_eq!(Frame::int_to_string(2), "CloseStream");
        assert_eq!(Frame::int_to_string(3), "DataHeader");
        assert_eq!(Frame::int_to_string(4), "Data");
    }
}
