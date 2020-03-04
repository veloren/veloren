#![feature(trait_alias)]
mod api;
mod channel;
mod controller;
mod message;
mod metrics;
mod mpsc;
mod tcp;
mod types;
mod udp;
mod worker;

pub use api::{
    Address, Network, NetworkError, Participant, ParticipantError, Promise, Stream, StreamError,
};

#[cfg(test)]
pub mod tests {
    use crate::api::*;
    use futures::executor::block_on;
    use std::{net::SocketAddr, sync::Arc, thread, time::Duration};
    use tracing::*;
    use tracing_subscriber::EnvFilter;
    use uuid::Uuid;
    use uvth::ThreadPoolBuilder;

    pub fn test_tracing() {
        let filter = EnvFilter::from_default_env()
            //.add_directive("[worker]=trace".parse().unwrap())
            .add_directive("trace".parse().unwrap())
            .add_directive("veloren_network::tests=trace".parse().unwrap())
            .add_directive("veloren_network::worker=debug".parse().unwrap())
            .add_directive("veloren_network::controller=trace".parse().unwrap())
            .add_directive("veloren_network::channel=trace".parse().unwrap())
            .add_directive("veloren_network::message=trace".parse().unwrap())
            .add_directive("veloren_network::metrics=trace".parse().unwrap())
            .add_directive("veloren_network::types=trace".parse().unwrap())
            .add_directive("veloren_network::mpsc=debug".parse().unwrap())
            .add_directive("veloren_network::udp=debug".parse().unwrap())
            .add_directive("veloren_network::tcp=debug".parse().unwrap());

        tracing_subscriber::FmtSubscriber::builder()
            // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
            // will be written to stdout.
            .with_max_level(Level::TRACE)
            .with_env_filter(filter)
            // sets this to be the default, global subscriber for this application.
            .init();
    }

    #[test]
    fn aaa() { test_tracing(); }

    #[test]
    fn client_server() {
        let thread_pool = Arc::new(
            ThreadPoolBuilder::new()
                .name("veloren-network-test".into())
                .build(),
        );
        thread::sleep(Duration::from_millis(200));
        let n1 = Network::new(Uuid::new_v4(), thread_pool.clone());
        let n2 = Network::new(Uuid::new_v4(), thread_pool.clone());
        let a1 = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], 52000)));
        let a2 = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], 52001)));
        block_on(n1.listen(&a1)).unwrap(); //await
        block_on(n2.listen(&a2)).unwrap(); // only requiered here, but doesnt hurt on n1
        thread::sleep(Duration::from_millis(3)); //TODO: listeing still doesnt block correctly!

        let p1 = block_on(n1.connect(&a2)).unwrap(); //await
        let s1 = block_on(p1.open(16, Promise::InOrder | Promise::NoCorrupt)).unwrap();

        assert!(s1.send("Hello World").is_ok());

        let p1_n2 = block_on(n2.connected()).unwrap(); //remote representation of p1
        let mut s1_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s1

        let s: Result<String, _> = block_on(s1_n2.recv());
        assert_eq!(s, Ok("Hello World".to_string()));

        assert!(p1.close(s1).is_ok());
    }

    #[test]
    fn client_server_stream() {
        let thread_pool = Arc::new(
            ThreadPoolBuilder::new()
                .name("veloren-network-test".into())
                .build(),
        );
        thread::sleep(Duration::from_millis(400));
        let n1 = Network::new(Uuid::new_v4(), thread_pool.clone());
        let n2 = Network::new(Uuid::new_v4(), thread_pool.clone());
        let a1 = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], 52010)));
        let a2 = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], 52011)));

        block_on(n1.listen(&a1)).unwrap(); //await
        block_on(n2.listen(&a2)).unwrap(); // only requiered here, but doesnt hurt on n1
        thread::sleep(Duration::from_millis(3)); //TODO: listeing still doesnt block correctly!

        let p1 = block_on(n1.connect(&a2)).unwrap(); //await

        let s1 = block_on(p1.open(16, Promise::InOrder | Promise::NoCorrupt)).unwrap();
        let s2 = block_on(p1.open(16, Promise::InOrder | Promise::NoCorrupt)).unwrap();
        let s3 = block_on(p1.open(16, Promise::InOrder | Promise::NoCorrupt)).unwrap();
        let s4 = block_on(p1.open(16, Promise::InOrder | Promise::NoCorrupt)).unwrap();
        let s5 = block_on(p1.open(16, Promise::InOrder | Promise::NoCorrupt)).unwrap();

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

        assert!(p1.close(s1).is_ok());
    }
}
