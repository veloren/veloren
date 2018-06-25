// Standard
use std::io::ErrorKind::UnexpectedEof;
use std::net::{TcpStream, TcpListener, Shutdown::Both};
use std::thread;
use std::sync::{Mutex};
use std::time::Duration;

// Library
use bincode;

// Parent
use super::packet::{OutgoingPacket, IncommingPacket, Frame, FrameError};
use super::message::{Message, Error, Error::NetworkErr};
use super::tcp::Tcp;
use super::udpmgr::UdpMgr;
use super::protocol::Protocol;

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
    SmallMessage { value: u64 },
    LargeMessage { text: String },
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
    let mut p = OutgoingPacket::new(TestMessage::SmallMessage{ value: 7 }, 3);
    let f = p.generate_frame(10);
    check_header(&f, 3, 12);
    let f = p.generate_frame(12);
    check_data(&f, 3, 0, vec!(0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0) );
    let f = p.generate_frame(10);
    check_done(&f );
}

#[test]
fn construct_frame2() {
    let mut p = OutgoingPacket::new(TestMessage::SmallMessage{ value: 7 }, 3);
    let f = p.generate_frame(10);
    check_header(&f, 3, 12);
    let f = p.generate_frame(100);
    check_data(&f, 3, 0, vec!(0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0) );
    let f = p.generate_frame(10);
    check_done(&f );
}


#[test]
fn construct_splitframe() {
    let mut p = OutgoingPacket::new(TestMessage::SmallMessage{ value: 7 }, 3);
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
    let mut p = OutgoingPacket::new(TestMessage::LargeMessage{ text: "1234567890A1234567890B1234567890C1234567890D1234567890E1234567890F1234567890G1234567890H1234567890".to_string() }, 123);
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
    let mut p = OutgoingPacket::new(TestMessage::LargeMessage{ text: "1234567890A1234567890B1234567890C1234567890D1234567890E1234567890F1234567890G1234567890H1234567890".to_string() }, 123);
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
    let mut p = OutgoingPacket::new(TestMessage::LargeMessage{ text: "1234567890A1234567890B1234567890C1234567890D1234567890E1234567890F1234567890G1234567890H1234567890".to_string() }, 123);
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
    let listen = TcpListener::bind(&serverip).unwrap();
    let handle = thread::spawn(move || {
        let stream = listen.accept().unwrap().0; //blocks until client connected
        let server = Tcp::new_stream(stream).unwrap();
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
    });
    let client = Tcp::new(&serverip).unwrap();
    client.send(Frame::Header{id: 123, length: 9876}).unwrap(); //send ping
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
    handle.join().unwrap();
}

#[test]
fn tcp_disconnect() {
    let serverip = PORTS.next();
    let listen = TcpListener::bind(&serverip).unwrap();
    let handle = thread::spawn(move || {
        let stream = listen.accept().unwrap().0; //blocks until client connected
        let server = Tcp::new_stream(stream).unwrap();
        let msg = server.recv(); //wait for ping
        match msg {
            Err(NetworkErr(ref e)) if e.kind() == UnexpectedEof =>{
                assert!(true);
            },
            _ => {
                assert!(false);
            }
        }
    });
    let clientstream = TcpStream::connect(&serverip).unwrap();
    let _client = Tcp::new_stream(clientstream.try_clone().unwrap()).unwrap();
    thread::sleep(Duration::from_millis(150));
    clientstream.shutdown(Both).unwrap();
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
    let listen = TcpListener::bind(&serverip).unwrap();
    let handle = thread::spawn(move || {
        let stream = listen.accept().unwrap().0; //blocks until client connected
        let server = Tcp::new_stream(stream).unwrap();
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
    });
    let clientstream = TcpStream::connect(&serverip).unwrap();
    let client = Tcp::new_stream(clientstream.try_clone().unwrap()).unwrap();
    let handle2 = thread::spawn(move || {
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
    });
    let client = Tcp::new_stream(clientstream.try_clone().unwrap()).unwrap();
    let handle3 = thread::spawn(move || {
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
    });
    let client = Tcp::new_stream(clientstream.try_clone().unwrap()).unwrap();
    client.send(Frame::Header{id: 123, length: 9876}).unwrap(); //send ping
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
    UdpMgr::stop_udp(mgr.clone(), server);
    UdpMgr::stop_udp(mgr.clone(), client);
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
    UdpMgr::stop_udp(mgr.clone(), server);
    UdpMgr::stop_udp(mgr.clone(), client);
    UdpMgr::stop_udp(mgr.clone(), server2);
    UdpMgr::stop_udp(mgr.clone(), client2);
}

#[test]
fn udp_pingpong_1000() {
    let mgr = UdpMgr::new();
    let serverip = PORTS.next();
    let clientip = PORTS.next();
    let server = UdpMgr::start_udp(mgr.clone(), &serverip, &clientip); // server has to know client ip
    let client = UdpMgr::start_udp(mgr.clone(), &clientip, &serverip);
    let clientclone = client.clone();
    let serverclone = server.clone();
    let handle = thread::spawn(move || {
        for i in 0 .. 1 {
            clientclone.send(Frame::Header{id: 123, length: 9876}).unwrap(); //send ping
            if i % 80 == 0 {
                thread::sleep(Duration::from_millis(150));
                // i cant send to much because then packages get droped by the udp UdpSocket
                // Its still a no guarantee protocol, and we expect that 1000 packets arive without loss

            }
        }
        println!(" -------- finished sending");
    });
    let handle2 = thread::spawn(move || {
        for i in 0 .. 1 {
            println!("{}", i);
            let frame = serverclone.recv().unwrap(); //wait for ping
            match frame {
                Frame::Header{id, length} => {
                    assert_eq!(id, 123);
                    assert_eq!(length, 9876);
                }
                Frame::Data{..} => {
                    assert!(false);
                }
            }
            if i % 60 == 0 {
                thread::sleep(Duration::from_millis(10));
            }
        }
        println!("finished receiving");
    });
    handle.join().unwrap();
    handle2.join().unwrap();
    UdpMgr::stop_udp(mgr.clone(), server);
    UdpMgr::stop_udp(mgr.clone(), client);
}

//this test should not succed, it should hang. try it manual
//#[test]
fn udp_pingpong_2clients_negative() {
    let mgr = UdpMgr::new();
    let serverip = PORTS.next();
    let clientip = PORTS.next();
    let clientip2 = PORTS.next();
    let _server = UdpMgr::start_udp(mgr.clone(), &serverip, &clientip); // server has to know client ip
    let server2 = UdpMgr::start_udp(mgr.clone(), &serverip, &clientip2); // server has to know client ip
    let client = UdpMgr::start_udp(mgr.clone(), &clientip, &serverip);
    let _client2 = UdpMgr::start_udp(mgr.clone(), &clientip2, &serverip);
    client.send(Frame::Header{id: 123, length: 9876}).unwrap(); //send ping
    let _frame = server2.recv().unwrap(); //wait for ping from other client
    assert!(false);
}
