use ::std::net::IpAddr;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TestServerTransport {
    HttpRandomPort,
    HttpIpPort {
        /// Set the IP to use for the server.
        ///
        /// **Defaults** to `127.0.0.1`.
        ip: Option<IpAddr>,

        /// Set the port number to use for the server.
        ///
        /// **Defaults** to a _random_ port.
        port: Option<u16>,
    },
    MockHttp,
}

impl Default for TestServerTransport {
    fn default() -> Self {
        Self::MockHttp
    }
}
