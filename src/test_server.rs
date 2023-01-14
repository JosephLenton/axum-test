use ::anyhow::Context;
use ::anyhow::Result;
use ::axum::body::Body;
use ::axum::http::Method;
use ::axum::http::Request;
use ::axum::http::StatusCode;
use ::axum::routing::IntoMakeService;
use ::axum::Router;
use ::axum::Server;
use ::hyper::body::to_bytes;
use ::hyper::body::Bytes;
use ::hyper::header;
use ::hyper::Client;
use ::serde::Deserialize;
use ::std::net::SocketAddr;
use ::std::net::TcpListener;
use ::tokio::spawn;
use ::tokio::task::JoinHandle;

use crate::new_random_socket_addr;

pub struct TestServer {
    server_thread: JoinHandle<()>,
    server_address: String,
}

pub struct TestResponse<T>
where
    for<'de> T: Deserialize<'de>,
{
    pub contents: T,
    pub status_code: StatusCode,
}

impl TestServer {
    pub fn new_with_random_address(app: IntoMakeService<Router>) -> Result<Self> {
        let addr = new_random_socket_addr()?;
        let test_server = Self::new(app, addr)?;

        Ok(test_server)
    }

    pub fn new(app: IntoMakeService<Router>, socket_address: SocketAddr) -> Result<Self> {
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

    pub async fn get(&self, path: &str) -> Result<TestResponse<String>> {
        self.send(Method::GET, path, &"", true).await
    }

    pub async fn get_as<T>(&self, path: &str) -> Result<TestResponse<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.send_as(Method::GET, path, &"", true).await
    }

    pub async fn get_fail(&self, path: &str) -> Result<TestResponse<String>> {
        self.send(Method::GET, path, &"", false).await
    }

    pub async fn get_fail_as<T>(&self, path: &str) -> Result<TestResponse<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.send_as(Method::GET, path, &"", false).await
    }

    pub async fn post(&self, path: &str, body: &str) -> Result<TestResponse<String>> {
        self.send(Method::POST, path, body, true).await
    }

    pub async fn post_as<T>(&self, path: &str, body: &str) -> Result<TestResponse<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.send_as(Method::POST, path, body, true).await
    }

    pub async fn post_fail(&self, path: &str, body: &str) -> Result<TestResponse<String>> {
        self.send(Method::POST, path, body, false).await
    }

    pub async fn post_fail_as<T>(&self, path: &str, body: &str) -> Result<TestResponse<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.send_as(Method::POST, path, body, false).await
    }

    pub async fn patch(&self, path: &str, body: &str) -> Result<TestResponse<String>> {
        self.send(Method::PATCH, path, body, true).await
    }

    pub async fn patch_as<T>(&self, path: &str, body: &str) -> Result<TestResponse<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.send_as(Method::PATCH, path, body, true).await
    }

    pub async fn patch_fail(&self, path: &str, body: &str) -> Result<TestResponse<String>> {
        self.send(Method::PATCH, path, body, false).await
    }

    pub async fn patch_fail_as<T>(&self, path: &str, body: &str) -> Result<TestResponse<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.send_as(Method::PATCH, path, body, false).await
    }

    pub async fn put(&self, path: &str, body: &str) -> Result<TestResponse<String>> {
        self.send(Method::PUT, path, body, true).await
    }

    pub async fn put_as<T>(&self, path: &str, body: &str) -> Result<TestResponse<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.send_as(Method::PUT, path, body, true).await
    }

    pub async fn put_fail(&self, path: &str, body: &str) -> Result<TestResponse<String>> {
        self.send(Method::PUT, path, body, false).await
    }

    pub async fn put_fail_as<T>(&self, path: &str, body: &str) -> Result<TestResponse<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.send_as(Method::PUT, path, body, false).await
    }

    pub async fn delete(&self, path: &str) -> Result<TestResponse<String>> {
        self.send(Method::DELETE, path, &"", true).await
    }

    pub async fn delete_as<T>(&self, path: &str) -> Result<TestResponse<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.send_as(Method::DELETE, path, &"", true).await
    }

    pub async fn delete_fail(&self, path: &str) -> Result<TestResponse<String>> {
        self.send(Method::DELETE, path, &"", false).await
    }

    pub async fn delete_fail_as<T>(&self, path: &str) -> Result<TestResponse<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.send_as(Method::DELETE, path, &"", false).await
    }

    async fn send_as<T>(
        &self,
        method: Method,
        path: &str,
        body_str: &str,
        expect_ok_return: bool,
    ) -> Result<TestResponse<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.send(method, path, body_str, expect_ok_return)
            .await
            .and_then(|response| {
                Ok(TestResponse {
                    contents: serde_json::from_str::<T>(&response.contents)?,
                    status_code: response.status_code,
                })
            })
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        body_str: &str,
        expect_ok_return: bool,
    ) -> Result<TestResponse<String>> {
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

        println!("request ... {:?}", hyper_response);
        let (parts, response_body) = hyper_response.into_parts();
        let response_bytes = to_bytes(response_body).await?;
        let contents = String::from_utf8_lossy(&response_bytes).to_string();
        let status_code = parts.status;

        if expect_ok_return && status_code != StatusCode::OK {
            eprintln!("{}", contents);
            panic!(
                "Request {} returned failure {}, expected success. Contents ... {}",
                path, status_code, contents
            );
        }

        if !expect_ok_return && status_code == StatusCode::OK {
            eprintln!("{}", contents);
            panic!(
                "Request {} returned success {}, expected failure. Contents ... {}",
                path, status_code, contents
            );
        }

        let response = TestResponse {
            contents,
            status_code,
        };

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
