use ::anyhow::Context;
use ::anyhow::Result;
use ::axum::routing::IntoMakeService;
use ::axum::Router;
use ::axum::Server as AxumServer;
use ::cookie::Cookie;
use ::cookie::CookieJar;
use ::hyper::http::Method;
use ::std::net::TcpListener;
use ::std::sync::Arc;
use ::std::sync::Mutex;
use ::tokio::spawn;
use ::tokio::task::JoinHandle;

use crate::util::new_socket_addr_from_defaults;
use crate::TestRequest;
use crate::TestRequestConfig;
use crate::TestServerConfig;

mod server_shared_state;
pub(crate) use self::server_shared_state::*;

///
/// The `TestServer` represents your application, running as a web server,
/// and you can make web requests to your application.
///
/// For most people's needs, this is where to start when writing a test.
/// This allows you Allowing you to create new requests that will go to this server.
///
/// You can make a request against the `TestServer` by calling the
/// `get`, `post`, `put`, `delete`, and `patch` methods (you can also use `method`).
///
#[derive(Debug)]
pub struct TestServer {
    state: Arc<Mutex<ServerSharedState>>,
    server_thread: JoinHandle<()>,
    server_address: String,
    save_cookies: bool,
    default_content_type: Option<String>,
}

impl TestServer {
    /// This will take the given app, and run it.
    /// It will use a randomly selected port for running.
    ///
    /// This is the same as creating a new `TestServer` with a configuration,
    /// and passing `TestServerConfig::default()`.
    pub fn new(app: IntoMakeService<Router>) -> Result<Self> {
        Self::new_with_config(app, TestServerConfig::default())
    }

    /// This very similar to `TestServer::new()`,
    /// however you can customise some of the configuration.
    /// This includes which port to run on, or default settings.
    ///
    /// See the `TestServerConfig` for more information on each configuration setting.
    pub fn new_with_config(app: IntoMakeService<Router>, config: TestServerConfig) -> Result<Self> {
        let socket_address = new_socket_addr_from_defaults(config.ip, config.port)
            .context("Cannot create socket address for use")?;
        let listener = TcpListener::bind(socket_address)
            .with_context(|| "Failed to create TCPListener for TestServer")?;
        let server = AxumServer::from_tcp(listener)
            .with_context(|| "Failed to create ::axum::Server for TestServer")?
            .serve(app);

        let server_thread = spawn(async move {
            server.await.expect("Expect server to start serving");
        });

        let shared_state = ServerSharedState::new();
        let shared_state_mutex = Mutex::new(shared_state);
        let state = Arc::new(shared_state_mutex);

        let this = Self {
            state,
            server_thread,
            server_address: socket_address.to_string(),
            save_cookies: config.save_cookies,
            default_content_type: config.default_content_type,
        };

        Ok(this)
    }

    /// Returns the address for the test server.
    ///
    /// By default this will be something like `0.0.0.0:1234`,
    /// where `1234` is a randomly assigned port numbr.
    pub fn server_address<'a>(&'a self) -> &'a str {
        &self.server_address
    }

    /// Clears all of the cookies stored internally.
    pub fn clear_cookies(&mut self) {
        ServerSharedState::clear_cookies(&mut self.state)
            .with_context(|| format!("Trying to clear_cookies"))
            .unwrap()
    }

    /// Adds extra cookies to be used on *all* future requests.
    ///
    /// Any cookies which have the same name as the new cookies,
    /// will get replaced.
    pub fn add_cookies(&mut self, cookies: CookieJar) {
        ServerSharedState::add_cookies(&mut self.state, cookies)
            .with_context(|| format!("Trying to add_cookies"))
            .unwrap()
    }

    /// Adds a cookie to be included on *all* future requests.
    ///
    /// If a cookie with the same name already exists,
    /// then it will be replaced.
    pub fn add_cookie(&mut self, cookie: Cookie) {
        ServerSharedState::add_cookie(&mut self.state, cookie)
            .with_context(|| format!("Trying to add_cookie"))
            .unwrap()
    }

    /// Creates a HTTP GET request to the path.
    pub fn get(&self, path: &str) -> TestRequest {
        self.method(Method::GET, path)
    }

    /// Creates a HTTP POST request to the given path.
    pub fn post(&self, path: &str) -> TestRequest {
        self.method(Method::POST, path)
    }

    /// Creates a HTTP PATCH request to the path.
    pub fn patch(&self, path: &str) -> TestRequest {
        self.method(Method::PATCH, path)
    }

    /// Creates a HTTP PUT request to the path.
    pub fn put(&self, path: &str) -> TestRequest {
        self.method(Method::PUT, path)
    }

    /// Creates a HTTP DELETE request to the path.
    pub fn delete(&self, path: &str) -> TestRequest {
        self.method(Method::DELETE, path)
    }

    /// Creates a HTTP request, to the path given, using the given method.
    pub fn method(&self, method: Method, path: &str) -> TestRequest {
        let debug_method = method.clone();
        let config = self.test_request_config(method, path);
        let maybe_request = TestRequest::new(self.state.clone(), config);

        maybe_request
            .with_context(|| {
                format!(
                    "Trying to create internal request for {} {}",
                    debug_method, path
                )
            })
            .unwrap()
    }

    pub(crate) fn test_request_config(&self, method: Method, path: &str) -> TestRequestConfig {
        let full_request_path = build_request_path(&self.server_address, path);

        TestRequestConfig {
            is_saving_cookies: self.save_cookies,
            content_type: self.default_content_type.clone(),
            full_request_path,
            method,
            path: path.to_string(),
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.server_thread.abort();
    }
}

fn build_request_path(root_path: &str, sub_path: &str) -> String {
    if sub_path == "" {
        return format!("http://{}", root_path.to_string());
    }

    if sub_path.starts_with("/") {
        return format!("http://{}{}", root_path, sub_path);
    }

    format!("http://{}/{}", root_path, sub_path)
}

#[cfg(test)]
mod server_address {
    use super::*;
    use ::axum::Router;
    use ::local_ip_address::local_ip;
    use ::regex::Regex;

    #[tokio::test]
    async fn it_should_return_address_used_from_config() {
        let ip = local_ip().unwrap();
        let config = TestServerConfig {
            ip: Some(ip),
            port: Some(3000),
            ..TestServerConfig::default()
        };

        // Build an application with a route.
        let app = Router::new().into_make_service();
        let server = TestServer::new_with_config(app, config).expect("Should create test server");

        let expected_ip_port = format!("{}:3000", ip);
        assert_eq!(server.server_address(), expected_ip_port);
    }

    #[tokio::test]
    async fn it_should_return_default_address_without_ending_slash() {
        let app = Router::new().into_make_service();
        let server = TestServer::new(app).expect("Should create test server");

        let address_regex = Regex::new("^127\\.0\\.0\\.1:[0-9]+$").unwrap();
        let is_match = address_regex.is_match(&server.server_address());
        assert!(is_match);
    }
}
