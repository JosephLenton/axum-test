use crate::TestServerTransport;

/// This is for customising the [`TestServer`](crate::TestServer) on construction.
///
/// It implements [`Default`] to ease building configurations:
///
/// ```rust
/// use ::axum_test::TestServerConfig;
///
/// let config = TestServerConfig {
///     save_cookies: true,
///     ..TestServerConfig::default()
/// };
/// ```
///
/// These can be passed to `TestServer::new_with_config`:
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use ::axum::Router;
/// use ::axum_test::TestServer;
/// use ::axum_test::TestServerConfig;
///
/// let my_app = Router::new();
///
/// let config = TestServerConfig {
///     save_cookies: true,
///     ..TestServerConfig::default()
/// };
///
/// // Build the Test Server
/// let server = TestServer::new_with_config(my_app, config)?;
/// #
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct TestServerConfig {
    pub transport: TestServerTransport,

    /// Set for the server to save cookies that are returned,
    /// for use in future requests.
    ///
    /// This is useful for automatically saving session cookies (and similar)
    /// like a browser would do.
    ///
    /// **Defaults** to false (being turned off).
    pub save_cookies: bool,

    /// Sets requests made by the server to always expect a status code returned in the 2xx range,
    /// and to panic if that is missing.
    ///
    /// This is useful when making multiple requests at a start of test
    /// which you presume should always work. It also helps to make tests more explicit.
    ///
    /// **Defaults** to false (being turned off).
    pub expect_success_by_default: bool,

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

    /// Set the default content type for all requests created by the `TestServer`.
    ///
    /// This overrides the default 'best efforts' approach of requests.
    pub default_content_type: Option<String>,
}

impl Default for TestServerConfig {
    fn default() -> Self {
        Self {
            transport: TestServerTransport::default(),
            save_cookies: false,
            expect_success_by_default: false,
            restrict_requests_with_http_schema: false,
            default_content_type: None,
        }
    }
}
