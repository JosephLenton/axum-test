use ::anyhow::Result;
use ::reserve_port::ReservedPort;
use ::std::net::IpAddr;
use ::std::net::Ipv4Addr;
use ::std::net::TcpListener;
use std::net::SocketAddr;

pub(crate) const DEFAULT_IP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Binds a `TcpListener` on the IP 127.0.0.1, using a random port.
///
/// This is the best way to pick a local port.
pub fn new_random_tcp_listener() -> Result<TcpListener> {
    let (tcp_listener, _) = ReservedPort::random_permanently_reserved_tcp(DEFAULT_IP_ADDRESS)?;
    Ok(tcp_listener)
}

/// Binds a `TcpListener` on the IP 127.0.0.1, using a random port.
///
/// It is returned with the `SocketAddr` available.
pub fn new_random_tcp_listener_with_socket_addr() -> Result<(TcpListener, SocketAddr)> {
    let result = ReservedPort::random_permanently_reserved_tcp(DEFAULT_IP_ADDRESS)?;
    Ok(result)
}
