use async_std::task;
use task::block_on;
use veloren_network::StreamError;
mod helper;
use helper::{network_participant_stream, tcp, udp};

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
    std::thread::sleep(std::time::Duration::from_millis(2000));
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
