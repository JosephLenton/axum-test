use ::anyhow::Result;

use crate::transport_layer::IntoTransportLayer;
use crate::TestServer;
use crate::TestServerConfigBuilder;
use crate::Transport;

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
#[derive(Debug, Clone, PartialEq)]
pub struct TestServerConfig {
    /// Which transport mode to use to process requests.
    /// For setting if the server should use mocked http (which uses [`tower::util::Oneshot`](tower::util::Oneshot)),
    /// or if it should run on a named or random IP address.
    ///
    /// The default is to use mocking, apart from services built using [`axum::extract::connect_info::IntoMakeServiceWithConnectInfo`](axum::extract::connect_info::IntoMakeServiceWithConnectInfo)
    /// (this is because it needs a real TCP stream).
    pub transport: Option<Transport>,

    /// Set for the server to save cookies that are returned,
    /// for use in future requests.
    ///
    /// This is useful for automatically saving session cookies (and similar)
    /// like a browser would do.
    ///
    /// **Defaults** to false (being turned off).
    pub save_cookies: bool,

    /// Asserts that requests made to the test server,
    /// will by default,
    /// return a status code in the 2xx range.
    ///
    /// This can be overridden on a per request basis using
    /// [`TestRequest::expect_failure()`](crate::TestRequest::expect_failure()).
    ///
    /// This is useful when making multiple requests at a start of test
    /// which you presume should always work.
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

    /// Set the default scheme to use for all requests created by the `TestServer`.
    ///
    /// This overrides the default 'http'.
    pub default_scheme: Option<String>,
}

impl TestServerConfig {
    /// Creates a builder for making it simpler to creating configs.
    ///
    /// ```rust
    /// use ::axum_test::TestServerConfig;
    ///
    /// let config = TestServerConfig::builder()
    ///     .save_cookies()
    ///     .default_content_type(&"application/json")
    ///     .build();
    /// ```
    pub fn builder() -> TestServerConfigBuilder {
        TestServerConfigBuilder::default()
    }

    /// This is shorthand for calling [`crate::TestServer::new_with_config`].
    ///
    /// ```rust
    /// use ::axum::Router;
    /// use ::axum_test::TestServerConfig;
    ///
    /// let app = Router::new();
    /// let config = TestServerConfig::builder()
    ///     .save_cookies()
    ///     .default_content_type(&"application/json")
    ///     .build();
    /// let server = config.build_server(app);
    /// ```
    pub fn build_server<A>(self, app: A) -> Result<TestServer>
    where
        A: IntoTransportLayer,
    {
        TestServer::new_with_config(app, self)
    }
}

impl Default for TestServerConfig {
    fn default() -> Self {
        Self {
            transport: None,
            save_cookies: false,
            expect_success_by_default: false,
            restrict_requests_with_http_schema: false,
            default_content_type: None,
            default_scheme: None,
        }
    }
}

#[cfg(test)]
mod test_scheme {
    use axum::extract::Request;
    use axum::routing::get;
    use axum::Router;

    use crate::TestServer;
    use crate::TestServerConfig;

    async fn route_get_scheme(request: Request) -> String {
        request.uri().scheme_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn it_should_set_scheme_when_present_in_config() {
        let router = Router::new().route("/scheme", get(route_get_scheme));

        let config = TestServerConfig {
            default_scheme: Some("https".to_string()),
            ..Default::default()
        };
        let server = TestServer::new_with_config(router, config).unwrap();

        server.get("/scheme").await.assert_text("https");
    }
}
