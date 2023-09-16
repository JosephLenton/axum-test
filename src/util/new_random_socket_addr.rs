use crate::util::ReservedPort;
use ::anyhow::anyhow;
use ::anyhow::Result;
use ::portpicker::pick_unused_port;
use ::std::net::IpAddr;
use ::std::net::Ipv4Addr;
use ::std::net::SocketAddr;

pub(crate) const DEFAULT_IP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

pub(crate) fn new_socket_addr_from_defaults(
    maybe_ip: Option<IpAddr>,
    maybe_port: Option<u16>,
) -> Result<(ReservedPort, SocketAddr)> {
    let reserved_port = maybe_port
        .map(ReservedPort::add_port_reservation)
        .unwrap_or_else(ReservedPort::reserve_random_port)?;

    let ip = maybe_ip.unwrap_or(DEFAULT_IP_ADDRESS);
    let socket_addr = SocketAddr::new(ip, reserved_port.port());

    Ok((reserved_port, socket_addr))
}

/// Generates a `SocketAddr` on the IP 127.0.0.1, using a random port.
pub fn new_random_socket_addr() -> Result<SocketAddr> {
    let ip_address = DEFAULT_IP_ADDRESS;
    let port = new_random_port()?;
    let addr = SocketAddr::new(ip_address, port);

    Ok(addr)
}

/// Returns a randomly selected port that is not in use.
pub fn new_random_port() -> Result<u16> {
    let port = pick_unused_port().ok_or_else(|| anyhow!("No free port was found"))?;

    Ok(port)
}

#[cfg(test)]
mod test_new_socket_addr_from_defaults {
    use super::*;
    use ::regex::Regex;
    use std::net::Ipv4Addr;

    #[test]
    fn it_should_create_default_ip_with_random_port_when_none() {
        let ip = None;
        let port = None;

        let (_, socket_addr) = new_socket_addr_from_defaults(ip, port).unwrap();
        let addr = format!("{}", socket_addr);

        let regex = Regex::new("^127\\.0\\.0\\.1:[0-9]+$").unwrap();
        let is_match = regex.is_match(&addr);
        assert!(is_match);
    }

    #[test]
    fn it_should_create_ip_with_random_port_when_ip_given() {
        let ip = Some(IpAddr::V4(Ipv4Addr::new(123, 210, 7, 8)));
        let port = None;

        let (_, socket_addr) = new_socket_addr_from_defaults(ip, port).unwrap();
        let addr = format!("{}", socket_addr);

        let regex = Regex::new("^123\\.210\\.7\\.8:[0-9]+$").unwrap();
        let is_match = regex.is_match(&addr);
        assert!(is_match);
    }

    #[test]
    fn it_should_create_default_ip_with_port_when_port_given() {
        let ip = None;
        let port = Some(123);

        let (_, socket_addr) = new_socket_addr_from_defaults(ip, port).unwrap();
        let addr = format!("{}", socket_addr);

        assert_eq!(addr, "127.0.0.1:123");
    }

    #[test]
    fn it_should_create_ip_port_given_when_both_given() {
        let ip = Some(IpAddr::V4(Ipv4Addr::new(123, 210, 7, 8)));
        let port = Some(123);

        let (_, socket_addr) = new_socket_addr_from_defaults(ip, port).unwrap();
        let addr = format!("{}", socket_addr);

        assert_eq!(addr, "123.210.7.8:123");
    }
}
