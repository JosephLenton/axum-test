use ::std::net::IpAddr;

/// The basic setup for the `TestServer`.
#[derive(Debug, Clone)]
pub struct TestServerConfig {
    /// Set the default content type for all requests created by the `TestServer`.
    ///
    /// This overrides the default 'best efforts' approach of requests.
    pub default_content_type: Option<String>,

    /// Set the IP to use for the server.
    ///
    /// **Defaults** to `127.0.0.1`.
    pub ip: Option<IpAddr>,

    /// Set the port number to use for the server.
    ///
    /// **Defaults** to a _random_ port.
    pub port: Option<u16>,

    /// Set for the server to save cookies that are returned,
    /// for use in future requests.
    ///
    /// This is useful for automatically saving session cookies (and similar)
    /// like a browser would do.
    ///
    /// **Defaults** to false (being turned off).
    pub save_cookies: bool,

    /// If you make a request with a 'http://' schema,
    /// then it will ignore the Test Server's address.
    ///
    /// For example if the test server is running at `http://localhost:1234`,
    /// and you make a request to `http://google.com`.
    /// Then the request will go to `http://google.com`.
    /// Ignoring the `localhost:1234` part.
    ///
    /// Turning this setting on will change this behaviour.
    ///
    /// After turning this on, the same request will go to
    /// `http://localhost:1234/http://google.com`.
    ///
    /// **Defaults** to false (being turned off).
    pub restrict_requests_with_http_schema: bool,
}

impl Default for TestServerConfig {
    fn default() -> Self {
        Self {
            default_content_type: None,
            ip: None,
            port: None,
            save_cookies: false,
            restrict_requests_with_http_schema: false,
        }
    }
}
