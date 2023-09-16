use ::anyhow::Result;
use ::std::net::SocketAddr;
use ::std::net::IpAddr;
use ::std::net::Ipv4Addr;

use super::ReservedPort;

pub(crate) const DEFAULT_IP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

#[derive(Debug)]
pub struct ReservedSocketAddr {
    reserved_port: ReservedPort,
    socket_addr: SocketAddr,
}

impl ReservedSocketAddr {
    #[must_use]
    pub fn reserve_random_socket_addr() -> Result<Self> {
        let ip_address = DEFAULT_IP_ADDRESS;
        let reserved_port = ReservedPort::reserve_random_port()?;
        let socket_addr = SocketAddr::new(ip_address, reserved_port.port());

        Ok(Self {
            reserved_port,
            socket_addr,
        })
    }

    pub fn port(&self) -> u16 {
        self.reserved_port.port()
    }

    pub fn ip(&self) -> IpAddr {
        self.socket_addr.ip()
    }

    pub fn socket_addr(&self) -> SocketAddr {
        self.socket_addr
    }
}
