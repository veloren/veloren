use super::packet::{OutgoingPacket, IncommingPacket, Frame, FrameError, PacketData};
use super::message::{Message, Error};
use bincode;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TestMessage {
    smallMessage { value: u64 },
    largeMessage { text: String },
}

impl Message for TestMessage {
    fn from_bytes(data: &[u8]) -> Result<TestMessage, Error> {
        bincode::deserialize(data).map_err(|_e| Error::CannotDeserialize)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(&self).map_err(|_e| Error::CannotSerialize)
    }
}

fn check_header(frame: &Result<Frame, FrameError>, id: u64, length: u64) {
    match frame {
        Ok(frame) => {
            match frame {
                Frame::Header{id: id2, length: length2} => {
                    assert_eq!(id, *id2);
                    assert_eq!(length, *length2);
                }
                Frame::Data{..} => {
                    assert!(false);
                }
            }
        },
        Err(FrameError::SendDone) => {
            assert!(false);
        },
    }
}

fn check_data(frame: &Result<Frame, FrameError>, id: u64, frame_no: u64, data: Vec<u8>) {
    match frame {
        Ok(frame) => {
            match frame {
                Frame::Header{..} => {
                    assert!(false);
                }
                Frame::Data{ id: id2, frame_no: frame_no2, data: data2 } => {
                    assert_eq!(id, *id2);
                    assert_eq!(frame_no, *frame_no2);
                    assert_eq!(data, *data2);
                }
            }
        },
        Err(FrameError::SendDone) => {
            assert!(false);
        },
    }
}

fn check_done(frame: &Result<Frame, FrameError>,) {
    match frame {
        Ok(_frame) => {
            assert!(false);
        },
        Err(FrameError::SendDone) => {
            assert!(true);
        },
    }
}

#[test]
fn construct_frame() {
    let mut p = OutgoingPacket::new(TestMessage::smallMessage{ value: 7 }, 3);
    let f = p.generateFrame(10);
    check_header(&f, 3, 12);
    let f = p.generateFrame(12);
    check_data(&f, 3, 0, vec!(0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0) );
    let f = p.generateFrame(10);
    check_done(&f );
}

#[test]
fn construct_frame2() {
    let mut p = OutgoingPacket::new(TestMessage::smallMessage{ value: 7 }, 3);
    let f = p.generateFrame(10);
    check_header(&f, 3, 12);
    let f = p.generateFrame(100);
    check_data(&f, 3, 0, vec!(0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0) );
    let f = p.generateFrame(10);
    check_done(&f );
}


#[test]
fn construct_splitframe() {
    let mut p = OutgoingPacket::new(TestMessage::smallMessage{ value: 7 }, 3);
    let f = p.generateFrame(10);
    check_header(&f, 3, 12);
    let f = p.generateFrame(10);
    check_data(&f, 3, 0, vec!(0, 0, 0, 0, 7, 0, 0, 0, 0, 0) );
    let f = p.generateFrame(10);
    check_data(&f, 3, 1, vec!(0, 0) );
    let f = p.generateFrame(10);
    check_done(&f);
}

#[test]
fn construct_largeframe() {
    let mut p = OutgoingPacket::new(TestMessage::largeMessage{ text: "1234567890A1234567890B1234567890C1234567890D1234567890E1234567890F1234567890G1234567890H1234567890".to_string() }, 123);
    let f = p.generateFrame(10);
    check_header(&f, 123, 110);
    let f = p.generateFrame(40);
    check_data(&f, 123, 0, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54) );
    let f = p.generateFrame(10);
    check_data(&f, 123, 1, vec!(55, 56, 57, 48, 67, 49, 50, 51, 52, 53) );
    let f = p.generateFrame(0);
    check_data(&f, 123, 2, vec!() );
    let f = p.generateFrame(50);
    check_data(&f, 123, 3, vec!(54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72) );
    let f = p.generateFrame(50);
    check_data(&f, 123, 4, vec!(49, 50, 51, 52, 53, 54, 55, 56, 57, 48) );
    let f = p.generateFrame(10);
    check_done(&f);
}

#[test]
fn construct_message() {
    let mut p = OutgoingPacket::new(TestMessage::largeMessage{ text: "1234567890A1234567890B1234567890C1234567890D1234567890E1234567890F1234567890G1234567890H1234567890".to_string() }, 123);
    let f1 = p.generateFrame(10);
    check_header(&f1, 123, 110);
    let f2= p.generateFrame(40);
    check_data(&f2, 123, 0, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54) );
    let f3 = p.generateFrame(10);
    check_data(&f3, 123, 1, vec!(55, 56, 57, 48, 67, 49, 50, 51, 52, 53) );
    let f4 = p.generateFrame(0);
    check_data(&f4, 123, 2, vec!() );
    let f5 = p.generateFrame(50);
    check_data(&f5, 123, 3, vec!(54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72) );
    let f6 = p.generateFrame(50);
    check_data(&f6, 123, 4, vec!(49, 50, 51, 52, 53, 54, 55, 56, 57, 48) );
    let f7 = p.generateFrame(10);
    check_done(&f7);
    let mut i = IncommingPacket::new(f1.unwrap());
    assert!(!i.loadDataFrame(f2.unwrap()));
    assert!(!i.loadDataFrame(f3.unwrap()));
    assert!(!i.loadDataFrame(f4.unwrap()));
    assert!(!i.loadDataFrame(f5.unwrap())); //false
    assert!(i.loadDataFrame(f6.unwrap())); //true
    let data = i.data();
    assert_eq!(*data, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 67, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48));
    assert_eq!(data.len(), 110);
}

#[test]
#[should_panic]
fn construct_message_wrong_order() {
    let mut p = OutgoingPacket::new(TestMessage::largeMessage{ text: "1234567890A1234567890B1234567890C1234567890D1234567890E1234567890F1234567890G1234567890H1234567890".to_string() }, 123);
    let f1 = p.generateFrame(10);
    check_header(&f1, 123, 110);
    let f2= p.generateFrame(40);
    check_data(&f2, 123, 0, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54) );
    let f3 = p.generateFrame(10);
    check_data(&f3, 123, 1, vec!(55, 56, 57, 48, 67, 49, 50, 51, 52, 53) );
    let f4 = p.generateFrame(0);
    check_data(&f4, 123, 2, vec!() );
    let f5 = p.generateFrame(50);
    check_data(&f5, 123, 3, vec!(54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72) );
    let f6 = p.generateFrame(50);
    check_data(&f6, 123, 4, vec!(49, 50, 51, 52, 53, 54, 55, 56, 57, 48) );
    let f7 = p.generateFrame(10);
    check_done(&f7);
    let mut i = IncommingPacket::new(f1.unwrap());
    assert!(!i.loadDataFrame(f6.unwrap()));
    assert!(!i.loadDataFrame(f4.unwrap()));
    assert!(!i.loadDataFrame(f2.unwrap()));
    assert!(!i.loadDataFrame(f5.unwrap())); //false
    assert!(i.loadDataFrame(f3.unwrap())); //true
    let data = i.data();
    assert_eq!(*data, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 67, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48));
    assert_eq!(data.len(), 110);
}

#[test]
fn tcp_pingpong() {
    let mut p = OutgoingPacket::new(TestMessage::largeMessage{ text: "1234567890A1234567890B1234567890C1234567890D1234567890E1234567890F1234567890G1234567890H1234567890".to_string() }, 123);
    let f1 = p.generateFrame(10);
    check_header(&f1, 123, 110);
    let f2= p.generateFrame(40);
    check_data(&f2, 123, 0, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54) );
    let f3 = p.generateFrame(10);
    check_data(&f3, 123, 1, vec!(55, 56, 57, 48, 67, 49, 50, 51, 52, 53) );
    let f4 = p.generateFrame(0);
    check_data(&f4, 123, 2, vec!() );
    let f5 = p.generateFrame(50);
    check_data(&f5, 123, 3, vec!(54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72) );
    let f6 = p.generateFrame(50);
    check_data(&f6, 123, 4, vec!(49, 50, 51, 52, 53, 54, 55, 56, 57, 48) );
    let f7 = p.generateFrame(10);
    check_done(&f7);
    let mut i = IncommingPacket::new(f1.unwrap());
    assert!(!i.loadDataFrame(f2.unwrap()));
    assert!(!i.loadDataFrame(f3.unwrap()));
    assert!(!i.loadDataFrame(f4.unwrap()));
    assert!(!i.loadDataFrame(f5.unwrap())); //false
    assert!(i.loadDataFrame(f6.unwrap())); //true
    let data = i.data();
    assert_eq!(*data, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 67, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48));
    assert_eq!(data.len(), 110);
}
