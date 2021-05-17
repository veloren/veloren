use std::net::SocketAddr;
use tokio::net::lookup_host;
use tracing::trace;

#[derive(Clone, Debug)]
pub enum ConnectionArgs {
    IpAndPort(Vec<SocketAddr>),
    Mpsc(u64),
}

impl ConnectionArgs {
    const DEFAULT_PORT: u16 = 14004;

    /// Parse ip address or resolves hostname.
    /// Note: If you use an ipv6 address, the number after the last
    /// colon will be used as the port unless you use [] around the address.
    pub async fn resolve(
        /* <hostname/ip>:[<port>] */ server_address: &str,
        prefer_ipv6: bool,
    ) -> Result<Self, std::io::Error> {
        // `lookup_host` will internally try to parse it as a SocketAddr
        // 1. Assume it's a hostname + port
        match lookup_host(server_address).await {
            Ok(s) => {
                trace!("Host lookup succeeded");
                Ok(Self::sort_ipv6(s, prefer_ipv6))
            },
            Err(e) => {
                // 2. Assume its a hostname without port
                match lookup_host((server_address, Self::DEFAULT_PORT)).await {
                    Ok(s) => {
                        trace!("Host lookup without ports succeeded");
                        Ok(Self::sort_ipv6(s, prefer_ipv6))
                    },
                    Err(_) => Err(e), // Todo: evaluate returning both errors
                }
            },
        }
    }

    fn sort_ipv6(s: impl Iterator<Item = SocketAddr>, prefer_ipv6: bool) -> Self {
        let (mut first_addrs, mut second_addrs) =
            s.partition::<Vec<_>, _>(|a| a.is_ipv6() == prefer_ipv6);
        let addr = std::iter::Iterator::chain(first_addrs.drain(..), second_addrs.drain(..))
            .collect::<Vec<_>>();
        ConnectionArgs::IpAndPort(addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[tokio::test]
    async fn resolve_localhost() {
        let args = ConnectionArgs::resolve("localhost", false)
            .await
            .expect("resolve failed");
        if let ConnectionArgs::IpAndPort(args) = args {
            assert!(args.len() == 1 || args.len() == 2);
            assert_eq!(args[0].ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
            assert_eq!(args[0].port(), 14004);
        } else {
            panic!("wrong resolution");
        }

        let args = ConnectionArgs::resolve("localhost:666", false)
            .await
            .expect("resolve failed");
        if let ConnectionArgs::IpAndPort(args) = args {
            assert!(args.len() == 1 || args.len() == 2);
            assert_eq!(args[0].port(), 666);
        } else {
            panic!("wrong resolution");
        }
    }

    #[tokio::test]
    async fn resolve_ipv6() {
        let args = ConnectionArgs::resolve("localhost", true)
            .await
            .expect("resolve failed");
        if let ConnectionArgs::IpAndPort(args) = args {
            assert!(args.len() == 1 || args.len() == 2);
            assert_eq!(args[0].ip(), Ipv6Addr::LOCALHOST);
            assert_eq!(args[0].port(), 14004);
        } else {
            panic!("wrong resolution");
        }
    }

    #[tokio::test]
    async fn resolve() {
        let args = ConnectionArgs::resolve("google.com", false)
            .await
            .expect("resolve failed");
        if let ConnectionArgs::IpAndPort(args) = args {
            assert!(args.len() == 1 || args.len() == 2);
            assert_eq!(args[0].port(), 14004);
        } else {
            panic!("wrong resolution");
        }

        let args = ConnectionArgs::resolve("127.0.0.1", false)
            .await
            .expect("resolve failed");
        if let ConnectionArgs::IpAndPort(args) = args {
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].port(), 14004);
            assert_eq!(args[0].ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        } else {
            panic!("wrong resolution");
        }

        let args = ConnectionArgs::resolve("55.66.77.88", false)
            .await
            .expect("resolve failed");
        if let ConnectionArgs::IpAndPort(args) = args {
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].port(), 14004);
            assert_eq!(args[0].ip(), IpAddr::V4(Ipv4Addr::new(55, 66, 77, 88)));
        } else {
            panic!("wrong resolution");
        }

        let args = ConnectionArgs::resolve("127.0.0.1:776", false)
            .await
            .expect("resolve failed");
        if let ConnectionArgs::IpAndPort(args) = args {
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].port(), 776);
            assert_eq!(args[0].ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        } else {
            panic!("wrong resolution");
        }
    }
}
