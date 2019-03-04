use std::{
    io::Write,
    str::FromStr,
    net::SocketAddr,
    thread,
    time::Duration,
};

use mio::{net::TcpStream, Events, Poll, PollOpt, Ready, Token};

use super::{error::PostError, PostBox, PostOffice};

fn new_local_addr(n: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 12345 + n))
}

#[test]
fn basic_run() {
    let srv_addr = new_local_addr(0);
    let mut server: PostOffice<String, String> = PostOffice::new(srv_addr).unwrap();
    let mut client: PostBox<String, String> = PostBox::to_server(srv_addr).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let mut scon = server.new_connections().next().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    scon.send(String::from("foo")).unwrap();
    client.send(String::from("bar")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert_eq!("foo", client.new_messages().next().unwrap());
    assert_eq!("bar", scon.new_messages().next().unwrap());
}

#[test]
fn huge_size_header() {
    let srv_addr = new_local_addr(1);

    let mut server: PostOffice<String, String> = PostOffice::new(srv_addr).unwrap();
    let mut client = TcpStream::connect(&srv_addr).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let mut scon = server.new_connections().next().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    client.write(&[0xffu8; 64]).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert_eq!(scon.new_messages().next(), None);
}

#[test]
fn disconnect() {
    let srv_addr = new_local_addr(2);

    let mut server = PostOffice::<_, String>::new(srv_addr)
        .unwrap();

    // Create then close client
    {
        PostBox::<String, String>::to_server(srv_addr).unwrap();
    }

    std::thread::sleep(std::time::Duration::from_millis(10));

    let mut to_client = server
        .new_connections()
        .next()
        .unwrap();

    to_client.send(String::from("foo")).unwrap();

    thread::sleep(Duration::from_millis(10));

    match to_client.new_messages().next() {
        None => {},
        _ => panic!("Unexpected message!"),
    }

    match to_client.status() {
        Some(PostError::Disconnected) => {},
        s => panic!("Did not expect {:?}", s),
    }
}
