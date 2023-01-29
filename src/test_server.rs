use ::anyhow::Context;
use ::anyhow::Result;
use ::axum::routing::IntoMakeService;
use ::axum::Router;
use ::axum::Server;
use ::hyper::http::Method;
use ::std::net::SocketAddr;
use ::std::net::TcpListener;
use ::tokio::spawn;
use ::tokio::task::JoinHandle;

use crate::util::new_random_socket_addr;
use crate::TestRequest;

/// A means to run Axum applications within a server that you can query.
/// This is for writing tests.
pub struct TestServer {
    server_thread: JoinHandle<()>,
    server_address: String,
}

impl TestServer {
    /// This will take the given app, and run it.
    /// It will be run on a randomly picked port.
    ///
    /// The webserver is then wrapped within a `TestServer`,
    /// and returned.
    pub fn new(app: IntoMakeService<Router>) -> Result<Self> {
        let addr = new_random_socket_addr().context("Cannot create socket address for use")?;
        let test_server = Self::new_with_address(app, addr).context("Cannot create TestServer")?;

        Ok(test_server)
    }

    /// Creates a `TestServer` running your app on the address given.
    pub fn new_with_address(
        app: IntoMakeService<Router>,
        socket_address: SocketAddr,
    ) -> Result<Self> {
        let listener = TcpListener::bind(socket_address)
            .with_context(|| "Failed to create TCPListener for TestServer")?;
        let server_address = socket_address.to_string();
        let server = Server::from_tcp(listener)
            .with_context(|| "Failed to create ::axum::Server for TestServer")?
            .serve(app);

        let server_thread = spawn(async move {
            server.await.expect("Expect server to start serving");
        });

        let test_server = Self {
            server_thread,
            server_address,
        };

        Ok(test_server)
    }

    /// Creates a GET request to the path.
    pub fn get(&self, path: &str) -> TestRequest {
        self.send(Method::GET, path)
    }

    /// Creates a POST request to the given path.
    pub fn post(&self, path: &str) -> TestRequest {
        self.send(Method::POST, path)
    }

    /// Creates a PATCH request to the path.
    pub fn patch(&self, path: &str) -> TestRequest {
        self.send(Method::PATCH, path)
    }

    /// Creates a PUT request to the path.
    pub fn put(&self, path: &str) -> TestRequest {
        self.send(Method::PUT, path)
    }

    /// Creates a DELETE request to the path.
    pub fn delete(&self, path: &str) -> TestRequest {
        self.send(Method::DELETE, path)
    }

    /// Creates a request to the path, using the method you provided.
    pub fn method(&self, method: Method, path: &str) -> TestRequest {
        self.send(method, path)
    }

    fn send(&self, method: Method, path: &str) -> TestRequest {
        let debug_path = path.to_string();
        let request_path = build_request_path(&self.server_address, path);

        TestRequest::new(method, request_path, debug_path)
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
