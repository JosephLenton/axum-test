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
/// A means to run Axum applications within a server that you can query.
/// This is for writing tests.
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
    /// It will be run on a randomly picked port.
    ///
    /// The webserver is then wrapped within a `TestServer`,
    /// and returned.
    pub fn new(app: IntoMakeService<Router>) -> Result<Self> {
        Self::new_with_config(app, TestServerConfig::default())
    }

    /// Creates a `TestServer` running your app on the address given.
    pub fn new_with_config(
        app: IntoMakeService<Router>,
        options: TestServerConfig,
    ) -> Result<Self> {
        let inner_test_server = InnerTestServer::new(app, options)?;
        let inner_mutex = Mutex::new(inner_test_server);
        let inner = Arc::new(inner_mutex);

        Ok(Self { inner })
    }

    /// Adds the given cookies.
    /// They will be included on all future requests.
    ///
    /// They will be stored over the top of the existing cookies.
    pub fn add_cookies(&mut self, cookies: CookieJar) {
        InnerTestServer::add_cookies(&mut self.inner, cookies)
            .with_context(|| format!("Trying to add_cookies"))
            .unwrap()
    }

    /// Adds the given cookie.
    /// It will be included on all future requests.
    ///
    /// It will be stored over the top of the existing cookies.
    pub fn add_cookie(&mut self, cookie: Cookie) {
        InnerTestServer::add_cookie(&mut self.inner, cookie)
            .with_context(|| format!("Trying to add_cookie"))
            .unwrap()
    }

    /// Creates a GET request to the path.
    pub fn get(&self, path: &str) -> TestRequest {
        self.method(Method::GET, path)
    }

    /// Creates a POST request to the given path.
    pub fn post(&self, path: &str) -> TestRequest {
        self.method(Method::POST, path)
    }

    /// Creates a PATCH request to the path.
    pub fn patch(&self, path: &str) -> TestRequest {
        self.method(Method::PATCH, path)
    }

    /// Creates a PUT request to the path.
    pub fn put(&self, path: &str) -> TestRequest {
        self.method(Method::PUT, path)
    }

    /// Creates a DELETE request to the path.
    pub fn delete(&self, path: &str) -> TestRequest {
        self.method(Method::DELETE, path)
    }

    /// Creates a request to the path, using the method you provided.
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
