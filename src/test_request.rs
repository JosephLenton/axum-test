use ::anyhow::anyhow;
use ::anyhow::Context;
use ::anyhow::Result;
use ::auto_future::AutoFuture;
use ::hyper::body::to_bytes;
use ::hyper::body::Body;
use ::hyper::body::Bytes;
use ::hyper::header;
use ::hyper::http::header::SET_COOKIE;
use ::hyper::http::Method;
use ::hyper::http::Request;
use ::hyper::Client;
use ::serde::Serialize;
use ::serde_json::to_vec as json_to_vec;
use ::std::convert::AsRef;
use ::std::fmt::Debug;
use ::std::fmt::Write;
use ::std::future::IntoFuture;
use ::std::sync::Arc;
use ::std::sync::Mutex;
use axum::http::HeaderValue;
use hyper::header::HeaderName;

use crate::InnerTestServer;
use crate::TestResponse;

/// This contains the response from the server.
///
/// Inside are the contents of the response, the status code, and some
/// debugging information.
///
/// You can get the contents out as it's raw string, or deserialise it.
/// One can also also use the `assert_*` functions to test against the
/// response.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct TestRequest {
    inner_test_server: Arc<Mutex<InnerTestServer>>,

    method: Method,
    full_request_path: String,
    body: Option<Body>,

    /// This is what we use for logging for when we display the path to the user.
    debug_path: String,

    is_expecting_failure: bool,
    is_saving_cookies: bool,

    headers: Vec<(HeaderName, HeaderValue)>,
}

impl TestRequest {
    pub(crate) fn new(
        inner_test_server: Arc<Mutex<InnerTestServer>>,
        method: Method,
        path: &str,
    ) -> Result<Self> {
        let debug_path = path.to_string();

        let server_locked = inner_test_server.as_ref().lock().map_err(|err| {
            anyhow!(
                "Failed to lock InternalTestServer for {} {}, received {:?}",
                method,
                path,
                err
            )
        })?;
        let full_request_path = build_request_path(server_locked.server_address(), path);

        let cookie_header_raw =
            server_locked
                .cookies()
                .iter()
                .fold(String::new(), |mut buffer, cookie| {
                    if buffer.len() > 0 {
                        write!(buffer, "; ").expect(
                            "Writing to internal string for cookie header should always work",
                        );
                    }

                    write!(buffer, "{}", cookie)
                        .expect("Writing to internal string for cookie header should always work");

                    buffer
                });

        ::std::mem::drop(server_locked);

        let mut initial_headers: Vec<(HeaderName, HeaderValue)> = vec![];
        if cookie_header_raw.len() > 0 {
            let header_value = HeaderValue::from_str(&cookie_header_raw)?;
            initial_headers.push((header::COOKIE, header_value));
        }

        Ok(Self {
            inner_test_server,
            method,
            full_request_path,
            body: None,
            debug_path,
            is_expecting_failure: false,
            is_saving_cookies: true,
            headers: initial_headers,
        })
    }

    pub fn do_save_cookies(mut self) -> Self {
        self.is_saving_cookies = true;
        self
    }

    /// By default, when this reuest finishes it will save all received cookies into the `TestServer`
    /// for future requests. This is done to help test environments with things like authentication.
    ///
    /// Call this method to opt out and turn this feature off,
    /// for just _this_ request.
    pub fn do_not_save_cookies(mut self) -> Self {
        self.is_saving_cookies = false;
        self
    }

    /// Marks that this request should expect to fail.
    /// Failiure is deemend as any response that isn't a 200.
    ///
    /// By default, requests are expct to always succeed.
    pub fn expect_fail(mut self) -> Self {
        self.is_expecting_failure = true;
        self
    }

    /// Set the body of the request to send up as Json.
    pub fn json<J>(mut self, body: &J) -> Self
    where
        J: Serialize,
    {
        let body_bytes = json_to_vec(body).expect("It should serialize the content into JSON");
        let body: Body = body_bytes.into();

        self.body = Some(body);

        self
    }

    /// Set the body of the request to send up as raw test.
    pub fn text<S>(self, raw_body: S) -> Self
    where
        S: AsRef<str>,
    {
        let body_bytes = Bytes::copy_from_slice(raw_body.as_ref().as_bytes());
        self.bytes(body_bytes)
    }

    /// Set the body of the request to send up as raw bytes.
    pub fn bytes(mut self, body_bytes: Bytes) -> Self {
        let body: Body = body_bytes.into();

        self.body = Some(body);
        self
    }

    pub fn content_type(mut self, content_type: &str) -> Self {
        self.push_header_content_type(content_type);
        self
    }

    fn push_header_content_type(&mut self, content_type: &str) {
        let header_value = HeaderValue::from_str(content_type)
            .with_context(|| format!("Failed to store header content type '{}'", content_type))
            .unwrap();

        self.headers.push((header::CONTENT_TYPE, header_value));
    }

    async fn send_or_panic(self) -> TestResponse {
        self.send().await.expect("Sending request failed")
    }

    async fn send(mut self) -> Result<TestResponse> {
        let body = self.body.unwrap_or(Body::empty());

        let mut request_builder = Request::builder()
            .uri(&self.full_request_path)
            .method(self.method);

        // Add all the headers we have.
        for (header_name, header_value) in self.headers {
            request_builder = request_builder.header(header_name, header_value);
        }

        let request = request_builder.body(body).with_context(|| {
            format!(
                "Expect valid hyper Request to be built on request to {}",
                self.debug_path
            )
        })?;

        let hyper_response = Client::new().request(request).await.with_context(|| {
            format!(
                "Expect Hyper Response to succeed on request to {}",
                self.debug_path
            )
        })?;

        let (parts, response_body) = hyper_response.into_parts();
        let response_bytes = to_bytes(response_body).await?;

        if self.is_saving_cookies {
            let cookie_headers = parts.headers.get_all(SET_COOKIE).into_iter();
            InnerTestServer::add_cookies_by_header(&mut self.inner_test_server, cookie_headers)?;
        }

        let mut response = TestResponse::new(self.debug_path, parts, response_bytes);

        // Assert if ok or not.
        if self.is_expecting_failure {
            response = response.assert_status_not_ok();
        } else {
            response = response.assert_status_ok();
        }

        Ok(response)
    }
}

impl IntoFuture for TestRequest {
    type Output = TestResponse;
    type IntoFuture = AutoFuture<TestResponse>;

    fn into_future(self) -> Self::IntoFuture {
        let raw_future = self.send_or_panic();
        AutoFuture::new(raw_future)
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
