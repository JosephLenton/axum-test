use ::anyhow::Context;
use ::anyhow::Result;
use ::axum::body::Body;
use ::axum::http::Method;
use ::axum::http::Request;
use ::axum::routing::IntoMakeService;
use ::axum::Router;
use ::axum::Server;
use ::hyper::body::to_bytes;
use ::hyper::body::Bytes;
use ::hyper::header;
use ::hyper::Client;
use ::std::net::SocketAddr;
use ::std::net::TcpListener;
use ::tokio::spawn;
use ::tokio::task::JoinHandle;

use crate::util::new_random_socket_addr;
use crate::TestResponse;

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
    ///
    /// This function is for quick use. It will panic if it cannot
    /// create the webserver.
    pub fn new(app: IntoMakeService<Router>) -> Self {
        let addr = new_random_socket_addr().expect("Cannot create socket address for use");
        let test_server = Self::new_with_address(app, addr).expect("Cannot create TestServer");

        test_server
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

    /// Performs a GET request to the path.
    ///
    /// This will presume the response is successful (a 200 status code).
    /// If a different status code is returned, then this will panic.
    pub async fn get(&self, path: &str) -> TestResponse {
        self.send(Method::GET, path, &"")
            .await
            .with_context(|| format!("Error calling get on path {}", path))
            .unwrap()
            .assert_status_ok()
    }

    /// Performs a GET request to the path.
    ///
    /// This will panic if the response is successful.
    /// It presumes the response would have failed.
    pub async fn get_fail(&self, path: &str) -> TestResponse {
        self.send(Method::GET, path, &"")
            .await
            .with_context(|| format!("Error calling get_fail on path {}", path))
            .unwrap()
            .assert_status_not_ok()
    }

    /// Performs a POST request to the path.
    ///
    /// This will presume the response is successful (a 200 status code).
    /// If a different status code is returned, then this will panic.
    pub async fn post(&self, path: &str, body: &str) -> TestResponse {
        self.send(Method::POST, path, body)
            .await
            .with_context(|| format!("Error calling post on path {}", path))
            .unwrap()
            .assert_status_ok()
    }

    /// Performs a POST request to the path.
    ///
    /// This will panic if the response is successful.
    /// It presumes the response would have failed.
    pub async fn post_fail(&self, path: &str, body: &str) -> TestResponse {
        self.send(Method::POST, path, body)
            .await
            .with_context(|| format!("Error calling post_fail on path {}", path))
            .unwrap()
            .assert_status_not_ok()
    }

    /// Performs a PATCH request to the path.
    ///
    /// This will panic if the response is successful.
    /// It presumes the response would have failed.
    pub async fn patch(&self, path: &str, body: &str) -> TestResponse {
        self.send(Method::PATCH, path, body)
            .await
            .with_context(|| format!("Error calling patch on path {}", path))
            .unwrap()
            .assert_status_ok()
    }

    /// Performs a PATCH request to the path.
    ///
    /// This will panic if the response is successful.
    /// It presumes the response would have failed.
    pub async fn patch_fail(&self, path: &str, body: &str) -> TestResponse {
        self.send(Method::PATCH, path, body)
            .await
            .with_context(|| format!("Error calling patch_fail on path {}", path))
            .unwrap()
            .assert_status_not_ok()
    }

    /// Performs a PUT request to the path.
    ///
    /// This will presume the response is successful (a 200 status code).
    /// If a different status code is returned, then this will panic.
    pub async fn put(&self, path: &str, body: &str) -> TestResponse {
        self.send(Method::PUT, path, body)
            .await
            .with_context(|| format!("Error calling put on path {}", path))
            .unwrap()
            .assert_status_ok()
    }

    /// Performs a PUT request to the path.
    ///
    /// This will panic if the response is successful.
    /// It presumes the response would have failed.
    pub async fn put_fail(&self, path: &str, body: &str) -> TestResponse {
        self.send(Method::PUT, path, body)
            .await
            .with_context(|| format!("Error calling put_fail on path {}", path))
            .unwrap()
            .assert_status_not_ok()
    }

    /// Performs a DELETE request to the path.
    ///
    /// This will presume the response is successful (a 200 status code).
    /// If a different status code is returned, then this will panic.
    pub async fn delete(&self, path: &str) -> TestResponse {
        self.send(Method::DELETE, path, &"")
            .await
            .with_context(|| format!("Error calling delete_fail on path {}", path))
            .unwrap()
            .assert_status_ok()
    }

    /// Performs a DELETE request to the path.
    ///
    /// This will panic if the response is successful.
    /// It presumes the response would have failed.
    pub async fn delete_fail(&self, path: &str) -> TestResponse {
        self.send(Method::DELETE, path, &"")
            .await
            .with_context(|| format!("Error calling delete_fail on path {}", path))
            .unwrap()
            .assert_status_not_ok()
    }

    async fn send(&self, method: Method, path: &str, body_str: &str) -> Result<TestResponse> {
        let request_url = path.to_string();
        let request_path = build_request_path(&self.server_address, path);
        let client = Client::new();
        let body_bytes = Bytes::copy_from_slice(body_str.as_bytes());
        let body: Body = body_bytes.into();

        let hyper_response = client
            .request(
                Request::builder()
                    .uri(request_path)
                    .header(header::CONTENT_TYPE, "application/json")
                    .method(method)
                    .body(body)
                    .expect("expect Request built to be valid"),
            )
            .await
            .expect("Expect TestResponse to come back");

        let (parts, response_body) = hyper_response.into_parts();
        let response_bytes = to_bytes(response_body).await?;
        let contents = String::from_utf8_lossy(&response_bytes).to_string();
        let status_code = parts.status;
        let response = TestResponse::new(request_url, contents, status_code);

        Ok(response)
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
