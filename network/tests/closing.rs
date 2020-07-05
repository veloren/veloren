use async_std::task;
use task::block_on;
use veloren_network::StreamError;
mod helper;
use helper::{network_participant_stream, tcp};

#[test]
fn close_network() {
    let (_, _) = helper::setup(false, 0);
    let (_, _p1_a, mut s1_a, _, _p1_b, mut s1_b) = block_on(network_participant_stream(tcp()));

    std::thread::sleep(std::time::Duration::from_millis(1000));

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
    std::thread::sleep(std::time::Duration::from_millis(1000));

    assert_eq!(s1_a.send("Hello World"), Err(StreamError::StreamClosed));
    assert_eq!(
        block_on(s1_a.recv::<String>()),
        Err(StreamError::StreamClosed)
    );
}

///THIS is actually a bug which currently luckily doesn't trigger, but with new
/// async-std WE must make sure, if a stream is `drop`ed inside a `block_on`,
/// that no panic is thrown.
#[test]
fn close_streams_in_block_on() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _p_a, s1_a, _n_b, _p_b, s1_b) = block_on(network_participant_stream(tcp()));
    block_on(async {
        //make it locally so that they are dropped later
        let mut s1_a = s1_a;
        let mut s1_b = s1_b;
        s1_a.send("ping").unwrap();
        assert_eq!(s1_b.recv().await, Ok("ping".to_string()));
        drop(s1_a);
    });
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
    std::thread::sleep(std::time::Duration::from_millis(1000));
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
    std::thread::sleep(std::time::Duration::from_millis(1000));
    assert_eq!(block_on(s1_b.recv()), Ok(1u8));
    assert_eq!(block_on(s1_b.recv()), Ok(42));
    assert_eq!(block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
    assert_eq!(s1_b.send("Hello World"), Err(StreamError::StreamClosed));
}

#[test]
fn stream_send_1_then_close_stream() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, mut s1_b) = block_on(network_participant_stream(tcp()));
    s1_a.send("this message must be received, even if stream is closed already!")
        .unwrap();
    drop(s1_a);
    std::thread::sleep(std::time::Duration::from_millis(1000));
    let exp = Ok("this message must be received, even if stream is closed already!".to_string());
    assert_eq!(block_on(s1_b.recv()), exp);
    println!("all received and done");
}

#[test]
fn stream_send_100000_then_close_stream() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, mut s1_b) = block_on(network_participant_stream(tcp()));
    for _ in 0..100000 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(s1_a);
    let exp = Ok("woop_PARTY_HARD_woop".to_string());
    println!("start receiving");
    block_on(async {
        for _ in 0..100000 {
            assert_eq!(s1_b.recv().await, exp);
        }
    });
    println!("all received and done");
}

#[test]
fn stream_send_100000_then_close_stream_remote() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, _s1_b) = block_on(network_participant_stream(tcp()));
    for _ in 0..100000 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(s1_a);
    drop(_s1_b);
    //no receiving
}

#[test]
fn stream_send_100000_then_close_stream_remote2() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, _s1_b) = block_on(network_participant_stream(tcp()));
    for _ in 0..100000 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(_s1_b);
    std::thread::sleep(std::time::Duration::from_millis(1000));
    drop(s1_a);
    //no receiving
}

#[test]
fn stream_send_100000_then_close_stream_remote3() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, mut s1_a, _n_b, _, _s1_b) = block_on(network_participant_stream(tcp()));
    for _ in 0..100000 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(_s1_b);
    std::thread::sleep(std::time::Duration::from_millis(1000));
    drop(s1_a);
    //no receiving
}
