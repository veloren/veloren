use std::sync::Arc;
use tokio::runtime::Runtime;
use veloren_network::{NetworkError, StreamError};
mod helper;
use helper::{mpsc, network_participant_stream, quic, tcp, udp};
use std::io::ErrorKind;
use veloren_network::{ConnectAddr, ListenAddr, Network, Pid, Promises};

#[test]
#[ignore]
fn network_20s() {
    let (_, _) = helper::setup(false, 0);
    let (_, _n_a, _, _, _n_b, _, _) = network_participant_stream(tcp());
    std::thread::sleep(std::time::Duration::from_secs(30));
}

#[test]
fn stream_simple() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send("Hello World").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
fn stream_try_recv() {
    let (_, _) = helper::setup(false, 0);
    let (_r, _n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send(4242u32).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    assert_eq!(s1_b.try_recv(), Ok(Some(4242u32)));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
fn stream_simple_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send("Hello World").unwrap();
    s1_a.send(1337).unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    assert_eq!(r.block_on(s1_b.recv()), Ok(1337));
    s1_a.send("3rdMessage").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
fn stream_simple_mpsc() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(mpsc());

    s1_a.send("Hello World").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
fn stream_simple_mpsc_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(mpsc());

    s1_a.send("Hello World").unwrap();
    s1_a.send(1337).unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    assert_eq!(r.block_on(s1_b.recv()), Ok(1337));
    s1_a.send("3rdMessage").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
fn stream_simple_quic() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(quic());

    s1_a.send("Hello World").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
fn stream_simple_quic_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(quic());

    s1_a.send("Hello World").unwrap();
    s1_a.send(1337).unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    assert_eq!(r.block_on(s1_b.recv()), Ok(1337));
    s1_a.send("3rdMessage").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
#[ignore]
fn stream_simple_udp() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(udp());

    s1_a.send("Hello World").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
#[ignore]
fn stream_simple_udp_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(udp());

    s1_a.send("Hello World").unwrap();
    s1_a.send(1337).unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    assert_eq!(r.block_on(s1_b.recv()), Ok(1337));
    s1_a.send("3rdMessage").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("3rdMessage".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
#[ignore]
fn tcp_and_udp_2_connections() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (_, _) = helper::setup(false, 0);
    let r = Arc::new(Runtime::new().unwrap());
    let network = Network::new(Pid::new(), &r);
    let remote = Network::new(Pid::new(), &r);
    r.block_on(async {
        let network = network;
        let remote = remote;
        remote
            .listen(ListenAddr::Tcp("127.0.0.1:2000".parse().unwrap()))
            .await?;
        remote
            .listen(ListenAddr::Udp("127.0.0.1:2001".parse().unwrap()))
            .await?;
        let p1 = network
            .connect(ConnectAddr::Tcp("127.0.0.1:2000".parse().unwrap()))
            .await?;
        let p2 = network
            .connect(ConnectAddr::Udp("127.0.0.1:2001".parse().unwrap()))
            .await?;
        assert_eq!(&p1, &p2);
        Ok(())
    })
}

#[test]
#[ignore]
fn failed_listen_on_used_ports() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (_, _) = helper::setup(false, 0);
    let r = Arc::new(Runtime::new().unwrap());
    let network = Network::new(Pid::new(), &r);
    let udp1 = udp();
    let tcp1 = tcp();
    r.block_on(network.listen(udp1.0.clone()))?;
    r.block_on(network.listen(tcp1.0.clone()))?;
    std::thread::sleep(std::time::Duration::from_millis(200));

    let network2 = Network::new(Pid::new(), &r);
    let e1 = r.block_on(network2.listen(udp1.0));
    let e2 = r.block_on(network2.listen(tcp1.0));
    match e1 {
        Err(NetworkError::ListenFailed(e)) if e.kind() == ErrorKind::AddrInUse => (),
        _ => panic!(),
    };
    match e2 {
        Err(NetworkError::ListenFailed(e)) if e.kind() == ErrorKind::AddrInUse => (),
        _ => panic!(),
    };
    drop((network, network2)); //clean teardown
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
    let r = Arc::new(Runtime::new().unwrap());
    let network = Network::new(Pid::new(), &r);
    let remote = Network::new(Pid::new(), &r);
    r.block_on(async {
        let network = network;
        let remote = remote;
        network
            .listen(ListenAddr::Tcp("127.0.0.1:1200".parse().unwrap()))
            .await?;
        let remote_p = remote
            .connect(ConnectAddr::Tcp("127.0.0.1:1200".parse().unwrap()))
            .await?;
        // keep it alive
        let _stream_p = remote_p
            .open(4, Promises::ORDERED | Promises::CONSISTENCY, 0)
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
    let r = Arc::new(Runtime::new().unwrap());
    let network = Network::new(Pid::new(), &r);
    let remote = Network::new(Pid::new(), &r);
    r.block_on(async {
        let network = network;
        let remote = remote;
        network
            .listen(ListenAddr::Tcp("127.0.0.1:1220".parse().unwrap()))
            .await?;
        let remote_p = remote
            .connect(ConnectAddr::Tcp("127.0.0.1:1220".parse().unwrap()))
            .await?;
        let mut stream_p = remote_p
            .open(4, Promises::ORDERED | Promises::CONSISTENCY, 0)
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
    let (r, _n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send(1337).unwrap();
    match r.block_on(s1_b.recv::<String>()) {
        Err(StreamError::Deserialize(_)) => (),
        _ => panic!("this should fail, but it doesnt!"),
    }
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
fn multiple_try_recv() {
    let (_, _) = helper::setup(false, 0);
    let (_r, _n_a, _p_a, mut s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send("asd").unwrap();
    s1_a.send(11u32).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    assert_eq!(s1_b.try_recv(), Ok(Some("asd".to_string())));
    assert_eq!(s1_b.try_recv::<u32>(), Ok(Some(11u32)));
    assert_eq!(s1_b.try_recv::<String>(), Ok(None));

    drop(s1_a);
    std::thread::sleep(std::time::Duration::from_secs(1));
    assert_eq!(s1_b.try_recv::<String>(), Err(StreamError::StreamClosed));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}
