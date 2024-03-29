use ::std::net::IpAddr;

use crate::TestServerConfig;
use crate::Transport;

/// This is for easing the building of [`TestServerConfig`](crate::TestServerConfig).
///
/// For full documentation see there.
///
/// ```rust
/// use ::axum_test::TestServerConfig;
///
/// let config = TestServerConfig::builder()
///     .save_cookies()
///     .default_content_type(&"application/json")
///     .build();
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
/// let config = TestServerConfig::builder()
///     .save_cookies()
///     .default_content_type(&"application/json")
///     .build();
///
/// // Build the Test Server
/// let server = TestServer::new_with_config(my_app, config)?;
/// #
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct TestServerConfigBuilder {
    config: TestServerConfig,
}

impl TestServerConfigBuilder {
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

    pub fn build(self) -> TestServerConfig {
        self.config
    }
}

impl Default for TestServerConfigBuilder {
    fn default() -> Self {
        Self {
            config: TestServerConfig::default(),
        }
    }
}

#[cfg(test)]
mod test_build {
    use super::*;
    use ::std::net::Ipv4Addr;

    #[test]
    fn it_should_build_default_config_by_default() {
        let config = TestServerConfig::builder().build();
        let expected = TestServerConfig::default();

        assert_eq!(config, expected);
    }

    #[test]
    fn it_should_save_cookies_when_set() {
        let config = TestServerConfig::builder().save_cookies().build();

        assert_eq!(config.save_cookies, true);
    }

    #[test]
    fn it_should_not_save_cookies_when_set() {
        let config = TestServerConfig::builder().do_not_save_cookies().build();

        assert_eq!(config.save_cookies, false);
    }

    #[test]
    fn it_should_mock_transport_when_set() {
        let config = TestServerConfig::builder().mock_transport().build();

        assert_eq!(config.transport, Some(Transport::MockHttp));
    }

    #[test]
    fn it_should_use_random_http_transport_when_set() {
        let config = TestServerConfig::builder().http_transport().build();

        assert_eq!(config.transport, Some(Transport::HttpRandomPort));
    }

    #[test]
    fn it_should_use_http_transport_with_ip_port_when_set() {
        let config = TestServerConfig::builder()
            .http_transport_with_ip_port(Some(IpAddr::V4(Ipv4Addr::new(123, 4, 5, 6))), Some(987))
            .build();

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
        let config = TestServerConfig::builder()
            .default_content_type("text/csv")
            .build();

        assert_eq!(config.default_content_type, Some("text/csv".to_string()));
    }

    #[test]
    fn it_should_set_default_scheme_when_set() {
        let config = TestServerConfig::builder().default_scheme("ftps").build();

        assert_eq!(config.default_scheme, Some("ftps".to_string()));
    }

    #[test]
    fn it_should_set_expect_success_by_default_when_set() {
        let config = TestServerConfig::builder()
            .expect_success_by_default()
            .build();

        assert_eq!(config.expect_success_by_default, true);
    }

    #[test]
    fn it_should_set_restrict_requests_with_http_schema_when_set() {
        let config = TestServerConfig::builder()
            .restrict_requests_with_http_schema()
            .build();

        assert_eq!(config.restrict_requests_with_http_schema, true);
    }
}
