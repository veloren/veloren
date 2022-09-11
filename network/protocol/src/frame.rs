use crate::types::{Bandwidth, Mid, Pid, Prio, Promises, Sid};
use bytes::{Buf, BufMut, Bytes, BytesMut};

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
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum InitFrame {
    Handshake {
        magic_number: [u8; 7],
        version: [u32; 3],
    },
    Init {
        pid: Pid,
        secret: u128,
    },
    /// WARNING: sending RAW is only for debug purposes and will drop the
    /// connection
    Raw(Vec<u8>),
}

/// Used for OUT TCP Communication between Channel --(TCP)--> Channel
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum OTFrame {
    Shutdown, /* Shutdown this channel gracefully, if all channels are shutdown (gracefully),
               * Participant is deleted */
    OpenStream {
        sid: Sid,
        prio: Prio,
        promises: Promises,
        guaranteed_bandwidth: Bandwidth,
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
        data: Bytes,
    },
}

/// Used for IN TCP Communication between Channel <--(TCP)-- Channel
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ITFrame {
    Shutdown, /* Shutdown this channel gracefully, if all channels are shutdown (gracefully),
               * Participant is deleted */
    OpenStream {
        sid: Sid,
        prio: Prio,
        promises: Promises,
        guaranteed_bandwidth: Bandwidth,
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
        data: BytesMut,
    },
}

impl InitFrame {
    // Size WITHOUT the 1rst indicating byte
    pub(crate) const HANDSHAKE_CNS: usize = 19;
    pub(crate) const INIT_CNS: usize = 32;
    /// const part of the RAW frame, actual size is variable
    pub(crate) const RAW_CNS: usize = 2;

    //provide an appropriate buffer size. > 1500
    pub(crate) fn write_bytes(self, bytes: &mut BytesMut) {
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

    pub(crate) fn read_frame(bytes: &mut BytesMut) -> Option<Self> {
        let frame_no = match bytes.first() {
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
                let mut data = vec![0; max_length];
                data.copy_from_slice(&bytes[..max_length]);
                InitFrame::Raw(data)
            },
            _ => InitFrame::Raw(bytes.to_vec()),
        };
        Some(frame)
    }
}

pub(crate) const TCP_CLOSE_STREAM_CNS: usize = 8;
/// const part of the DATA frame, actual size is variable
pub(crate) const TCP_DATA_CNS: usize = 10;
pub(crate) const TCP_DATA_HEADER_CNS: usize = 24;
pub(crate) const TCP_OPEN_STREAM_CNS: usize = 18;
// Size WITHOUT the 1rst indicating byte
pub(crate) const TCP_SHUTDOWN_CNS: usize = 0;

impl OTFrame {
    pub fn write_bytes(self, bytes: &mut BytesMut) {
        match self {
            Self::Shutdown => {
                bytes.put_u8(FRAME_SHUTDOWN);
            },
            Self::OpenStream {
                sid,
                prio,
                promises,
                guaranteed_bandwidth,
            } => {
                bytes.put_u8(FRAME_OPEN_STREAM);
                sid.to_bytes(bytes);
                bytes.put_u8(prio);
                bytes.put_u8(promises.to_le_bytes()[0]);
                bytes.put_u64_le(guaranteed_bandwidth);
            },
            Self::CloseStream { sid } => {
                bytes.put_u8(FRAME_CLOSE_STREAM);
                sid.to_bytes(bytes);
            },
            Self::DataHeader { mid, sid, length } => {
                bytes.put_u8(FRAME_DATA_HEADER);
                bytes.put_u64_le(mid);
                sid.to_bytes(bytes);
                bytes.put_u64_le(length);
            },
            Self::Data { mid, data } => {
                bytes.put_u8(FRAME_DATA);
                bytes.put_u64_le(mid);
                bytes.put_u16_le(data.len() as u16);
                bytes.put_slice(&data);
            },
        }
    }
}

impl ITFrame {
    /// Err => cannot recover
    /// Ok(None) => waiting for more data
    pub(crate) fn read_frame(bytes: &mut BytesMut) -> Result<Option<Self>, ()> {
        let frame_no = match bytes.first() {
            Some(&f) => f,
            None => return Ok(None),
        };
        let size = match frame_no {
            FRAME_SHUTDOWN => TCP_SHUTDOWN_CNS,
            FRAME_OPEN_STREAM => TCP_OPEN_STREAM_CNS,
            FRAME_CLOSE_STREAM => TCP_CLOSE_STREAM_CNS,
            FRAME_DATA_HEADER => TCP_DATA_HEADER_CNS,
            FRAME_DATA => {
                if bytes.len() < 9 + 1 + 1 {
                    return Ok(None);
                }
                u16::from_le_bytes([bytes[8 + 1], bytes[9 + 1]]) as usize + TCP_DATA_CNS
            },
            _ => return Err(()),
        };

        if bytes.len() < size + 1 {
            return Ok(None);
        }

        let frame = match frame_no {
            FRAME_SHUTDOWN => {
                let _ = bytes.split_to(size + 1);
                Self::Shutdown
            },
            FRAME_OPEN_STREAM => {
                let mut bytes = bytes.split_to(size + 1);
                bytes.advance(1);
                Self::OpenStream {
                    sid: Sid::from_bytes(&mut bytes),
                    prio: bytes.get_u8(),
                    promises: Promises::from_bits_truncate(bytes.get_u8()),
                    guaranteed_bandwidth: bytes.get_u64_le(),
                }
            },
            FRAME_CLOSE_STREAM => {
                let mut bytes = bytes.split_to(size + 1);
                bytes.advance(1);
                Self::CloseStream {
                    sid: Sid::from_bytes(&mut bytes),
                }
            },
            FRAME_DATA_HEADER => {
                let mut bytes = bytes.split_to(size + 1);
                bytes.advance(1);
                Self::DataHeader {
                    mid: bytes.get_u64_le(),
                    sid: Sid::from_bytes(&mut bytes),
                    length: bytes.get_u64_le(),
                }
            },
            FRAME_DATA => {
                bytes.advance(1);
                let mid = bytes.get_u64_le();
                let length = bytes.get_u16_le();
                debug_assert_eq!(length as usize, size - TCP_DATA_CNS);
                let data = bytes.split_to(length as usize);
                Self::Data { mid, data }
            },
            _ => unreachable!("Frame::to_frame should be handled before!"),
        };
        Ok(Some(frame))
    }
}

#[allow(unused_variables)]
impl PartialEq<ITFrame> for OTFrame {
    fn eq(&self, other: &ITFrame) -> bool {
        match self {
            Self::Shutdown => matches!(other, ITFrame::Shutdown),
            Self::OpenStream {
                sid,
                prio,
                promises,
                guaranteed_bandwidth,
            } => matches!(other, ITFrame::OpenStream {
                sid,
                prio,
                promises,
                guaranteed_bandwidth,
            }),
            Self::CloseStream { sid } => matches!(other, ITFrame::CloseStream { sid }),
            Self::DataHeader { mid, sid, length } => {
                matches!(other, ITFrame::DataHeader { mid, sid, length })
            },
            Self::Data { mid, data } => matches!(other, ITFrame::Data { mid, data }),
        }
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

    fn get_otframes() -> Vec<OTFrame> {
        vec![
            OTFrame::OpenStream {
                sid: Sid::new(1337),
                prio: 14,
                promises: Promises::GUARANTEED_DELIVERY,
                guaranteed_bandwidth: 1_000_000,
            },
            OTFrame::DataHeader {
                sid: Sid::new(1337),
                mid: 0,
                length: 36,
            },
            OTFrame::Data {
                mid: 0,
                data: Bytes::from(&[77u8; 20][..]),
            },
            OTFrame::Data {
                mid: 0,
                data: Bytes::from(&[42u8; 16][..]),
            },
            OTFrame::CloseStream {
                sid: Sid::new(1337),
            },
            OTFrame::Shutdown,
        ]
    }

    #[test]
    fn initframe_individual() {
        let dupl = |frame: InitFrame| {
            let mut buffer = BytesMut::with_capacity(1500);
            InitFrame::write_bytes(frame, &mut buffer);
            InitFrame::read_frame(&mut buffer)
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
            InitFrame::write_bytes(f.clone(), &mut buffer);
        }

        // from string
        let mut framesd = frames
            .iter()
            .map(|&_| InitFrame::read_frame(&mut buffer))
            .collect::<Vec<_>>();

        // compare
        for (f, fd) in frames.drain(..).zip(framesd.drain(..)) {
            println!("initframe: {:?}", &f);
            assert_eq!(Some(f), fd);
        }
    }

    #[test]
    fn frame_individual() {
        let dupl = |frame: OTFrame| {
            let mut buffer = BytesMut::with_capacity(1500);
            OTFrame::write_bytes(frame, &mut buffer);
            ITFrame::read_frame(&mut buffer)
        };

        for frame in get_otframes() {
            println!("frame: {:?}", &frame);
            assert_eq!(frame.clone(), dupl(frame).expect("ERR").expect("NONE"));
        }
    }

    #[test]
    fn frame_multiple() {
        let mut buffer = BytesMut::with_capacity(3000);

        let mut frames = get_otframes();
        // to string
        for f in &frames {
            OTFrame::write_bytes(f.clone(), &mut buffer);
        }

        // from string
        let mut framesd = frames
            .iter()
            .map(|&_| ITFrame::read_frame(&mut buffer))
            .collect::<Vec<_>>();

        // compare
        for (f, fd) in frames.drain(..).zip(framesd.drain(..)) {
            println!("frame: {:?}", &f);
            assert_eq!(f, fd.expect("ERR").expect("NONE"));
        }
    }

    #[test]
    fn frame_exact_size() {
        const SIZE: usize = TCP_CLOSE_STREAM_CNS+1/*first byte*/;
        let mut buffer = BytesMut::with_capacity(SIZE);

        let frame1 = OTFrame::CloseStream { sid: Sid::new(2) };
        OTFrame::write_bytes(frame1.clone(), &mut buffer);
        assert_eq!(buffer.len(), SIZE);
        let mut deque = buffer.iter().copied().collect();
        let frame2 = ITFrame::read_frame(&mut deque);
        assert_eq!(frame1, frame2.expect("ERR").expect("NONE"));
    }

    #[test]
    fn initframe_too_short_buffer() {
        let mut buffer = BytesMut::with_capacity(10);

        let frame1 = InitFrame::Handshake {
            magic_number: VELOREN_MAGIC_NUMBER,
            version: VELOREN_NETWORK_VERSION,
        };
        InitFrame::write_bytes(frame1, &mut buffer);
    }

    #[test]
    fn initframe_too_less_data() {
        let mut buffer = BytesMut::with_capacity(20);

        let frame1 = InitFrame::Handshake {
            magic_number: VELOREN_MAGIC_NUMBER,
            version: VELOREN_NETWORK_VERSION,
        };
        InitFrame::write_bytes(frame1, &mut buffer);
        buffer.truncate(6); // simulate partial retrieve
        let frame1d = InitFrame::read_frame(&mut buffer);
        assert_eq!(frame1d, None);
    }

    #[test]
    fn initframe_rubbish() {
        let mut buffer = BytesMut::from(&b"dtrgwcser"[..]);
        assert_eq!(
            InitFrame::read_frame(&mut buffer),
            Some(InitFrame::Raw(b"dtrgwcser".to_vec()))
        );
    }

    #[test]
    fn initframe_attack_too_much_length() {
        let mut buffer = BytesMut::with_capacity(50);

        let frame1 = InitFrame::Raw(b"foobar".to_vec());
        InitFrame::write_bytes(frame1.clone(), &mut buffer);
        buffer[1] = 255;
        let framed = InitFrame::read_frame(&mut buffer);
        assert_eq!(framed, Some(frame1));
    }

    #[test]
    fn initframe_attack_too_low_length() {
        let mut buffer = BytesMut::with_capacity(50);

        let frame1 = InitFrame::Raw(b"foobar".to_vec());
        InitFrame::write_bytes(frame1, &mut buffer);
        buffer[1] = 3;
        let framed = InitFrame::read_frame(&mut buffer);
        // we accept a different frame here, as it's RAW and debug only!
        assert_eq!(framed, Some(InitFrame::Raw(b"foo".to_vec())));
    }

    #[test]
    fn frame_too_short_buffer() {
        let mut buffer = BytesMut::with_capacity(10);

        let frame1 = OTFrame::OpenStream {
            sid: Sid::new(88),
            promises: Promises::ENCRYPTED,
            prio: 88,
            guaranteed_bandwidth: 1_000_000,
        };
        OTFrame::write_bytes(frame1, &mut buffer);
    }

    #[test]
    fn frame_too_less_data() {
        let mut buffer = BytesMut::with_capacity(20);

        let frame1 = OTFrame::OpenStream {
            sid: Sid::new(88),
            promises: Promises::ENCRYPTED,
            prio: 88,
            guaranteed_bandwidth: 1_000_000,
        };
        OTFrame::write_bytes(frame1, &mut buffer);
        buffer.truncate(6); // simulate partial retrieve
        let frame1d = ITFrame::read_frame(&mut buffer);
        assert_eq!(frame1d, Ok(None));
    }

    #[test]
    fn frame_rubish() {
        let mut buffer = BytesMut::from(&b"dtrgwcser"[..]);
        assert_eq!(ITFrame::read_frame(&mut buffer), Err(()));
    }

    #[test]
    fn frame_attack_too_much_length() {
        let mut buffer = BytesMut::with_capacity(50);

        let frame1 = OTFrame::Data {
            mid: 7u64,
            data: Bytes::from(&b"foobar"[..]),
        };

        OTFrame::write_bytes(frame1, &mut buffer);
        buffer[9] = 255;
        let framed = ITFrame::read_frame(&mut buffer);
        assert_eq!(framed, Ok(None));
    }

    #[test]
    fn frame_attack_too_low_length() {
        let mut buffer = BytesMut::with_capacity(50);

        let frame1 = OTFrame::Data {
            mid: 7u64,
            data: Bytes::from(&b"foobar"[..]),
        };

        OTFrame::write_bytes(frame1, &mut buffer);
        buffer[9] = 3;
        let framed = ITFrame::read_frame(&mut buffer);
        assert_eq!(
            framed,
            Ok(Some(ITFrame::Data {
                mid: 7u64,
                data: BytesMut::from(&b"foo"[..]),
            }))
        );
        //next = Invalid => Empty
        let framed = ITFrame::read_frame(&mut buffer);
        assert_eq!(framed, Err(()));
    }
}
