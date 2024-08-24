use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use cookie::Cookie;
use cookie::CookieJar;
use http::HeaderName;
use http::HeaderValue;
use http::Method;
use http::Uri;
use serde::Serialize;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::Mutex;
use url::Url;

#[cfg(feature = "typed-routing")]
use axum_extra::routing::TypedPath;

#[cfg(feature = "reqwest")]
use crate::transport_layer::TransportLayerType;
#[cfg(feature = "reqwest")]
use reqwest::Client;
#[cfg(feature = "reqwest")]
use reqwest::RequestBuilder;

use crate::internals::ExpectedState;
use crate::internals::QueryParamsStore;
use crate::internals::RequestPathFormatter;
use crate::transport_layer::IntoTransportLayer;
use crate::transport_layer::TransportLayer;
use crate::transport_layer::TransportLayerBuilder;
use crate::TestRequest;
use crate::TestRequestConfig;
use crate::TestServerBuilder;
use crate::TestServerConfig;
use crate::Transport;

mod server_shared_state;
pub(crate) use self::server_shared_state::*;

const DEFAULT_URL_ADDRESS: &'static str = "http://localhost";

///
/// The `TestServer` runs your Axum application,
/// allowing you to make HTTP requests against it.
///
/// # Building
///
/// A `TestServer` can be used to run an [`axum::Router`], an [`::axum::routing::IntoMakeService`],
/// a [`shuttle_axum::ShuttleAxum`], and others.
///
/// The most straight forward approach is to call [`crate::TestServer::new`],
/// and pass in your application:
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use axum::Router;
/// use axum::routing::get;
///
/// use axum_test::TestServer;
///
/// let app = Router::new()
///     .route(&"/hello", get(|| async { "hello!" }));
///
/// let server = TestServer::new(app)?;
/// #
/// # Ok(())
/// # }
/// ```
///
/// # Requests
///
/// Requests are built by calling [`TestServer::get()`](crate::TestServer::get()),
/// [`TestServer::post()`](crate::TestServer::post()), [`TestServer::put()`](crate::TestServer::put()),
/// [`TestServer::delete()`](crate::TestServer::delete()), and [`TestServer::patch()`](crate::TestServer::patch()) methods.
/// Each returns a [`TestRequest`](crate::TestRequest), which allows for customising the request content.
///
/// For example:
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use axum::Router;
/// use axum::routing::get;
///
/// use axum_test::TestServer;
///
/// let app = Router::new()
///     .route(&"/hello", get(|| async { "hello!" }));
///
/// let server = TestServer::new(app)?;
///
/// let response = server.get(&"/hello")
///     .authorization_bearer("password12345")
///     .add_header("x-custom-header", "custom-value")
///     .await;
///
/// response.assert_text("hello!");
/// #
/// # Ok(())
/// # }
/// ```
///
/// Request methods also exist for using Axum Extra [`axum_extra::routing::TypedPath`],
/// or for building Reqwest [`reqwest::RequestBuilder`]. See those methods for detauls.
///
/// # Customising
///
/// A `TestServer` can be built from a builder, by calling [`crate::TestServer::builder`],
/// and customising settings. This allows one to set **mocked** (default when possible)
/// or **real http** networking for your service.
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use axum::Router;
/// use axum::routing::get;
///
/// use axum_test::TestServer;
///
/// let app = Router::new()
///     .route(&"/hello", get(|| async { "hello!" }));
///
/// // Customise server when building
/// let mut server = TestServer::builder()
///     .http_transport()
///     .expect_success_by_default()
///     .do_save_cookies()
///     .build(app)?;
///
/// // Add items to be sent on _all_ all requests
/// server.add_header("x-custom-for-all", "common-value");
///
/// let response = server.get("/hello").await;
/// #
/// # Ok(())
/// # }
/// ```
///
#[derive(Debug)]
pub struct TestServer {
    state: Arc<Mutex<ServerSharedState>>,
    transport: Arc<Box<dyn TransportLayer>>,
    save_cookies: bool,
    expected_state: ExpectedState,
    default_content_type: Option<String>,
    is_http_path_restricted: bool,

    #[cfg(feature = "reqwest")]
    maybe_reqwest_client: Option<Client>,
}

impl TestServer {
    /// A helper function to create a builder for creating a [`TestServer`].
    pub fn builder() -> TestServerBuilder {
        TestServerBuilder::default()
    }

    /// This will run the given Axum app,
    /// allowing you to make requests against it.
    ///
    /// This is the same as creating a new `TestServer` with a configuration,
    /// and passing [`crate::TestServerConfig::default()`].
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum::routing::get;
    /// use axum_test::TestServer;
    ///
    /// let app = Router::new()
    ///     .route(&"/hello", get(|| async { "hello!" }));
    ///
    /// let server = TestServer::new(app)?;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Types of applications that can be passed in include:
    ///
    ///  - [`axum::Router`]
    ///  - [`axum::routin::IntoMakeService`]
    ///  - [`axum::extract::connect_info::IntoMakeServiceWithConnectInfo`]
    ///  - [`shuttle_axum::ShuttleAxum`]
    ///
    pub fn new<A>(app: A) -> Result<Self>
    where
        A: IntoTransportLayer,
    {
        Self::new_with_config(app, TestServerConfig::default())
    }

    /// Similar to [`TestServer::new()`], with a customised configuration.
    /// This includes type of transport in use (i.e. specify a specific port),
    /// or change default settings (like the default content type for requests).
    ///
    /// This can take a [`crate::TestServerConfig`] or a [`crate::TestServerBuilder`].
    /// See those for more information on configuration settings.
    pub fn new_with_config<A, C>(app: A, config: C) -> Result<Self>
    where
        A: IntoTransportLayer,
        C: Into<TestServerConfig>,
    {
        let config = config.into();
        let mut shared_state = ServerSharedState::new();
        if let Some(scheme) = config.default_scheme {
            shared_state.set_scheme_unlocked(scheme);
        }

        let shared_state_mutex = Mutex::new(shared_state);
        let state = Arc::new(shared_state_mutex);

        let transport = match config.transport {
            None => {
                let builder = TransportLayerBuilder::new(None, None);
                let transport = app.into_default_transport(builder)?;
                Arc::new(transport)
            }
            Some(Transport::HttpRandomPort) => {
                let builder = TransportLayerBuilder::new(None, None);
                let transport = app.into_http_transport_layer(builder)?;
                Arc::new(transport)
            }
            Some(Transport::HttpIpPort { ip, port }) => {
                let builder = TransportLayerBuilder::new(ip, port);
                let transport = app.into_http_transport_layer(builder)?;
                Arc::new(transport)
            }
            Some(Transport::MockHttp) => {
                let transport = app.into_mock_transport_layer()?;
                Arc::new(transport)
            }
        };

        let expected_state = match config.expect_success_by_default {
            true => ExpectedState::Success,
            false => ExpectedState::None,
        };

        #[cfg(feature = "reqwest")]
        let maybe_reqwest_client = match transport.transport_layer_type() {
            TransportLayerType::Http => {
                let reqwest_client = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .cookie_store(config.save_cookies)
                    .build()
                    .expect("Failed to build Reqwest Client");

                Some(reqwest_client)
            }
            TransportLayerType::Mock => None,
        };

        Ok(Self {
            state,
            transport,
            save_cookies: config.save_cookies,
            expected_state,
            default_content_type: config.default_content_type,
            is_http_path_restricted: config.restrict_requests_with_http_schema,

            #[cfg(feature = "reqwest")]
            maybe_reqwest_client,
        })
    }

    /// Creates a HTTP GET request to the path.
    pub fn get(&self, path: &str) -> TestRequest {
        self.method(Method::GET, path)
    }

    /// Creates a HTTP POST request to the given path.
    pub fn post(&self, path: &str) -> TestRequest {
        self.method(Method::POST, path)
    }

    /// Creates a HTTP PATCH request to the path.
    pub fn patch(&self, path: &str) -> TestRequest {
        self.method(Method::PATCH, path)
    }

    /// Creates a HTTP PUT request to the path.
    pub fn put(&self, path: &str) -> TestRequest {
        self.method(Method::PUT, path)
    }

    /// Creates a HTTP DELETE request to the path.
    pub fn delete(&self, path: &str) -> TestRequest {
        self.method(Method::DELETE, path)
    }

    /// Creates a HTTP request, to the method and path provided.
    pub fn method(&self, method: Method, path: &str) -> TestRequest {
        let maybe_config = self.build_test_request_config(method.clone(), path);
        let config = maybe_config
            .with_context(|| format!("Failed to build, for request {method} {path}"))
            .unwrap();

        TestRequest::new(self.state.clone(), self.transport.clone(), config)
    }

    #[cfg(feature = "reqwest")]
    fn reqwest_client(&self) -> &Client {
        self.maybe_reqwest_client
            .as_ref()
            .expect("Reqwest client is not available, TestServer must be build with HTTP transport for Reqwest to be available")
    }

    #[cfg(feature = "reqwest")]
    pub fn reqwest_get(&self, path: &str) -> RequestBuilder {
        self.reqwest_method(Method::GET, path)
    }

    #[cfg(feature = "reqwest")]
    pub fn reqwest_post(&self, path: &str) -> RequestBuilder {
        self.reqwest_method(Method::POST, path)
    }

    #[cfg(feature = "reqwest")]
    pub fn reqwest_put(&self, path: &str) -> RequestBuilder {
        self.reqwest_method(Method::PUT, path)
    }

    #[cfg(feature = "reqwest")]
    pub fn reqwest_patch(&self, path: &str) -> RequestBuilder {
        self.reqwest_method(Method::PATCH, path)
    }

    #[cfg(feature = "reqwest")]
    pub fn reqwest_delete(&self, path: &str) -> RequestBuilder {
        self.reqwest_method(Method::DELETE, path)
    }

    #[cfg(feature = "reqwest")]
    pub fn reqwest_head(&self, path: &str) -> RequestBuilder {
        self.reqwest_method(Method::HEAD, path)
    }

    /// Creates a HTTP request, using Reqwest, using the method + path described.
    /// This expects a relative url to the `TestServer`.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum_test::TestServer;
    ///
    /// let my_app = Router::new();
    /// let server = TestServer::builder()
    ///     .http_transport() // Important, must be HTTP!
    ///     .build(my_app)?;
    ///
    /// // Build your request
    /// let request = server.get(&"/user")
    ///     .add_header("x-custom-header", "example.com")
    ///     .content_type("application/yaml");
    ///
    /// // await request to execute
    /// let response = request.await;
    /// #
    /// # Ok(()) }
    /// ```
    #[cfg(feature = "reqwest")]
    pub fn reqwest_method(&self, method: Method, path: &str) -> RequestBuilder {
        let request_url = self
            .server_url(path)
            .expect("Failed to generate server url for request {method} {path}");

        self.reqwest_client().request(method, request_url)
    }

    /// Creates a request to the server, to start a Websocket connection,
    /// on the path given.
    ///
    /// This is the requivalent of making a GET request to the endpoint,
    /// and setting the various headers needed for making an upgrade request.
    ///
    /// *Note*, this requires the server to be running on a real HTTP
    /// port. Either using a randomly assigned port, or a specified one.
    /// See the [`TestServerConfig::transport`](crate::TestServerConfig::transport) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum_test::TestServer;
    ///
    /// let app = Router::new();
    /// let server = TestServer::builder()
    ///     .http_transport()
    ///     .build(app)?;
    ///
    /// let mut websocket = server
    ///     .get_websocket(&"/my-web-socket-end-point")
    ///     .await
    ///     .into_websocket()
    ///     .await;
    ///
    /// websocket.send_text("Hello!").await;
    /// #
    /// # Ok(()) }
    /// ```
    ///
    #[cfg(feature = "ws")]
    pub fn get_websocket(&self, path: &str) -> TestRequest {
        use http::header;

        self.get(path)
            .add_header(header::CONNECTION, "upgrade")
            .add_header(header::UPGRADE, "websocket")
            .add_header(header::SEC_WEBSOCKET_VERSION, "13")
            .add_header(
                header::SEC_WEBSOCKET_KEY,
                crate::internals::generate_ws_key(),
            )
    }

    /// Creates a HTTP GET request, using the typed path provided.
    ///
    /// See [`axum-extra`](https://docs.rs/axum-extra) for full documentation on [`TypedPath`](axum_extra::routing::TypedPath).
    ///
    /// # Example Test
    ///
    /// Using a `TypedPath` you can write build and test a route like below:
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Json;
    /// use axum::Router;
    /// use axum::routing::get;
    /// use axum_extra::routing::RouterExt;
    /// use axum_extra::routing::TypedPath;
    /// use serde::Deserialize;
    /// use serde::Serialize;
    ///
    /// use axum_test::TestServer;
    ///
    /// #[derive(TypedPath, Deserialize)]
    /// #[typed_path("/users/:user_id")]
    /// struct UserPath {
    ///     pub user_id: u32,
    /// }
    ///
    /// // Build a typed route:
    /// async fn route_get_user(UserPath { user_id }: UserPath) -> String {
    ///     format!("hello user {user_id}")
    /// }
    ///
    /// let app = Router::new()
    ///     .typed_get(route_get_user);
    ///
    /// // Then test the route:
    /// let server = TestServer::new(app)?;
    /// server
    ///     .typed_get(&UserPath { user_id: 123 })
    ///     .await
    ///     .assert_text("hello user 123");
    /// #
    /// # Ok(())
    /// # }
    /// ```
    ///
    #[cfg(feature = "typed-routing")]
    pub fn typed_get<P>(&self, path: &P) -> TestRequest
    where
        P: TypedPath,
    {
        self.typed_method(Method::GET, path)
    }

    /// Creates a HTTP POST request, using the typed path provided.
    ///
    /// See [`axum-extra`](https://docs.rs/axum-extra) for full documentation on [`TypedPath`](axum_extra::routing::TypedPath).
    #[cfg(feature = "typed-routing")]
    pub fn typed_post<P>(&self, path: &P) -> TestRequest
    where
        P: TypedPath,
    {
        self.typed_method(Method::POST, path)
    }

    /// Creates a HTTP PATCH request, using the typed path provided.
    ///
    /// See [`axum-extra`](https://docs.rs/axum-extra) for full documentation on [`TypedPath`](axum_extra::routing::TypedPath).
    #[cfg(feature = "typed-routing")]
    pub fn typed_patch<P>(&self, path: &P) -> TestRequest
    where
        P: TypedPath,
    {
        self.typed_method(Method::PATCH, path)
    }

    /// Creates a HTTP PUT request, using the typed path provided.
    ///
    /// See [`axum-extra`](https://docs.rs/axum-extra) for full documentation on [`TypedPath`](axum_extra::routing::TypedPath).
    #[cfg(feature = "typed-routing")]
    pub fn typed_put<P>(&self, path: &P) -> TestRequest
    where
        P: TypedPath,
    {
        self.typed_method(Method::PUT, path)
    }

    /// Creates a HTTP DELETE request, using the typed path provided.
    ///
    /// See [`axum-extra`](https://docs.rs/axum-extra) for full documentation on [`TypedPath`](axum_extra::routing::TypedPath).
    #[cfg(feature = "typed-routing")]
    pub fn typed_delete<P>(&self, path: &P) -> TestRequest
    where
        P: TypedPath,
    {
        self.typed_method(Method::DELETE, path)
    }

    /// Creates a typed HTTP request, using the method provided.
    ///
    /// See [`axum-extra`](https://docs.rs/axum-extra) for full documentation on [`TypedPath`](axum_extra::routing::TypedPath).
    #[cfg(feature = "typed-routing")]
    pub fn typed_method<P>(&self, method: Method, path: &P) -> TestRequest
    where
        P: TypedPath,
    {
        self.method(method, &path.to_string())
    }

    /// Returns the local web address for the test server,
    /// if an address is available.
    ///
    /// The address is available when running as a real web server,
    /// by setting the [`TestServerConfig`](crate::TestServerConfig) `transport` field to `Transport::HttpRandomPort` or `Transport::HttpIpPort`.
    ///
    /// This will return `None` when there is mock HTTP transport (the default).
    pub fn server_address(&self) -> Option<Url> {
        self.url()
    }

    /// This turns a relative path, into an absolute path to the server.
    /// i.e. A path like `/users/123` will become something like `http://127.0.0.1:1234/users/123`.
    ///
    /// The absolute address can be used to make requests to the running server,
    /// using any appropriate client you wish.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum_test::TestServer;
    ///
    /// let app = Router::new();
    /// let server = TestServer::builder()
    ///         .http_transport()
    ///         .build(app)?;
    ///
    /// let full_url = server.server_url(&"/users/123?filter=enabled")?;
    ///
    /// // Prints something like ... http://127.0.0.1:1234/users/123?filter=enabled
    /// println!("{full_url}");
    /// #
    /// # Ok(()) }
    /// ```
    ///
    /// This will return an error if you are using the mock transport.
    /// Real HTTP transport is required to use this method (see [`TestServerConfig`](crate::TestServerConfig) `transport` field).
    ///
    /// It will also return an error if you provide an absolute path,
    /// for example if you pass in `http://google.com`.
    pub fn server_url(&self, path: &str) -> Result<Url> {
        let path_uri = path.parse::<Uri>()?;
        if is_absolute_uri(&path_uri) {
            return Err(anyhow!(
                "Absolute path provided for building server url, need to provide a relative uri"
            ));
        }

        let server_url = self.url()
            .ok_or_else(||
                anyhow!(
                    "No local address for server, need to run with HTTP transport to have a server address",
                )
            )?;

        let server_locked = self.state.as_ref().lock().map_err(|err| {
            anyhow!("Failed to lock InternalTestServer, for building server_url, received {err:?}",)
        })?;
        let mut query_params = server_locked.query_params().clone();
        let mut full_server_url = build_url(
            server_url,
            path,
            &mut query_params,
            self.is_http_path_restricted,
        )?;

        // Ensure the query params are present
        if query_params.has_content() {
            full_server_url.set_query(Some(&query_params.to_string()));
        }

        Ok(full_server_url)
    }

    /// Adds a single cookie to be included on *all* future requests.
    ///
    /// If a cookie with the same name already exists,
    /// then it will be replaced.
    pub fn add_cookie(&mut self, cookie: Cookie) {
        ServerSharedState::add_cookie(&self.state, cookie)
            .with_context(|| format!("Trying to call add_cookie"))
            .unwrap()
    }

    /// Adds extra cookies to be used on *all* future requests.
    ///
    /// Any cookies which have the same name as the new cookies,
    /// will get replaced.
    pub fn add_cookies(&mut self, cookies: CookieJar) {
        ServerSharedState::add_cookies(&self.state, cookies)
            .with_context(|| format!("Trying to call add_cookies"))
            .unwrap()
    }

    /// Clears all of the cookies stored internally.
    pub fn clear_cookies(&mut self) {
        ServerSharedState::clear_cookies(&self.state)
            .with_context(|| format!("Trying to call clear_cookies"))
            .unwrap()
    }

    /// Requests made using this `TestServer` will save their cookies for future requests to send.
    ///
    /// This behaviour is off by default.
    pub fn do_save_cookies(&mut self) {
        self.save_cookies = true;
    }

    /// Requests made using this `TestServer` will _not_ save their cookies for future requests to send up.
    ///
    /// This is the default behaviour.
    pub fn do_not_save_cookies(&mut self) {
        self.save_cookies = false;
    }

    /// Requests made using this `TestServer` will assert a HTTP status in the 2xx range will be returned, unless marked otherwise.
    ///
    /// By default this behaviour is off.
    pub fn expect_success(&mut self) {
        self.expected_state = ExpectedState::Success;
    }

    /// Requests made using this `TestServer` will assert a HTTP status is outside the 2xx range will be returned, unless marked otherwise.
    ///
    /// By default this behaviour is off.
    pub fn expect_failure(&mut self) {
        self.expected_state = ExpectedState::Failure;
    }

    /// Adds a query parameter to be sent on *all* future requests.
    pub fn add_query_param<V>(&mut self, key: &str, value: V)
    where
        V: Serialize,
    {
        ServerSharedState::add_query_param(&self.state, key, value)
            .with_context(|| format!("Trying to call add_query_param"))
            .unwrap()
    }

    /// Adds query parameters to be sent on *all* future requests.
    pub fn add_query_params<V>(&mut self, query_params: V)
    where
        V: Serialize,
    {
        ServerSharedState::add_query_params(&self.state, query_params)
            .with_context(|| format!("Trying to call add_query_params"))
            .unwrap()
    }

    /// Adds a raw query param, with no urlencoding of any kind,
    /// to be send on *all* future requests.
    pub fn add_raw_query_param(&mut self, raw_query_param: &str) {
        ServerSharedState::add_raw_query_param(&self.state, raw_query_param)
            .with_context(|| format!("Trying to call add_raw_query_param"))
            .unwrap()
    }

    /// Clears all query params set.
    pub fn clear_query_params(&mut self) {
        ServerSharedState::clear_query_params(&self.state)
            .with_context(|| format!("Trying to call clear_query_params"))
            .unwrap()
    }

    /// Adds a header to be sent with all future requests built from this `TestServer`.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum_test::TestServer;
    ///
    /// let app = Router::new();
    /// let mut server = TestServer::new(app)?;
    ///
    /// server.add_header("x-custom-header", "custom-value");
    /// server.add_header(http::header::CONTENT_LENGTH, 12345);
    /// server.add_header(http::header::HOST, "example.com");
    ///
    /// let response = server.get(&"/my-end-point")
    ///     .await;
    /// #
    /// # Ok(()) }
    /// ```
    pub fn add_header<'c, N, V>(&mut self, name: N, value: V)
    where
        N: TryInto<HeaderName>,
        N::Error: Debug,
        V: TryInto<HeaderValue>,
        V::Error: Debug,
    {
        let header_name: HeaderName = name
            .try_into()
            .expect("Failed to convert header name to HeaderName");
        let header_value: HeaderValue = value
            .try_into()
            .expect("Failed to convert header vlue to HeaderValue");

        ServerSharedState::add_header(&self.state, header_name, header_value)
            .with_context(|| format!("Trying to call add_header"))
            .unwrap()
    }

    /// Clears all headers set so far.
    pub fn clear_headers(&mut self) {
        ServerSharedState::clear_headers(&self.state)
            .with_context(|| format!("Trying to call clear_headers"))
            .unwrap()
    }

    /// Sets the scheme to use when making _all_ requests from the `TestServer`.
    /// i.e. http or https.
    ///
    /// The default scheme is 'http'.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum_test::TestServer;
    ///
    /// let app = Router::new();
    /// let mut server = TestServer::new(app)?;
    /// server
    ///     .scheme(&"https");
    ///
    /// let response = server
    ///     .get(&"/my-end-point")
    ///     .await;
    /// #
    /// # Ok(()) }
    /// ```
    ///
    pub fn scheme(&mut self, scheme: &str) {
        ServerSharedState::set_scheme(&self.state, scheme.to_string())
            .with_context(|| format!("Trying to call set_scheme"))
            .unwrap()
    }

    pub(crate) fn url(&self) -> Option<Url> {
        self.transport.url().map(|url| url.clone())
    }

    pub(crate) fn build_test_request_config(
        &self,
        method: Method,
        path: &str,
    ) -> Result<TestRequestConfig> {
        let url = self
            .url()
            .unwrap_or_else(|| DEFAULT_URL_ADDRESS.parse().unwrap());

        let server_locked = self.state.as_ref().lock().map_err(|err| {
            anyhow!(
                "Failed to lock InternalTestServer, for request {method} {path}, received {err:?}",
            )
        })?;

        let cookies = server_locked.cookies().clone();
        let mut query_params = server_locked.query_params().clone();
        let headers = server_locked.headers().clone();
        let mut full_request_url =
            build_url(url, path, &mut query_params, self.is_http_path_restricted)?;

        if let Some(scheme) = server_locked.scheme() {
            full_request_url.set_scheme(scheme).map_err(|_| {
                let debug_request_format = RequestPathFormatter::new(&method, full_request_url.as_str(), Some(&query_params));
                anyhow!("Scheme '{scheme}' from TestServer cannot be set to request {debug_request_format}")
            })?;
        }

        ::std::mem::drop(server_locked);

        Ok(TestRequestConfig {
            is_saving_cookies: self.save_cookies,
            expected_state: self.expected_state,
            content_type: self.default_content_type.clone(),
            method,

            full_request_url,
            cookies,
            query_params,
            headers,
        })
    }
}

fn build_url(
    mut url: Url,
    path: &str,
    query_params: &mut QueryParamsStore,
    is_http_restricted: bool,
) -> Result<Url> {
    let path_uri = path.parse::<Uri>()?;

    // If there is a scheme, then this is an absolute path.
    if let Some(scheme) = path_uri.scheme_str() {
        if is_http_restricted {
            if has_different_schema(&url, &path_uri) || has_different_authority(&url, &path_uri) {
                return Err(anyhow!("Request disallowed for path '{path}', requests are only allowed to local server. Turn off 'restrict_requests_with_http_schema' to change this."));
            }
        } else {
            url.set_scheme(scheme)
                .map_err(|_| anyhow!("Failed to set scheme for request, with path '{path}'"))?;

            // We only set the host/port if the scheme is also present.
            if let Some(authority) = path_uri.authority() {
                url.set_host(Some(authority.host()))
                    .map_err(|_| anyhow!("Failed to set host for request, with path '{path}'"))?;
                url.set_port(authority.port().map(|p| p.as_u16()))
                    .map_err(|_| anyhow!("Failed to set port for request, with path '{path}'"))?;

                // todo, add username:password support
            }
        }
    }

    // Why does this exist?
    //
    // This exists to allow `server.get("/users")` and `server.get("users")` (without a slash)
    // to go to the same place.
    //
    // It does this by saying ...
    //  - if there is a scheme, it's a full path.
    //  - if no scheme, it must be a path
    //
    if is_absolute_uri(&path_uri) {
        url.set_path(path_uri.path());

        // In this path we are replacing, so drop any query params on the original url.
        if url.query().is_some() {
            url.set_query(None);
        }
    } else {
        // Grab everything up until the query parameters, or everything after that
        let calculated_path = path.split('?').next().unwrap_or(&path);
        url.set_path(calculated_path);

        // Move any query parameters from the url to the query params store.
        if let Some(url_query) = url.query() {
            query_params.add_raw(url_query.to_string());
            url.set_query(None);
        }
    }

    if let Some(path_query) = path_uri.query() {
        query_params.add_raw(path_query.to_string());
    }

    Ok(url)
}

fn is_absolute_uri(path_uri: &Uri) -> bool {
    path_uri.scheme_str().is_some()
}

fn has_different_schema(base_url: &Url, path_uri: &Uri) -> bool {
    if let Some(scheme) = path_uri.scheme_str() {
        return scheme != base_url.scheme();
    }

    false
}

fn has_different_authority(base_url: &Url, path_uri: &Uri) -> bool {
    if let Some(authority) = path_uri.authority() {
        return authority.as_str() != base_url.authority();
    }

    false
}

#[cfg(test)]
mod test_build_url {
    use super::*;

    #[test]
    fn it_should_copy_path_to_url_returned_when_restricted() {
        let base_url = "http://example.com".parse::<Url>().unwrap();
        let path = "/users";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, true).unwrap();

        assert_eq!("http://example.com/users", result.as_str());
        assert!(query_params.is_empty());
    }

    #[test]
    fn it_should_copy_all_query_params_to_store_when_restricted() {
        let base_url = "http://example.com?base=aaa".parse::<Url>().unwrap();
        let path = "/users?path=bbb&path-flag";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, true).unwrap();

        assert_eq!("http://example.com/users", result.as_str());
        assert_eq!("base=aaa&path=bbb&path-flag", query_params.to_string());
    }

    #[test]
    fn it_should_not_replace_url_when_restricted_with_different_scheme() {
        let base_url = "http://example.com?base=666".parse::<Url>().unwrap();
        let path = "ftp://google.com:123/users.csv?limit=456";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_not_replace_url_when_restricted_with_same_scheme() {
        let base_url = "http://example.com?base=666".parse::<Url>().unwrap();
        let path = "http://google.com:123/users.csv?limit=456";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_block_url_when_restricted_with_same_scheme() {
        let base_url = "http://example.com?base=666".parse::<Url>().unwrap();
        let path = "http://google.com";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_block_url_when_restricted_and_same_domain_with_different_scheme() {
        let base_url = "http://example.com?base=666".parse::<Url>().unwrap();
        let path = "ftp://example.com/users";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_copy_path_to_url_returned_when_unrestricted() {
        let base_url = "http://example.com".parse::<Url>().unwrap();
        let path = "/users";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, false).unwrap();

        assert_eq!("http://example.com/users", result.as_str());
        assert!(query_params.is_empty());
    }

    #[test]
    fn it_should_copy_all_query_params_to_store_when_unrestricted() {
        let base_url = "http://example.com?base=aaa".parse::<Url>().unwrap();
        let path = "/users?path=bbb&path-flag";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, false).unwrap();

        assert_eq!("http://example.com/users", result.as_str());
        assert_eq!("base=aaa&path=bbb&path-flag", query_params.to_string());
    }

    #[test]
    fn it_should_copy_host_like_a_path_when_unrestricted() {
        let base_url = "http://example.com".parse::<Url>().unwrap();
        let path = "google.com";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, false).unwrap();

        assert_eq!("http://example.com/google.com", result.as_str());
        assert!(query_params.is_empty());
    }

    #[test]
    fn it_should_copy_host_like_a_path_when_restricted() {
        let base_url = "http://example.com".parse::<Url>().unwrap();
        let path = "google.com";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, true).unwrap();

        assert_eq!("http://example.com/google.com", result.as_str());
        assert!(query_params.is_empty());
    }

    #[test]
    fn it_should_replace_url_when_unrestricted() {
        let base_url = "http://example.com?base=666".parse::<Url>().unwrap();
        let path = "ftp://google.com:123/users.csv?limit=456";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, false).unwrap();

        assert_eq!("ftp://google.com:123/users.csv", result.as_str());
        assert_eq!("limit=456", query_params.to_string());
    }

    #[test]
    fn it_should_allow_different_scheme_when_unrestricted() {
        let base_url = "http://example.com".parse::<Url>().unwrap();
        let path = "ftp://example.com";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, false).unwrap();

        assert_eq!("ftp://example.com/", result.as_str());
    }

    #[test]
    fn it_should_allow_different_host_when_unrestricted() {
        let base_url = "http://example.com".parse::<Url>().unwrap();
        let path = "http://google.com";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, false).unwrap();

        assert_eq!("http://google.com/", result.as_str());
    }

    #[test]
    fn it_should_allow_different_port_when_unrestricted() {
        let base_url = "http://example.com:123".parse::<Url>().unwrap();
        let path = "http://example.com:456";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, false).unwrap();

        assert_eq!("http://example.com:456/", result.as_str());
    }

    #[test]
    fn it_should_allow_same_host_port_when_unrestricted() {
        let base_url = "http://example.com:123".parse::<Url>().unwrap();
        let path = "http://example.com:123";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, false).unwrap();

        assert_eq!("http://example.com:123/", result.as_str());
    }

    #[test]
    fn it_should_not_allow_different_scheme_when_restricted() {
        let base_url = "http://example.com".parse::<Url>().unwrap();
        let path = "ftp://example.com";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_not_allow_different_host_when_restricted() {
        let base_url = "http://example.com".parse::<Url>().unwrap();
        let path = "http://google.com";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_not_allow_different_port_when_restricted() {
        let base_url = "http://example.com:123".parse::<Url>().unwrap();
        let path = "http://example.com:456";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_allow_same_host_port_when_restricted() {
        let base_url = "http://example.com:123".parse::<Url>().unwrap();
        let path = "http://example.com:123";
        let mut query_params = QueryParamsStore::new();
        let result = build_url(base_url, &path, &mut query_params, true).unwrap();

        assert_eq!("http://example.com:123/", result.as_str());
    }
}

#[cfg(test)]
mod test_new {
    use axum::routing::get;
    use axum::Router;
    use std::net::SocketAddr;

    use crate::TestServer;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    #[tokio::test]
    async fn it_should_run_into_make_into_service_with_connect_info_by_default() {
        // Build an application with a route.
        let app = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service_with_connect_info::<SocketAddr>();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }
}

#[cfg(test)]
mod test_get {
    use super::*;

    use axum::routing::get;
    use axum::Router;
    use reserve_port::ReservedSocketAddr;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    #[tokio::test]
    async fn it_should_get_using_relative_path_with_slash() {
        let app = Router::new().route("/ping", get(get_ping));
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request _with_ slash
        server.get(&"/ping").await.assert_text(&"pong!");
    }

    #[tokio::test]
    async fn it_should_get_using_relative_path_without_slash() {
        let app = Router::new().route("/ping", get(get_ping));
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request _without_ slash
        server.get(&"ping").await.assert_text(&"pong!");
    }

    #[tokio::test]
    async fn it_should_get_using_absolute_path() {
        // Build an application with a route.
        let app = Router::new().route("/ping", get(get_ping));

        // Reserve an address
        let reserved_address = ReservedSocketAddr::reserve_random_socket_addr().unwrap();
        let ip = reserved_address.ip();
        let port = reserved_address.port();

        // Run the server.
        let server = TestServer::builder()
            .http_transport_with_ip_port(Some(ip), Some(port))
            .build(app)
            .with_context(|| format!("Should create test server with address {}:{}", ip, port))
            .unwrap();

        // Get the request.
        let absolute_url = format!("http://{ip}:{port}/ping");
        let response = server.get(&absolute_url).await;

        response.assert_text(&"pong!");
        let request_path = response.request_url();
        assert_eq!(request_path.to_string(), format!("http://{ip}:{port}/ping"));
    }

    #[tokio::test]
    async fn it_should_get_using_absolute_path_and_restricted_if_path_is_for_server() {
        // Build an application with a route.
        let app = Router::new().route("/ping", get(get_ping));

        // Reserve an IP / Port
        let reserved_address = ReservedSocketAddr::reserve_random_socket_addr().unwrap();
        let ip = reserved_address.ip();
        let port = reserved_address.port();

        // Run the server.
        let server = TestServer::builder()
            .http_transport_with_ip_port(Some(ip), Some(port))
            .restrict_requests_with_http_schema() // Key part of the test!
            .build(app)
            .with_context(|| format!("Should create test server with address {}:{}", ip, port))
            .unwrap();

        // Get the request.
        let absolute_url = format!("http://{ip}:{port}/ping");
        let response = server.get(&absolute_url).await;

        response.assert_text(&"pong!");
        let request_path = response.request_url();
        assert_eq!(request_path.to_string(), format!("http://{ip}:{port}/ping"));
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_not_get_using_absolute_path_if_restricted_and_different_port() {
        // Build an application with a route.
        let app = Router::new().route("/ping", get(get_ping));

        // Reserve an IP / Port
        let reserved_address = ReservedSocketAddr::reserve_random_socket_addr().unwrap();
        let ip = reserved_address.ip();
        let mut port = reserved_address.port();

        // Run the server.
        let server = TestServer::builder()
            .http_transport_with_ip_port(Some(ip), Some(port))
            .restrict_requests_with_http_schema() // Key part of the test!
            .build(app)
            .with_context(|| format!("Should create test server with address {}:{}", ip, port))
            .unwrap();

        // Get the request.
        port += 1; // << Change the port to be off by one and not match the server
        let absolute_url = format!("http://{ip}:{port}/ping");
        server.get(&absolute_url).await;
    }

    #[tokio::test]
    async fn it_should_work_in_parallel() {
        let app = Router::new().route("/ping", get(get_ping));
        let server = TestServer::new(app).expect("Should create test server");

        let future1 = async { server.get("/ping").await };
        let future2 = async { server.get("/ping").await };
        let (r1, r2) = tokio::join!(future1, future2);

        assert_eq!(r1.text(), r2.text());
    }

    #[tokio::test]
    async fn it_should_work_in_parallel_with_sleeping_requests() {
        let app = axum::Router::new().route(
            &"/slow",
            axum::routing::get(|| async {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                "hello!"
            }),
        );

        let server = TestServer::new(app).expect("Should create test server");

        let future1 = async { server.get("/slow").await };
        let future2 = async { server.get("/slow").await };
        let (r1, r2) = tokio::join!(future1, future2);

        assert_eq!(r1.text(), r2.text());
    }
}

#[cfg(test)]
mod test_server_address {
    use super::*;

    use axum::Router;
    use local_ip_address::local_ip;
    use regex::Regex;
    use reserve_port::ReservedPort;

    #[tokio::test]
    async fn it_should_return_address_used_from_config() {
        let reserved_port = ReservedPort::random().unwrap();
        let ip = local_ip().unwrap();
        let port = reserved_port.port();

        // Build an application with a route.
        let app = Router::new();
        let server = TestServer::builder()
            .http_transport_with_ip_port(Some(ip), Some(port))
            .build(app)
            .with_context(|| format!("Should create test server with address {}:{}", ip, port))
            .unwrap();

        let expected_ip_port = format!("http://{}:{}/", ip, reserved_port.port());
        assert_eq!(
            server.server_address().unwrap().to_string(),
            expected_ip_port
        );
    }

    #[tokio::test]
    async fn it_should_return_default_address_without_ending_slash() {
        let app = Router::new();
        let server = TestServer::builder()
            .http_transport()
            .build(app)
            .expect("Should create test server");

        let address_regex = Regex::new("^http://127\\.0\\.0\\.1:[0-9]+/$").unwrap();
        let is_match = address_regex.is_match(&server.server_address().unwrap().to_string());
        assert!(is_match);
    }

    #[tokio::test]
    async fn it_should_return_none_on_mock_transport() {
        let app = Router::new();
        let server = TestServer::builder()
            .mock_transport()
            .build(app)
            .expect("Should create test server");

        assert!(server.server_address().is_none());
    }
}

#[cfg(test)]
mod test_server_url {
    use super::*;

    use axum::Router;
    use local_ip_address::local_ip;
    use regex::Regex;
    use reserve_port::ReservedPort;

    #[tokio::test]
    async fn it_should_return_address_with_url_on_http_ip_port() {
        let reserved_port = ReservedPort::random().unwrap();
        let ip = local_ip().unwrap();
        let port = reserved_port.port();

        // Build an application with a route.
        let app = Router::new();
        let server = TestServer::builder()
            .http_transport_with_ip_port(Some(ip), Some(port))
            .build(app)
            .with_context(|| format!("Should create test server with address {}:{}", ip, port))
            .unwrap();

        let expected_ip_port_url = format!("http://{}:{}/users", ip, reserved_port.port());
        let absolute_url = server.server_url("/users").unwrap().to_string();
        assert_eq!(absolute_url, expected_ip_port_url);
    }

    #[tokio::test]
    async fn it_should_return_address_with_url_on_random_http() {
        let app = Router::new();
        let server = TestServer::builder()
            .http_transport()
            .build(app)
            .expect("Should create test server");

        let address_regex =
            Regex::new("^http://127\\.0\\.0\\.1:[0-9]+/users/123\\?filter=enabled$").unwrap();
        let absolute_url = &server
            .server_url(&"/users/123?filter=enabled")
            .unwrap()
            .to_string();

        let is_match = address_regex.is_match(absolute_url);
        assert!(is_match);
    }

    #[tokio::test]
    async fn it_should_error_on_mock_transport() {
        // Build an application with a route.
        let app = Router::new();
        let server = TestServer::builder()
            .mock_transport()
            .build(app)
            .expect("Should create test server");

        let result = server.server_url("/users");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn it_should_include_path_query_params() {
        let reserved_port = ReservedPort::random().unwrap();
        let ip = local_ip().unwrap();
        let port = reserved_port.port();

        // Build an application with a route.
        let app = Router::new();
        let server = TestServer::builder()
            .http_transport_with_ip_port(Some(ip), Some(port))
            .build(app)
            .with_context(|| format!("Should create test server with address {}:{}", ip, port))
            .unwrap();

        let expected_url = format!(
            "http://{}:{}/users?filter=enabled",
            ip,
            reserved_port.port()
        );
        let received_url = server
            .server_url("/users?filter=enabled")
            .unwrap()
            .to_string();

        assert_eq!(received_url, expected_url);
    }

    #[tokio::test]
    async fn it_should_include_server_query_params() {
        let reserved_port = ReservedPort::random().unwrap();
        let ip = local_ip().unwrap();
        let port = reserved_port.port();

        // Build an application with a route.
        let app = Router::new();
        let mut server = TestServer::builder()
            .http_transport_with_ip_port(Some(ip), Some(port))
            .build(app)
            .with_context(|| format!("Should create test server with address {}:{}", ip, port))
            .unwrap();

        server.add_query_param("filter", "enabled");

        let expected_url = format!(
            "http://{}:{}/users?filter=enabled",
            ip,
            reserved_port.port()
        );
        let received_url = server.server_url("/users").unwrap().to_string();

        assert_eq!(received_url, expected_url);
    }

    #[tokio::test]
    async fn it_should_include_server_and_path_query_params() {
        let reserved_port = ReservedPort::random().unwrap();
        let ip = local_ip().unwrap();
        let port = reserved_port.port();

        // Build an application with a route.
        let app = Router::new();
        let mut server = TestServer::builder()
            .http_transport_with_ip_port(Some(ip), Some(port))
            .build(app)
            .with_context(|| format!("Should create test server with address {}:{}", ip, port))
            .unwrap();

        server.add_query_param("filter", "enabled");

        let expected_url = format!(
            "http://{}:{}/users?filter=enabled&animal=donkeys",
            ip,
            reserved_port.port()
        );
        let received_url = server
            .server_url("/users?animal=donkeys")
            .unwrap()
            .to_string();

        assert_eq!(received_url, expected_url);
    }
}

#[cfg(test)]
mod test_add_cookie {
    use crate::TestServer;

    use axum::routing::get;
    use axum::Router;
    use axum_extra::extract::cookie::CookieJar;
    use cookie::Cookie;

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
        let mut server = TestServer::new(app).expect("Should create test server");

        let cookie = Cookie::new(TEST_COOKIE_NAME, "my-custom-cookie");
        server.add_cookie(cookie);

        let response_text = server.get(&"/cookie").await.text();
        assert_eq!(response_text, "my-custom-cookie");
    }
}

#[cfg(test)]
mod test_add_cookies {
    use crate::TestServer;

    use axum::routing::get;
    use axum::Router;
    use axum_extra::extract::cookie::CookieJar as AxumCookieJar;
    use cookie::Cookie;
    use cookie::CookieJar;

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
        let mut server = TestServer::new(app).expect("Should create test server");

        // Build cookies to send up
        let cookie_1 = Cookie::new("first-cookie", "my-custom-cookie");
        let cookie_2 = Cookie::new("second-cookie", "other-cookie");
        let mut cookie_jar = CookieJar::new();
        cookie_jar.add(cookie_1);
        cookie_jar.add(cookie_2);

        server.add_cookies(cookie_jar);

        server
            .get(&"/cookies")
            .await
            .assert_text("first-cookie=my-custom-cookie, second-cookie=other-cookie");
    }
}

#[cfg(test)]
mod test_clear_cookies {
    use crate::TestServer;

    use axum::routing::get;
    use axum::Router;
    use axum_extra::extract::cookie::CookieJar as AxumCookieJar;
    use cookie::Cookie;
    use cookie::CookieJar;

    async fn route_get_cookies(cookies: AxumCookieJar) -> String {
        let mut all_cookies = cookies
            .iter()
            .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
            .collect::<Vec<String>>();
        all_cookies.sort();

        all_cookies.join(&", ")
    }

    #[tokio::test]
    async fn it_should_not_send_cookies_cleared() {
        let app = Router::new().route("/cookies", get(route_get_cookies));
        let mut server = TestServer::new(app).expect("Should create test server");

        let cookie_1 = Cookie::new("first-cookie", "my-custom-cookie");
        let cookie_2 = Cookie::new("second-cookie", "other-cookie");
        let mut cookie_jar = CookieJar::new();
        cookie_jar.add(cookie_1);
        cookie_jar.add(cookie_2);

        server.add_cookies(cookie_jar);

        // The important bit of this test
        server.clear_cookies();

        server.get(&"/cookies").await.assert_text("");
    }
}

#[cfg(test)]
mod test_add_header {
    use super::*;

    use axum::async_trait;
    use axum::extract::FromRequestParts;
    use axum::routing::get;
    use axum::Router;
    use http::request::Parts;
    use http::HeaderName;
    use http::HeaderValue;
    use hyper::StatusCode;
    use std::marker::Sync;

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
    async fn it_should_send_header_added_to_server() {
        // Build an application with a route.
        let app = Router::new().route("/header", get(ping_header));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_header(
            HeaderName::from_static(TEST_HEADER_NAME),
            HeaderValue::from_static(TEST_HEADER_CONTENT),
        );

        // Send a request with the header
        let response = server.get(&"/header").await;

        // Check it sent back the right text
        response.assert_text(TEST_HEADER_CONTENT)
    }
}

#[cfg(test)]
mod test_clear_headers {
    use super::*;

    use axum::async_trait;
    use axum::extract::FromRequestParts;
    use axum::routing::get;
    use axum::Router;
    use http::request::Parts;
    use http::HeaderName;
    use http::HeaderValue;
    use hyper::StatusCode;
    use std::marker::Sync;

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
    async fn it_should_not_send_headers_cleared_by_server() {
        // Build an application with a route.
        let app = Router::new().route("/header", get(ping_header));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_header(
            HeaderName::from_static(TEST_HEADER_NAME),
            HeaderValue::from_static(TEST_HEADER_CONTENT),
        );
        server.clear_headers();

        // Send a request with the header
        let response = server.get(&"/header").await;

        // Check it sent back the right text
        response.assert_status_bad_request();
        response.assert_text("Missing test header");
    }
}

#[cfg(test)]
mod test_add_query_params {
    use axum::extract::Query;
    use axum::routing::get;
    use axum::Router;

    use serde::Deserialize;
    use serde::Serialize;
    use serde_json::json;

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
        let app = Router::new().route("/query", get(get_query_param));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(QueryParam {
            message: "it works".to_string(),
        });

        // Get the request.
        server.get(&"/query").await.assert_text(&"it works");
    }

    #[tokio::test]
    async fn it_should_pass_up_query_params_from_pairs() {
        // Build an application with a route.
        let app = Router::new().route("/query", get(get_query_param));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(&[("message", "it works")]);

        // Get the request.
        server.get(&"/query").await.assert_text(&"it works");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_multiple_params() {
        // Build an application with a route.
        let app = Router::new().route("/query-2", get(get_query_param_2));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(&[("message", "it works"), ("other", "yup")]);

        // Get the request.
        server.get(&"/query-2").await.assert_text(&"it works-yup");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_multiple_calls() {
        // Build an application with a route.
        let app = Router::new().route("/query-2", get(get_query_param_2));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(&[("message", "it works")]);
        server.add_query_params(&[("other", "yup")]);

        // Get the request.
        server.get(&"/query-2").await.assert_text(&"it works-yup");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_json() {
        // Build an application with a route.
        let app = Router::new().route("/query-2", get(get_query_param_2));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(json!({
            "message": "it works",
            "other": "yup"
        }));

        // Get the request.
        server.get(&"/query-2").await.assert_text(&"it works-yup");
    }
}

#[cfg(test)]
mod test_add_query_param {
    use axum::extract::Query;
    use axum::routing::get;
    use axum::Router;

    use serde::Deserialize;
    use serde::Serialize;

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
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_param("message", "it works");

        // Get the request.
        server.get(&"/query").await.assert_text(&"it works");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_multiple_calls() {
        // Build an application with a route.
        let app = Router::new().route("/query-2", get(get_query_param_2));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_param("message", "it works");
        server.add_query_param("other", "yup");

        // Get the request.
        server.get(&"/query-2").await.assert_text(&"it works-yup");
    }

    #[tokio::test]
    async fn it_should_pass_up_multiple_query_params_from_calls_across_server_and_request() {
        // Build an application with a route.
        let app = Router::new().route("/query-2", get(get_query_param_2));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_param("message", "it works");

        // Get the request.
        server
            .get(&"/query-2")
            .add_query_param("other", "yup")
            .await
            .assert_text(&"it works-yup");
    }
}

#[cfg(test)]
mod test_add_raw_query_param {
    use axum::extract::Query as AxumStdQuery;
    use axum::routing::get;
    use axum::Router;
    use axum_extra::extract::Query as AxumExtraQuery;
    use serde::Deserialize;
    use serde::Serialize;
    use std::fmt::Write;

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
        let mut server = TestServer::new(build_app()).expect("Should create test server");
        server.add_raw_query_param(&"message=it-works");

        // Get the request.
        server.get(&"/query").await.assert_text(&"it-works");
    }

    #[tokio::test]
    async fn it_should_pass_up_array_query_params_as_one_string() {
        // Run the server.
        let mut server = TestServer::new(build_app()).expect("Should create test server");
        server.add_raw_query_param(&"items=one&items=two&items=three");

        // Get the request.
        server
            .get(&"/query-extra")
            .await
            .assert_text(&"one, two, three");
    }

    #[tokio::test]
    async fn it_should_pass_up_array_query_params_as_multiple_params() {
        // Run the server.
        let mut server = TestServer::new(build_app()).expect("Should create test server");
        server.add_raw_query_param(&"arrs[]=one");
        server.add_raw_query_param(&"arrs[]=two");
        server.add_raw_query_param(&"arrs[]=three");

        // Get the request.
        server
            .get(&"/query-extra")
            .await
            .assert_text(&"one, two, three");
    }
}

#[cfg(test)]
mod test_clear_query_params {
    use axum::extract::Query;
    use axum::routing::get;
    use axum::Router;

    use serde::Deserialize;
    use serde::Serialize;

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
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(QueryParams {
            first: Some("first".to_string()),
            second: Some("second".to_string()),
        });
        server.clear_query_params();

        // Get the request.
        server
            .get(&"/query")
            .await
            .assert_text(&"has first? false, has second? false");
    }

    #[tokio::test]
    async fn it_should_clear_all_params_set_and_allow_replacement() {
        // Build an application with a route.
        let app = Router::new().route("/query", get(get_query_params));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.add_query_params(QueryParams {
            first: Some("first".to_string()),
            second: Some("second".to_string()),
        });
        server.clear_query_params();
        server.add_query_params(QueryParams {
            first: Some("first".to_string()),
            second: Some("second".to_string()),
        });

        // Get the request.
        server
            .get(&"/query")
            .await
            .assert_text(&"has first? true, has second? true");
    }
}

#[cfg(test)]
mod test_expect_success_by_default {
    use super::*;

    use axum::routing::get;
    use axum::Router;

    #[tokio::test]
    async fn it_should_not_panic_by_default_if_accessing_404_route() {
        let app = Router::new();
        let server = TestServer::new(app).expect("Should create test server");

        server.get(&"/some_unknown_route").await;
    }

    #[tokio::test]
    async fn it_should_not_panic_by_default_if_accessing_200_route() {
        let app = Router::new().route("/known_route", get(|| async { "🦊🦊🦊" }));
        let server = TestServer::new(app).expect("Should create test server");

        server.get(&"/known_route").await;
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_by_default_if_accessing_404_route_and_expect_success_on() {
        let app = Router::new();
        let server = TestServer::builder()
            .expect_success_by_default()
            .build(app)
            .expect("Should create test server");

        server.get(&"/some_unknown_route").await;
    }

    #[tokio::test]
    async fn it_should_not_panic_by_default_if_accessing_200_route_and_expect_success_on() {
        let app = Router::new().route("/known_route", get(|| async { "🦊🦊🦊" }));
        let server = TestServer::builder()
            .expect_success_by_default()
            .build(app)
            .expect("Should create test server");

        server.get(&"/known_route").await;
    }
}

#[cfg(test)]
mod test_content_type {
    use super::*;

    use axum::routing::get;
    use axum::Router;
    use http::header::CONTENT_TYPE;
    use http::HeaderMap;

    async fn get_content_type(headers: HeaderMap) -> String {
        headers
            .get(CONTENT_TYPE)
            .map(|h| h.to_str().unwrap().to_string())
            .unwrap_or_else(|| "".to_string())
    }

    #[tokio::test]
    async fn it_should_default_to_server_content_type_when_present() {
        // Build an application with a route.
        let app = Router::new().route("/content_type", get(get_content_type));

        // Run the server.
        let server = TestServer::builder()
            .default_content_type("text/plain")
            .build(app)
            .expect("Should create test server");

        // Get the request.
        let text = server.get(&"/content_type").await.text();

        assert_eq!(text, "text/plain");
    }
}

#[cfg(test)]
mod test_expect_success {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Router;
    use http::StatusCode;

    #[tokio::test]
    async fn it_should_not_panic_if_success_is_returned() {
        async fn get_ping() -> &'static str {
            "pong!"
        }

        // Build an application with a route.
        let app = Router::new().route("/ping", get(get_ping));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.expect_success();

        // Get the request.
        server.get(&"/ping").await;
    }

    #[tokio::test]
    async fn it_should_not_panic_on_other_2xx_status_code() {
        async fn get_accepted() -> StatusCode {
            StatusCode::ACCEPTED
        }

        // Build an application with a route.
        let app = Router::new().route("/accepted", get(get_accepted));

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.expect_success();

        // Get the request.
        server.get(&"/accepted").await;
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_on_404() {
        // Build an application with a route.
        let app = Router::new();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.expect_success();

        // Get the request.
        server.get(&"/some_unknown_route").await;
    }
}

#[cfg(test)]
mod test_expect_failure {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Router;
    use http::StatusCode;

    #[tokio::test]
    async fn it_should_not_panic_if_expect_failure_on_404() {
        // Build an application with a route.
        let app = Router::new();

        // Run the server.
        let mut server = TestServer::new(app).expect("Should create test server");
        server.expect_failure();

        // Get the request.
        server.get(&"/some_unknown_route").await;
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
        let mut server = TestServer::new(app).expect("Should create test server");
        server.expect_failure();

        // Get the request.
        server.get(&"/ping").await;
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
        let mut server = TestServer::new(app).expect("Should create test server");
        server.expect_failure();

        // Get the request.
        server.get(&"/accepted").await;
    }
}

#[cfg(test)]
mod test_scheme {
    use axum::extract::Request;
    use axum::routing::get;
    use axum::Router;

    use crate::TestServer;

    async fn route_get_scheme(request: Request) -> String {
        request.uri().scheme_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn it_should_return_http_by_default() {
        let router = Router::new().route("/scheme", get(route_get_scheme));
        let server = TestServer::builder().build(router).unwrap();

        server.get("/scheme").await.assert_text("http");
    }

    #[tokio::test]
    async fn it_should_return_https_across_multiple_requests_when_set() {
        let router = Router::new().route("/scheme", get(route_get_scheme));
        let mut server = TestServer::builder().build(router).unwrap();
        server.scheme(&"https");

        server.get("/scheme").await.assert_text("https");
    }
}

#[cfg(feature = "typed-routing")]
#[cfg(test)]
mod test_typed_get {
    use super::*;

    use axum::Router;
    use axum_extra::routing::RouterExt;
    use serde::Deserialize;

    #[derive(TypedPath, Deserialize)]
    #[typed_path("/path/:id")]
    struct TestingPath {
        id: u32,
    }

    async fn route_get(TestingPath { id }: TestingPath) -> String {
        format!("get {id}")
    }

    fn new_app() -> Router {
        Router::new().typed_get(route_get)
    }

    #[tokio::test]
    async fn it_should_send_get() {
        let server = TestServer::new(new_app()).unwrap();

        server
            .typed_get(&TestingPath { id: 123 })
            .await
            .assert_text("get 123");
    }
}

#[cfg(feature = "typed-routing")]
#[cfg(test)]
mod test_typed_post {
    use super::*;

    use axum::Router;
    use axum_extra::routing::RouterExt;
    use serde::Deserialize;

    #[derive(TypedPath, Deserialize)]
    #[typed_path("/path/:id")]
    struct TestingPath {
        id: u32,
    }

    async fn route_post(TestingPath { id }: TestingPath) -> String {
        format!("post {id}")
    }

    fn new_app() -> Router {
        Router::new().typed_post(route_post)
    }

    #[tokio::test]
    async fn it_should_send_post() {
        let server = TestServer::new(new_app()).unwrap();

        server
            .typed_post(&TestingPath { id: 123 })
            .await
            .assert_text("post 123");
    }
}

#[cfg(feature = "typed-routing")]
#[cfg(test)]
mod test_typed_patch {
    use super::*;

    use axum::Router;
    use axum_extra::routing::RouterExt;
    use serde::Deserialize;

    #[derive(TypedPath, Deserialize)]
    #[typed_path("/path/:id")]
    struct TestingPath {
        id: u32,
    }

    async fn route_patch(TestingPath { id }: TestingPath) -> String {
        format!("patch {id}")
    }

    fn new_app() -> Router {
        Router::new().typed_patch(route_patch)
    }

    #[tokio::test]
    async fn it_should_send_patch() {
        let server = TestServer::new(new_app()).unwrap();

        server
            .typed_patch(&TestingPath { id: 123 })
            .await
            .assert_text("patch 123");
    }
}

#[cfg(feature = "typed-routing")]
#[cfg(test)]
mod test_typed_put {
    use super::*;

    use axum::Router;
    use axum_extra::routing::RouterExt;
    use serde::Deserialize;

    #[derive(TypedPath, Deserialize)]
    #[typed_path("/path/:id")]
    struct TestingPath {
        id: u32,
    }

    async fn route_put(TestingPath { id }: TestingPath) -> String {
        format!("put {id}")
    }

    fn new_app() -> Router {
        Router::new().typed_put(route_put)
    }

    #[tokio::test]
    async fn it_should_send_put() {
        let server = TestServer::new(new_app()).unwrap();

        server
            .typed_put(&TestingPath { id: 123 })
            .await
            .assert_text("put 123");
    }
}

#[cfg(feature = "typed-routing")]
#[cfg(test)]
mod test_typed_delete {
    use super::*;

    use axum::Router;
    use axum_extra::routing::RouterExt;
    use serde::Deserialize;

    #[derive(TypedPath, Deserialize)]
    #[typed_path("/path/:id")]
    struct TestingPath {
        id: u32,
    }

    async fn route_delete(TestingPath { id }: TestingPath) -> String {
        format!("delete {id}")
    }

    fn new_app() -> Router {
        Router::new().typed_delete(route_delete)
    }

    #[tokio::test]
    async fn it_should_send_delete() {
        let server = TestServer::new(new_app()).unwrap();

        server
            .typed_delete(&TestingPath { id: 123 })
            .await
            .assert_text("delete 123");
    }
}

#[cfg(feature = "typed-routing")]
#[cfg(test)]
mod test_typed_method {
    use super::*;

    use axum::Router;
    use axum_extra::routing::RouterExt;
    use serde::Deserialize;

    #[derive(TypedPath, Deserialize)]
    #[typed_path("/path/:id")]
    struct TestingPath {
        id: u32,
    }

    async fn route_get(TestingPath { id }: TestingPath) -> String {
        format!("get {id}")
    }

    async fn route_post(TestingPath { id }: TestingPath) -> String {
        format!("post {id}")
    }

    async fn route_patch(TestingPath { id }: TestingPath) -> String {
        format!("patch {id}")
    }

    async fn route_put(TestingPath { id }: TestingPath) -> String {
        format!("put {id}")
    }

    async fn route_delete(TestingPath { id }: TestingPath) -> String {
        format!("delete {id}")
    }

    fn new_app() -> Router {
        Router::new()
            .typed_get(route_get)
            .typed_post(route_post)
            .typed_patch(route_patch)
            .typed_put(route_put)
            .typed_delete(route_delete)
    }

    #[tokio::test]
    async fn it_should_send_get() {
        let server = TestServer::new(new_app()).unwrap();

        server
            .typed_method(Method::GET, &TestingPath { id: 123 })
            .await
            .assert_text("get 123");
    }

    #[tokio::test]
    async fn it_should_send_post() {
        let server = TestServer::new(new_app()).unwrap();

        server
            .typed_method(Method::POST, &TestingPath { id: 123 })
            .await
            .assert_text("post 123");
    }

    #[tokio::test]
    async fn it_should_send_patch() {
        let server = TestServer::new(new_app()).unwrap();

        server
            .typed_method(Method::PATCH, &TestingPath { id: 123 })
            .await
            .assert_text("patch 123");
    }

    #[tokio::test]
    async fn it_should_send_put() {
        let server = TestServer::new(new_app()).unwrap();

        server
            .typed_method(Method::PUT, &TestingPath { id: 123 })
            .await
            .assert_text("put 123");
    }

    #[tokio::test]
    async fn it_should_send_delete() {
        let server = TestServer::new(new_app()).unwrap();

        server
            .typed_method(Method::DELETE, &TestingPath { id: 123 })
            .await
            .assert_text("delete 123");
    }
}

#[cfg(test)]
mod test_sync {
    use super::*;
    use axum::routing::get;
    use axum::Router;
    use std::cell::OnceCell;

    #[tokio::test]
    async fn it_should_be_able_to_be_in_one_cell() {
        let cell: OnceCell<TestServer> = OnceCell::new();
        let server = cell.get_or_init(|| {
            async fn route_get() -> &'static str {
                "it works"
            }

            let router = Router::new().route("/test", get(route_get));

            TestServer::new(router).unwrap()
        });

        server.get("/test").await.assert_text("it works");
    }
}
