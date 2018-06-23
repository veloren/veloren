use super::packet::{OutgoingPacket, IncommingPacket, Frame, FrameError, PacketData};
use super::message::{Message, Error};
use bincode;
use super::tcp::Tcp;
use super::udp::Udp;
use super::udpmgr::UdpMgr;
use super::protocol::Protocol;
use std::net::{TcpStream, TcpListener, UdpSocket};
use std::thread;
use std::sync::{Arc, Mutex, MutexGuard};

struct TestPorts {
    next: Mutex<u32>,
}

impl TestPorts {
    pub fn new() -> TestPorts {
        TestPorts {
            next: Mutex::new(50000),
        }
    }

    pub fn next(&self) -> String {
        let mut n = self.next.lock().unwrap();
        *n += 1;
        format!("127.0.0.1:{}", *n)
    }
}

lazy_static! {
    static ref PORTS: TestPorts = TestPorts::new();
}

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
    let f = p.generate_frame(10);
    check_header(&f, 3, 12);
    let f = p.generate_frame(12);
    check_data(&f, 3, 0, vec!(0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0) );
    let f = p.generate_frame(10);
    check_done(&f );
}

#[test]
fn construct_frame2() {
    let mut p = OutgoingPacket::new(TestMessage::smallMessage{ value: 7 }, 3);
    let f = p.generate_frame(10);
    check_header(&f, 3, 12);
    let f = p.generate_frame(100);
    check_data(&f, 3, 0, vec!(0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0) );
    let f = p.generate_frame(10);
    check_done(&f );
}


#[test]
fn construct_splitframe() {
    let mut p = OutgoingPacket::new(TestMessage::smallMessage{ value: 7 }, 3);
    let f = p.generate_frame(10);
    check_header(&f, 3, 12);
    let f = p.generate_frame(10);
    check_data(&f, 3, 0, vec!(0, 0, 0, 0, 7, 0, 0, 0, 0, 0) );
    let f = p.generate_frame(10);
    check_data(&f, 3, 1, vec!(0, 0) );
    let f = p.generate_frame(10);
    check_done(&f);
}

#[test]
fn construct_largeframe() {
    let mut p = OutgoingPacket::new(TestMessage::largeMessage{ text: "1234567890A1234567890B1234567890C1234567890D1234567890E1234567890F1234567890G1234567890H1234567890".to_string() }, 123);
    let f = p.generate_frame(10);
    check_header(&f, 123, 110);
    let f = p.generate_frame(40);
    check_data(&f, 123, 0, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54) );
    let f = p.generate_frame(10);
    check_data(&f, 123, 1, vec!(55, 56, 57, 48, 67, 49, 50, 51, 52, 53) );
    let f = p.generate_frame(0);
    check_data(&f, 123, 2, vec!() );
    let f = p.generate_frame(50);
    check_data(&f, 123, 3, vec!(54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72) );
    let f = p.generate_frame(50);
    check_data(&f, 123, 4, vec!(49, 50, 51, 52, 53, 54, 55, 56, 57, 48) );
    let f = p.generate_frame(10);
    check_done(&f);
}

#[test]
fn construct_message() {
    let mut p = OutgoingPacket::new(TestMessage::largeMessage{ text: "1234567890A1234567890B1234567890C1234567890D1234567890E1234567890F1234567890G1234567890H1234567890".to_string() }, 123);
    let f1 = p.generate_frame(10);
    check_header(&f1, 123, 110);
    let f2= p.generate_frame(40);
    check_data(&f2, 123, 0, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54) );
    let f3 = p.generate_frame(10);
    check_data(&f3, 123, 1, vec!(55, 56, 57, 48, 67, 49, 50, 51, 52, 53) );
    let f4 = p.generate_frame(0);
    check_data(&f4, 123, 2, vec!() );
    let f5 = p.generate_frame(50);
    check_data(&f5, 123, 3, vec!(54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72) );
    let f6 = p.generate_frame(50);
    check_data(&f6, 123, 4, vec!(49, 50, 51, 52, 53, 54, 55, 56, 57, 48) );
    let f7 = p.generate_frame(10);
    check_done(&f7);
    let mut i = IncommingPacket::new(f1.unwrap());
    assert!(!i.load_data_frame(f2.unwrap()));
    assert!(!i.load_data_frame(f3.unwrap()));
    assert!(!i.load_data_frame(f4.unwrap()));
    assert!(!i.load_data_frame(f5.unwrap())); //false
    assert!(i.load_data_frame(f6.unwrap())); //true
    let data = i.data();
    assert_eq!(*data, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 67, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48));
    assert_eq!(data.len(), 110);
}

#[test]
#[should_panic]
fn construct_message_wrong_order() {
    let mut p = OutgoingPacket::new(TestMessage::largeMessage{ text: "1234567890A1234567890B1234567890C1234567890D1234567890E1234567890F1234567890G1234567890H1234567890".to_string() }, 123);
    let f1 = p.generate_frame(10);
    check_header(&f1, 123, 110);
    let f2= p.generate_frame(40);
    check_data(&f2, 123, 0, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54) );
    let f3 = p.generate_frame(10);
    check_data(&f3, 123, 1, vec!(55, 56, 57, 48, 67, 49, 50, 51, 52, 53) );
    let f4 = p.generate_frame(0);
    check_data(&f4, 123, 2, vec!() );
    let f5 = p.generate_frame(50);
    check_data(&f5, 123, 3, vec!(54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72) );
    let f6 = p.generate_frame(50);
    check_data(&f6, 123, 4, vec!(49, 50, 51, 52, 53, 54, 55, 56, 57, 48) );
    let f7 = p.generate_frame(10);
    check_done(&f7);
    let mut i = IncommingPacket::new(f1.unwrap());
    assert!(!i.load_data_frame(f6.unwrap()));
    assert!(!i.load_data_frame(f4.unwrap()));
    assert!(!i.load_data_frame(f2.unwrap()));
    assert!(!i.load_data_frame(f5.unwrap())); //false
    assert!(i.load_data_frame(f3.unwrap())); //true
    let data = i.data();
    assert_eq!(*data, vec!(1, 0, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 65, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 66, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 67, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 68, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 69, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 70, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 71, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 72, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48));
    assert_eq!(data.len(), 110);
}

#[test]
fn tcp_pingpong() {
    let serverip = PORTS.next();
    let mut listen = TcpListener::bind(&serverip).unwrap();
    let handle = thread::spawn(move || {
        let mut stream = listen.accept().unwrap().0; //blocks until client connected
        let mut server = Tcp::new_stream(stream).unwrap();
        let frame = server.recv().unwrap(); //wait for ping
        match frame {
            Frame::Header{id, length} => {
                assert_eq!(id, 123);
                assert_eq!(length, 9876);
            }
            Frame::Data{id, frame_no, data} => {
                assert!(false);
            }
        }
        server.send(Frame::Data{id: 777, frame_no: 333, data: vec!(0, 10)}); //send pong
    });
    let mut client = Tcp::new(&serverip).unwrap();
    client.send(Frame::Header{id: 123, length: 9876}); //send ping
    let frame = client.recv().unwrap(); //wait for pong
    match frame {
        Frame::Header{id, length} => {
            assert!(false);
        }
        Frame::Data{id, frame_no, data} => {
            assert_eq!(id, 777);
            assert_eq!(frame_no, 333);
            assert_eq!(data, vec!(0, 10));
            assert_ne!(data, vec!(0, 11));
        }
    }
    handle.join().unwrap();
}

//test for manual testing
//#[test]
fn tcp_doublerecv() {
    // Server waits for ping
    // then server will send a pong
    // but there are 2 threads listening on the same client
    // only one will recv it, the other one will panic
    let serverip = PORTS.next();
    let mut listen = TcpListener::bind(&serverip).unwrap();
    let handle = thread::spawn(move || {
        let mut stream = listen.accept().unwrap().0; //blocks until client connected
        let mut server = Tcp::new_stream(stream).unwrap();
        let frame = server.recv().unwrap(); //wait for ping
        match frame {
            Frame::Header{id, length} => {
                assert_eq!(id, 123);
                assert_eq!(length, 9876);
            }
            Frame::Data{id, frame_no, data} => {
                assert!(false);
            }
        }
        server.send(Frame::Data{id: 777, frame_no: 333, data: vec!(0, 10)}); //send pong
    });
    let clientstream = TcpStream::connect(&serverip).unwrap();
    let mut client = Tcp::new_stream(clientstream.try_clone().unwrap()).unwrap();
    let handle2 = thread::spawn(move || {
        let frame = client.recv().unwrap(); //wait for pong
        match frame {
            Frame::Header{id, length} => {
                assert!(false);
            }
            Frame::Data{id, frame_no, data} => {
                assert_eq!(id, 777);
                assert_eq!(frame_no, 333);
                assert_eq!(data, vec!(0, 10));
                assert_ne!(data, vec!(0, 11));
            }
        }
    });
    let mut client = Tcp::new_stream(clientstream.try_clone().unwrap()).unwrap();
    let handle3 = thread::spawn(move || {
        let frame = client.recv().unwrap(); //wait for pong
        match frame {
            Frame::Header{id, length} => {
                assert!(false);
            }
            Frame::Data{id, frame_no, data} => {
                assert_eq!(id, 777);
                assert_eq!(frame_no, 333);
                assert_eq!(data, vec!(0, 10));
                assert_ne!(data, vec!(0, 11));
            }
        }
    });
    let mut client = Tcp::new_stream(clientstream.try_clone().unwrap()).unwrap();
    client.send(Frame::Header{id: 123, length: 9876}); //send ping
    handle.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
}

#[test]
fn udp_pingpong() {
    let mgr = UdpMgr::new();
    let serverip = PORTS.next();
    let clientip = PORTS.next();
    let server = UdpMgr::start_udp(mgr.clone(), &serverip, &clientip); // server has to know client ip
    let client = UdpMgr::start_udp(mgr.clone(), &clientip, &serverip);
    client.send(Frame::Header{id: 123, length: 9876}).unwrap(); //send ping
    let frame = server.recv().unwrap(); //wait for ping
    match frame {
        Frame::Header{id, length} => {
            assert_eq!(id, 123);
            assert_eq!(length, 9876);
        }
        Frame::Data{..} => {
            assert!(false);
        }
    }
    server.send(Frame::Data{id: 777, frame_no: 333, data: vec!(0, 10)}).unwrap(); //send pong
    let frame = client.recv().unwrap(); //wait for pong
    match frame {
        Frame::Header{..} => {
            assert!(false);
        }
        Frame::Data{id, frame_no, data} => {
            assert_eq!(id, 777);
            assert_eq!(frame_no, 333);
            assert_eq!(data, vec!(0, 10));
            assert_ne!(data, vec!(0, 11));
        }
    }
}

#[test]
fn udp_pingpong_2clients() {
    let mgr = UdpMgr::new();
    let serverip = PORTS.next();
    let clientip = PORTS.next();
    let clientip2 = PORTS.next();
    let server = UdpMgr::start_udp(mgr.clone(), &serverip, &clientip);
    let server2 = UdpMgr::start_udp(mgr.clone(), &serverip, &clientip2);
    let client = UdpMgr::start_udp(mgr.clone(), &clientip, &serverip);
    let client2 = UdpMgr::start_udp(mgr.clone(), &clientip2, &serverip);
    client.send(Frame::Header{id: 123, length: 9876}).unwrap(); //send ping
    println!("send");
    let frame = server.recv().unwrap(); //wait for ping
    println!("recved");
    match frame {
        Frame::Header{id, length} => {
            assert_eq!(id, 123);
            assert_eq!(length, 9876);
        }
        Frame::Data{..} => {
            assert!(false);
        }
    }
    server2.send(Frame::Data{id: 777, frame_no: 333, data: vec!(0, 10)}).unwrap(); //send pong
    let frame = client2.recv().unwrap(); //wait for pong
    match frame {
        Frame::Header{..} => {
            assert!(false);
        }
        Frame::Data{id, frame_no, data} => {
            assert_eq!(id, 777);
            assert_eq!(frame_no, 333);
            assert_eq!(data, vec!(0, 10));
            assert_ne!(data, vec!(0, 11));
        }
    }
}

//this test should not succed, it should hang. try it manual
//#[test]
fn udp_pingpong_2clients_negative() {
    let serversock = UdpSocket::bind("127.0.0.1:51234").unwrap();
    let clientsock = UdpSocket::bind("127.0.0.1:51235").unwrap();
    let client2sock = UdpSocket::bind("127.0.0.1:51236").unwrap();
    clientsock.connect("127.0.0.1:51234").unwrap();
    let client = Udp::new_stream(clientsock, "127.0.0.1:51234").unwrap();
    let client2 = Udp::new_stream(client2sock, "127.0.0.1:51234").unwrap();
    let server = Udp::new_stream(serversock.try_clone().unwrap(), "127.0.0.1:51235").unwrap();
    let server2 = Udp::new_stream(serversock.try_clone().unwrap(), "127.0.0.1:51236").unwrap();
    client.send(Frame::Header{id: 123, length: 9876}).unwrap(); //send ping
    let frame = server2.recv().unwrap(); //wait for ping from other client
    assert!(false);
}
