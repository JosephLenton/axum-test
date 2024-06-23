use ::anyhow::anyhow;
use ::anyhow::Context;
use ::anyhow::Error as AnyhowError;
use ::anyhow::Result;
#[cfg(feature = "graphql")]
use ::async_graphql::Request as GraphQlRequest;
use ::auto_future::AutoFuture;
use ::axum::body::Body;
use ::bytes::Bytes;
use ::cookie::time::OffsetDateTime;
use ::cookie::Cookie;
use ::cookie::CookieJar;
use ::http::header;
use ::http::header::SET_COOKIE;
use ::http::HeaderName;
use ::http::HeaderValue;
use ::http::Method;
use ::http::Request;
use ::http_body_util::BodyExt;
use ::serde::Serialize;
use ::std::fmt::Debug;
use ::std::fmt::Display;
use ::std::future::IntoFuture;
use ::std::sync::Arc;
use ::std::sync::Mutex;
use ::url::Url;

use crate::internals::ExpectedState;
use crate::internals::QueryParamsStore;
use crate::internals::RequestPathFormatter;
use crate::multipart::MultipartForm;
use crate::transport_layer::TransportLayer;
use crate::ServerSharedState;
use crate::TestResponse;

pub(crate) use self::test_request_config::*;
mod test_request_config;

///
/// A `TestRequest` is for building and executing a HTTP request to the [`TestServer`](crate::TestServer).
///
/// ## Building
///
/// Requests are created by the [`TestServer`](crate::TestServer), using it's builder functions.
/// They correspond to the appropriate HTTP method: [`TestServer::get()`](crate::TestServer::get()),
/// [`TestServer::post()`](crate::TestServer::post()), etc.
///
/// See there for documentation.
///
/// ## Customising
///
/// The `TestRequest` allows the caller to fill in the rest of the request
/// to be sent to the server. Including the headers, the body, cookies,
/// and the content type, using the relevant functions.
///
/// The TestRequest struct provides a number of methods to set up the request,
/// such as json, text, bytes, expect_failure, content_type, etc.
///
/// ## Sending
///
/// Once fully configured you send the request by awaiting the request object.
///
/// ```rust,ignore
/// let request = server.get(&"/user");
/// let response = request.await;
/// ```
///
/// You will receive a `TestResponse`.
///
/// ## Cookie Saving
///
/// [`TestRequest::do_save_cookies()`](crate::TestRequest::do_save_cookies()) and [`TestRequest::do_not_save_cookies()`](crate::TestRequest::do_not_save_cookies())
/// methods allow you to set the request to save cookies to the `TestServer`,
/// for reuse on any future requests.
///
/// This behaviour is **off** by default, and can be changed for all `TestRequests`
/// when building the `TestServer`. By building it with a `TestServerConfig` where `save_cookies` is set to true.
///
/// ## Expecting Failure and Success
///
/// When making a request you can mark it to expect a response within,
/// or outside, of the 2xx range of HTTP status codes.
///
/// If the response returns a status code different to what is expected,
/// then it will panic.
///
/// This is useful when making multiple requests within a test.
/// As it can find issues earlier than later.
///
/// See the [`TestRequest::expect_failure()`](crate::TestRequest::expect_failure()),
/// and [`TestRequest::expect_success()`](crate::TestRequest::expect_success()).
///
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct TestRequest {
    config: TestRequestConfig,

    server_state: Arc<Mutex<ServerSharedState>>,
    transport: Arc<Box<dyn TransportLayer>>,

    body: Option<Body>,

    expected_state: ExpectedState,
}

impl TestRequest {
    pub(crate) fn new(
        server_state: Arc<Mutex<ServerSharedState>>,
        transport: Arc<Box<dyn TransportLayer>>,
        config: TestRequestConfig,
    ) -> Self {
        let expected_state = config.expected_state;

        Self {
            config,
            server_state,
            transport,
            body: None,
            expected_state,
        }
    }

    /// Set the body of the request to send up data as Json,
    /// and changes the content type to `application/json`.
    pub fn json<J>(self, body: &J) -> Self
    where
        J: ?Sized + Serialize,
    {
        let body_bytes =
            ::serde_json::to_vec(body).expect("It should serialize the content into Json");

        self.bytes(body_bytes.into())
            .content_type(mime::APPLICATION_JSON.essence_str())
    }

    /// Set a GraphQL Request as the body,
    /// and changes the content type to `application/json`.
    #[cfg(feature = "graphql")]
    pub fn graphql<Y>(self, body: &GraphQlRequest) -> Self {
        self.json(body)
    }

    /// Set the body of the request to send up data as Yaml,
    /// and changes the content type to `application/yaml`.
    #[cfg(feature = "yaml")]
    pub fn yaml<Y>(self, body: &Y) -> Self
    where
        Y: ?Sized + Serialize,
    {
        let body =
            ::serde_yaml::to_string(body).expect("It should serialize the content into Yaml");

        self.bytes(body.into_bytes().into())
            .content_type("application/yaml")
    }

    /// Set the body of the request to send up data as MsgPack,
    /// and changes the content type to `application/msgpack`.
    #[cfg(feature = "msgpack")]
    pub fn msgpack<M>(self, body: &M) -> Self
    where
        M: ?Sized + Serialize,
    {
        let body_bytes =
            ::rmp_serde::to_vec(body).expect("It should serialize the content into MsgPack");

        self.bytes(body_bytes.into())
            .content_type("application/msgpack")
    }

    /// Sets the body of the request, with the content type
    /// of 'application/x-www-form-urlencoded'.
    pub fn form<F>(self, body: &F) -> Self
    where
        F: ?Sized + Serialize,
    {
        let body_text =
            serde_urlencoded::to_string(body).expect("It should serialize the content into a Form");

        self.bytes(body_text.into())
            .content_type(mime::APPLICATION_WWW_FORM_URLENCODED.essence_str())
    }

    /// For sending multipart forms.
    /// The payload is built using [`MultipartForm`](crate::multipart::MultipartForm) and [`Part`](crate::multipart::Part).
    ///
    /// This will be sent with the content type of 'multipart/form-data'.
    ///
    /// # Simple example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::Router;
    /// use ::axum_test::TestServer;
    /// use ::axum_test::multipart::MultipartForm;
    ///
    /// let app = Router::new();
    /// let server = TestServer::new(app)?;
    ///
    /// let multipart_form = MultipartForm::new()
    ///     .add_text("name", "Joe")
    ///     .add_text("animals", "foxes");
    ///
    /// let response = server.post(&"/my-form")
    ///     .multipart(multipart_form)
    ///     .await;
    /// #
    /// # Ok(()) }
    /// ```
    ///
    /// # Sending byte parts
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::Router;
    /// use ::axum_test::TestServer;
    /// use ::axum_test::multipart::MultipartForm;
    /// use ::axum_test::multipart::Part;
    ///
    /// let app = Router::new();
    /// let server = TestServer::new(app)?;
    ///
    /// let image_bytes = include_bytes!("../README.md");
    /// let image_part = Part::bytes(image_bytes.as_slice())
    ///     .file_name(&"README.md")
    ///     .mime_type(&"text/markdown");
    ///
    /// let multipart_form = MultipartForm::new()
    ///     .add_part("file", image_part);
    ///
    /// let response = server.post(&"/my-form")
    ///     .multipart(multipart_form)
    ///     .await;
    /// #
    /// # Ok(()) }
    /// ```
    ///
    pub fn multipart(mut self, multipart: MultipartForm) -> Self {
        self.config.content_type = Some(multipart.content_type());
        self.body = Some(multipart.into());

        self
    }

    /// Set raw text as the body of the request,
    /// and sets the content type to `text/plain`.
    pub fn text<T>(self, raw_text: T) -> Self
    where
        T: Display,
    {
        let body_text = format!("{}", raw_text);

        self.bytes(body_text.into())
            .content_type(mime::TEXT_PLAIN.essence_str())
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

    /// Adds a Cookie to be sent with this request.
    pub fn add_cookie<'c>(mut self, cookie: Cookie<'c>) -> Self {
        self.config.cookies.add(cookie.into_owned());
        self
    }

    /// Adds many cookies to be used with this request.
    pub fn add_cookies(mut self, cookies: CookieJar) -> Self {
        for cookie in cookies.iter() {
            self.config.cookies.add(cookie.clone());
        }

        self
    }

    /// Clears all cookies used internally within this Request,
    /// including any that came from the `TestServer`.
    pub fn clear_cookies(mut self) -> Self {
        self.config.cookies = CookieJar::new();
        self
    }

    /// Any cookies returned will be saved to the [`TestServer`](crate::TestServer) that created this,
    /// which will continue to use those cookies on future requests.
    pub fn do_save_cookies(mut self) -> Self {
        self.config.is_saving_cookies = true;
        self
    }

    /// Cookies returned by this will _not_ be saved to the `TestServer`.
    /// For use by future requests.
    ///
    /// This is the default behaviour.
    /// You can change that default in [`TestServerConfig`](crate::TestServerConfig).
    pub fn do_not_save_cookies(mut self) -> Self {
        self.config.is_saving_cookies = false;
        self
    }

    /// Adds query parameters to be sent with this request.
    pub fn add_query_param<V>(self, key: &str, value: V) -> Self
    where
        V: Serialize,
    {
        self.add_query_params(&[(key, value)])
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
    /// let app = Router::new();
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
    /// let app = Router::new();
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
    /// let app = Router::new();
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
        self.config
            .query_params
            .add(query_params)
            .with_context(|| {
                format!(
                    "It should serialize query parameters, for request {}",
                    self.debug_request_format()
                )
            })
            .unwrap();

        self
    }

    /// Adds a query param onto the end of the request,
    /// with no urlencoding of any kind.
    ///
    /// This exists to allow custom query parameters,
    /// such as for the many versions of query param arrays.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::Router;
    /// use ::axum_test::TestServer;
    ///
    /// let app = Router::new();
    /// let server = TestServer::new(app)?;
    ///
    /// let response = server.get(&"/my-end-point")
    ///     .add_raw_query_param(&"my-flag")
    ///     .add_raw_query_param(&"array[]=123")
    ///     .add_raw_query_param(&"filter[value]=some-value")
    ///     .await;
    /// #
    /// # Ok(()) }
    /// ```
    ///
    pub fn add_raw_query_param(mut self, query_param: &str) -> Self {
        self.config.query_params.add_raw(query_param.to_string());

        self
    }

    /// Clears all query params set,
    /// including any that came from the [`TestServer`](crate::TestServer).
    pub fn clear_query_params(mut self) -> Self {
        self.config.query_params.clear();
        self
    }

    /// Adds a header to be sent with this request.
    pub fn add_header<'c>(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.config.headers.push((name, value));
        self
    }

    /// Adds an 'AUTHORIZATION' HTTP header to the request,
    /// with no internal formatting of what is given.
    pub fn authorization<T>(self, authorization_header: T) -> Self
    where
        T: AsRef<str>,
    {
        let authorization_header_value = HeaderValue::from_str(authorization_header.as_ref())
            .expect("Cannot build Authorization HeaderValue from token");

        self.add_header(header::AUTHORIZATION, authorization_header_value)
    }

    /// Adds an 'AUTHORIZATION' HTTP header to the request,
    /// in the 'Bearer {token}' format.
    pub fn authorization_bearer<T>(self, authorization_bearer_token: T) -> Self
    where
        T: Display,
    {
        let authorization_bearer_header_str = format!("Bearer {authorization_bearer_token}");
        self.authorization(authorization_bearer_header_str)
    }

    /// Clears all headers set.
    pub fn clear_headers(mut self) -> Self {
        self.config.headers = vec![];
        self
    }

    /// Sets the scheme to use when making the request. i.e. http or https.
    /// The default scheme is 'http'.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::Router;
    /// use ::axum_test::TestServer;
    ///
    /// let app = Router::new();
    /// let server = TestServer::new(app)?;
    ///
    /// let response = server
    ///     .get(&"/my-end-point")
    ///     .scheme(&"https")
    ///     .await;
    /// #
    /// # Ok(()) }
    /// ```
    ///
    pub fn scheme(mut self, scheme: &str) -> Self {
        self.config
            .full_request_url
            .set_scheme(scheme)
            .map_err(|_| anyhow!("Scheme '{scheme}' cannot be set to request"))
            .unwrap();
        self
    }

    /// Marks that this request is expected to always return a HTTP
    /// status code within the 2xx range (200 to 299).
    ///
    /// If a code _outside_ of that range is returned,
    /// then this will panic.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::Json;
    /// use ::axum::routing::Router;
    /// use ::axum::routing::put;
    /// use ::serde_json::json;
    ///
    /// use ::axum_test::TestServer;
    ///
    /// let app = Router::new()
    ///     .route(&"/todo", put(|| async { unimplemented!() }));
    ///
    /// let server = TestServer::new(app)?;
    ///
    /// // If this doesn't return a value in the 2xx range,
    /// // then it will panic.
    /// server.put(&"/todo")
    ///     .expect_success()
    ///     .json(&json!({
    ///         "task": "buy milk",
    ///     }))
    ///     .await;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    ///
    pub fn expect_success(self) -> Self {
        self.expect_state(ExpectedState::Success)
    }

    /// Marks that this request is expected to return a HTTP status code
    /// outside of the 2xx range.
    ///
    /// If a code _within_ the 2xx range is returned,
    /// then this will panic.
    pub fn expect_failure(self) -> Self {
        self.expect_state(ExpectedState::Failure)
    }

    fn expect_state(mut self, expected_state: ExpectedState) -> Self {
        self.expected_state = expected_state;
        self
    }

    async fn send(mut self) -> Result<TestResponse> {
        let debug_request_format = self.debug_request_format().to_string();

        let method = self.config.method;
        let expected_state = self.expected_state;
        let save_cookies = self.config.is_saving_cookies;
        let body = self.body.unwrap_or(Body::empty());
        let url =
            Self::build_url_query_params(self.config.full_request_url, &self.config.query_params);

        let request = Self::build_request(
            method.clone(),
            &url,
            body,
            self.config.content_type,
            self.config.cookies,
            self.config.headers,
            &debug_request_format,
        )?;

        #[allow(unused_mut)] // Allowed for the `ws` use immediately after.
        let mut http_response = self.transport.send(request).await?;

        #[cfg(feature = "ws")]
        let websockets = {
            let maybe_on_upgrade = http_response
                .extensions_mut()
                .remove::<hyper::upgrade::OnUpgrade>();
            let transport_type = self.transport.get_type();

            crate::internals::TestResponseWebSocket {
                maybe_on_upgrade,
                transport_type,
            }
        };

        let (parts, response_body) = http_response.into_parts();
        let response_bytes = response_body.collect().await?.to_bytes();

        if save_cookies {
            let cookie_headers = parts.headers.get_all(SET_COOKIE).into_iter();
            ServerSharedState::add_cookies_by_header(&mut self.server_state, cookie_headers)?;
        }

        let test_response = TestResponse::new(
            method,
            url,
            parts,
            response_bytes,
            #[cfg(feature = "ws")]
            websockets,
        );

        // Assert if ok or not.
        match expected_state {
            ExpectedState::Success => test_response.assert_status_success(),
            ExpectedState::Failure => test_response.assert_status_failure(),
            ExpectedState::None => {}
        }

        Ok(test_response)
    }

    fn build_url_query_params(mut url: Url, query_params: &QueryParamsStore) -> Url {
        // Add all the query params we have
        if query_params.has_content() {
            url.set_query(Some(&query_params.to_string()));
        }

        url
    }

    fn build_request(
        method: Method,
        url: &Url,
        body: Body,
        content_type: Option<String>,
        cookies: CookieJar,
        headers: Vec<(HeaderName, HeaderValue)>,
        debug_request_format: &str,
    ) -> Result<Request<Body>> {
        let mut request_builder = Request::builder().uri(url.as_str()).method(method);

        // Add all the headers we have.
        if let Some(content_type) = content_type {
            let (header_key, header_value) =
                build_content_type_header(&content_type, &debug_request_format)?;
            request_builder = request_builder.header(header_key, header_value);
        }

        // Add all the non-expired cookies as headers
        let now = OffsetDateTime::now_utc();
        for cookie in cookies.iter() {
            let expired = cookie
                .expires_datetime()
                .map(|expires| expires <= now)
                .unwrap_or(false);

            if !expired {
                let cookie_raw = cookie.to_string();
                let header_value = HeaderValue::from_str(&cookie_raw)?;
                request_builder = request_builder.header(header::COOKIE, header_value);
            }
        }

        // Put headers into the request
        for (header_name, header_value) in headers {
            request_builder = request_builder.header(header_name, header_value);
        }

        let request = request_builder.body(body).with_context(|| {
            format!("Expect valid hyper Request to be built, for request {debug_request_format}")
        })?;

        Ok(request)
    }

    fn debug_request_format<'a>(&'a self) -> RequestPathFormatter<'a> {
        RequestPathFormatter::new(
            &self.config.method,
            &self.config.full_request_url.as_str(),
            Some(&self.config.query_params),
        )
    }
}

impl TryFrom<TestRequest> for Request<Body> {
    type Error = AnyhowError;

    fn try_from(test_request: TestRequest) -> Result<Request<Body>> {
        let debug_request_format = test_request.debug_request_format().to_string();
        let url = TestRequest::build_url_query_params(
            test_request.config.full_request_url,
            &test_request.config.query_params,
        );
        let body = test_request.body.unwrap_or(Body::empty());

        TestRequest::build_request(
            test_request.config.method,
            &url,
            body,
            test_request.config.content_type,
            test_request.config.cookies,
            test_request.config.headers,
            &debug_request_format,
        )
    }
}

impl IntoFuture for TestRequest {
    type Output = TestResponse;
    type IntoFuture = AutoFuture<TestResponse>;

    fn into_future(self) -> Self::IntoFuture {
        AutoFuture::new(async {
            self.send()
                .await
                .with_context(|| format!("Sending request failed"))
                .unwrap()
        })
    }
}

fn build_content_type_header(
    content_type: &str,
    debug_request_format: &str,
) -> Result<(HeaderName, HeaderValue)> {
    let header_value = HeaderValue::from_str(content_type).with_context(|| {
        format!(
            "Failed to store header content type '{content_type}', for request {debug_request_format}"
        )
    })?;

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
        let app = Router::new().route("/content_type", get(get_content_type));

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server.get(&"/content_type").await.text();

        assert_eq!(text, "");
    }

    #[tokio::test]
    async fn it_should_override_server_content_type_when_present() {
        // Build an application with a route.
        let app = Router::new().route("/content_type", get(get_content_type));

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
        let app = Router::new().route("/content_type", get(get_content_type));

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
        let app = Router::new().route("/json", post(get_json));

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
        let app = Router::new().route("/content_type", post(get_content_type));

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server.post(&"/content_type").json(&json!({})).await.text();

        assert_eq!(text, "application/json");
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_yaml {
    use crate::TestServer;

    use ::axum::routing::post;
    use ::axum::Router;
    use ::axum_yaml::Yaml;
    use ::http::header::CONTENT_TYPE;
    use ::http::HeaderMap;
    use ::serde::Deserialize;
    use ::serde::Serialize;
    use ::serde_json::json;

    #[tokio::test]
    async fn it_should_pass_yaml_up_to_be_read() {
        #[derive(Deserialize, Serialize)]
        struct TestYaml {
            name: String,
            age: u32,
            pets: Option<String>,
        }

        async fn get_yaml(Yaml(yaml): Yaml<TestYaml>) -> String {
            format!(
                "yaml: {}, {}, {}",
                yaml.name,
                yaml.age,
                yaml.pets.unwrap_or_else(|| "pandas".to_string())
            )
        }

        // Build an application with a route.
        let app = Router::new().route("/yaml", post(get_yaml));

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server
            .post(&"/yaml")
            .yaml(&TestYaml {
                name: "Joe".to_string(),
                age: 20,
                pets: Some("foxes".to_string()),
            })
            .await
            .text();

        assert_eq!(text, "yaml: Joe, 20, foxes");
    }

    #[tokio::test]
    async fn it_should_pass_yaml_content_type_for_yaml() {
        async fn get_content_type(headers: HeaderMap) -> String {
            headers
                .get(CONTENT_TYPE)
                .map(|h| h.to_str().unwrap().to_string())
                .unwrap_or_else(|| "".to_string())
        }

        // Build an application with a route.
        let app = Router::new().route("/content_type", post(get_content_type));

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server.post(&"/content_type").yaml(&json!({})).await.text();

        assert_eq!(text, "application/yaml");
    }
}

#[cfg(feature = "msgpack")]
#[cfg(test)]
mod test_msgpack {
    use crate::TestServer;

    use ::axum::routing::post;
    use ::axum::Router;
    use ::axum_msgpack::MsgPack;
    use ::http::header::CONTENT_TYPE;
    use ::http::HeaderMap;
    use ::serde::Deserialize;
    use ::serde::Serialize;
    use ::serde_json::json;

    #[tokio::test]
    async fn it_should_pass_msgpack_up_to_be_read() {
        #[derive(Deserialize, Serialize)]
        struct TestMsgPack {
            name: String,
            age: u32,
            pets: Option<String>,
        }

        async fn get_msgpack(MsgPack(msgpack): MsgPack<TestMsgPack>) -> String {
            format!(
                "yaml: {}, {}, {}",
                msgpack.name,
                msgpack.age,
                msgpack.pets.unwrap_or_else(|| "pandas".to_string())
            )
        }

        // Build an application with a route.
        let app = Router::new().route("/msgpack", post(get_msgpack));

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server
            .post(&"/msgpack")
            .msgpack(&TestMsgPack {
                name: "Joe".to_string(),
                age: 20,
                pets: Some("foxes".to_string()),
            })
            .await
            .text();

        assert_eq!(text, "yaml: Joe, 20, foxes");
    }

    #[tokio::test]
    async fn it_should_pass_msgpck_content_type_for_msgpack() {
        async fn get_content_type(headers: HeaderMap) -> String {
            headers
                .get(CONTENT_TYPE)
                .map(|h| h.to_str().unwrap().to_string())
                .unwrap_or_else(|| "".to_string())
        }

        // Build an application with a route.
        let app = Router::new().route("/content_type", post(get_content_type));

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        let text = server
            .post(&"/content_type")
            .msgpack(&json!({}))
            .await
            .text();

        assert_eq!(text, "application/msgpack");
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
        let app = Router::new().route("/form", post(get_form));

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server
            .post(&"/form")
            .form(&TestForm {
                name: "Joe".to_string(),
                age: 20,
                pets: Some("foxes".to_string()),
            })
            .await
            .assert_text("form: Joe, 20, foxes");
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
        let app = Router::new().route("/content_type", post(get_content_type));

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        #[derive(Serialize)]
        struct MyForm {
            message: String,
        }

        // Get the request.
        server
            .post(&"/content_type")
            .form(&MyForm {
                message: "hello".to_string(),
            })
            .await
            .assert_text("application/x-www-form-urlencoded");
    }
}

#[cfg(test)]
mod test_text {
    use crate::TestServer;

    use ::axum::extract::Request;
    use ::axum::routing::post;
    use ::axum::Router;
    use ::http::header::CONTENT_TYPE;
    use ::http::HeaderMap;
    use ::http_body_util::BodyExt;

    #[tokio::test]
    async fn it_should_pass_text_up_to_be_read() {
        async fn get_text(request: Request) -> String {
            let body_bytes = request
                .into_body()
                .collect()
                .await
                .expect("Should read body to bytes")
                .to_bytes();
            let body_text = String::from_utf8_lossy(&body_bytes);

            format!("{}", body_text)
        }

        // Build an application with a route.
        let app = Router::new().route("/text", post(get_text));

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
        let app = Router::new().route("/content_type", post(get_content_type));

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
        let app = Router::new().route("/ping", get(get_ping));

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
        let app = Router::new().route("/accepted", get(get_accepted));

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/accepted").expect_success().await;
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_on_404() {
        // Build an application with a route.
        let app = Router::new();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/some_unknown_route").expect_success().await;
    }

    #[tokio::test]
    async fn it_should_override_what_test_server_has_set() {
        async fn get_ping() -> &'static str {
            "pong!"
        }

        // Build an application with a route.
        let app = Router::new().route("/ping", get(get_ping));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.expect_failure();

        // Get the request.
        server.get(&"/ping").expect_success().await;
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
        let app = Router::new();

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
        let app = Router::new().route("/ping", get(get_ping));

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
        let app = Router::new().route("/accepted", get(get_accepted));

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/accepted").expect_failure().await;
    }

    #[tokio::test]
    async fn it_should_should_override_what_test_server_has_set() {
        // Build an application with a route.
        let app = Router::new();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.expect_success();

        // Get the request.
        server.get(&"/some_unknown_route").expect_failure().await;
    }
}

#[cfg(test)]
mod test_add_cookie {
    use crate::TestServer;

    use ::axum::routing::get;
    use ::axum::Router;
    use ::axum_extra::extract::cookie::CookieJar;
    use ::cookie::time::Duration;
    use ::cookie::time::OffsetDateTime;
    use ::cookie::Cookie;

    const TEST_COOKIE_NAME: &'static str = &"test-cookie";

    async fn get_cookie(cookies: CookieJar) -> (CookieJar, String) {
        let cookie = cookies.get(&TEST_COOKIE_NAME);
        let cookie_value = cookie
            .map(|c| c.value().to_string())
            .unwrap_or_else(|| "cookie-not-found".to_string());

        (cookies, cookie_value)
    }

    #[tokio::test]
    async fn it_should_send_cookies_added_to_request() {
        let app = Router::new().route("/cookie", get(get_cookie));
        let server = TestServer::new(app).expect("Should create test server");

        let cookie = Cookie::new(TEST_COOKIE_NAME, "my-custom-cookie");
        let response_text = server.get(&"/cookie").add_cookie(cookie).await.text();
        assert_eq!(response_text, "my-custom-cookie");
    }

    #[tokio::test]
    async fn it_should_send_non_expired_cookies_added_to_request() {
        let app = Router::new().route("/cookie", get(get_cookie));
        let server = TestServer::new(app).expect("Should create test server");

        let mut cookie = Cookie::new(TEST_COOKIE_NAME, "my-custom-cookie");
        cookie.set_expires(
            OffsetDateTime::now_utc()
                .checked_add(Duration::minutes(10))
                .unwrap(),
        );
        let response_text = server.get(&"/cookie").add_cookie(cookie).await.text();
        assert_eq!(response_text, "my-custom-cookie");
    }

    #[tokio::test]
    async fn it_should_not_send_expired_cookies_added_to_request() {
        let app = Router::new().route("/cookie", get(get_cookie));
        let server = TestServer::new(app).expect("Should create test server");

        let mut cookie = Cookie::new(TEST_COOKIE_NAME, "my-custom-cookie");
        cookie.set_expires(OffsetDateTime::now_utc());
        let response_text = server.get(&"/cookie").add_cookie(cookie).await.text();
        assert_eq!(response_text, "cookie-not-found");
    }
}

#[cfg(test)]
mod test_add_cookies {
    use crate::TestServer;

    use ::axum::routing::get;
    use ::axum::Router;
    use ::axum_extra::extract::cookie::CookieJar as AxumCookieJar;
    use ::cookie::Cookie;
    use ::cookie::CookieJar;

    async fn route_get_cookies(cookies: AxumCookieJar) -> String {
        let mut all_cookies = cookies
            .iter()
            .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
            .collect::<Vec<String>>();
        all_cookies.sort();

        all_cookies.join(&", ")
    }

    #[tokio::test]
    async fn it_should_send_all_cookies_added_by_jar() {
        let app = Router::new().route("/cookies", get(route_get_cookies));
        let server = TestServer::new(app).expect("Should create test server");

        // Build cookies to send up
        let cookie_1 = Cookie::new("first-cookie", "my-custom-cookie");
        let cookie_2 = Cookie::new("second-cookie", "other-cookie");
        let mut cookie_jar = CookieJar::new();
        cookie_jar.add(cookie_1);
        cookie_jar.add(cookie_2);

        server
            .get(&"/cookies")
            .add_cookies(cookie_jar)
            .await
            .assert_text("first-cookie=my-custom-cookie, second-cookie=other-cookie");
    }
}

#[cfg(test)]
mod test_clear_cookies {
    use crate::TestServer;

    use ::axum::extract::Request;
    use ::axum::routing::get;
    use ::axum::routing::put;
    use ::axum::Router;
    use ::axum_extra::extract::cookie::Cookie as AxumCookie;
    use ::axum_extra::extract::cookie::CookieJar as AxumCookieJar;
    use ::cookie::Cookie;
    use ::cookie::CookieJar;
    use ::http_body_util::BodyExt;

    const TEST_COOKIE_NAME: &'static str = &"test-cookie";

    async fn get_cookie(cookies: AxumCookieJar) -> (AxumCookieJar, String) {
        let cookie = cookies.get(&TEST_COOKIE_NAME);
        let cookie_value = cookie
            .map(|c| c.value().to_string())
            .unwrap_or_else(|| "cookie-not-found".to_string());

        (cookies, cookie_value)
    }

    async fn put_cookie(
        mut cookies: AxumCookieJar,
        request: Request,
    ) -> (AxumCookieJar, &'static str) {
        let body_bytes = request
            .into_body()
            .collect()
            .await
            .expect("Should turn the body into bytes")
            .to_bytes();

        let body_text: String = String::from_utf8_lossy(&body_bytes).to_string();
        let cookie = AxumCookie::new(TEST_COOKIE_NAME, body_text);
        cookies = cookies.add(cookie);

        (cookies, &"done")
    }

    #[tokio::test]
    async fn it_should_clear_cookie_added_to_request() {
        let app = Router::new().route("/cookie", get(get_cookie));
        let server = TestServer::new(app).expect("Should create test server");

        let cookie = Cookie::new(TEST_COOKIE_NAME, "my-custom-cookie");
        let response_text = server
            .get(&"/cookie")
            .add_cookie(cookie)
            .clear_cookies()
            .await
            .text();

        assert_eq!(response_text, "cookie-not-found");
    }

    #[tokio::test]
    async fn it_should_clear_cookie_jar_added_to_request() {
        let app = Router::new().route("/cookie", get(get_cookie));
        let server = TestServer::new(app).expect("Should create test server");

        let cookie = Cookie::new(TEST_COOKIE_NAME, "my-custom-cookie");
        let mut cookie_jar = CookieJar::new();
        cookie_jar.add(cookie);

        let response_text = server
            .get(&"/cookie")
            .add_cookies(cookie_jar)
            .clear_cookies()
            .await
            .text();

        assert_eq!(response_text, "cookie-not-found");
    }

    #[tokio::test]
    async fn it_should_clear_cookies_saved_by_past_request() {
        let app = Router::new()
            .route("/cookie", put(put_cookie))
            .route("/cookie", get(get_cookie));
        let server = TestServer::new(app).expect("Should create test server");

        // Create a cookie.
        server
            .put(&"/cookie")
            .text(&"cookie-found!")
            .do_save_cookies()
            .await;

        // Check it comes back.
        let response_text = server.get(&"/cookie").clear_cookies().await.text();

        assert_eq!(response_text, "cookie-not-found");
    }

    #[tokio::test]
    async fn it_should_clear_cookies_added_to_test_server() {
        let app = Router::new()
            .route("/cookie", put(put_cookie))
            .route("/cookie", get(get_cookie));
        let mut server = TestServer::new(app).expect("Should create test server");

        let cookie = Cookie::new(TEST_COOKIE_NAME, "my-custom-cookie");
        server.add_cookie(cookie);

        // Check it comes back.
        let response_text = server.get(&"/cookie").clear_cookies().await.text();

        assert_eq!(response_text, "cookie-not-found");
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
    async fn it_should_send_header_added_to_request() {
        // Build an application with a route.
        let app = Router::new().route("/header", get(ping_header));

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
mod test_authorization {
    use super::*;

    use ::axum::async_trait;
    use ::axum::extract::FromRequestParts;
    use ::axum::routing::get;
    use ::axum::Router;
    use ::http::request::Parts;
    use ::hyper::StatusCode;
    use ::std::marker::Sync;

    use crate::TestServer;

    fn new_test_server() -> TestServer {
        struct TestHeader(String);

        #[async_trait]
        impl<S: Sync> FromRequestParts<S> for TestHeader {
            type Rejection = (StatusCode, &'static str);

            async fn from_request_parts(
                parts: &mut Parts,
                _state: &S,
            ) -> Result<TestHeader, Self::Rejection> {
                parts
                    .headers
                    .get(header::AUTHORIZATION)
                    .map(|v| TestHeader(v.to_str().unwrap().to_string()))
                    .ok_or((StatusCode::BAD_REQUEST, "Missing test header"))
            }
        }

        async fn ping_auth_header(TestHeader(header): TestHeader) -> String {
            header
        }

        // Build an application with a route.
        let app = Router::new().route("/auth-header", get(ping_auth_header));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.expect_success();

        server
    }

    #[tokio::test]
    async fn it_should_send_header_added_to_request() {
        let server = new_test_server();

        // Send a request with the header
        let response = server
            .get(&"/auth-header")
            .authorization("Bearer abc123")
            .await;

        // Check it sent back the right text
        response.assert_text("Bearer abc123")
    }
}

#[cfg(test)]
mod test_authorization_bearer {
    use super::*;

    use ::axum::async_trait;
    use ::axum::extract::FromRequestParts;
    use ::axum::routing::get;
    use ::axum::Router;
    use ::http::request::Parts;
    use ::hyper::StatusCode;
    use ::std::marker::Sync;

    use crate::TestServer;

    fn new_test_server() -> TestServer {
        struct TestHeader(String);

        #[async_trait]
        impl<S: Sync> FromRequestParts<S> for TestHeader {
            type Rejection = (StatusCode, &'static str);

            async fn from_request_parts(
                parts: &mut Parts,
                _state: &S,
            ) -> Result<TestHeader, Self::Rejection> {
                parts
                    .headers
                    .get(header::AUTHORIZATION)
                    .map(|v| TestHeader(v.to_str().unwrap().to_string().replace("Bearer ", "")))
                    .ok_or((StatusCode::BAD_REQUEST, "Missing test header"))
            }
        }

        async fn ping_auth_header(TestHeader(header): TestHeader) -> String {
            header
        }

        // Build an application with a route.
        let app = Router::new().route("/auth-header", get(ping_auth_header));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.expect_success();

        server
    }

    #[tokio::test]
    async fn it_should_send_header_added_to_request() {
        let server = new_test_server();

        // Send a request with the header
        let response = server
            .get(&"/auth-header")
            .authorization_bearer("abc123")
            .await;

        // Check it sent back the right text
        response.assert_text("abc123")
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
    async fn it_should_clear_headers_added_to_request() {
        // Build an application with a route.
        let app = Router::new().route("/header", get(ping_header));

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
            .await;

        // Check it sent back the right text
        response.assert_status_bad_request();
        response.assert_text("Missing test header");
    }

    #[tokio::test]
    async fn it_should_clear_headers_added_to_server() {
        // Build an application with a route.
        let app = Router::new().route("/header", get(ping_header));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_header(
            HeaderName::from_static(TEST_HEADER_NAME),
            HeaderValue::from_static(TEST_HEADER_CONTENT),
        );

        // Send a request with the header
        let response = server.get(&"/header").clear_headers().await;

        // Check it sent back the right text
        response.assert_status_bad_request();
        response.assert_text("Missing test header");
    }
}

#[cfg(test)]
mod test_add_query_params {
    use ::axum::extract::Query as AxumStdQuery;
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

    async fn get_query_param(AxumStdQuery(params): AxumStdQuery<QueryParam>) -> String {
        params.message
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParam2 {
        message: String,
        other: String,
    }

    async fn get_query_param_2(AxumStdQuery(params): AxumStdQuery<QueryParam2>) -> String {
        format!("{}-{}", params.message, params.other)
    }

    fn build_app() -> Router {
        Router::new()
            .route("/query", get(get_query_param))
            .route("/query-2", get(get_query_param_2))
    }

    #[tokio::test]
    async fn it_should_pass_up_query_params_from_serialization() {
        // Run the server.
        let server = TestServer::new(build_app()).expect("Should create test server");

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
        // Run the server.
        let server = TestServer::new(build_app()).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query")
            .add_query_params(&[("message", "it works")])
            .await
            .assert_text(&"it works");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_multiple_params() {
        // Run the server.
        let server = TestServer::new(build_app()).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query-2")
            .add_query_params(&[("message", "it works"), ("other", "yup")])
            .await
            .assert_text(&"it works-yup");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_multiple_calls() {
        // Run the server.
        let server = TestServer::new(build_app()).expect("Should create test server");

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
        // Run the server.
        let server = TestServer::new(build_app()).expect("Should create test server");

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
mod test_add_raw_query_param {
    use ::axum::extract::Query as AxumStdQuery;
    use ::axum::routing::get;
    use ::axum::Router;
    use ::axum_extra::extract::Query as AxumExtraQuery;
    use ::serde::Deserialize;
    use ::serde::Serialize;
    use ::std::fmt::Write;

    use crate::TestServer;

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParam {
        message: String,
    }

    async fn get_query_param(AxumStdQuery(params): AxumStdQuery<QueryParam>) -> String {
        params.message
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct QueryParamExtra {
        #[serde(default)]
        items: Vec<String>,

        #[serde(default, rename = "arrs[]")]
        arrs: Vec<String>,
    }

    async fn get_query_param_extra(
        AxumExtraQuery(params): AxumExtraQuery<QueryParamExtra>,
    ) -> String {
        let mut output = String::new();

        if params.items.len() > 0 {
            write!(output, "{}", params.items.join(", ")).unwrap();
        }

        if params.arrs.len() > 0 {
            write!(output, "{}", params.arrs.join(", ")).unwrap();
        }

        output
    }

    fn build_app() -> Router {
        Router::new()
            .route("/query", get(get_query_param))
            .route("/query-extra", get(get_query_param_extra))
    }

    #[tokio::test]
    async fn it_should_pass_up_query_param_as_is() {
        // Run the server.
        let server = TestServer::new(build_app()).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query")
            .add_raw_query_param(&"message=it-works")
            .await
            .assert_text(&"it-works");
    }

    #[tokio::test]
    async fn it_should_pass_up_array_query_params_as_one_string() {
        // Run the server.
        let server = TestServer::new(build_app()).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query-extra")
            .add_raw_query_param(&"items=one&items=two&items=three")
            .await
            .assert_text(&"one, two, three");
    }

    #[tokio::test]
    async fn it_should_pass_up_array_query_params_as_multiple_params() {
        // Run the server.
        let server = TestServer::new(build_app()).expect("Should create test server");

        // Get the request.
        server
            .get(&"/query-extra")
            .add_raw_query_param(&"arrs[]=one")
            .add_raw_query_param(&"arrs[]=two")
            .add_raw_query_param(&"arrs[]=three")
            .await
            .assert_text(&"one, two, three");
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
        let app = Router::new().route("/query", get(get_query_param));

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
        let app = Router::new().route("/query-2", get(get_query_param_2));

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
        let app = Router::new().route("/query", get(get_query_params));

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
        let app = Router::new().route("/query", get(get_query_params));

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

#[cfg(test)]
mod test_scheme {
    use axum::extract::Request;
    use axum::routing::get;
    use axum::Router;

    use crate::TestServer;
    use crate::TestServerConfig;

    async fn route_get_scheme(request: Request) -> String {
        request.uri().scheme_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn it_should_return_http_by_default() {
        let router = Router::new().route("/scheme", get(route_get_scheme));

        let config = TestServerConfig::builder().build();
        let server = TestServer::new_with_config(router, config).unwrap();

        server.get("/scheme").await.assert_text("http");
    }

    #[tokio::test]
    async fn it_should_return_http_when_set() {
        let router = Router::new().route("/scheme", get(route_get_scheme));

        let config = TestServerConfig::builder().build();
        let server = TestServer::new_with_config(router, config).unwrap();

        server
            .get("/scheme")
            .scheme(&"http")
            .await
            .assert_text("http");
    }

    #[tokio::test]
    async fn it_should_return_https_when_set() {
        let router = Router::new().route("/scheme", get(route_get_scheme));

        let config = TestServerConfig::builder().build();
        let server = TestServer::new_with_config(router, config).unwrap();

        server
            .get("/scheme")
            .scheme(&"https")
            .await
            .assert_text("https");
    }

    #[tokio::test]
    async fn it_should_override_test_server_when_set() {
        let router = Router::new().route("/scheme", get(route_get_scheme));

        let config = TestServerConfig::builder().build();
        let mut server = TestServer::new_with_config(router, config).unwrap();
        server.scheme(&"https");

        server
            .get("/scheme")
            .scheme(&"http") // set it back to http
            .await
            .assert_text("http");
    }
}

#[cfg(test)]
mod test_multipart {
    use ::axum::extract::Multipart;
    use ::axum::routing::post;
    use ::axum::Json;
    use ::axum::Router;

    use crate::multipart::MultipartForm;
    use crate::multipart::Part;
    use crate::TestServer;
    use crate::TestServerConfig;

    async fn route_post_multipart(mut multipart: Multipart) -> Json<Vec<String>> {
        let mut fields = vec![];

        while let Some(field) = multipart.next_field().await.unwrap() {
            let name = field.name().unwrap().to_string();
            let content_type = field.content_type().unwrap().to_owned();
            let data = field.bytes().await.unwrap();

            let field_stats = format!("{name} is {} bytes, {content_type}", data.len());
            fields.push(field_stats);
        }

        Json(fields)
    }

    fn test_router() -> Router {
        Router::new().route("/multipart", post(route_post_multipart))
    }

    #[tokio::test]
    async fn it_should_get_multipart_stats_on_mock_transport() {
        // Run the server.
        let config = TestServerConfig::builder().mock_transport().build();
        let server =
            TestServer::new_with_config(test_router(), config).expect("Should create test server");

        let form = MultipartForm::new()
            .add_text("penguins?", "lots")
            .add_text("animals", "🦊🦊🦊")
            .add_text("carrots", 123 as u32);

        // Get the request.
        server
            .post(&"/multipart")
            .multipart(form)
            .await
            .assert_json(&vec![
                "penguins? is 4 bytes, text/plain".to_string(),
                "animals is 12 bytes, text/plain".to_string(),
                "carrots is 3 bytes, text/plain".to_string(),
            ]);
    }

    #[tokio::test]
    async fn it_should_get_multipart_stats_on_http_transport() {
        // Run the server.
        let config = TestServerConfig::builder().http_transport().build();
        let server =
            TestServer::new_with_config(test_router(), config).expect("Should create test server");

        let form = MultipartForm::new()
            .add_text("penguins?", "lots")
            .add_text("animals", "🦊🦊🦊")
            .add_text("carrots", 123 as u32);

        // Get the request.
        server
            .post(&"/multipart")
            .multipart(form)
            .await
            .assert_json(&vec![
                "penguins? is 4 bytes, text/plain".to_string(),
                "animals is 12 bytes, text/plain".to_string(),
                "carrots is 3 bytes, text/plain".to_string(),
            ]);
    }

    #[tokio::test]
    async fn it_should_send_text_parts_as_text() {
        // Run the server.
        let config = TestServerConfig::builder().mock_transport().build();
        let server =
            TestServer::new_with_config(test_router(), config).expect("Should create test server");

        let form = MultipartForm::new().add_part("animals", Part::text("🦊🦊🦊"));

        // Get the request.
        server
            .post(&"/multipart")
            .multipart(form)
            .await
            .assert_json(&vec!["animals is 12 bytes, text/plain".to_string()]);
    }

    #[tokio::test]
    async fn it_should_send_custom_mime_type() {
        // Run the server.
        let config = TestServerConfig::builder().mock_transport().build();
        let server =
            TestServer::new_with_config(test_router(), config).expect("Should create test server");

        let form = MultipartForm::new().add_part(
            "animals",
            Part::bytes("🦊,🦊,🦊".as_bytes()).mime_type(mime::TEXT_CSV),
        );

        // Get the request.
        server
            .post(&"/multipart")
            .multipart(form)
            .await
            .assert_json(&vec!["animals is 14 bytes, text/csv".to_string()]);
    }

    #[tokio::test]
    async fn it_should_send_using_include_bytes() {
        // Run the server.
        let config = TestServerConfig::builder().mock_transport().build();
        let server =
            TestServer::new_with_config(test_router(), config).expect("Should create test server");

        let form = MultipartForm::new().add_part(
            "file",
            Part::bytes(include_bytes!("../rust-toolchain").as_slice()).mime_type(mime::TEXT_PLAIN),
        );

        // Get the request.
        server
            .post(&"/multipart")
            .multipart(form)
            .await
            .assert_json(&vec!["file is 6 bytes, text/plain".to_string()]);
    }
}
