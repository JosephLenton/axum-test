use ::anyhow::Context;
use ::anyhow::Result;
use ::auto_future::AutoFuture;
use ::hyper::body::to_bytes;
use ::hyper::body::Body;
use ::hyper::body::Bytes;
use ::hyper::header;
use ::hyper::http::Method;
use ::hyper::http::Request;
use ::hyper::Client;
use ::serde::Serialize;
use ::serde_json::to_vec as json_to_vec;
use ::std::convert::AsRef;
use ::std::fmt::Debug;
use ::std::future::IntoFuture;

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
    method: Method,
    path: String,
    body: Option<Body>,

    /// This is what we use for logging for when we display the path to the user.
    debug_path: String,

    is_expecting_failure: bool,
}

impl TestRequest {
    pub(crate) fn new(method: Method, path: String, debug_path: String) -> Self {
        Self {
            method,
            path,
            body: None,
            debug_path,
            is_expecting_failure: false,
        }
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

    async fn send_or_panic(self) -> TestResponse {
        self.send().await.expect("Sending request failed")
    }

    async fn send(self) -> Result<TestResponse> {
        let body = self.body.unwrap_or(Body::empty());

        let request = Request::builder()
            .uri(&self.path)
            .header(header::CONTENT_TYPE, "application/json")
            .method(self.method)
            .body(body)
            .with_context(|| {
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
