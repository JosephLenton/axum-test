use ::anyhow::Context;
use ::anyhow::Result;
use ::reserve_port::ReservedPort;
use ::std::net::IpAddr;
use ::std::net::Ipv4Addr;
use ::std::net::SocketAddr;
use ::std::net::TcpListener;

pub const DEFAULT_IP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

pub struct StartingTcpSetup {
    pub maybe_reserved_port: Option<ReservedPort>,
    pub socket_addr: SocketAddr,
    pub tcp_listener: TcpListener,
}

impl StartingTcpSetup {
    pub fn new(maybe_ip: Option<IpAddr>, maybe_port: Option<u16>) -> Result<Self> {
        let ip = maybe_ip.unwrap_or(DEFAULT_IP_ADDRESS);

        maybe_port
            .map(|port| Self::new_with_port(ip, port))
            .unwrap_or_else(|| Self::new_without_port(ip))
    }

    fn new_with_port(ip: IpAddr, port: u16) -> Result<Self> {
        ReservedPort::reserve_port(port)?;
        let socket_addr = SocketAddr::new(ip, port);
        let tcp_listener = TcpListener::bind(socket_addr)
            .with_context(|| "Failed to create TCPListener for TestServer")?;

        Ok(Self {
            maybe_reserved_port: None,
            socket_addr,
            tcp_listener,
        })
    }

    fn new_without_port(ip: IpAddr) -> Result<Self> {
        let (reserved_port, tcp_listener) = ReservedPort::random_with_tcp(ip)?;
        let socket_addr = SocketAddr::new(ip, reserved_port.port());

        Ok(Self {
            maybe_reserved_port: Some(reserved_port),
            socket_addr,
            tcp_listener,
        })
    }
}

#[cfg(test)]
mod test_new {
    use super::*;
    use ::regex::Regex;
    use std::net::Ipv4Addr;

    #[test]
    fn it_should_create_default_ip_with_random_port_when_none() {
        let ip = None;
        let port = None;

        let setup = StartingTcpSetup::new(ip, port).unwrap();
        let addr = format!("{}", setup.socket_addr);

        let regex = Regex::new("^127\\.0\\.0\\.1:[0-9]+$").unwrap();
        let is_match = regex.is_match(&addr);
        assert!(is_match);
    }

    #[test]
    fn it_should_create_ip_with_random_port_when_ip_given() {
        let ip = Some(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let port = None;

        let setup = StartingTcpSetup::new(ip, port).unwrap();
        let addr = format!("{}", setup.socket_addr);

        let regex = Regex::new("^127\\.0\\.0\\.1:[0-9]+$").unwrap();
        let is_match = regex.is_match(&addr);
        assert!(is_match);
    }

    #[test]
    fn it_should_create_default_ip_with_port_when_port_given() {
        let ip = None;
        let port = Some(8123);

        let setup = StartingTcpSetup::new(ip, port).unwrap();
        let addr = format!("{}", setup.socket_addr);

        assert_eq!(addr, "127.0.0.1:8123");
    }

    #[test]
    fn it_should_create_ip_port_given_when_both_given() {
        let ip = Some(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let port = Some(8124);

        let setup = StartingTcpSetup::new(ip, port).unwrap();
        let addr = format!("{}", setup.socket_addr);

        assert_eq!(addr, "127.0.0.1:8124");
    }
}
