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

    pub fn block_on_recv(stream: &Stream) -> Result<String, StreamError> {
        let mut s: Result<Option<String>, StreamError> = stream.recv();
        while let Ok(None) = s {
            thread::sleep(Duration::from_millis(1));
            s = stream.recv();
        }
        if let Ok(Some(s)) = s {
            return Ok(s);
        }
        if let Err(e) = s {
            return Err(e);
        }
        unreachable!("invalid test");
    }

    #[test]
    fn aaa() { test_tracing(); }

    #[test]
    #[ignore]
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

        s1.send("Hello World");

        let p1_n2 = block_on(n2.connected()).unwrap(); //remote representation of p1
        let s1_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s1

        let s = block_on_recv(&s1_n2);
        assert_eq!(s, Ok("Hello World".to_string()));

        p1.close(s1);
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

        thread::sleep(Duration::from_millis(3));
        s3.send("Hello World3");
        thread::sleep(Duration::from_millis(3));
        s1.send("Hello World1");
        s5.send("Hello World5");
        s2.send("Hello World2");
        s4.send("Hello World4");
        thread::sleep(Duration::from_millis(3));

        let p1_n2 = block_on(n2.connected()).unwrap(); //remote representation of p1
        let s1_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s1
        let s2_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s2
        let s3_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s3
        let s4_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s4
        let s5_n2 = block_on(p1_n2.opened()).unwrap(); //remote representation of s5

        info!("all streams opened");

        let s = block_on_recv(&s3_n2);
        assert_eq!(s, Ok("Hello World3".to_string()));
        info!("1 read");
        let s = block_on_recv(&s1_n2);
        assert_eq!(s, Ok("Hello World1".to_string()));
        info!("2 read");
        let s = block_on_recv(&s2_n2);
        assert_eq!(s, Ok("Hello World2".to_string()));
        info!("3 read");
        let s = block_on_recv(&s5_n2);
        assert_eq!(s, Ok("Hello World5".to_string()));
        info!("4 read");
        let s = block_on_recv(&s4_n2);
        assert_eq!(s, Ok("Hello World4".to_string()));
        info!("5 read");

        p1.close(s1);
    }
}
