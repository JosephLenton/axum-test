use anyhow::Context;
use anyhow::Result;
use reserve_port::ReservedPort;
use std::net::IpAddr;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use crate::internals::StartingTcpSetup;

pub struct TransportLayerBuilder {
    ip: Option<IpAddr>,
    port: Option<u16>,
}

impl TransportLayerBuilder {
    pub(crate) fn new(ip: Option<IpAddr>, port: Option<u16>) -> Self {
        Self { ip, port }
    }

    pub(crate) fn tcp_listener_with_reserved_port(
        self,
    ) -> Result<(SocketAddr, TcpListener, Option<ReservedPort>)> {
        let setup = StartingTcpSetup::new(self.ip, self.port)
            .context("Cannot create socket address for use")?;

        let socket_addr = setup.socket_addr;
        let tcp_listener = setup.tcp_listener;
        let maybe_reserved_port = setup.maybe_reserved_port;

        Ok((socket_addr, tcp_listener, maybe_reserved_port))
    }

    pub fn tcp_listener(self) -> Result<TcpListener> {
        let (_, tcp_listener, _) = self.tcp_listener_with_reserved_port()?;
        Ok(tcp_listener)
    }
}
