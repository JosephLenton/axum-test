use ::anyhow::Result;
use ::std::net::IpAddr;
use ::std::net::Ipv4Addr;
use ::std::net::SocketAddr;

use crate::util::new_random_port;

pub(crate) const DEFAULT_IP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

/// Generates a `SocketAddr` on the IP 127.0.0.1, using a random port.
pub fn new_random_socket_addr() -> Result<SocketAddr> {
    let ip_address = DEFAULT_IP_ADDRESS;
    let port = new_random_port()?;
    let addr = SocketAddr::new(ip_address, port);

    Ok(addr)
}
