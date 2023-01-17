use ::anyhow::anyhow;
use ::anyhow::Result;
use ::portpicker::pick_unused_port;
use ::std::net::IpAddr;
use ::std::net::Ipv4Addr;
use ::std::net::SocketAddr;

/// Generates a `SocketAddr` on the IP 0.0.0.0, using a random port.
pub fn new_random_socket_addr() -> Result<SocketAddr> {
    let ip_address = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
    let port = pick_unused_port().ok_or_else(|| anyhow!("No free port was found"))?;
    let addr = SocketAddr::new(ip_address, port);

    Ok(addr)
}
