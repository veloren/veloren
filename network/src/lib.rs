#![feature(trait_alias)]
mod api;
mod message;
mod protocol;

#[cfg(test)]
mod tests {
    use crate::api::*;
    use std::net::SocketAddr;

    struct N {
        id: u8,
    }
    impl Events for N {
        fn OnRemoteConnectionOpen(net: &Network<N>, con: &Connection) {}

        fn OnRemoteConnectionClose(net: &Network<N>, con: &Connection) {}

        fn OnRemoteStreamOpen(net: &Network<N>, st: &Stream) {}

        fn OnRemoteStreamClose(net: &Network<N>, st: &Stream) {}
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn client_server() {
        let n1 = Network::<N>::new();
        let n2 = Network::<N>::new();
        let a1s = Address::Tcp(SocketAddr::from(([0, 0, 0, 0], 52000u16)));
        let a1 = Address::Tcp(SocketAddr::from(([1, 0, 0, 127], 52000u16)));
        let a2s = Address::Tcp(SocketAddr::from(([0, 0, 0, 0], 52001u16)));
        let a2 = Address::Tcp(SocketAddr::from(([1, 0, 0, 127], 52001u16)));
        n1.listen(&a1s); //await
        n2.listen(&a2s); // only requiered here, but doesnt hurt on n1

        let p1 = n1.connect(&a2); //await
        //n2.OnRemoteConnectionOpen triggered

        let s1 = n1.open(p1, 16, Promise::InOrder | Promise::NoCorrupt);
        //n2.OnRemoteStreamOpen triggered

        n1.send("", &s1);
        // receive on n2 now

        n1.close(s1);
        //n2.OnRemoteStreamClose triggered
    }
}
