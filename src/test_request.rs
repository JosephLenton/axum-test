use ::anyhow::anyhow;
use ::anyhow::Context;
use ::anyhow::Result;
use ::auto_future::AutoFuture;
use ::cookie::Cookie;
use ::cookie::CookieJar;
use ::http::header::SET_COOKIE;
use ::http::HeaderValue;
use ::http::Request;
use ::hyper::body::to_bytes;
use ::hyper::body::Body;
use ::hyper::body::Bytes;
use ::hyper::header;
use ::hyper::header::HeaderName;
use ::hyper::Client;
use ::serde::Serialize;
use ::serde_json::to_vec as json_to_vec;
use ::serde_urlencoded::to_string;
use ::std::convert::AsRef;
use ::std::fmt::Debug;
use ::std::fmt::Display;
use ::std::future::IntoFuture;
use ::std::sync::Arc;
use ::std::sync::Mutex;
use ::url::Url;

use crate::internals::QueryParamsStore;
use crate::ServerSharedState;
use crate::TestResponse;

mod test_request_config;
pub(crate) use self::test_request_config::*;

const JSON_CONTENT_TYPE: &'static str = &"application/json";
const FORM_CONTENT_TYPE: &'static str = &"application/x-www-form-urlencoded";
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
    query_params: QueryParamsStore,

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
        let query_params = server_locked.query_params().clone();

        ::std::mem::drop(server_locked);

        Ok(Self {
            config,
            server_state,
            body: None,
            headers: vec![],
            cookies,
            query_params,
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

    /// Clears all cookies used internally within this Request,
    /// including any that came from the `TestServer`.
    pub fn clear_cookies(mut self) -> Self {
        self.cookies = CookieJar::new();
        self
    }

    /// Adds a Cookie to be sent with this request.
    pub fn add_cookie<'c>(mut self, cookie: Cookie<'c>) -> Self {
        self.cookies.add(cookie.into_owned());
        self
    }

    /// Adds the structure given as query parameters for this request.
    ///
    /// This is designed to take a list of parameters, or a body of parameters,
    /// and then serializes them into the parameters of the request.
    ///
    /// # Sending a body of parameters using `json!`
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::Router;
    /// use ::axum_test::TestServer;
    /// use ::serde_json::json;
    ///
    /// let app = Router::new().into_make_service();
    /// let server = TestServer::new(app)?;
    ///
    /// let response = server.get(&"/my-end-point")
    ///     .add_query_params(json!({
    ///         "username": "Brian",
    ///         "age": 20
    ///     }))
    ///     .await;
    /// #
    /// # Ok(()) }
    /// ```
    ///
    /// # Sending a body of parameters with Serde
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::Router;
    /// use ::axum_test::TestServer;
    /// use ::serde::Deserialize;
    /// use ::serde::Serialize;
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct UserQueryParams {
    ///     username: String,
    ///     age: u32,
    /// }
    ///
    /// let app = Router::new().into_make_service();
    /// let server = TestServer::new(app)?;
    ///
    /// let response = server.get(&"/my-end-point")
    ///     .add_query_params(UserQueryParams {
    ///         username: "Brian".to_string(),
    ///         age: 20
    ///     })
    ///     .await;
    /// #
    /// # Ok(()) }
    /// ```
    ///
    /// # Sending a list of parameters
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::Router;
    /// use ::axum_test::TestServer;
    ///
    /// let app = Router::new().into_make_service();
    /// let server = TestServer::new(app)?;
    ///
    /// let response = server.get(&"/my-end-point")
    ///     .add_query_params(&[
    ///         ("username", "Brian"),
    ///         ("age", "20"),
    ///     ])
    ///     .await;
    /// #
    /// # Ok(()) }
    /// ```
    ///
    pub fn add_query_params<V>(mut self, query_params: V) -> Self
    where
        V: Serialize,
    {
        self.query_params
            .add(query_params)
            .expect("It should serialize query parameters");
        self
    }

    /// Adds query parameters to be sent with this request.
    pub fn add_query_param<V>(self, key: &str, value: V) -> Self
    where
        V: Serialize,
    {
        self.add_query_params(&[(key, value)])
    }

    /// Clears all query params set,
    /// including any that came from the `TestServer`.
    pub fn clear_query_params(mut self) -> Self {
        self.query_params.clear();
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

    /// Set the body of the request to send up as Json,
    /// and changes the content type to `application/json`.
    pub fn json<J>(self, body: &J) -> Self
    where
        J: ?Sized + Serialize,
    {
        let body_bytes = json_to_vec(body).expect("It should serialize the content into JSON");

        self.bytes(body_bytes.into())
            .content_type(JSON_CONTENT_TYPE)
    }

    /// Sets the body of the request, with the content type
    /// of 'application/x-www-form-urlencoded'.
    pub fn form<F>(self, body: &F) -> Self
    where
        F: ?Sized + Serialize,
    {
        let body_text = to_string(body).expect("It should serialize the content into a Form");

        self.bytes(body_text.into()).content_type(FORM_CONTENT_TYPE)
    }

    /// Set raw text as the body of the request,
    /// and sets the content type to `text/plain`.
    pub fn text<T>(self, raw_text: T) -> Self
    where
        T: Display,
    {
        let body_text = format!("{}", raw_text);

        self.bytes(body_text.into()).content_type(TEXT_CONTENT_TYPE)
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

        let mut url: Url = full_request_path.parse()?;
        // Add all the query params we have
        if self.query_params.has_content() {
            url.set_query(Some(&self.query_params.to_string()));
        }

        let mut request_builder = Request::builder().uri(url.as_str()).method(method);

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
mod test_content_type {
    use crate::TestServer;
    use crate::TestServerConfig;

    use ::axum::routing::get;
    use ::axum::Router;
    use ::http::header::CONTENT_TYPE;
    use ::http::HeaderMap;

    async fn get_content_type(headers: HeaderMap) -> String {
        headers
            .get(CONTENT_TYPE)
            .map(|h| h.to_str().unwrap().to_string())
            .unwrap_or_else(|| "".to_string())
    }

    #[tokio::test]
    async fn it_should_not_set_a_content_type_by_default() {
        // Build an application with a route.
        let app = Router::new()
            .route("/content_type", get(get_content_type))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server.get(&"/content_type").await.text();

        assert_eq!(text, "");
    }

    #[tokio::test]
    async fn it_should_override_server_content_type_when_present() {
        // Build an application with a route.
        let app = Router::new()
            .route("/content_type", get(get_content_type))
            .into_make_service();

        // Run the server.
        let config = TestServerConfig {
            default_content_type: Some("text/plain".to_string()),
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).expect("Should create test server");

        // Get the request.
        let text = server
            .get(&"/content_type")
            .content_type(&"application/json")
            .await
            .text();

        assert_eq!(text, "application/json");
    }

    #[tokio::test]
    async fn it_should_set_content_type_when_present() {
        // Build an application with a route.
        let app = Router::new()
            .route("/content_type", get(get_content_type))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server
            .get(&"/content_type")
            .content_type(&"application/custom")
            .await
            .text();

        assert_eq!(text, "application/custom");
    }
}

#[cfg(test)]
mod test_json {
    use crate::TestServer;

    use ::axum::routing::post;
    use ::axum::Json;
    use ::axum::Router;
    use ::http::header::CONTENT_TYPE;
    use ::http::HeaderMap;
    use ::serde::Deserialize;
    use ::serde::Serialize;
    use ::serde_json::json;

    #[tokio::test]
    async fn it_should_pass_json_up_to_be_read() {
        #[derive(Deserialize, Serialize)]
        struct TestJson {
            name: String,
            age: u32,
            pets: Option<String>,
        }

        async fn get_json(Json(json): Json<TestJson>) -> String {
            format!(
                "json: {}, {}, {}",
                json.name,
                json.age,
                json.pets.unwrap_or_else(|| "pandas".to_string())
            )
        }

        // Build an application with a route.
        let app = Router::new()
            .route("/json", post(get_json))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server
            .post(&"/json")
            .json(&TestJson {
                name: "Joe".to_string(),
                age: 20,
                pets: Some("foxes".to_string()),
            })
            .await
            .text();

        assert_eq!(text, "json: Joe, 20, foxes");
    }

    #[tokio::test]
    async fn it_should_pass_json_content_type_for_json() {
        async fn get_content_type(headers: HeaderMap) -> String {
            headers
                .get(CONTENT_TYPE)
                .map(|h| h.to_str().unwrap().to_string())
                .unwrap_or_else(|| "".to_string())
        }

        // Build an application with a route.
        let app = Router::new()
            .route("/content_type", post(get_content_type))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server.post(&"/content_type").json(&json!({})).await.text();

        assert_eq!(text, "application/json");
    }
}

#[cfg(test)]
mod test_form {
    use crate::TestServer;

    use ::axum::routing::post;
    use ::axum::Form;
    use ::axum::Router;
    use ::http::header::CONTENT_TYPE;
    use ::http::HeaderMap;
    use ::serde::Deserialize;
    use ::serde::Serialize;

    #[tokio::test]
    async fn it_should_pass_form_up_to_be_read() {
        #[derive(Deserialize, Serialize)]
        struct TestForm {
            name: String,
            age: u32,
            pets: Option<String>,
        }

        async fn get_form(Form(form): Form<TestForm>) -> String {
            format!(
                "form: {}, {}, {}",
                form.name,
                form.age,
                form.pets.unwrap_or_else(|| "pandas".to_string())
            )
        }

        // Build an application with a route.
        let app = Router::new()
            .route("/form", post(get_form))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server
            .post(&"/form")
            .form(&TestForm {
                name: "Joe".to_string(),
                age: 20,
                pets: Some("foxes".to_string()),
            })
            .await
            .text();

        assert_eq!(text, "form: Joe, 20, foxes");
    }

    #[tokio::test]
    async fn it_should_pass_form_content_type_for_form() {
        async fn get_content_type(headers: HeaderMap) -> String {
            headers
                .get(CONTENT_TYPE)
                .map(|h| h.to_str().unwrap().to_string())
                .unwrap_or_else(|| "".to_string())
        }

        // Build an application with a route.
        let app = Router::new()
            .route("/content_type", post(get_content_type))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        #[derive(Serialize)]
        struct MyForm {
            message: String,
        }

        // Get the request.
        let text = server
            .post(&"/content_type")
            .form(&MyForm {
                message: "hello".to_string(),
            })
            .await
            .text();

        assert_eq!(text, "application/x-www-form-urlencoded");
    }
}

#[cfg(test)]
mod test_text {
    use crate::TestServer;

    use ::axum::extract::RawBody;
    use ::axum::routing::post;
    use ::axum::Router;
    use ::http::header::CONTENT_TYPE;
    use ::http::HeaderMap;
    use ::hyper::body::to_bytes;

    #[tokio::test]
    async fn it_should_pass_text_up_to_be_read() {
        async fn get_text(RawBody(body): RawBody) -> String {
            let bytes = to_bytes(body).await.expect("Should read body to bytes");
            let body_text = String::from_utf8_lossy(&bytes);

            format!("{}", body_text)
        }

        // Build an application with a route.
        let app = Router::new()
            .route("/text", post(get_text))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server.post(&"/text").text(&"hello!").await.text();

        assert_eq!(text, "hello!");
    }

    #[tokio::test]
    async fn it_should_pass_text_content_type_for_text() {
        async fn get_content_type(headers: HeaderMap) -> String {
            headers
                .get(CONTENT_TYPE)
                .map(|h| h.to_str().unwrap().to_string())
                .unwrap_or_else(|| "".to_string())
        }

        // Build an application with a route.
        let app = Router::new()
            .route("/content_type", post(get_content_type))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server.post(&"/content_type").text(&"hello!").await.text();

        assert_eq!(text, "text/plain");
    }
}

#[cfg(test)]
mod test_expect_success {
    use crate::TestServer;
    use ::axum::routing::get;
    use ::axum::Router;
    use ::http::StatusCode;

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
    use crate::TestServer;
    use ::axum::routing::get;
    use ::axum::Router;
    use ::http::StatusCode;

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
    use ::http::request::Parts;
    use ::http::HeaderName;
    use ::http::HeaderValue;
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
    use ::http::request::Parts;
    use ::http::HeaderName;
    use ::http::HeaderValue;
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

#[cfg(test)]
mod test_add_query_params {
    use ::axum::extract::Query;
    use ::axum::routing::get;
    use ::axum::Router;

    use ::serde::Deserialize;
    use ::serde::Serialize;
    use ::serde_json::json;

    use crate::TestServer;

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParam {
        message: String,
    }

    async fn get_query_param(Query(params): Query<QueryParam>) -> String {
        params.message
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParam2 {
        message: String,
        other: String,
    }

    async fn get_query_param_2(Query(params): Query<QueryParam2>) -> String {
        format!("{}-{}", params.message, params.other)
    }

    #[tokio::test]
    async fn it_should_pass_up_query_params_from_serialization() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query", get(get_query_param))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query")
            .add_query_params(QueryParam {
                message: "it works".to_string(),
            })
            .await
            .assert_text(&"it works");
    }

    #[tokio::test]
    async fn it_should_pass_up_query_params_from_pairs() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query", get(get_query_param))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query")
            .add_query_params(&[("message", "it works")])
            .await
            .assert_text(&"it works");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_multiple_params() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query-2", get(get_query_param_2))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query-2")
            .add_query_params(&[("message", "it works"), ("other", "yup")])
            .await
            .assert_text(&"it works-yup");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_multiple_calls() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query-2", get(get_query_param_2))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query-2")
            .add_query_params(&[("message", "it works")])
            .add_query_params(&[("other", "yup")])
            .await
            .assert_text(&"it works-yup");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_json() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query-2", get(get_query_param_2))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query-2")
            .add_query_params(json!({
                "message": "it works",
                "other": "yup"
            }))
            .await
            .assert_text(&"it works-yup");
    }
}

#[cfg(test)]
mod test_add_query_param {
    use ::axum::extract::Query;
    use ::axum::routing::get;
    use ::axum::Router;

    use ::serde::Deserialize;
    use ::serde::Serialize;

    use crate::TestServer;

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParam {
        message: String,
    }

    async fn get_query_param(Query(params): Query<QueryParam>) -> String {
        params.message
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParam2 {
        message: String,
        other: String,
    }

    async fn get_query_param_2(Query(params): Query<QueryParam2>) -> String {
        format!("{}-{}", params.message, params.other)
    }

    #[tokio::test]
    async fn it_should_pass_up_query_params_from_pairs() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query", get(get_query_param))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query")
            .add_query_param("message", "it works")
            .await
            .assert_text(&"it works");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_multiple_calls() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query-2", get(get_query_param_2))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query-2")
            .add_query_param("message", "it works")
            .add_query_param("other", "yup")
            .await
            .assert_text(&"it works-yup");
    }
}

#[cfg(test)]
mod test_clear_query_params {
    use ::axum::extract::Query;
    use ::axum::routing::get;
    use ::axum::Router;

    use ::serde::Deserialize;
    use ::serde::Serialize;

    use crate::TestServer;

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParams {
        first: Option<String>,
        second: Option<String>,
    }

    async fn get_query_params(Query(params): Query<QueryParams>) -> String {
        format!(
            "has first? {}, has second? {}",
            params.first.is_some(),
            params.second.is_some()
        )
    }

    #[tokio::test]
    async fn it_should_clear_all_params_set() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query", get(get_query_params))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query")
            .add_query_params(QueryParams {
                first: Some("first".to_string()),
                second: Some("second".to_string()),
            })
            .clear_query_params()
            .await
            .assert_text(&"has first? false, has second? false");
    }

    #[tokio::test]
    async fn it_should_clear_all_params_set_and_allow_replacement() {
        // Build an application with a route.
        let app = Router::new()
            .route("/query", get(get_query_params))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query")
            .add_query_params(QueryParams {
                first: Some("first".to_string()),
                second: Some("second".to_string()),
            })
            .clear_query_params()
            .add_query_params(QueryParams {
                first: Some("first".to_string()),
                second: Some("second".to_string()),
            })
            .await
            .assert_text(&"has first? true, has second? true");
    }
}
