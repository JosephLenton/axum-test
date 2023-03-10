use ::std::net::SocketAddr;

/// The basic setup for the `TestServer`.
#[derive(Debug, Clone)]
pub struct TestServerConfig {
    /// Set the default content type for all requests created by the `TestServer`.
    ///
    /// This overrides the default 'best efforts' approach of requests.
    pub default_content_type: Option<String>,

    /// Set the socket to use for the server.
    ///
    /// **Defaults** to a _random_ socket.
    pub socket_address: Option<SocketAddr>,

    /// Set for the server to save cookies that are returned,
    /// for use in future requests.
    ///
    /// This is useful for automatically saving session cookies (and similar)
    /// like a browser would do.
    ///
    /// **Defaults** to false (being turned off).
    pub save_cookies: bool,
}

impl Default for TestServerConfig {
    fn default() -> Self {
        Self {
            default_content_type: None,
            socket_address: None,
            save_cookies: false,
        }
    }
}
