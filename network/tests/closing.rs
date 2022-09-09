#![feature(assert_matches)]
//! How to read those tests:
//!  - in the first line we call the helper, this is only debug code. in case
//!    you want to have tracing for a special test you set set the bool = true
//!    and the sleep to 10000 and your test will start 10 sec delayed with
//!    tracing. You need a delay as otherwise the other tests pollute your trace
//!  - the second line is to simulate a client and a server
//!    `network_participant_stream` will return
//!      - 2 networks
//!      - 2 participants
//!      - 2 streams
//!    each one `linked` to their counterpart.
//!    You see a cryptic use of rust `_` this is because we are testing the
//! `drop` behavior here.
//!      - A `_` means this is directly dropped after the line executes, thus
//!        immediately executing its `Drop` impl.
//!      - A `_p1_a` e.g. means we don't use that Participant yet, but we must
//!        not `drop` it yet as we might want to use the Streams.
//!  - You sometimes see sleep(1000ms) this is used when we rely on the
//!    underlying TCP functionality, as this simulates client and server

use std::{assert_matches::assert_matches, sync::Arc};
use tokio::runtime::Runtime;
use veloren_network::{Network, ParticipantError, ParticipantEvent, Pid, Promises, StreamError};
mod helper;
use helper::{network_participant_stream, tcp, SLEEP_EXTERNAL, SLEEP_INTERNAL};

#[test]
fn close_network() {
    let (_, _) = helper::setup(false, 0);
    let (r, _, _p1_a, s1_a, _, _p1_b, mut s1_b) = network_participant_stream(tcp());

    std::thread::sleep(SLEEP_INTERNAL);

    assert_eq!(s1_a.send("Hello World"), Err(StreamError::StreamClosed));
    let msg1: Result<String, _> = r.block_on(s1_b.recv());
    assert_eq!(msg1, Err(StreamError::StreamClosed));
}

#[test]
fn close_participant() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, p1_a, s1_a, _n_b, p1_b, mut s1_b) = network_participant_stream(tcp());

    r.block_on(p1_a.disconnect()).unwrap();
    r.block_on(p1_b.disconnect()).unwrap();

    assert_eq!(s1_a.send("Hello World"), Err(StreamError::StreamClosed));
    assert_eq!(
        r.block_on(s1_b.recv::<String>()),
        Err(StreamError::StreamClosed)
    );
}

#[test]
fn close_stream() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _, mut s1_a, _n_b, _, _) = network_participant_stream(tcp());

    // s1_b is dropped directly while s1_a isn't
    std::thread::sleep(SLEEP_INTERNAL);

    assert_eq!(s1_a.send("Hello World"), Err(StreamError::StreamClosed));
    assert_eq!(
        r.block_on(s1_a.recv::<String>()),
        Err(StreamError::StreamClosed)
    );
}

///WE must NOT create runtimes inside a Runtime, this check needs to verify
/// that we dont panic there
#[test]
fn close_streams_in_block_on() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, s1_b) = network_participant_stream(tcp());
    r.block_on(async {
        //make it locally so that they are dropped later
        let s1_a = s1_a;
        let mut s1_b = s1_b;
        s1_a.send("ping").unwrap();
        assert_eq!(s1_b.recv().await, Ok("ping".to_string()));
        drop(s1_a);
    });
    drop((_n_a, _p_a, _n_b, _p_b)); //clean teardown
}

#[test]
fn stream_simple_3msg_then_close() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send(1u8).unwrap();
    s1_a.send(42).unwrap();
    s1_a.send("3rdMessage").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok(1u8));
    assert_eq!(r.block_on(s1_b.recv()), Ok(42));
    assert_eq!(r.block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
    drop(s1_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    assert_eq!(s1_b.send("Hello World"), Err(StreamError::StreamClosed));
}

#[test]
fn stream_send_first_then_receive() {
    // recv should still be possible even if stream got closed if they are in queue
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send(1u8).unwrap();
    s1_a.send(42).unwrap();
    s1_a.send("3rdMessage").unwrap();
    drop(s1_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    assert_eq!(r.block_on(s1_b.recv()), Ok(1u8));
    assert_eq!(r.block_on(s1_b.recv()), Ok(42));
    assert_eq!(r.block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
    assert_eq!(s1_b.send("Hello World"), Err(StreamError::StreamClosed));
}

#[test]
fn stream_send_1_then_close_stream() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());
    s1_a.send("this message must be received, even if stream is closed already!")
        .unwrap();
    drop(s1_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    let exp = Ok("this message must be received, even if stream is closed already!".to_string());
    assert_eq!(r.block_on(s1_b.recv()), exp);
    println!("all received and done");
}

#[test]
fn stream_send_100000_then_close_stream() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());
    for _ in 0..100000 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(s1_a);
    let exp = Ok("woop_PARTY_HARD_woop".to_string());
    println!("start receiving");
    r.block_on(async {
        for _ in 0..100000 {
            assert_eq!(s1_b.recv().await, exp);
        }
    });
    println!("all received and done");
}

#[test]
fn stream_send_100000_then_close_stream_remote() {
    let (_, _) = helper::setup(false, 0);
    let (_r, _n_a, _p_a, s1_a, _n_b, _p_b, _s1_b) = network_participant_stream(tcp());
    for _ in 0..100000 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(s1_a);
    drop(_s1_b);
    //no receiving
    drop((_n_a, _p_a, _n_b, _p_b)); //clean teardown
}

#[test]
fn stream_send_100000_then_close_stream_remote2() {
    let (_, _) = helper::setup(false, 0);
    let (_r, _n_a, _p_a, s1_a, _n_b, _p_b, _s1_b) = network_participant_stream(tcp());
    for _ in 0..100000 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(_s1_b);
    std::thread::sleep(SLEEP_EXTERNAL);
    drop(s1_a);
    //no receiving
    drop((_n_a, _p_a, _n_b, _p_b)); //clean teardown
}

#[test]
fn stream_send_100000_then_close_stream_remote3() {
    let (_, _) = helper::setup(false, 0);
    let (_r, _n_a, _p_a, s1_a, _n_b, _p_b, _s1_b) = network_participant_stream(tcp());
    for _ in 0..100000 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(_s1_b);
    std::thread::sleep(SLEEP_EXTERNAL);
    drop(s1_a);
    //no receiving
    drop((_n_a, _p_a, _n_b, _p_b)); //clean teardown
}

#[test]
fn close_part_then_network() {
    let (_, _) = helper::setup(false, 0);
    let (_r, n_a, p_a, s1_a, _n_b, _p_b, _s1_b) = network_participant_stream(tcp());
    for _ in 0..1000 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(p_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    drop(n_a);
    std::thread::sleep(SLEEP_INTERNAL);
}

#[test]
fn close_network_then_part() {
    let (_, _) = helper::setup(false, 0);
    let (_r, n_a, p_a, s1_a, _n_b, _p_b, _s1_b) = network_participant_stream(tcp());
    for _ in 0..1000 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(n_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    drop(p_a);
    std::thread::sleep(SLEEP_INTERNAL);
}

#[test]
fn close_network_then_disconnect_part() {
    let (_, _) = helper::setup(false, 0);
    let (r, n_a, p_a, s1_a, _n_b, _p_b, _s1_b) = network_participant_stream(tcp());
    for _ in 0..1000 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(n_a);
    assert!(r.block_on(p_a.disconnect()).is_err());
    std::thread::sleep(SLEEP_EXTERNAL);
    drop((_n_b, _p_b)); //clean teardown
}

#[test]
fn close_runtime_then_network() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, _s1_b) = network_participant_stream(tcp());
    for _ in 0..100 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(r);
    drop(_n_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    drop(_p_b);
}

#[test]
fn close_runtime_then_part() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, _s1_b) = network_participant_stream(tcp());
    for _ in 0..100 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    drop(r);
    drop(_p_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    drop(_p_b);
    drop(_n_a);
}

#[test]
fn close_network_from_async() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, _s1_b) = network_participant_stream(tcp());
    for _ in 0..100 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    r.block_on(async move {
        drop(_n_a);
    });
    drop(_p_b);
}

#[test]
fn close_part_from_async() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, p_a, s1_a, _n_b, _p_b, _s1_b) = network_participant_stream(tcp());
    for _ in 0..100 {
        s1_a.send("woop_PARTY_HARD_woop").unwrap();
    }
    r.block_on(async move {
        p_a.disconnect().await.unwrap();
        drop(_p_b);
    });
    drop(_n_a);
}

#[test]
fn opened_stream_before_remote_part_is_closed() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, p_a, _, _n_b, mut p_b, _) = network_participant_stream(tcp());
    let s2_a = r.block_on(p_a.open(4, Promises::empty(), 0)).unwrap();
    s2_a.send("HelloWorld").unwrap();
    let mut s2_b = r.block_on(p_b.opened()).unwrap();
    drop(p_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    assert_eq!(r.block_on(s2_b.recv()), Ok("HelloWorld".to_string()));
    drop((_n_a, _n_b, p_b)); //clean teardown
}

#[test]
fn opened_stream_after_remote_part_is_closed() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, p_a, _, _n_b, mut p_b, _) = network_participant_stream(tcp());
    let s2_a = r.block_on(p_a.open(3, Promises::empty(), 0)).unwrap();
    s2_a.send("HelloWorld").unwrap();
    drop(p_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    let mut s2_b = r.block_on(p_b.opened()).unwrap();
    assert_eq!(r.block_on(s2_b.recv()), Ok("HelloWorld".to_string()));
    assert_eq!(
        r.block_on(p_b.opened()).unwrap_err(),
        ParticipantError::ParticipantDisconnected
    );
    drop((_n_a, _n_b, p_b)); //clean teardown
}

#[test]
fn open_stream_after_remote_part_is_closed() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, p_a, _, _n_b, mut p_b, _) = network_participant_stream(tcp());
    let s2_a = r.block_on(p_a.open(4, Promises::empty(), 0)).unwrap();
    s2_a.send("HelloWorld").unwrap();
    drop(p_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    let mut s2_b = r.block_on(p_b.opened()).unwrap();
    assert_eq!(r.block_on(s2_b.recv()), Ok("HelloWorld".to_string()));
    assert_eq!(
        r.block_on(p_b.open(5, Promises::empty(), 0)).unwrap_err(),
        ParticipantError::ParticipantDisconnected
    );
    drop((_n_a, _n_b, p_b)); //clean teardown
}

#[test]
fn failed_stream_open_after_remote_part_is_closed() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, p_a, _, _n_b, mut p_b, _) = network_participant_stream(tcp());
    drop(p_a);
    assert_eq!(
        r.block_on(p_b.opened()).unwrap_err(),
        ParticipantError::ParticipantDisconnected
    );
    drop((_n_a, _n_b, p_b)); //clean teardown
}

#[test]
fn open_participant_before_remote_part_is_closed() {
    let (_, _) = helper::setup(false, 0);
    let r = Arc::new(Runtime::new().unwrap());
    let mut n_a = Network::new(Pid::fake(0), &r);
    let n_b = Network::new(Pid::fake(1), &r);
    let addr = tcp();
    r.block_on(n_a.listen(addr.0)).unwrap();
    let p_b = r.block_on(n_b.connect(addr.1)).unwrap();
    let s1_b = r.block_on(p_b.open(4, Promises::empty(), 0)).unwrap();
    s1_b.send("HelloWorld").unwrap();
    let mut p_a = r.block_on(n_a.connected()).unwrap();
    drop(s1_b);
    drop(p_b);
    drop(n_b);
    std::thread::sleep(SLEEP_EXTERNAL);
    let mut s1_a = r.block_on(p_a.opened()).unwrap();
    assert_eq!(r.block_on(s1_a.recv()), Ok("HelloWorld".to_string()));
}

#[test]
fn open_participant_after_remote_part_is_closed() {
    let (_, _) = helper::setup(false, 0);
    let r = Arc::new(Runtime::new().unwrap());
    let mut n_a = Network::new(Pid::fake(0), &r);
    let n_b = Network::new(Pid::fake(1), &r);
    let addr = tcp();
    r.block_on(n_a.listen(addr.0)).unwrap();
    let p_b = r.block_on(n_b.connect(addr.1)).unwrap();
    let s1_b = r.block_on(p_b.open(4, Promises::empty(), 0)).unwrap();
    s1_b.send("HelloWorld").unwrap();
    drop(s1_b);
    drop(p_b);
    drop(n_b);
    std::thread::sleep(SLEEP_EXTERNAL);
    let mut p_a = r.block_on(n_a.connected()).unwrap();
    let mut s1_a = r.block_on(p_a.opened()).unwrap();
    assert_eq!(r.block_on(s1_a.recv()), Ok("HelloWorld".to_string()));
}

#[test]
fn close_network_scheduler_completely() {
    let (_, _) = helper::setup(false, 0);
    let r = Arc::new(Runtime::new().unwrap());
    let mut n_a = Network::new(Pid::fake(0), &r);
    let n_b = Network::new(Pid::fake(1), &r);
    let addr = tcp();
    r.block_on(n_a.listen(addr.0)).unwrap();
    let mut p_b = r.block_on(n_b.connect(addr.1)).unwrap();
    assert_matches!(
        r.block_on(p_b.fetch_event()),
        Ok(ParticipantEvent::ChannelCreated(_))
    );
    let s1_b = r.block_on(p_b.open(4, Promises::empty(), 0)).unwrap();
    s1_b.send("HelloWorld").unwrap();

    let mut p_a = r.block_on(n_a.connected()).unwrap();
    assert_matches!(
        r.block_on(p_a.fetch_event()),
        Ok(ParticipantEvent::ChannelCreated(_))
    );
    assert_matches!(p_a.try_fetch_event(), Ok(None));
    assert_matches!(p_b.try_fetch_event(), Ok(None));
    let mut s1_a = r.block_on(p_a.opened()).unwrap();
    assert_eq!(r.block_on(s1_a.recv()), Ok("HelloWorld".to_string()));
    drop(n_a);
    drop(n_b);
    std::thread::sleep(SLEEP_EXTERNAL); //p_b is INTERNAL, but p_a is EXTERNAL
    assert_matches!(
        p_a.try_fetch_event(),
        Ok(Some(ParticipantEvent::ChannelDeleted(_)))
    );
    assert_matches!(
        r.block_on(p_b.fetch_event()),
        Ok(ParticipantEvent::ChannelDeleted(_))
    );
    assert_matches!(p_a.try_fetch_event(), Err(_));
    assert_matches!(p_b.try_fetch_event(), Err(_));

    drop(p_b);
    drop(p_a);
    let runtime = Arc::try_unwrap(r).expect("runtime is not alone, there still exist a reference");
    runtime.shutdown_timeout(SLEEP_INTERNAL);
}

#[test]
fn dont_panic_on_multiply_recv_after_close() {
    let (_, _) = helper::setup(false, 0);
    let (_r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send(11u32).unwrap();
    drop(s1_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    assert_eq!(s1_b.try_recv::<u32>(), Ok(Some(11u32)));
    assert_eq!(s1_b.try_recv::<String>(), Err(StreamError::StreamClosed));
    // There was a "Feature" in futures::channels that they panic when you call recv
    // a second time after it showed end of stream
    assert_eq!(s1_b.try_recv::<String>(), Err(StreamError::StreamClosed));
}

#[test]
fn dont_panic_on_recv_send_after_close() {
    let (_, _) = helper::setup(false, 0);
    let (_r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send(11u32).unwrap();
    drop(s1_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    assert_eq!(s1_b.try_recv::<u32>(), Ok(Some(11u32)));
    assert_eq!(s1_b.try_recv::<String>(), Err(StreamError::StreamClosed));
    assert_eq!(s1_b.send("foobar"), Err(StreamError::StreamClosed));
}

#[test]
fn dont_panic_on_multiple_send_after_close() {
    let (_, _) = helper::setup(false, 0);
    let (_r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send(11u32).unwrap();
    drop(s1_a);
    drop(_p_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    assert_eq!(s1_b.try_recv::<u32>(), Ok(Some(11u32)));
    assert_eq!(s1_b.try_recv::<String>(), Err(StreamError::StreamClosed));
    assert_eq!(s1_b.send("foobar"), Err(StreamError::StreamClosed));
    assert_eq!(s1_b.send("foobar"), Err(StreamError::StreamClosed));
}
