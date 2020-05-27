use async_std::task;
use task::block_on;
use veloren_network::{NetworkError, StreamError};
mod helper;
use helper::{network_participant_stream, tcp, udp};
use std::io::ErrorKind;

#[test]
#[ignore]
fn network_20s() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, _, _n_b, _, _) = block_on(network_participant_stream(tcp()));
    std::thread::sleep(std::time::Duration::from_secs(30));
}

#[test]
fn close_network() {
    let (_, _) = helper::setup(false, 0);
    let (_, _p1_a, mut s1_a, _, _p1_b, mut s1_b) = block_on(network_participant_stream(tcp()));

    std::thread::sleep(std::time::Duration::from_millis(30));

    assert_eq!(s1_a.send("Hello World"), Err(StreamError::StreamClosed));
    let msg1: Result<String, _> = block_on(s1_b.recv());
    assert_eq!(msg1, Err(StreamError::StreamClosed));
}

#[test]
fn close_participant() {
    let (_, _) = helper::setup(false, 0);
    let (n_a, p1_a, mut s1_a, n_b, p1_b, mut s1_b) = block_on(network_participant_stream(tcp()));

    block_on(n_a.disconnect(p1_a)).unwrap();
    block_on(n_b.disconnect(p1_b)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(30));
    assert_eq!(s1_a.send("Hello World"), Err(StreamError::StreamClosed));
    assert_eq!(
        block_on(s1_b.recv::<String>()),
        Err(StreamError::StreamClosed)
    );
}

#[test]
fn close_stream() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, _) = block_on(network_participant_stream(tcp()));

    // s1_b is dropped directly while s1_a isn't
    std::thread::sleep(std::time::Duration::from_millis(30));

    assert_eq!(s1_a.send("Hello World"), Err(StreamError::StreamClosed));
    assert_eq!(
        block_on(s1_a.recv::<String>()),
        Err(StreamError::StreamClosed)
    );
}

#[test]
fn stream_simple() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, mut s1_b) = block_on(network_participant_stream(tcp()));

    s1_a.send("Hello World").unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("Hello World".to_string()));
}

#[test]
fn stream_simple_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, mut s1_b) = block_on(network_participant_stream(tcp()));

    s1_a.send("Hello World").unwrap();
    s1_a.send(1337).unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("Hello World".to_string()));
    assert_eq!(block_on(s1_b.recv()), Ok(1337));
    s1_a.send("3rdMessage").unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
}

#[test]
fn stream_simple_3msg_then_close() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, mut s1_b) = block_on(network_participant_stream(tcp()));

    s1_a.send(1u8).unwrap();
    s1_a.send(42).unwrap();
    s1_a.send("3rdMessage").unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok(1u8));
    assert_eq!(block_on(s1_b.recv()), Ok(42));
    assert_eq!(block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
    drop(s1_a);
    std::thread::sleep(std::time::Duration::from_millis(30));
    assert_eq!(s1_b.send("Hello World"), Err(StreamError::StreamClosed));
}

#[test]
fn stream_send_first_then_receive() {
    // recv should still be possible even if stream got closed if they are in queue
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, mut s1_b) = block_on(network_participant_stream(tcp()));

    s1_a.send(1u8).unwrap();
    s1_a.send(42).unwrap();
    s1_a.send("3rdMessage").unwrap();
    drop(s1_a);
    std::thread::sleep(std::time::Duration::from_millis(500));
    assert_eq!(block_on(s1_b.recv()), Ok(1u8));
    assert_eq!(block_on(s1_b.recv()), Ok(42));
    assert_eq!(block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
    assert_eq!(s1_b.send("Hello World"), Err(StreamError::StreamClosed));
}

#[test]
fn stream_simple_udp() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, mut s1_b) = block_on(network_participant_stream(udp()));

    s1_a.send("Hello World").unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("Hello World".to_string()));
}

#[test]
fn stream_simple_udp_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, mut s1_b) = block_on(network_participant_stream(udp()));

    s1_a.send("Hello World").unwrap();
    s1_a.send(1337).unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("Hello World".to_string()));
    assert_eq!(block_on(s1_b.recv()), Ok(1337));
    s1_a.send("3rdMessage").unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
}

use uvth::ThreadPoolBuilder;
use veloren_network::{Address, Network, Pid};
#[test]
#[ignore]
fn tcp_and_udp_2_connections() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (_, _) = helper::setup(true, 0);
    let network = Network::new(Pid::new(), &ThreadPoolBuilder::new().build(), None);
    let remote = Network::new(Pid::new(), &ThreadPoolBuilder::new().build(), None);
    block_on(async {
        remote
            .listen(Address::Tcp("0.0.0.0:2000".parse().unwrap()))
            .await?;
        remote
            .listen(Address::Udp("0.0.0.0:2001".parse().unwrap()))
            .await?;
        let p1 = network
            .connect(Address::Tcp("127.0.0.1:2000".parse().unwrap()))
            .await?;
        let p2 = network
            .connect(Address::Udp("127.0.0.1:2001".parse().unwrap()))
            .await?;
        assert!(std::sync::Arc::ptr_eq(&p1, &p2));
        Ok(())
    })
}

#[test]
fn failed_listen_on_used_ports() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (_, _) = helper::setup(false, 0);
    let network = Network::new(Pid::new(), &ThreadPoolBuilder::new().build(), None);
    let udp1 = udp();
    let tcp1 = tcp();
    block_on(network.listen(udp1.clone()))?;
    block_on(network.listen(tcp1.clone()))?;
    std::thread::sleep(std::time::Duration::from_millis(50));

    let network2 = Network::new(Pid::new(), &ThreadPoolBuilder::new().build(), None);
    let e1 = block_on(network2.listen(udp1));
    let e2 = block_on(network2.listen(tcp1));
    match e1 {
        Err(NetworkError::ListenFailed(e)) if e.kind() == ErrorKind::AddrInUse => (),
        _ => assert!(false),
    };
    match e2 {
        Err(NetworkError::ListenFailed(e)) if e.kind() == ErrorKind::AddrInUse => (),
        _ => assert!(false),
    };
    Ok(())
}
