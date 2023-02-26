use ::anyhow::Context;
use ::anyhow::Result;
use ::axum::routing::IntoMakeService;
use ::axum::Router;
use ::cookie::Cookie;
use ::cookie::CookieJar;
use ::hyper::http::Method;
use ::std::sync::Arc;
use ::std::sync::Mutex;

use crate::TestRequest;
use crate::TestServerConfig;

mod inner_test_server;
pub(crate) use self::inner_test_server::*;

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
    inner: Arc<Mutex<InnerTestServer>>,
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
    pub fn new_with_config(
        app: IntoMakeService<Router>,
        options: TestServerConfig,
    ) -> Result<Self> {
        let inner_test_server = InnerTestServer::new(app, options)?;
        let inner_mutex = Mutex::new(inner_test_server);
        let inner = Arc::new(inner_mutex);

        Ok(Self { inner })
    }

    /// Adds extra cookies to be used on *all* future requests.
    ///
    /// Any cookies which have the same name as the new cookies,
    /// will get replaced.
    pub fn add_cookies(&mut self, cookies: CookieJar) {
        InnerTestServer::add_cookies(&mut self.inner, cookies)
            .with_context(|| format!("Trying to add_cookies"))
            .unwrap()
    }

    /// Adds a cookie to be included on *all* future requests.
    ///
    /// If a cookie with the same name already exists,
    /// then it will be replaced.
    pub fn add_cookie(&mut self, cookie: Cookie) {
        InnerTestServer::add_cookie(&mut self.inner, cookie)
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
        InnerTestServer::send(&self.inner, method, path)
            .with_context(|| {
                format!(
                    "Trying to create internal request for {} {}",
                    debug_method, path
                )
            })
            .unwrap()
    }
}
