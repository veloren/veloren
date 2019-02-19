use std::io::Write;
use std::net::SocketAddr;

use mio::{net::TcpStream, Events, Poll, PollOpt, Ready, Token};

use super::{error::PostError, PostBox, PostOffice};

#[test]
fn basic_run() {
    let listen_addr = SocketAddr::from(([0, 0, 0, 0], 12345u16));
    let conn_addr = SocketAddr::from(([127, 0, 0, 1], 12345u16));
    let server: PostOffice<String, String> = PostOffice::new(&listen_addr).unwrap();
    let client: PostBox<String, String> = PostBox::to_server(&conn_addr).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let scon = server.get_iter().unwrap().next().unwrap().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    scon.send(String::from("foo"));
    client.send(String::from("bar"));
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert_eq!("foo", client.recv_iter().unwrap().next().unwrap().unwrap());
    assert_eq!("bar", scon.recv_iter().unwrap().next().unwrap().unwrap());
}

#[test]
fn huge_size_header() {
    let listen_addr = SocketAddr::from(([0, 0, 0, 0], 12346u16));
    let conn_addr = SocketAddr::from(([127, 0, 0, 1], 12346u16));
    let server: PostOffice<String, String> = PostOffice::new(&listen_addr).unwrap();
    let mut client = TcpStream::connect(&conn_addr).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let scon = server.get_iter().unwrap().next().unwrap().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    client.write(&[0xffu8; 64]).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert!(match scon.recv_iter().unwrap().next().unwrap() {
        Err(PostError::MsgSizeLimitExceeded) => true,
        _ => false,
    });
}

#[test]
fn disconnect() {
    let listen_addr = SocketAddr::from(([0, 0, 0, 0], 12347u16));
    let conn_addr = SocketAddr::from(([127, 0, 0, 1], 12347u16));
    let server: PostOffice<String, String> = PostOffice::new(&listen_addr).unwrap();
    {
        #[allow(unused_variables)]
        let client: PostBox<String, String> = PostBox::to_server(&conn_addr).unwrap();
    }
    std::thread::sleep(std::time::Duration::from_millis(10));
    let scon = server.get_iter().unwrap().next().unwrap().unwrap();
    scon.send(String::from("foo"));
    std::thread::sleep(std::time::Duration::from_millis(10));

    match scon.recv_iter().unwrap().next().unwrap() {
        Ok(_) => panic!("Didn't expect to receive anything"),
        Err(err) => {
            if !(match err {
                PostError::Io(e) => e,
                _ => panic!("PostError different than expected"),
            }
            .kind()
                == std::io::ErrorKind::BrokenPipe)
            {
                panic!("Error different than disconnection")
            }
        }
    }
}
