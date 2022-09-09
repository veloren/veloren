#![feature(assert_matches)]
use std::sync::Arc;
use tokio::runtime::Runtime;
use veloren_network::{NetworkError, StreamError};
mod helper;
use helper::{mpsc, network_participant_stream, quic, tcp, udp, SLEEP_EXTERNAL, SLEEP_INTERNAL};
use std::io::ErrorKind;
use veloren_network::{ConnectAddr, ListenAddr, Network, ParticipantEvent, Pid, Promises};

#[test]
fn stream_simple() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send("Hello World").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
fn stream_try_recv() {
    let (_, _) = helper::setup(false, 0);
    let (_r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send(4242u32).unwrap();
    std::thread::sleep(SLEEP_EXTERNAL);
    assert_eq!(s1_b.try_recv(), Ok(Some(4242u32)));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
fn stream_simple_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

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
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(mpsc());

    s1_a.send("Hello World").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
fn stream_simple_mpsc_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(mpsc());

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
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(quic());

    s1_a.send("Hello World").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
fn stream_simple_quic_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(quic());

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
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(udp());

    s1_a.send("Hello World").unwrap();
    assert_eq!(r.block_on(s1_b.recv()), Ok("Hello World".to_string()));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

#[test]
#[ignore]
fn stream_simple_udp_3msg() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(udp());

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
fn tcp_and_udp_2_connections() -> Result<(), Box<dyn std::error::Error>> {
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
fn failed_listen_on_used_ports() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _) = helper::setup(false, 0);
    let r = Arc::new(Runtime::new().unwrap());
    let network = Network::new(Pid::new(), &r);
    let udp1 = udp();
    let tcp1 = tcp();
    r.block_on(network.listen(udp1.0.clone()))?;
    r.block_on(network.listen(tcp1.0.clone()))?;
    std::thread::sleep(SLEEP_INTERNAL);

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
fn api_stream_send_main() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _) = helper::setup(false, 0);
    // Create a Network, listen on Port `1200` and wait for a Stream to be opened,
    // then answer `Hello World`
    let r = Arc::new(Runtime::new().unwrap());
    let network = Network::new(Pid::new(), &r);
    let remote = Network::new(Pid::new(), &r);
    r.block_on(async {
        let mut network = network;
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
        let mut participant_a = network.connected().await?;
        let stream_a = participant_a.opened().await?;
        //Send  Message
        stream_a.send("Hello World")?;
        Ok(())
    })
}

#[test]
fn api_stream_recv_main() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _) = helper::setup(false, 0);
    // Create a Network, listen on Port `1220` and wait for a Stream to be opened,
    // then listen on it
    let r = Arc::new(Runtime::new().unwrap());
    let network = Network::new(Pid::new(), &r);
    let remote = Network::new(Pid::new(), &r);
    r.block_on(async {
        let mut network = network;
        let remote = remote;
        network
            .listen(ListenAddr::Tcp("127.0.0.1:1220".parse().unwrap()))
            .await?;
        let remote_p = remote
            .connect(ConnectAddr::Tcp("127.0.0.1:1220".parse().unwrap()))
            .await?;
        let stream_p = remote_p
            .open(4, Promises::ORDERED | Promises::CONSISTENCY, 0)
            .await?;
        stream_p.send("Hello World")?;
        let mut participant_a = network.connected().await?;
        let mut stream_a = participant_a.opened().await?;
        //Send  Message
        assert_eq!("Hello World".to_string(), stream_a.recv::<String>().await?);
        Ok(())
    })
}

#[test]
fn wrong_parse() {
    let (_, _) = helper::setup(false, 0);
    let (r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

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
    let (_r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcp());

    s1_a.send("asd").unwrap();
    s1_a.send(11u32).unwrap();
    std::thread::sleep(SLEEP_EXTERNAL);
    assert_eq!(s1_b.try_recv(), Ok(Some("asd".to_string())));
    assert_eq!(s1_b.try_recv::<u32>(), Ok(Some(11u32)));
    assert_eq!(s1_b.try_recv::<String>(), Ok(None));

    drop(s1_a);
    std::thread::sleep(SLEEP_EXTERNAL);
    assert_eq!(s1_b.try_recv::<String>(), Err(StreamError::StreamClosed));
    drop((_n_a, _n_b, _p_a, _p_b)); //clean teardown
}

/// If we listen on a IPv6 UNSPECIFIED address, on linux it will automatically
/// listen on the respective IPv4 address. This must not be as we should behave
/// similar under windows and linux.
///
/// As most CI servers don't have IPv6 configured, this would return
/// ConnectFailed(Io(Os { code: 99, kind: AddrNotAvailable, message: "Cannot
/// assign requested address" })) we have to disable this test in CI, but it was
/// manually tested on linux and windows
///
/// On Windows this test must be executed as root to listen on IPv6::UNSPECIFIED
#[test]
#[ignore]
fn listen_on_ipv6_doesnt_block_ipv4() {
    let (_, _) = helper::setup(false, 0);
    let tcpv4 = tcp();
    let port = if let ListenAddr::Tcp(x) = tcpv4.0 {
        x.port()
    } else {
        unreachable!()
    };
    let tcpv6 = (
        ListenAddr::Tcp(std::net::SocketAddr::from((
            std::net::Ipv6Addr::UNSPECIFIED,
            port,
        ))),
        ConnectAddr::Tcp(std::net::SocketAddr::from((
            std::net::Ipv6Addr::UNSPECIFIED,
            port,
        ))),
    );

    let (_r, _n_a, _p_a, s1_a, _n_b, _p_b, mut s1_b) = network_participant_stream(tcpv6);
    std::thread::sleep(SLEEP_EXTERNAL);
    let (_r2, _n_a2, _p_a2, s1_a2, _n_b2, _p_b2, mut s1_b2) = network_participant_stream(tcpv4);

    s1_a.send(42u32).unwrap();
    s1_a2.send(1337u32).unwrap();
    std::thread::sleep(SLEEP_EXTERNAL);
    assert_eq!(s1_b.try_recv::<u32>(), Ok(Some(42u32)));
    assert_eq!(s1_b2.try_recv::<u32>(), Ok(Some(1337u32)));

    drop((s1_a, s1_b, _n_a, _n_b, _p_a, _p_b));
    drop((s1_a2, s1_b2, _n_a2, _n_b2, _p_a2, _p_b2)); //clean teardown
}

#[test]
fn check_correct_channel_events() {
    let (_, _) = helper::setup(false, 0);
    let con_addr = tcp();
    let (r, _n_a, mut p_a, _, _n_b, mut p_b, _) = network_participant_stream(con_addr.clone());

    let event_a = r.block_on(p_a.fetch_event()).unwrap();
    let event_b = r.block_on(p_b.fetch_event()).unwrap();
    if let ConnectAddr::Tcp(listen_addr) = con_addr.1 {
        match event_a {
            ParticipantEvent::ChannelCreated(ConnectAddr::Tcp(socket_addr)) => {
                assert_ne!(socket_addr, listen_addr);
                assert_eq!(socket_addr.ip(), std::net::Ipv4Addr::LOCALHOST);
            },
            e => panic!("wrong event {:?}", e),
        }
        match event_b {
            ParticipantEvent::ChannelCreated(ConnectAddr::Tcp(socket_addr)) => {
                assert_eq!(socket_addr, listen_addr);
            },
            e => panic!("wrong event {:?}", e),
        }
    } else {
        unreachable!();
    }

    std::thread::sleep(SLEEP_EXTERNAL);
    drop((_n_a, _n_b)); //drop network

    let event_a = r.block_on(p_a.fetch_event()).unwrap();
    let event_b = r.block_on(p_b.fetch_event()).unwrap();
    if let ConnectAddr::Tcp(listen_addr) = con_addr.1 {
        match event_a {
            ParticipantEvent::ChannelDeleted(ConnectAddr::Tcp(socket_addr)) => {
                assert_ne!(socket_addr, listen_addr);
                assert_eq!(socket_addr.ip(), std::net::Ipv4Addr::LOCALHOST);
            },
            e => panic!("wrong event {:?}", e),
        }
        match event_b {
            ParticipantEvent::ChannelDeleted(ConnectAddr::Tcp(socket_addr)) => {
                assert_eq!(socket_addr, listen_addr);
            },
            e => panic!("wrong event {:?}", e),
        }
    } else {
        unreachable!();
    }

    drop((p_a, p_b)); //clean teardown
}
