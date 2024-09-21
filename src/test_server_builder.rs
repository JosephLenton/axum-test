use anyhow::Result;
use std::net::IpAddr;

use crate::transport_layer::IntoTransportLayer;
use crate::TestServer;
use crate::TestServerConfig;
use crate::Transport;

/// A builder for [`crate::TestServer`]. Inside is a [`crate::TestServerConfig`],
/// configured by each method, and then turn into a server by [`crate::TestServerBuilder::build`].
///
/// The recommended way to make instances is to call [`crate::TestServer::builder`].
///
/// # Creating a [`crate::TestServer`]
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use axum::Router;
/// use axum_test::TestServerBuilder;
///
/// let my_app = Router::new();
/// let server = TestServerBuilder::new()
///     .save_cookies()
///     .default_content_type(&"application/json")
///     .build(my_app)?;
/// #
/// # Ok(())
/// # }
/// ```
///
/// # Creating a [`crate::TestServerConfig`]
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use axum::Router;
/// use axum_test::TestServer;
/// use axum_test::TestServerBuilder;
///
/// let my_app = Router::new();
/// let config = TestServerBuilder::new()
///     .save_cookies()
///     .default_content_type(&"application/json")
///     .into_config();
///
/// // Build the Test Server
/// let server = TestServer::new_with_config(my_app, config)?;
/// #
/// # Ok(())
/// # }
/// ```
///
/// These can be passed to [`crate::TestServer::new_with_config`].
///
#[derive(Debug, Clone)]
pub struct TestServerBuilder {
    config: TestServerConfig,
}

impl TestServerBuilder {
    /// Creates a default `TestServerBuilder`.
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_config(config: TestServerConfig) -> Self {
        Self { config }
    }

    pub fn http_transport(self) -> Self {
        self.transport(Transport::HttpRandomPort)
    }

    pub fn http_transport_with_ip_port(self, ip: Option<IpAddr>, port: Option<u16>) -> Self {
        self.transport(Transport::HttpIpPort { ip, port })
    }

    pub fn mock_transport(self) -> Self {
        self.transport(Transport::MockHttp)
    }

    pub fn transport(mut self, transport: Transport) -> Self {
        self.config.transport = Some(transport);
        self
    }

    pub fn save_cookies(mut self) -> Self {
        self.config.save_cookies = true;
        self
    }

    pub fn do_not_save_cookies(mut self) -> Self {
        self.config.save_cookies = false;
        self
    }

    pub fn default_content_type(mut self, content_type: &str) -> Self {
        self.config.default_content_type = Some(content_type.to_string());
        self
    }

    pub fn default_scheme(mut self, scheme: &str) -> Self {
        self.config.default_scheme = Some(scheme.to_string());
        self
    }

    pub fn expect_success_by_default(mut self) -> Self {
        self.config.expect_success_by_default = true;
        self
    }

    pub fn restrict_requests_with_http_schema(mut self) -> Self {
        self.config.restrict_requests_with_http_schema = true;
        self
    }

    /// For turning this into a [`crate::TestServerConfig`] object,
    /// with can be passed to [`crate::TestServer::new_with_config`].
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum_test::TestServer;
    ///
    /// let my_app = Router::new();
    /// let config = TestServer::builder()
    ///     .save_cookies()
    ///     .default_content_type(&"application/json")
    ///     .into_config();
    ///
    /// // Build the Test Server
    /// let server = TestServer::new_with_config(my_app, config)?;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn into_config(self) -> TestServerConfig {
        self.config
    }

    /// Creates a new [`crate::TestServer`], running the application given,
    /// and with all settings from this `TestServerBuilder` applied.
    ///
    /// ```rust
    /// use axum::Router;
    /// use axum_test::TestServer;
    ///
    /// let app = Router::new();
    /// let server = TestServer::builder()
    ///     .save_cookies()
    ///     .default_content_type(&"application/json")
    ///     .build(app);
    /// ```
    ///
    /// This is the equivalent to building [`crate::TestServerConfig`] yourself,
    /// and calling [`crate::TestServer::new_with_config`].
    pub fn build<A>(self, app: A) -> Result<TestServer>
    where
        A: IntoTransportLayer,
    {
        self.into_config().build(app)
    }
}

impl Default for TestServerBuilder {
    fn default() -> Self {
        Self {
            config: TestServerConfig::default(),
        }
    }
}

impl From<TestServerConfig> for TestServerBuilder {
    fn from(config: TestServerConfig) -> Self {
        TestServerBuilder::from_config(config)
    }
}

#[cfg(test)]
mod test_build {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn it_should_build_default_config_by_default() {
        let config = TestServer::builder().into_config();
        let expected = TestServerConfig::default();

        assert_eq!(config, expected);
    }

    #[test]
    fn it_should_save_cookies_when_set() {
        let config = TestServer::builder().save_cookies().into_config();

        assert_eq!(config.save_cookies, true);
    }

    #[test]
    fn it_should_not_save_cookies_when_set() {
        let config = TestServer::builder().do_not_save_cookies().into_config();

        assert_eq!(config.save_cookies, false);
    }

    #[test]
    fn it_should_mock_transport_when_set() {
        let config = TestServer::builder().mock_transport().into_config();

        assert_eq!(config.transport, Some(Transport::MockHttp));
    }

    #[test]
    fn it_should_use_random_http_transport_when_set() {
        let config = TestServer::builder().http_transport().into_config();

        assert_eq!(config.transport, Some(Transport::HttpRandomPort));
    }

    #[test]
    fn it_should_use_http_transport_with_ip_port_when_set() {
        let config = TestServer::builder()
            .http_transport_with_ip_port(Some(IpAddr::V4(Ipv4Addr::new(123, 4, 5, 6))), Some(987))
            .into_config();

        assert_eq!(
            config.transport,
            Some(Transport::HttpIpPort {
                ip: Some(IpAddr::V4(Ipv4Addr::new(123, 4, 5, 6))),
                port: Some(987),
            })
        );
    }

    #[test]
    fn it_should_set_default_content_type_when_set() {
        let config = TestServer::builder()
            .default_content_type("text/csv")
            .into_config();

        assert_eq!(config.default_content_type, Some("text/csv".to_string()));
    }

    #[test]
    fn it_should_set_default_scheme_when_set() {
        let config = TestServer::builder().default_scheme("ftps").into_config();

        assert_eq!(config.default_scheme, Some("ftps".to_string()));
    }

    #[test]
    fn it_should_set_expect_success_by_default_when_set() {
        let config = TestServer::builder()
            .expect_success_by_default()
            .into_config();

        assert_eq!(config.expect_success_by_default, true);
    }

    #[test]
    fn it_should_set_restrict_requests_with_http_schema_when_set() {
        let config = TestServer::builder()
            .restrict_requests_with_http_schema()
            .into_config();

        assert_eq!(config.restrict_requests_with_http_schema, true);
    }
}
