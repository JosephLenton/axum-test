use ::anyhow::Result;
use ::reserve_port::ReservedPort;
use ::std::net::IpAddr;
use ::std::net::Ipv4Addr;
use ::std::net::SocketAddr;
use ::tokio::net::TcpListener as TokioTcpListener;

pub(crate) const DEFAULT_IP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Binds a [`tokio::net::TcpListener`] on the IP 127.0.0.1, using a random port.
///
/// This is the best way to pick a local port.
pub fn new_random_tokio_tcp_listener() -> Result<TokioTcpListener> {
    new_random_tokio_tcp_listener_with_socket_addr()
        .map(|(tokio_tcp_listener, _)| tokio_tcp_listener)
}

/// Binds a [`tokio::net::TcpListener`] on the IP 127.0.0.1, using a random port.
///
/// It is returned with the [`std::net::SocketAddr`] available.
pub fn new_random_tokio_tcp_listener_with_socket_addr() -> Result<(TokioTcpListener, SocketAddr)> {
    let (tcp_listener, random_socket) =
        ReservedPort::random_permanently_reserved_tcp(DEFAULT_IP_ADDRESS)?;

    tcp_listener.set_nonblocking(true)?;
    let tokio_tcp_listener = TokioTcpListener::from_std(tcp_listener)?;

    Ok((tokio_tcp_listener, random_socket))
}
