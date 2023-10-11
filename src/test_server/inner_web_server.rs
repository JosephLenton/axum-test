use ::anyhow::Context;
use ::anyhow::Result;
use ::axum::Server as AxumServer;
use ::reserve_port::ReservedPort;
use ::std::net::IpAddr;
use ::tokio::task::JoinHandle;
use ::url::Url;

use crate::internals::StartingTcpSetup;
use crate::IntoTestServerThread;

#[derive(Debug)]
pub struct InnerWebServer {
    server_thread: JoinHandle<()>,
    server_url: Url,

    /// If this has reserved a port for the test,
    /// then it is stored here.
    ///
    /// It's stored here until we `Drop` (as it's reserved).
    #[allow(dead_code)]
    maybe_reserved_port: Option<ReservedPort>,
}

impl InnerWebServer {
    pub fn random<A>(app: A) -> Result<Self>
    where
        A: IntoTestServerThread,
    {
        Self::from_ip_port(app, None, None)
    }

    pub fn from_ip_port<A>(app: A, ip: Option<IpAddr>, port: Option<u16>) -> Result<Self>
    where
        A: IntoTestServerThread,
    {
        let setup =
            StartingTcpSetup::new(ip, port).context("Cannot create socket address for use")?;

        let server_builder = AxumServer::from_tcp(setup.tcp_listener)
            .with_context(|| "Failed to create ::axum::Server for TestServer")?;

        let server_thread = app.into_server_thread(server_builder);

        let socket_address = setup.socket_addr;
        let server_address = format!("http://{socket_address}");
        let server_url: Url = server_address.parse()?;

        let this = Self {
            server_thread,
            server_url,
            maybe_reserved_port: setup.maybe_reserved_port,
        };

        Ok(this)
    }

    /// Returns the local web address for the test server.
    ///
    /// By default this will be something like `http://0.0.0.0:1234/`,
    /// where `1234` is a randomly assigned port numbr.
    pub fn server_address<'a>(&'a self) -> &'a str {
        &self.server_url.as_str()
    }

    pub fn url(&self) -> Url {
        self.server_url.clone()
    }
}

impl Drop for InnerWebServer {
    fn drop(&mut self) {
        self.server_thread.abort();
    }
}
