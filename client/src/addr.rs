use std::net::SocketAddr;
use tokio::net::lookup_host;
use tracing::trace;

#[derive(Clone, Debug)]
pub enum ConnectionArgs {
    /// <hostname/ip>:[<port>] + preferIpv6 flag
    HostnameAndOptionalPort(String, bool),
    IpAndPort(Vec<SocketAddr>),
    Mpsc(u64),
}

impl ConnectionArgs {
    const DEFAULT_PORT: u16 = 14004;

    /// Parse ip address or resolves hostname, moves HostnameAndOptionalPort to
    /// IpAndPort state.
    /// Note: If you use an ipv6 address, the number after
    /// the last colon will be used as the port unless you use [] around the
    /// address.
    pub async fn resolve(&mut self) -> Result<(), std::io::Error> {
        if let ConnectionArgs::HostnameAndOptionalPort(server_address, prefer_ipv6) = self {
            // 1. Try if server_address already contains a port
            if let Ok(addr) = server_address.parse::<SocketAddr>() {
                trace!("Server address with port found");
                *self = ConnectionArgs::IpAndPort(vec![addr]);
                return Ok(());
            }

            // 2, Try server_address and port
            if let Ok(addr) =
                format!("{}:{}", server_address, Self::DEFAULT_PORT).parse::<SocketAddr>()
            {
                trace!("Server address without port found");
                *self = ConnectionArgs::IpAndPort(vec![addr]);
                return Ok(());
            }

            // 3. Assert it's a hostname + port
            let new = match lookup_host(server_address.to_string()).await {
                Ok(s) => {
                    trace!("Host lookup succeeded");
                    Ok(Self::sort_ipv6(s, *prefer_ipv6))
                },
                Err(e) => {
                    // 4. Assume its a hostname without port
                    match lookup_host((server_address.to_string(), Self::DEFAULT_PORT)).await {
                        Ok(s) => {
                            trace!("Host lookup without ports succeeded");
                            Ok(Self::sort_ipv6(s, *prefer_ipv6))
                        },
                        Err(_) => Err(e),
                    }
                },
            }?;

            *self = new;
        }
        Ok(())
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
    async fn keep_mpcs() {
        let mut args = ConnectionArgs::Mpsc(1337);
        assert!(args.resolve().await.is_ok());
        assert!(matches!(args, ConnectionArgs::Mpsc(1337)));
    }

    #[tokio::test]
    async fn keep_ip() {
        let mut args = ConnectionArgs::IpAndPort(vec![SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            ConnectionArgs::DEFAULT_PORT,
        )]);
        assert!(args.resolve().await.is_ok());
        assert!(matches!(args, ConnectionArgs::IpAndPort(..)));
    }

    #[tokio::test]
    async fn resolve_localhost() {
        let mut args = ConnectionArgs::HostnameAndOptionalPort("localhost".to_string(), false);
        assert!(args.resolve().await.is_ok());
        if let ConnectionArgs::IpAndPort(args) = args {
            assert!(args.len() == 1 || args.len() == 2);
            assert_eq!(args[0].ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
            assert_eq!(args[0].port(), 14004);
        } else {
            panic!("wrong resolution");
        }

        let mut args = ConnectionArgs::HostnameAndOptionalPort("localhost:666".to_string(), false);
        assert!(args.resolve().await.is_ok());
        if let ConnectionArgs::IpAndPort(args) = args {
            assert!(args.len() == 1 || args.len() == 2);
            assert_eq!(args[0].port(), 666);
        } else {
            panic!("wrong resolution");
        }
    }

    #[tokio::test]
    async fn resolve_ipv6() {
        let mut args = ConnectionArgs::HostnameAndOptionalPort("localhost".to_string(), true);
        assert!(args.resolve().await.is_ok());
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
        let mut args = ConnectionArgs::HostnameAndOptionalPort("google.com".to_string(), false);
        assert!(args.resolve().await.is_ok());
        if let ConnectionArgs::IpAndPort(args) = args {
            assert!(args.len() == 1 || args.len() == 2);
            assert_eq!(args[0].port(), 14004);
        } else {
            panic!("wrong resolution");
        }

        let mut args = ConnectionArgs::HostnameAndOptionalPort("127.0.0.1".to_string(), false);
        assert!(args.resolve().await.is_ok());
        if let ConnectionArgs::IpAndPort(args) = args {
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].port(), 14004);
            assert_eq!(args[0].ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        } else {
            panic!("wrong resolution");
        }

        let mut args = ConnectionArgs::HostnameAndOptionalPort("55.66.77.88".to_string(), false);
        assert!(args.resolve().await.is_ok());
        if let ConnectionArgs::IpAndPort(args) = args {
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].port(), 14004);
            assert_eq!(args[0].ip(), IpAddr::V4(Ipv4Addr::new(55, 66, 77, 88)));
        } else {
            panic!("wrong resolution");
        }

        let mut args = ConnectionArgs::HostnameAndOptionalPort("127.0.0.1:776".to_string(), false);
        assert!(args.resolve().await.is_ok());
        if let ConnectionArgs::IpAndPort(args) = args {
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].port(), 776);
            assert_eq!(args[0].ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        } else {
            panic!("wrong resolution");
        }
    }
}
