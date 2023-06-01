use ::anyhow::anyhow;
use ::anyhow::Context;
use ::anyhow::Result;
use ::auto_future::AutoFuture;
use ::axum::http::HeaderValue;
use ::cookie::Cookie;
use ::cookie::CookieJar;
use ::hyper::body::to_bytes;
use ::hyper::body::Body;
use ::hyper::body::Bytes;
use ::hyper::header;
use ::hyper::header::HeaderName;
use ::hyper::http::header::SET_COOKIE;
use ::hyper::http::Request;
use ::hyper::Client;
use ::serde::Serialize;
use ::serde_json::to_vec as json_to_vec;
use ::std::convert::AsRef;
use ::std::fmt::Debug;
use ::std::fmt::Display;
use ::std::future::IntoFuture;
use ::std::sync::Arc;
use ::std::sync::Mutex;

use crate::ServerSharedState;
use crate::TestResponse;

mod test_request_config;
pub(crate) use self::test_request_config::*;

const JSON_CONTENT_TYPE: &'static str = &"application/json";
const TEXT_CONTENT_TYPE: &'static str = &"text/plain";

///
/// A `TestRequest` represents a HTTP request to the test server.
///
/// ## Creating
///
/// Requests are created by the `TestServer`. You do not create them yourself.
///
/// The `TestServer` has functions corresponding to specific requests.
/// For example calling `TestServer::get` to create a new HTTP GET request,
/// or `TestServer::post to create a HTTP POST request.
///
/// ## Customising
///
/// The `TestRequest` allows the caller to fill in the rest of the request
/// to be sent to the server. Including the headers, the body, cookies, the content type,
/// and other relevant details.
///
/// The TestRequest struct provides a number of methods to set up the request,
/// such as json, text, bytes, expect_failure, content_type, etc.
/// The do_save_cookies and do_not_save_cookies methods are used to control cookie handling.
///
/// ## Sending
///
/// Once fully configured you send the rquest by awaiting the request object.
///
/// ```rust,ignore
/// let request = server.get(&"/user");
/// let response = request.await;
/// ```
///
/// You will receive back a `TestResponse`.
///
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct TestRequest {
    config: TestRequestConfig,

    server_state: Arc<Mutex<ServerSharedState>>,

    body: Option<Body>,
    headers: Vec<(HeaderName, HeaderValue)>,
    cookies: CookieJar,

    is_expecting_failure: bool,
}

impl TestRequest {
    pub(crate) fn new(
        server_state: Arc<Mutex<ServerSharedState>>,
        config: TestRequestConfig,
    ) -> Result<Self> {
        let server_locked = server_state.as_ref().lock().map_err(|err| {
            anyhow!(
                "Failed to lock InternalTestServer for {} {}, received {:?}",
                config.method,
                config.path,
                err
            )
        })?;

        let cookies = server_locked.cookies().clone();

        ::std::mem::drop(server_locked);

        Ok(Self {
            config,
            server_state,
            body: None,
            headers: vec![],
            cookies,
            is_expecting_failure: false,
        })
    }

    /// Any cookies returned will be saved to the `TestServer` that created this,
    /// which will continue to use those cookies on future requests.
    pub fn do_save_cookies(mut self) -> Self {
        self.config.is_saving_cookies = true;
        self
    }

    /// Cookies returned by this will _not_ be saved to the `TestServer`.
    /// For use by future requests.
    ///
    /// This is the default behaviour.
    /// You can change that default in `TestServerConfig`.
    pub fn do_not_save_cookies(mut self) -> Self {
        self.config.is_saving_cookies = false;
        self
    }

    /// Clears all cookies used internally within this Request.
    pub fn clear_cookies(mut self) -> Self {
        self.cookies = CookieJar::new();
        self
    }

    /// Adds a Cookie to be sent with this request.
    pub fn add_cookie<'c>(mut self, cookie: Cookie<'c>) -> Self {
        self.cookies.add(cookie.into_owned());
        self
    }

    /// Clears all headers set.
    pub fn clear_headers(mut self) -> Self {
        self.headers = vec![];
        self
    }

    /// Adds a header to be sent with this request.
    pub fn add_header<'c>(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.push((name, value));
        self
    }

    /// Marks that this request should expect to fail.
    /// Failiure is deemend as any response that isn't a 200.
    ///
    /// By default, requests are expct to always succeed.
    pub fn expect_failure(mut self) -> Self {
        self.is_expecting_failure = true;
        self
    }

    /// Marks that this request should expect to succeed.
    /// Success is deemend as returning a 2xx status code.
    ///
    /// Note this is the default behaviour when creating a new `TestRequest`.
    pub fn expect_success(mut self) -> Self {
        self.is_expecting_failure = false;
        self
    }

    /// Set the body of the request to send up as Json.
    pub fn json<J>(mut self, body: &J) -> Self
    where
        J: ?Sized + Serialize,
    {
        let body_bytes = json_to_vec(body).expect("It should serialize the content into JSON");
        let body: Body = body_bytes.into();
        self.body = Some(body);

        if self.config.content_type == None {
            self.config.content_type = Some(JSON_CONTENT_TYPE.to_string());
        }

        self
    }

    /// Set raw text as the body of the request.
    ///
    /// If there isn't a content type set, this will default to `text/plain`.
    pub fn text<T>(mut self, raw_text: T) -> Self
    where
        T: Display,
    {
        let body_text = format!("{}", raw_text);
        let body_bytes = Bytes::from(body_text.into_bytes());

        if self.config.content_type == None {
            self.config.content_type = Some(TEXT_CONTENT_TYPE.to_string());
        }

        self.bytes(body_bytes)
    }

    /// Set raw bytes as the body of the request.
    ///
    /// The content type is left unchanged.
    pub fn bytes(mut self, body_bytes: Bytes) -> Self {
        let body: Body = body_bytes.into();

        self.body = Some(body);
        self
    }

    /// Set the content type to use for this request in the header.
    pub fn content_type(mut self, content_type: &str) -> Self {
        self.config.content_type = Some(content_type.to_string());
        self
    }

    async fn send_or_panic(self) -> TestResponse {
        self.send().await.expect("Sending request failed")
    }

    async fn send(mut self) -> Result<TestResponse> {
        let full_request_path = self.config.full_request_path;
        let method = self.config.method;
        let path = self.config.path;
        let save_cookies = self.config.is_saving_cookies;
        let body = self.body.unwrap_or(Body::empty());

        let mut request_builder = Request::builder().uri(&full_request_path).method(method);

        // Add all the headers we have.
        let mut headers = self.headers;
        if let Some(content_type) = self.config.content_type {
            let header = build_content_type_header(content_type)?;
            headers.push(header);
        }

        // Add all the cookies as headers
        for cookie in self.cookies.iter() {
            let cookie_raw = cookie.to_string();
            let header_value = HeaderValue::from_str(&cookie_raw)?;
            headers.push((header::COOKIE, header_value));
        }

        // Put headers into the request
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
            ServerSharedState::add_cookies_by_header(&mut self.server_state, cookie_headers)?;
        }

        let response = TestResponse::new(path, parts, response_bytes);

        // Assert if ok or not.
        if self.is_expecting_failure {
            response.assert_status_failure();
        } else {
            response.assert_status_success();
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

fn build_content_type_header(content_type: String) -> Result<(HeaderName, HeaderValue)> {
    let header_value = HeaderValue::from_str(&content_type)
        .with_context(|| format!("Failed to store header content type '{}'", content_type))?;

    Ok((header::CONTENT_TYPE, header_value))
}

#[cfg(test)]
mod test_expect_success {
    use super::*;

    use crate::TestServer;
    use ::axum::http::StatusCode;
    use ::axum::routing::get;
    use ::axum::Router;

    #[tokio::test]
    async fn it_should_not_panic_if_success_is_returned() {
        async fn get_ping() -> &'static str {
            "pong!"
        }

        // Build an application with a route.
        let app = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/ping").expect_success().await;
    }

    #[tokio::test]
    async fn it_should_not_panic_on_other_2xx_status_code() {
        async fn get_accepted() -> StatusCode {
            StatusCode::ACCEPTED
        }

        // Build an application with a route.
        let app = Router::new()
            .route("/accepted", get(get_accepted))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/accepted").expect_success().await;
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_on_404() {
        // Build an application with a route.
        let app = Router::new().into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/some_unknown_route").expect_success().await;
    }
}

#[cfg(test)]
mod test_expect_failure {
    use super::*;

    use crate::TestServer;
    use ::axum::http::StatusCode;
    use ::axum::routing::get;
    use ::axum::Router;

    #[tokio::test]
    async fn it_should_not_panic_if_expect_failure_on_404() {
        // Build an application with a route.
        let app = Router::new().into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/some_unknown_route").expect_failure().await;
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_success_is_returned() {
        async fn get_ping() -> &'static str {
            "pong!"
        }

        // Build an application with a route.
        let app = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/ping").expect_failure().await;
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_on_other_2xx_status_code() {
        async fn get_accepted() -> StatusCode {
            StatusCode::ACCEPTED
        }

        // Build an application with a route.
        let app = Router::new()
            .route("/accepted", get(get_accepted))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/accepted").expect_failure().await;
    }
}

#[cfg(test)]
mod test_add_header {
    use super::*;

    use ::axum::async_trait;
    use ::axum::extract::FromRequestParts;
    use ::axum::routing::get;
    use ::axum::Router;
    use ::hyper::http::request::Parts;
    use ::hyper::http::HeaderName;
    use ::hyper::http::HeaderValue;
    use ::hyper::StatusCode;
    use ::std::marker::Sync;

    use crate::TestServer;

    const TEST_HEADER_NAME: &'static str = &"test-header";
    const TEST_HEADER_CONTENT: &'static str = &"Test header content";

    struct TestHeader(Vec<u8>);

    #[async_trait]
    impl<S: Sync> FromRequestParts<S> for TestHeader {
        type Rejection = (StatusCode, &'static str);

        async fn from_request_parts(
            parts: &mut Parts,
            _state: &S,
        ) -> Result<TestHeader, Self::Rejection> {
            parts
                .headers
                .get(HeaderName::from_static(TEST_HEADER_NAME))
                .map(|v| TestHeader(v.as_bytes().to_vec()))
                .ok_or((StatusCode::BAD_REQUEST, "Missing test header"))
        }
    }

    async fn ping_header(TestHeader(header): TestHeader) -> Vec<u8> {
        header
    }

    #[tokio::test]
    async fn it_should_send_the_header() {
        // Build an application with a route.
        let app = Router::new()
            .route("/header", get(ping_header))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Send a request with the header
        let response = server
            .get(&"/header")
            .add_header(
                HeaderName::from_static(TEST_HEADER_NAME),
                HeaderValue::from_static(TEST_HEADER_CONTENT),
            )
            .await;

        // Check it sent back the right text
        response.assert_text(TEST_HEADER_CONTENT)
    }
}

#[cfg(test)]
mod test_clear_headers {
    use super::*;

    use ::axum::async_trait;
    use ::axum::extract::FromRequestParts;
    use ::axum::routing::get;
    use ::axum::Router;
    use ::hyper::http::request::Parts;
    use ::hyper::http::HeaderName;
    use ::hyper::http::HeaderValue;
    use ::hyper::StatusCode;
    use ::std::marker::Sync;

    use crate::TestServer;

    const TEST_HEADER_NAME: &'static str = &"test-header";
    const TEST_HEADER_CONTENT: &'static str = &"Test header content";

    struct TestHeader(Vec<u8>);

    #[async_trait]
    impl<S: Sync> FromRequestParts<S> for TestHeader {
        type Rejection = (StatusCode, &'static str);

        async fn from_request_parts(
            parts: &mut Parts,
            _state: &S,
        ) -> Result<TestHeader, Self::Rejection> {
            parts
                .headers
                .get(HeaderName::from_static(TEST_HEADER_NAME))
                .map(|v| TestHeader(v.as_bytes().to_vec()))
                .ok_or((StatusCode::BAD_REQUEST, "Missing test header"))
        }
    }

    async fn ping_header(TestHeader(header): TestHeader) -> Vec<u8> {
        header
    }

    #[tokio::test]
    async fn it_should_send_the_header() {
        // Build an application with a route.
        let app = Router::new()
            .route("/header", get(ping_header))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Send a request with the header
        let response = server
            .get(&"/header")
            .add_header(
                HeaderName::from_static(TEST_HEADER_NAME),
                HeaderValue::from_static(TEST_HEADER_CONTENT),
            )
            .clear_headers()
            .expect_failure()
            .await;

        // Check it sent back the right text
        response.assert_status_bad_request();
        response.assert_text("Missing test header");
    }
}
