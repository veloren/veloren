#![feature(trait_alias)]
mod api;
mod internal;
mod message;
mod mio_worker;
mod tcp_channel;

#[cfg(test)]
pub mod tests {
    use crate::api::*;
    use std::{net::SocketAddr, sync::Arc};
    use uuid::Uuid;
    use uvth::ThreadPoolBuilder;

    struct N {
        id: u8,
    }

    impl Events for N {
        fn on_remote_connection_open(_net: &Network<N>, _con: &Connection) {}

        fn on_remote_connection_close(_net: &Network<N>, _con: &Connection) {}

        fn on_remote_stream_open(_net: &Network<N>, _st: &Stream) {}

        fn on_remote_stream_close(_net: &Network<N>, _st: &Stream) {}
    }

    pub fn test_tracing() {
        use tracing::Level;

        tracing_subscriber::FmtSubscriber::builder()
            // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
            // will be written to stdout.
            .with_max_level(Level::TRACE)
            //.with_env_filter("veloren_network::api=info,my_crate::my_mod=debug,[my_span]=trace")
            // sets this to be the default, global subscriber for this application.
            .init();
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn client_server() {
        let thread_pool = Arc::new(
            ThreadPoolBuilder::new()
                .name("veloren-network-test".into())
                .build(),
        );
        test_tracing();
        let n1 = Network::<N>::new(Uuid::new_v4(), thread_pool.clone());
        let n2 = Network::<N>::new(Uuid::new_v4(), thread_pool.clone());
        let a1 = Address::Tcp(SocketAddr::from(([10, 52, 0, 101], 52000)));
        let a2 = Address::Tcp(SocketAddr::from(([10, 52, 0, 101], 52001)));
        //let a1 = Address::Tcp(SocketAddr::from(([10, 42, 2, 2], 52000)));
        //let a2 = Address::Tcp(SocketAddr::from(([10, 42, 2, 2], 52001)));
        n1.listen(&a1); //await
        n2.listen(&a2); // only requiered here, but doesnt hurt on n1
        std::thread::sleep(std::time::Duration::from_millis(20));

        let p1 = n1.connect(&a2); //await
        //n2.OnRemoteConnectionOpen triggered
        std::thread::sleep(std::time::Duration::from_millis(20));

        let s1 = n1.open(p1, 16, Promise::InOrder | Promise::NoCorrupt);
        std::thread::sleep(std::time::Duration::from_millis(20));
        //n2.OnRemoteStreamOpen triggered

        n1.send("", &s1);
        // receive on n2 now

        n1.close(s1);
        //n2.OnRemoteStreamClose triggered

        std::thread::sleep(std::time::Duration::from_millis(20000));
    }
}
