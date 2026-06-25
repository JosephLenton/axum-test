use crate::internals::StartingTcpSetup;
use anyhow::Context;
use anyhow::Result;
use reserve_port::ReservedPort;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::TcpListener as StdTcpListener;
use tokio::net::TcpListener as TokioTcpListener;

#[derive(Debug)]
pub struct TransportLayerBuilder {
    kind: TransportLayerBuilderKind,
}

impl TransportLayerBuilder {
    pub(crate) fn from_ip_port(ip: Option<IpAddr>, port: Option<u16>) -> Self {
        Self {
            kind: TransportLayerBuilderKind::IpPort { ip, port },
        }
    }

    pub(crate) fn from_tcp_listener(tcp_listener: StdTcpListener) -> Self {
        Self {
            kind: TransportLayerBuilderKind::TcpListener { tcp_listener },
        }
    }

    pub(crate) fn tcp_listener_with_reserved_port(
        self,
    ) -> Result<(SocketAddr, TokioTcpListener, Option<ReservedPort>)> {
        let setup = self
            .kind
            .into_tcp_setup()
            .context("Cannot create socket address for use")?;

        let socket_addr = setup.socket_addr;
        let tcp_listener = setup.tcp_listener;
        let maybe_reserved_port = setup.maybe_reserved_port;

        Ok((socket_addr, tcp_listener, maybe_reserved_port))
    }

    pub fn tcp_listener(self) -> Result<TokioTcpListener> {
        let (_, tcp_listener, _) = self.tcp_listener_with_reserved_port()?;
        Ok(tcp_listener)
    }
}

#[derive(Debug)]
pub(crate) enum TransportLayerBuilderKind {
    IpPort {
        ip: Option<IpAddr>,
        port: Option<u16>,
    },
    TcpListener {
        tcp_listener: StdTcpListener,
    },
}

impl TransportLayerBuilderKind {
    fn into_tcp_setup(self) -> Result<StartingTcpSetup> {
        match self {
            Self::IpPort { ip, port } => StartingTcpSetup::from_ip_port(ip, port),
            Self::TcpListener { tcp_listener } => StartingTcpSetup::from_tcp_listener(tcp_listener),
        }
    }
}
