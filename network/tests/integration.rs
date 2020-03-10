use futures::executor::block_on;
use std::{net::SocketAddr, thread, time::Duration};
use uuid::Uuid;
use veloren_network::{Address, Network, Promise};

mod helper;

/*
#[test]
fn tcp_simple() {
    let (thread_pool, _) = helper::setup(true, 100);
    let n1 = Network::new(Uuid::new_v4(), thread_pool.clone());
    let n2 = Network::new(Uuid::new_v4(), thread_pool.clone());
    let a1 = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], 52000)));
    let a2 = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], 52001)));
    n1.listen(&a1).unwrap(); //await
    n2.listen(&a2).unwrap(); // only requiered here, but doesnt hurt on n1
    thread::sleep(Duration::from_millis(3)); //TODO: listeing still doesnt block correctly!

    let p1 = block_on(n1.connect(&a2)).unwrap(); //await
    let s1 = p1.open(16, Promise::InOrder | Promise::NoCorrupt).unwrap();

    assert!(s1.send("Hello World").is_ok());

    let p1_n2 = block_on(n2.connected()).unwrap(); //remote representation of p1
    let mut s1_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s1

    let s: Result<String, _> = block_on(s1_n2.recv());
    assert_eq!(s, Ok("Hello World".to_string()));

    assert!(s1.close().is_ok());
}
*/

/*
#[test]
fn tcp_5streams() {
    let (thread_pool, _) = helper::setup(false, 200);
    let n1 = Network::new(Uuid::new_v4(), thread_pool.clone());
    let n2 = Network::new(Uuid::new_v4(), thread_pool.clone());
    let a1 = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], 52010)));
    let a2 = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], 52011)));

    n1.listen(&a1).unwrap(); //await
    n2.listen(&a2).unwrap(); // only requiered here, but doesnt hurt on n1
    thread::sleep(Duration::from_millis(3)); //TODO: listeing still doesnt block correctly!

    let p1 = block_on(n1.connect(&a2)).unwrap(); //await

    let s1 = p1.open(16, Promise::InOrder | Promise::NoCorrupt).unwrap();
    let s2 = p1.open(16, Promise::InOrder | Promise::NoCorrupt).unwrap();
    let s3 = p1.open(16, Promise::InOrder | Promise::NoCorrupt).unwrap();
    let s4 = p1.open(16, Promise::InOrder | Promise::NoCorrupt).unwrap();
    let s5 = p1.open(16, Promise::InOrder | Promise::NoCorrupt).unwrap();

    assert!(s3.send("Hello World3").is_ok());
    assert!(s1.send("Hello World1").is_ok());
    assert!(s5.send("Hello World5").is_ok());
    assert!(s2.send("Hello World2").is_ok());
    assert!(s4.send("Hello World4").is_ok());

    let p1_n2 = block_on(n2.connected()).unwrap(); //remote representation of p1
    let mut s1_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s1
    let mut s2_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s2
    let mut s3_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s3
    let mut s4_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s4
    let mut s5_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s5

    info!("all streams opened");

    let s: Result<String, _> = block_on(s3_n2.recv());
    assert_eq!(s, Ok("Hello World3".to_string()));
    let s: Result<String, _> = block_on(s1_n2.recv());
    assert_eq!(s, Ok("Hello World1".to_string()));
    let s: Result<String, _> = block_on(s2_n2.recv());
    assert_eq!(s, Ok("Hello World2".to_string()));
    let s: Result<String, _> = block_on(s5_n2.recv());
    assert_eq!(s, Ok("Hello World5".to_string()));
    let s: Result<String, _> = block_on(s4_n2.recv());
    assert_eq!(s, Ok("Hello World4".to_string()));

    assert!(s1.close().is_ok());
}
*/
#[test]
fn mpsc_simple() {
    let (thread_pool, _) = helper::setup(true, 2300);
    let n1 = Network::new(Uuid::new_v4(), thread_pool.clone());
    let n2 = Network::new(Uuid::new_v4(), thread_pool.clone());
    let a1 = Address::Mpsc(42);
    let a2 = Address::Mpsc(1337);
    //n1.listen(&a1).unwrap(); //await //TODO: evaluate if this should be allowed
    // or is forbidden behavior...
    n2.listen(&a2).unwrap(); // only requiered here, but doesnt hurt on n1
    thread::sleep(Duration::from_millis(3)); //TODO: listeing still doesnt block correctly!

    let p1 = block_on(n1.connect(&a2)).unwrap(); //await
    let s1 = p1.open(16, Promise::InOrder | Promise::NoCorrupt).unwrap();

    assert!(s1.send("Hello World").is_ok());

    thread::sleep(Duration::from_millis(3)); //TODO: listeing still doesnt block correctly!
    let p1_n2 = block_on(n2.connected()).unwrap(); //remote representation of p1
    let mut s1_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s1

    let s: Result<String, _> = block_on(s1_n2.recv());
    assert_eq!(s, Ok("Hello World".to_string()));

    assert!(s1.close().is_ok());
}
