use async_std::task;
use task::block_on;
use veloren_network::{NetworkError, StreamError};
mod helper;
use helper::{network_participant_stream, tcp, udp};
use std::io::ErrorKind;
use veloren_network::{Address, Network, Pid, PROMISES_CONSISTENCY, PROMISES_ORDERED};

#[test]
#[ignore]
fn network_20s() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _, _, _n_b, _, _) = block_on(network_participant_stream(tcp()));
    std::thread::sleep(std::time::Duration::from_secs(30));
}

#[test]
fn stream_simple() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = block_on(network_participant_stream(tcp()));

    s1_a.send("Hello World").unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("Hello World".to_string()));
}

#[test]
fn stream_simple_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = block_on(network_participant_stream(tcp()));

    s1_a.send("Hello World").unwrap();
    s1_a.send(1337).unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("Hello World".to_string()));
    assert_eq!(block_on(s1_b.recv()), Ok(1337));
    s1_a.send("3rdMessage").unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
}

#[test]
fn stream_simple_udp() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = block_on(network_participant_stream(udp()));

    s1_a.send("Hello World").unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("Hello World".to_string()));
}

#[test]
fn stream_simple_udp_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = block_on(network_participant_stream(udp()));

    s1_a.send("Hello World").unwrap();
    s1_a.send(1337).unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("Hello World".to_string()));
    assert_eq!(block_on(s1_b.recv()), Ok(1337));
    s1_a.send("3rdMessage").unwrap();
    assert_eq!(block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
}

#[test]
#[ignore]
fn tcp_and_udp_2_connections() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (_, _) = helper::setup(false, 0);
    let (network, f) = Network::new(Pid::new(), None);
    let (remote, fr) = Network::new(Pid::new(), None);
    std::thread::spawn(f);
    std::thread::spawn(fr);
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
        assert_eq!(&p1, &p2);
        Ok(())
    })
}

#[test]
fn failed_listen_on_used_ports() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (_, _) = helper::setup(false, 0);
    let (network, f) = Network::new(Pid::new(), None);
    std::thread::spawn(f);
    let udp1 = udp();
    let tcp1 = tcp();
    block_on(network.listen(udp1.clone()))?;
    block_on(network.listen(tcp1.clone()))?;
    std::thread::sleep(std::time::Duration::from_millis(200));

    let (network2, f2) = Network::new(Pid::new(), None);
    std::thread::spawn(f2);
    let e1 = block_on(network2.listen(udp1));
    let e2 = block_on(network2.listen(tcp1));
    match e1 {
        Err(NetworkError::ListenFailed(e)) if e.kind() == ErrorKind::AddrInUse => (),
        _ => panic!(),
    };
    match e2 {
        Err(NetworkError::ListenFailed(e)) if e.kind() == ErrorKind::AddrInUse => (),
        _ => panic!(),
    };
    Ok(())
}

/// There is a bug an impris-desktop-1 which fails the DOC tests,
/// it fails exactly `api_stream_send_main` and `api_stream_recv_main` by
/// deadlocking at different times!
/// So i rather put the same test into a unit test, these are now duplicate to
/// the api, but are left here, just to be save!
#[test]
fn api_stream_send_main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (_, _) = helper::setup(false, 0);
    // Create a Network, listen on Port `1200` and wait for a Stream to be opened,
    // then answer `Hello World`
    let (network, f) = Network::new(Pid::new(), None);
    let (remote, fr) = Network::new(Pid::new(), None);
    std::thread::spawn(f);
    std::thread::spawn(fr);
    block_on(async {
        network
            .listen(Address::Tcp("127.0.0.1:1200".parse().unwrap()))
            .await?;
        let remote_p = remote
            .connect(Address::Tcp("127.0.0.1:1200".parse().unwrap()))
            .await?;
        // keep it alive
        let _stream_p = remote_p
            .open(16, PROMISES_ORDERED | PROMISES_CONSISTENCY)
            .await?;
        let participant_a = network.connected().await?;
        let mut stream_a = participant_a.opened().await?;
        //Send  Message
        stream_a.send("Hello World")?;
        Ok(())
    })
}

#[test]
fn api_stream_recv_main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (_, _) = helper::setup(false, 0);
    // Create a Network, listen on Port `1220` and wait for a Stream to be opened,
    // then listen on it
    let (network, f) = Network::new(Pid::new(), None);
    let (remote, fr) = Network::new(Pid::new(), None);
    std::thread::spawn(f);
    std::thread::spawn(fr);
    block_on(async {
        network
            .listen(Address::Tcp("127.0.0.1:1220".parse().unwrap()))
            .await?;
        let remote_p = remote
            .connect(Address::Tcp("127.0.0.1:1220".parse().unwrap()))
            .await?;
        let mut stream_p = remote_p
            .open(16, PROMISES_ORDERED | PROMISES_CONSISTENCY)
            .await?;
        stream_p.send("Hello World")?;
        let participant_a = network.connected().await?;
        let mut stream_a = participant_a.opened().await?;
        //Send  Message
        assert_eq!("Hello World".to_string(), stream_a.recv::<String>().await?);
        Ok(())
    })
}

#[test]
fn wrong_parse() {
    let (_, _) = helper::setup(false, 0);
    let (_n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = block_on(network_participant_stream(tcp()));

    s1_a.send(1337).unwrap();
    match block_on(s1_b.recv::<String>()) {
        Err(StreamError::DeserializeError(_)) => (),
        _ => panic!("this should fail, but it doesnt!"),
    }
}
