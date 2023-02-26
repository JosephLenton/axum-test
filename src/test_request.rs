use ::anyhow::anyhow;
use ::anyhow::Context;
use ::anyhow::Result;
use ::auto_future::AutoFuture;
use ::hyper::body::to_bytes;
use ::hyper::body::Body;
use ::hyper::body::Bytes;
use ::hyper::header;
use ::hyper::http::header::SET_COOKIE;
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

mod test_request_config;
pub(crate) use self::test_request_config::*;

mod test_request_details;
pub(crate) use self::test_request_details::*;

const JSON_CONTENT_TYPE: &'static str = &"application/json";
const TEXT_CONTENT_TYPE: &'static str = &"text/plain";

///
/// This contains the response from the server.
///
/// A `TestRequest` represents a HTTP request to the test server.
/// It is created by using the `TestServer`. Such as calling `TestServer::get`
/// or `TestServer::post.
///
/// The `TestRequest` allows the caller to modify the request to be sent to the server.
/// Including the headers, body, and other relevant details.
///
/// The TestRequest struct provides a number of methods to set up the request,
/// such as json, text, bytes, expect_fail, content_type, etc.
/// The do_save_cookies and do_not_save_cookies methods are used to control cookie handling.
///
/// Once the request is fully configured, the caller should await this object.
/// That runs the request to the server, and resolves to a `TestResponse`.
///
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct TestRequest {
    details: TestRequestDetails,

    inner_test_server: Arc<Mutex<InnerTestServer>>,

    full_request_path: String,
    body: Option<Body>,
    headers: Vec<(HeaderName, HeaderValue)>,
    content_type: Option<String>,

    is_expecting_failure: bool,
    is_saving_cookies: bool,
}

impl TestRequest {
    pub(crate) fn new(
        inner_test_server: Arc<Mutex<InnerTestServer>>,
        config: TestRequestConfig,
        details: TestRequestDetails,
    ) -> Result<Self> {
        let server_locked = inner_test_server.as_ref().lock().map_err(|err| {
            anyhow!(
                "Failed to lock InternalTestServer for {} {}, received {:?}",
                details.method,
                details.path,
                err
            )
        })?;
        let full_request_path = build_request_path(server_locked.server_address(), &details.path);

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
            details,
            inner_test_server,
            full_request_path,
            body: None,
            headers: initial_headers,
            content_type: config.content_type,
            is_expecting_failure: false,
            is_saving_cookies: config.save_cookies,
        })
    }

    /// Any cookies returned will be saved to the `TestServer` that created this,
    /// which will continue to use those cookies on future requests.
    pub fn do_save_cookies(mut self) -> Self {
        self.is_saving_cookies = true;
        self
    }

    /// Cookies returned by this will _not_ be saved to the `TestServer`.
    /// For use by future requests.
    ///
    /// This is the default behaviour.
    /// You can change that default in `TestServerConfig`.
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

        if self.content_type == None {
            self.content_type = Some(JSON_CONTENT_TYPE.to_string());
        }

        self
    }

    /// Set the body of the request to send up as raw test.
    pub fn text<S>(mut self, raw_body: S) -> Self
    where
        S: AsRef<str>,
    {
        let body_bytes = Bytes::copy_from_slice(raw_body.as_ref().as_bytes());

        if self.content_type == None {
            self.content_type = Some(TEXT_CONTENT_TYPE.to_string());
        }

        self.bytes(body_bytes)
    }

    /// Set the body of the request to send up as raw bytes.
    pub fn bytes(mut self, body_bytes: Bytes) -> Self {
        let body: Body = body_bytes.into();

        self.body = Some(body);
        self
    }

    pub fn content_type(mut self, content_type: &str) -> Self {
        self.content_type = Some(content_type.to_string());
        self
    }

    async fn send_or_panic(self) -> TestResponse {
        self.send().await.expect("Sending request failed")
    }

    async fn send(mut self) -> Result<TestResponse> {
        let path = self.details.path;
        let save_cookies = self.is_saving_cookies;
        let body = self.body.unwrap_or(Body::empty());

        let mut request_builder = Request::builder()
            .uri(&self.full_request_path)
            .method(self.details.method);

        // Add all the headers we have.
        let mut headers = self.headers;
        if let Some(content_type) = self.content_type {
            let header = build_content_type_header(content_type)?;
            headers.push(header);
        }

        for (header_name, header_value) in headers {
            request_builder = request_builder.header(header_name, header_value);
        }

        let request = request_builder.body(body).with_context(|| {
            format!(
                "Expect valid hyper Request to be built on request to {}",
                path
            )
        })?;

        let hyper_response = Client::new()
            .request(request)
            .await
            .with_context(|| format!("Expect Hyper Response to succeed on request to {}", path))?;

        let (parts, response_body) = hyper_response.into_parts();
        let response_bytes = to_bytes(response_body).await?;

        if save_cookies {
            let cookie_headers = parts.headers.get_all(SET_COOKIE).into_iter();
            InnerTestServer::add_cookies_by_header(&mut self.inner_test_server, cookie_headers)?;
        }

        let mut response = TestResponse::new(path, parts, response_bytes);

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

fn build_content_type_header(content_type: String) -> Result<(HeaderName, HeaderValue)> {
    let header_value = HeaderValue::from_str(&content_type)
        .with_context(|| format!("Failed to store header content type '{}'", content_type))?;

    Ok((header::CONTENT_TYPE, header_value))
}
