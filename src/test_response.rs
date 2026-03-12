use crate::internals::DebugResponseBody;
use crate::internals::ErrorMessage;
use crate::internals::RequestPathFormatter;
use crate::internals::StatusCodeFormatter;
use crate::internals::StatusCodeRangeFormatter;
use crate::internals::TryIntoRangeBounds;
use bytes::Bytes;
use cookie::Cookie;
use cookie::CookieJar;
use expect_json::expect;
use expect_json::expect_json_eq;
use http::HeaderMap;
use http::HeaderValue;
use http::Method;
use http::StatusCode;
use http::Version;
use http::header::HeaderName;
use http::header::SET_COOKIE;
use http::response::Parts;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::convert::AsRef;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::fs::File;
use std::fs::read_to_string;
use std::io::BufReader;
use std::ops::RangeBounds;
use std::path::Path;

#[cfg(feature = "pretty-assertions")]
use pretty_assertions::{assert_eq, assert_ne};

#[cfg(feature = "ws")]
use crate::TestWebSocket;
#[cfg(feature = "ws")]
use crate::internals::TestResponseWebSocket;
use http::Uri;

///
/// The `TestResponse` is the result of a request created using a [`TestServer`](crate::TestServer).
/// The `TestServer` builds a [`TestRequest`](crate::TestRequest), which when awaited,
/// will produce the response.
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use axum::Json;
/// use axum::Router;
/// use axum::routing::get;
/// use axum_test::TestServer;
///
/// let app = Router::new()
///     .route(&"/test", get(|| async { "hello!" }));
///
/// let server = TestServer::new(app);
///
/// // This builds a `TestResponse`
/// let response = server.get(&"/todo").await;
/// #
/// # Ok(())
/// # }
/// ```
///
/// # Extracting Response
///
/// The functions [`TestResponse::json()`](crate::TestResponse::json()), [`TestResponse::text()`](crate::TestResponse::text()),
/// and [`TestResponse::form()`](crate::TestResponse::form()),
/// allow you to extract the underlying response content in different formats.
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// # use axum::Json;
/// # use axum::Router;
/// # use axum::routing::get;
/// # use serde::Deserialize;
/// # use serde::Serialize;
/// # use axum_test::TestServer;
/// #
/// # #[derive(Serialize, Deserialize, Debug)]
/// # struct Todo {}
/// #
/// # let app = Router::new()
/// #     .route(&"/test", get(|| async { "hello!" }));
/// #
/// # let server = TestServer::new(app);
/// let todo_response = server.get(&"/todo")
///         .await
///         .json::<Todo>();
///
/// let response_as_raw_text = server.get(&"/todo")
///         .await
///         .text();
/// #
/// # Ok(())
/// # }
/// ```
///
/// [`TestResponse::as_bytes()`](crate::TestResponse::as_bytes()) and [`TestResponse::into_bytes()`](crate::TestResponse::into_bytes()),
/// offer the underlying raw bytes to allow custom decoding.
///
/// Full code examples can be found within their documentation.
///
/// # Assertions
///
/// The result of a response can be asserted using the many `assert_*`
/// methods.
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use axum::Json;
/// use axum::Router;
/// use axum_test::TestServer;
/// use axum::routing::get;
///
/// let app = Router::new()
///     .route(&"/test", get(|| async { "hello!" }));
///
/// let server = TestServer::new(app);
///
/// let response = server.get(&"/todo").await;
///
/// // These assertions will panic if they are not fulfilled by the response.
/// response.assert_status_ok();
/// response.assert_text("hello!");
/// #
/// # Ok(())
/// # }
/// ```
///
/// These methods all return `&self` to allow chaining:
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// # use axum::*;
/// # use axum_test::TestServer;
/// # use axum::routing::get;
/// #
/// # let app = Router::new()
/// #      .route(&"/test", get(|| async { "hello!" }));
/// #
/// # let server = TestServer::new(app);
/// # let response = server.get(&"/todo").await;
/// #
/// response
///     .assert_status_ok()
///     .assert_text("hello!");
/// #
/// # Ok(())
/// # }
/// ```
///
#[derive(Debug, Clone)]
pub struct TestResponse {
    version: Version,
    method: Method,

    /// This is the actual url that was used for the request.
    full_request_url: Uri,
    headers: HeaderMap<HeaderValue>,
    status_code: StatusCode,
    response_body: Bytes,

    #[cfg(feature = "ws")]
    websockets: TestResponseWebSocket,
}

impl TestResponse {
    pub(crate) fn new(
        version: Version,
        method: Method,
        full_request_url: Uri,
        response_parts: Parts,
        response_body: Bytes,
        #[cfg(feature = "ws")] websockets: TestResponseWebSocket,
    ) -> Self {
        Self {
            version,
            method,
            full_request_url,
            headers: response_parts.headers,
            status_code: response_parts.status,
            response_body,

            #[cfg(feature = "ws")]
            websockets,
        }
    }

    /// Returns the underlying response, extracted as a UTF-8 string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Json;
    /// use axum::Router;
    /// use axum::routing::get;
    /// use serde_json::json;
    /// use serde_json::Value;
    ///
    /// use axum_test::TestServer;
    ///
    /// async fn route_get_todo() -> Json<Value> {
    ///     Json(json!({
    ///         "description": "buy milk",
    ///     }))
    /// }
    ///
    /// let app = Router::new()
    ///     .route(&"/todo", get(route_get_todo));
    ///
    /// let server = TestServer::new(app);
    /// let response = server.get(&"/todo").await;
    ///
    /// // Extract the response as a string on it's own.
    /// let raw_text = response.text();
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn text(&self) -> String {
        String::from_utf8_lossy(self.as_bytes()).to_string()
    }

    /// Deserializes the response, as Json, into the type given.
    ///
    /// If deserialization fails then this will panic.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Json;
    /// use axum::Router;
    /// use axum::routing::get;
    /// use serde::Deserialize;
    /// use serde::Serialize;
    ///
    /// use axum_test::TestServer;
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct Todo {
    ///     description: String,
    /// }
    ///
    /// async fn route_get_todo() -> Json<Todo> {
    ///     Json(Todo {
    ///         description: "buy milk".to_string(),
    ///     })
    /// }
    ///
    /// let app = Router::new()
    ///     .route(&"/todo", get(route_get_todo));
    ///
    /// let server = TestServer::new(app);
    /// let response = server.get(&"/todo").await;
    ///
    /// // Extract the response as a `Todo` item.
    /// let todo = response.json::<Todo>();
    /// #
    /// # Ok(())
    /// # }
    /// ```
    ///
    #[must_use]
    #[track_caller]
    pub fn json<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice::<T>(self.as_bytes())
            .error_response_with_body("Failed to deserialize Json response", self)
    }

    /// Deserializes the response, as Yaml, into the type given.
    ///
    /// If deserialization fails then this will panic.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum::routing::get;
    /// use axum_yaml::Yaml;
    /// use serde::Deserialize;
    /// use serde::Serialize;
    ///
    /// use axum_test::TestServer;
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct Todo {
    ///     description: String,
    /// }
    ///
    /// async fn route_get_todo() -> Yaml<Todo> {
    ///     Yaml(Todo {
    ///         description: "buy milk".to_string(),
    ///     })
    /// }
    ///
    /// let app = Router::new()
    ///     .route(&"/todo", get(route_get_todo));
    ///
    /// let server = TestServer::new(app);
    /// let response = server.get(&"/todo").await;
    ///
    /// // Extract the response as a `Todo` item.
    /// let todo = response.yaml::<Todo>();
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "yaml")]
    #[must_use]
    #[track_caller]
    pub fn yaml<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        serde_yaml::from_slice::<T>(self.as_bytes())
            .error_response_with_body("Failed to deserialize Yaml response", self)
    }

    /// Deserializes the response, as MsgPack, into the type given.
    ///
    /// If deserialization fails then this will panic.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum::routing::get;
    /// use axum_msgpack::MsgPack;
    /// use serde::Deserialize;
    /// use serde::Serialize;
    ///
    /// use axum_test::TestServer;
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct Todo {
    ///     description: String,
    /// }
    ///
    /// async fn route_get_todo() -> MsgPack<Todo> {
    ///     MsgPack(Todo {
    ///         description: "buy milk".to_string(),
    ///     })
    /// }
    ///
    /// let app = Router::new()
    ///     .route(&"/todo", get(route_get_todo));
    ///
    /// let server = TestServer::new(app);
    /// let response = server.get(&"/todo").await;
    ///
    /// // Extract the response as a `Todo` item.
    /// let todo = response.msgpack::<Todo>();
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "msgpack")]
    #[must_use]
    #[track_caller]
    pub fn msgpack<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        rmp_serde::from_slice::<T>(self.as_bytes())
            .error_response("Failed to deserialize Msgpack response", self)
    }

    /// Deserializes the response, as an urlencoded Form, into the type given.
    ///
    /// If deserialization fails then this will panic.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Form;
    /// use axum::Router;
    /// use axum::routing::get;
    /// use serde::Deserialize;
    /// use serde::Serialize;
    ///
    /// use axum_test::TestServer;
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct Todo {
    ///     description: String,
    /// }
    ///
    /// async fn route_get_todo() -> Form<Todo> {
    ///     Form(Todo {
    ///         description: "buy milk".to_string(),
    ///     })
    /// }
    ///
    /// let app = Router::new()
    ///     .route(&"/todo", get(route_get_todo));
    ///
    /// let server = TestServer::new(app);
    /// let response = server.get(&"/todo").await;
    ///
    /// // Extract the response as a `Todo` item.
    /// let todo = response.form::<Todo>();
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    #[track_caller]
    pub fn form<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        serde_urlencoded::from_bytes::<T>(self.as_bytes())
            .error_response_with_body("Failed to deserialize Form response", self)
    }

    /// Returns the raw underlying response as `Bytes`.
    #[must_use]
    pub fn as_bytes(&self) -> &Bytes {
        &self.response_body
    }

    /// Consumes this returning the underlying `Bytes`
    /// in the response.
    #[must_use]
    pub fn into_bytes(self) -> Bytes {
        self.response_body
    }

    /// The status_code of the response.
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    /// The Method used to produce this response.
    #[must_use]
    pub fn request_method(&self) -> Method {
        self.method.clone()
    }

    /// The URI that was used to produce this response.
    #[must_use]
    pub fn request_uri(&self) -> Uri {
        self.full_request_url.clone()
    }

    /// Finds a header with the given name.
    /// If there are multiple headers with the same name,
    /// then only the first [`HeaderValue`](::http::HeaderValue) will be returned.
    ///
    /// `None` is returned when no header was found.
    #[must_use]
    pub fn maybe_header<N>(&self, name: N) -> Option<HeaderValue>
    where
        N: TryInto<HeaderName>,
        N::Error: Debug,
    {
        let header_name = name
            .try_into()
            .expect("Failed to build HeaderName from name given");

        self.headers.get(header_name).map(|h| h.to_owned())
    }

    /// Returns the headers returned from the response.
    #[must_use]
    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.headers
    }

    #[must_use]
    #[track_caller]
    pub fn maybe_content_type(&self) -> Option<String> {
        self.headers.get(http::header::CONTENT_TYPE).map(|header| {
            header
                .to_str()
                .error_message_fn(|| {
                    format!("Failed to decode header CONTENT_TYPE, received '{header:?}'")
                })
                .to_string()
        })
    }

    #[must_use]
    pub fn content_type(&self) -> String {
        self.maybe_content_type()
            .expect("CONTENT_TYPE not found in response header")
    }

    /// Finds a header with the given name.
    /// If there are multiple headers with the same name,
    /// then only the first will be returned.
    ///
    /// If no header is found, then this will panic.
    #[must_use]
    #[track_caller]
    pub fn header<N>(&self, name: N) -> HeaderValue
    where
        N: TryInto<HeaderName> + Display + Clone,
        N::Error: Debug,
    {
        let debug_header = name.clone();
        let header_name = name
            .try_into()
            .expect("Failed to build HeaderName from name given, '{debug_header}'");
        self.headers
            .get(header_name)
            .map(|h| h.to_owned())
            .error_response_fn(|| format!("Cannot find header {debug_header}"), self)
    }

    /// Iterates over all of the headers contained in the response.
    pub fn iter_headers(&self) -> impl Iterator<Item = (&'_ HeaderName, &'_ HeaderValue)> {
        self.headers.iter()
    }

    /// Iterates over all of the headers for a specific name, contained in the response.
    pub fn iter_headers_by_name<N>(&self, name: N) -> impl Iterator<Item = &'_ HeaderValue>
    where
        N: TryInto<HeaderName>,
        N::Error: Debug,
    {
        let header_name = name
            .try_into()
            .expect("Failed to build HeaderName from name given");
        self.headers.get_all(header_name).iter()
    }

    #[must_use]
    pub fn contains_header<N>(&self, name: N) -> bool
    where
        N: TryInto<HeaderName>,
        N::Error: Debug,
    {
        let header_name = name
            .try_into()
            .expect("Failed to build HeaderName from name given");
        self.headers.contains_key(header_name)
    }

    /// Asserts the header named is present in the response.
    ///
    /// If the header is not present, then the assertion fails.
    #[track_caller]
    pub fn assert_contains_header<N>(&self, name: N) -> &Self
    where
        N: TryInto<HeaderName> + Display + Clone,
        N::Error: Debug,
    {
        let debug_header_name = name.clone();
        let debug_request_format = self.debug_request_format();
        let has_header = self.contains_header(name);

        assert!(
            has_header,
            "Expected header '{debug_header_name}' to be present in response, header was not found, for request {debug_request_format}"
        );

        self
    }

    #[track_caller]
    pub fn assert_header<N, V>(&self, name: N, value: V) -> &Self
    where
        N: TryInto<HeaderName> + Display + Clone,
        N::Error: Debug,
        V: TryInto<HeaderValue>,
        V::Error: Debug,
    {
        let debug_header_name = name.clone();
        let header_name = name
            .try_into()
            .expect("Failed to build HeaderName from name given");
        let expected_header_value = value
            .try_into()
            .expect("Could not turn given value into HeaderValue");
        let debug_request_format = self.debug_request_format();
        let maybe_found_header_value = self.maybe_header(header_name);

        match maybe_found_header_value {
            None => {
                panic!(
                    "Expected header '{debug_header_name}' to be present in response, header was not found, for request {debug_request_format}"
                )
            }
            Some(found_header_value) => {
                assert_eq!(expected_header_value, found_header_value,)
            }
        }

        self
    }

    /// Finds a [`Cookie`] with the given name.
    /// If there are multiple matching cookies,
    /// then only the first will be returned.
    ///
    /// `None` is returned if no Cookie is found.
    #[must_use]
    pub fn maybe_cookie(&self, cookie_name: &str) -> Option<Cookie<'static>> {
        for cookie in self.iter_cookies() {
            if cookie.name() == cookie_name {
                return Some(cookie.into_owned());
            }
        }

        None
    }

    /// Finds a [`Cookie`](::cookie::Cookie) with the given name.
    /// If there are multiple matching cookies,
    /// then only the first will be returned.
    ///
    /// If no `Cookie` is found, then this will panic.
    #[must_use]
    #[track_caller]
    pub fn cookie(&self, cookie_name: &str) -> Cookie<'static> {
        self.maybe_cookie(cookie_name)
            .error_response_fn(|| format!("Cannot find cookie {cookie_name}"), self)
    }

    /// Returns all of the cookies contained in the response,
    /// within a [`CookieJar`](::cookie::CookieJar) object.
    ///
    /// See the `cookie` crate for details.
    #[must_use]
    pub fn cookies(&self) -> CookieJar {
        let mut cookies = CookieJar::new();

        for cookie in self.iter_cookies() {
            cookies.add(cookie.into_owned());
        }

        cookies
    }

    /// Iterate over all of the cookies in the response.
    #[track_caller]
    pub fn iter_cookies(&self) -> impl Iterator<Item = Cookie<'_>> {
        self.iter_headers_by_name(SET_COOKIE).map(|header| {
            let header_str =
                header.to_str().error_message_fn(|| {
                    let debug_request_format = self.debug_request_format();

                    format!(
                        "Reading header 'Set-Cookie' as string, for request {debug_request_format}",
                    )
                });

            Cookie::parse(header_str).error_message_fn(|| {
                let debug_request_format = self.debug_request_format();

                format!("Parsing 'Set-Cookie' header, for request {debug_request_format}",)
            })
        })
    }

    /// Consumes the request, turning it into a `TestWebSocket`.
    /// If this cannot be done, then the response will panic.
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
    ///     .build(app);
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
    #[must_use]
    pub async fn into_websocket(self) -> TestWebSocket {
        use crate::transport_layer::TransportLayerType;

        // Using the mock approach will just fail.
        if self.websockets.transport_type != TransportLayerType::Http {
            panic!(
                "WebSocket requires a HTTP based transport layer, see `TestServerConfig::transport`"
            );
        }

        let debug_request_format = self.debug_request_format().to_string();

        let on_upgrade = self.websockets.maybe_on_upgrade
            .error_message_fn(|| {
                format!("Expected WebSocket upgrade to be found, it is None, for request {debug_request_format}")
            });

        let upgraded = on_upgrade.await.error_message_fn(|| {
            format!("Failed to upgrade connection for, for request {debug_request_format}")
        });

        TestWebSocket::new(upgraded).await
    }

    /// This performs an assertion comparing the whole body of the response,
    /// against the text provided.
    #[track_caller]
    pub fn assert_text<C>(&self, expected: C) -> &Self
    where
        C: AsRef<str>,
    {
        let expected_contents = expected.as_ref();
        assert_eq!(expected_contents, &self.text());

        self
    }

    /// This asserts if the text given is contained, somewhere, within the response.
    #[track_caller]
    pub fn assert_text_contains<C>(&self, expected: C) -> &Self
    where
        C: AsRef<str>,
    {
        let expected_contents = expected.as_ref();
        let received = self.text();
        let is_contained = received.contains(expected_contents);

        assert!(
            is_contained,
            "Failed to find '{expected_contents}', received '{received}'"
        );

        self
    }

    /// Asserts the response from the server matches the contents of the file.
    #[track_caller]
    pub fn assert_text_from_file<P>(&self, path: P) -> &Self
    where
        P: AsRef<Path>,
    {
        let path_ref = path.as_ref();
        let expected = read_to_string(path_ref)
            .error_message_fn(|| format!("Failed to read from file '{}'", path_ref.display()));

        self.assert_text(expected);

        self
    }

    /// Deserializes the contents of the request as Json,
    /// and asserts it matches the value given.
    ///
    /// If `other` does not match, or the response is not Json,
    /// then this will panic. Failing the assertion.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum::extract::Json;
    /// use axum::routing::get;
    /// use axum_test::TestServer;
    /// use serde_json::json;
    ///
    /// let app = Router::new()
    ///     .route(&"/user", get(|| async {
    ///         Json(json!({
    ///            "name": "Joe",
    ///            "age": 20,
    ///        }))
    ///     }));
    /// let server = TestServer::new(app);
    ///
    /// server.get(&"/user")
    ///     .await
    ///     .assert_json(&json!({
    ///         "name": "Joe",
    ///         "age": 20,
    ///     }));
    /// #
    /// # Ok(()) }
    /// ```
    ///
    /// This includes all of the abilities from [`crate::expect_json`],
    /// to allow you to check if things partially match.
    /// See that module for more information.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// # use axum::Router;
    /// # use axum::extract::Json;
    /// # use axum::routing::get;
    /// # use axum_test::TestServer;
    /// # use serde_json::json;
    /// #
    /// # let app = Router::new()
    /// #     .route(&"/user", get(|| async {
    /// #         Json(json!({
    /// #            "name": "Joe",
    /// #            "age": 20,
    /// #        }))
    /// #     }));
    /// # let server = TestServer::new(app);
    /// #
    /// // Validate aspects of the data, without needing the exact values
    /// server.get(&"/user")
    ///     .await
    ///     .assert_json(&json!({
    ///         "name": axum_test::expect_json::string(),
    ///         "age": axum_test::expect_json::integer().in_range(18..=30),
    ///     }));
    /// #
    /// # Ok(()) }
    /// ```
    #[track_caller]
    pub fn assert_json<T>(&self, expected: &T) -> &Self
    where
        T: Serialize + DeserializeOwned + PartialEq<T> + Debug,
    {
        let received = self.json::<T>();

        if *expected != received {
            if let Err(error) = expect_json_eq(&received, &expected) {
                panic!(
                    "
{error}
",
                );
            }
        }

        self
    }

    /// Asserts the content is within the json returned.
    /// This is useful for when servers return times and IDs that you
    /// wish to ignore.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Router;
    /// use axum::extract::Json;
    /// use axum::routing::get;
    /// use axum_test::TestServer;
    /// use serde_json::json;
    /// use std::time::Instant;
    ///
    /// let app = Router::new()
    ///     .route(&"/user", get(|| async {
    ///         let id = Instant::now().elapsed().as_millis();
    ///
    ///         Json(json!({
    ///            "id": id,
    ///            "name": "Joe",
    ///            "age": 20,
    ///        }))
    ///     }));
    /// let server = TestServer::new(app);
    ///
    /// // Checks the response contains _only_ the values listed here,
    /// // and ignores the rest.
    /// server.get(&"/user")
    ///     .await
    ///     .assert_json_contains(&json!({
    ///         "name": "Joe",
    ///         "age": 20,
    ///     }));
    /// #
    /// # Ok(()) }
    /// ```
    #[track_caller]
    pub fn assert_json_contains<T>(&self, expected: &T) -> &Self
    where
        T: Serialize,
    {
        let received = self.json::<Value>();
        let expected_value = serde_json::to_value(expected).unwrap();
        let result = expect_json_eq(
            &received,
            &expect::object().propagated_contains(expected_value),
        );

        if let Err(error) = result {
            panic!(
                "
{error}
",
            );
        }

        self
    }

    /// Read json file from given path and assert it with json response.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Json;
    /// use axum::routing::get;
    /// use axum::routing::Router;
    /// use axum_test::TestServer;
    /// use serde_json::json;
    ///
    /// let app = Router::new()
    ///     .route(&"/json", get(|| async {
    ///         Json(json!({
    ///             "name": "Joe",
    ///             "age": 20,
    ///         }))
    ///     }));
    ///
    /// let server = TestServer::new(app);
    /// server
    ///     .get(&"/json")
    ///     .await
    ///     .assert_json_from_file("files/example.json");
    /// #
    /// # Ok(()) }
    /// ```
    ///
    #[track_caller]
    pub fn assert_json_from_file<P>(&self, path: P) -> &Self
    where
        P: AsRef<Path>,
    {
        let path_ref = path.as_ref();
        let file = File::open(path_ref)
            .error_message_fn(|| format!("Failed to read from file '{}'", path_ref.display()));

        let reader = BufReader::new(file);
        let expected =
            serde_json::from_reader::<_, serde_json::Value>(reader).error_message_fn(|| {
                format!(
                    "Failed to deserialize file '{}' as json",
                    path_ref.display()
                )
            });

        self.assert_json(&expected);

        self
    }

    /// Deserializes the contents of the request as Yaml,
    /// and asserts it matches the value given.
    ///
    /// If `other` does not match, or the response is not Yaml,
    /// then this will panic.
    #[cfg(feature = "yaml")]
    #[track_caller]
    pub fn assert_yaml<T>(&self, other: &T) -> &Self
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*other, self.yaml::<T>());

        self
    }

    /// Read yaml file from given path and assert it with yaml response.
    #[cfg(feature = "yaml")]
    #[track_caller]
    pub fn assert_yaml_from_file<P>(&self, path: P) -> &Self
    where
        P: AsRef<Path>,
    {
        let path_ref = path.as_ref();
        let file = File::open(path_ref)
            .error_message_fn(|| format!("Failed to read from file '{}'", path_ref.display()));

        let reader = BufReader::new(file);
        let expected =
            serde_yaml::from_reader::<_, serde_yaml::Value>(reader).error_message_fn(|| {
                format!(
                    "Failed to deserialize file '{}' as yaml",
                    path_ref.display()
                )
            });

        self.assert_yaml(&expected);

        self
    }

    /// Deserializes the contents of the request as MsgPack,
    /// and asserts it matches the value given.
    ///
    /// If `other` does not match, or the response is not MsgPack,
    /// then this will panic.
    #[cfg(feature = "msgpack")]
    #[track_caller]
    pub fn assert_msgpack<T>(&self, other: &T) -> &Self
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*other, self.msgpack::<T>());

        self
    }

    /// Deserializes the contents of the request as an url encoded form,
    /// and asserts it matches the value given.
    ///
    /// If `other` does not match, or the response cannot be deserialized,
    /// then this will panic.
    #[track_caller]
    pub fn assert_form<T>(&self, other: &T) -> &Self
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*other, self.form::<T>());

        self
    }

    /// Assert the response status code matches the one given.
    #[track_caller]
    pub fn assert_status(&self, expected_status_code: StatusCode) -> &Self {
        let received_debug = StatusCodeFormatter(self.status_code);
        let expected_debug = StatusCodeFormatter(expected_status_code);
        let debug_request_format = self.debug_request_format();
        let debug_body = DebugResponseBody(self);

        assert_eq!(
            expected_status_code, self.status_code,
            "Expected status code to be {expected_debug}, received {received_debug}, for request {debug_request_format}, with body {debug_body}"
        );

        self
    }

    /// Assert the response status code does **not** match the one given.
    #[track_caller]
    pub fn assert_not_status(&self, expected_status_code: StatusCode) -> &Self {
        let received_debug = StatusCodeFormatter(self.status_code);
        let expected_debug = StatusCodeFormatter(expected_status_code);
        let debug_request_format = self.debug_request_format();
        let debug_body = DebugResponseBody(self);

        assert_ne!(
            expected_status_code, self.status_code,
            "Expected status code to not be {expected_debug}, received {received_debug}, for request {debug_request_format}, with body {debug_body}"
        );

        self
    }

    /// Assert that the status code is **within** the 2xx range.
    /// i.e. The range from 200-299.
    #[track_caller]
    pub fn assert_status_success(&self) -> &Self {
        let status_code = self.status_code.as_u16();
        let received_debug = StatusCodeFormatter(self.status_code);
        let debug_request_format = self.debug_request_format();
        let debug_body = DebugResponseBody(self);

        // TODO, improve the formatting on these to match error_message
        assert!(
            200 <= status_code && status_code <= 299,
            "Expect status code within 2xx range, received {received_debug}, for request {debug_request_format}, with body {debug_body}"
        );

        self
    }

    /// Assert that the status code is **outside** the 2xx range.
    /// i.e. A status code less than 200, or 300 or more.
    #[track_caller]
    pub fn assert_status_failure(&self) -> &Self {
        let status_code = self.status_code.as_u16();
        let received_debug = StatusCodeFormatter(self.status_code);
        let debug_request_format = self.debug_request_format();
        let debug_body = DebugResponseBody(self);

        assert!(
            status_code < 200 || 299 < status_code,
            "Expect status code outside 2xx range, received {received_debug}, for request {debug_request_format}, with body {debug_body}"
        );

        self
    }

    /// Assert the status code is within the range given.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Json;
    /// use axum::routing::get;
    /// use axum::routing::Router;
    /// use axum_test::TestServer;
    /// use http::StatusCode;
    ///
    /// let app = Router::new()
    ///     .route(&"/json", get(|| async {
    ///         StatusCode::OK
    ///     }));
    /// let server = TestServer::new(app);
    ///
    /// // Within success statuses
    /// server
    ///     .get(&"/json")
    ///     .await
    ///     .assert_status_in_range(200..=299);
    ///
    /// // Outside success
    /// server
    ///     .get(&"/json")
    ///     .await
    ///     .assert_status_in_range(300..);
    ///
    /// // Before server error
    /// server
    ///     .get(&"/json")
    ///     .await
    ///     .assert_status_in_range(..StatusCode::INTERNAL_SERVER_ERROR);
    /// #
    /// # Ok(()) }
    /// ```
    #[track_caller]
    pub fn assert_status_in_range<R, S>(&self, expected_status_range: R) -> &Self
    where
        R: RangeBounds<S> + TryIntoRangeBounds<StatusCode> + Debug,
        S: TryInto<StatusCode>,
    {
        let range = TryIntoRangeBounds::<StatusCode>::try_into_range_bounds(expected_status_range)
            .expect("Failed to convert status code");

        let status_code = self.status_code();
        let is_in_range = range.contains(&status_code);
        let debug_request_format = self.debug_request_format();
        let debug_body = DebugResponseBody(self);

        assert!(
            is_in_range,
            "Expected status to be in range {}, received {status_code}, for request {debug_request_format}, with body {debug_body}",
            StatusCodeRangeFormatter(range)
        );

        self
    }

    /// Assert the status code is not within the range given.
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use axum::Json;
    /// use axum::routing::get;
    /// use axum::routing::Router;
    /// use axum_test::TestServer;
    /// use http::StatusCode;
    ///
    /// let app = Router::new()
    ///     .route(&"/json", get(|| async {
    ///         StatusCode::NOT_FOUND
    ///     }));
    /// let server = TestServer::new(app);
    ///
    /// // Is not success
    /// server
    ///     .get(&"/json")
    ///     .await
    ///     .assert_status_not_in_range(200..=299);
    ///
    /// // 300 or higher
    /// server
    ///     .get(&"/json")
    ///     .await
    ///     .assert_status_not_in_range(300..);
    ///
    /// // After server error
    /// server
    ///     .get(&"/json")
    ///     .await
    ///     .assert_status_not_in_range(..StatusCode::INTERNAL_SERVER_ERROR);
    /// #
    /// # Ok(()) }
    /// ```
    #[track_caller]
    pub fn assert_status_not_in_range<R, S>(&self, expected_status_range: R) -> &Self
    where
        R: RangeBounds<S> + TryIntoRangeBounds<StatusCode> + Debug,
        S: TryInto<StatusCode>,
    {
        let range = TryIntoRangeBounds::<StatusCode>::try_into_range_bounds(expected_status_range)
            .expect("Failed to convert status code");

        let status_code = self.status_code();
        let is_not_in_range = !range.contains(&status_code);
        let debug_request_format = self.debug_request_format();
        let debug_body = DebugResponseBody(self);

        assert!(
            is_not_in_range,
            "Expected status is not in range {}, received {status_code}, for request {debug_request_format}, with body {debug_body}",
            StatusCodeRangeFormatter(range)
        );

        self
    }

    /// Assert the response status code is 200.
    #[track_caller]
    pub fn assert_status_ok(&self) -> &Self {
        self.assert_status(StatusCode::OK)
    }

    /// Assert the response status code is **not** 200.
    #[track_caller]
    pub fn assert_status_not_ok(&self) -> &Self {
        self.assert_not_status(StatusCode::OK)
    }

    /// Assert the response status code is 204.
    #[track_caller]
    pub fn assert_status_no_content(&self) -> &Self {
        self.assert_status(StatusCode::NO_CONTENT)
    }

    /// Assert the response status code is 303.
    #[track_caller]
    pub fn assert_status_see_other(&self) -> &Self {
        self.assert_status(StatusCode::SEE_OTHER)
    }

    /// Assert the response status code is 400.
    #[track_caller]
    pub fn assert_status_bad_request(&self) -> &Self {
        self.assert_status(StatusCode::BAD_REQUEST)
    }

    /// Assert the response status code is 404.
    #[track_caller]
    pub fn assert_status_not_found(&self) -> &Self {
        self.assert_status(StatusCode::NOT_FOUND)
    }

    /// Assert the response status code is 401.
    #[track_caller]
    pub fn assert_status_unauthorized(&self) -> &Self {
        self.assert_status(StatusCode::UNAUTHORIZED)
    }

    /// Assert the response status code is 403.
    #[track_caller]
    pub fn assert_status_forbidden(&self) -> &Self {
        self.assert_status(StatusCode::FORBIDDEN)
    }

    /// Assert the response status code is 409.
    pub fn assert_status_conflict(&self) -> &Self {
        self.assert_status(StatusCode::CONFLICT)
    }

    /// Assert the response status code is 413.
    ///
    /// The payload is too large.
    #[track_caller]
    pub fn assert_status_payload_too_large(&self) -> &Self {
        self.assert_status(StatusCode::PAYLOAD_TOO_LARGE)
    }

    /// Assert the response status code is 422.
    #[track_caller]
    pub fn assert_status_unprocessable_entity(&self) -> &Self {
        self.assert_status(StatusCode::UNPROCESSABLE_ENTITY)
    }

    /// Assert the response status code is 429.
    #[track_caller]
    pub fn assert_status_too_many_requests(&self) -> &Self {
        self.assert_status(StatusCode::TOO_MANY_REQUESTS)
    }

    /// Assert the response status code is 101.
    ///
    /// This type of code is used in Web Socket connection when
    /// first request.
    #[track_caller]
    pub fn assert_status_switching_protocols(&self) -> &Self {
        self.assert_status(StatusCode::SWITCHING_PROTOCOLS)
    }

    /// Assert the response status code is 500.
    #[track_caller]
    pub fn assert_status_internal_server_error(&self) -> &Self {
        self.assert_status(StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Assert the response status code is 503.
    #[track_caller]
    pub fn assert_status_service_unavailable(&self) -> &Self {
        self.assert_status(StatusCode::SERVICE_UNAVAILABLE)
    }

    pub(crate) fn debug_request_format(&self) -> RequestPathFormatter<'_, Uri> {
        RequestPathFormatter::new(&self.method, &self.full_request_url)
    }
}

impl From<TestResponse> for Bytes {
    fn from(response: TestResponse) -> Self {
        response.into_bytes()
    }
}

/// Prints out the full response. Including the status code, headers, and the body.
///
/// The output is very similar to a standard HTTP response, for use with snapshotting.
impl Display for TestResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let version_str = version_str(self.version);
        let status = self.status_code();
        let status_int = status.as_u16();
        let status_reason = status.canonical_reason().unwrap_or("");

        writeln!(f, "{version_str} {status_int} {status_reason}",)?;

        for (name, value) in self.headers() {
            writeln!(f, "{}: {}", name, value.to_str().unwrap_or("<binary>"))?;
        }

        writeln!(f)?;

        let body_raw = String::from_utf8_lossy(&self.response_body);
        writeln!(f, "{body_raw}")?;

        Ok(())
    }
}

fn version_str(version: Version) -> &'static str {
    match version {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2",
        Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/?",
    }
}

#[cfg(test)]
mod test_assert_header {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Router;
    use axum::http::HeaderMap;
    use axum::routing::get;

    async fn route_get_header() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-my-custom-header", "content".parse().unwrap());
        headers
    }

    #[tokio::test]
    async fn it_should_not_panic_if_contains_header_and_content_matches() {
        let router = Router::new().route(&"/header", get(route_get_header));

        let server = TestServer::new(router);

        server
            .get(&"/header")
            .await
            .assert_header("x-my-custom-header", "content");
    }

    #[tokio::test]
    async fn it_should_panic_if_contains_header_and_content_does_not_match() {
        let router = Router::new().route(&"/header", get(route_get_header));
        let server = TestServer::new(router);

        let response = server.get(&"/header").await;
        let message = catch_panic_error_message(|| {
            response.assert_header("x-my-custom-header", "different-content");
        });
        assert_error_message(
            r#"assertion failed: `(left == right)`

Diff < left / right > :
<"different-content"
>"content"

"#,
            message,
        );
    }

    #[tokio::test]
    async fn it_should_panic_if_not_contains_header() {
        let router = Router::new().route(&"/header", get(route_get_header));
        let server = TestServer::new(router);

        let response = server.get(&"/header").await;
        let message = catch_panic_error_message(|| {
            response.assert_header("x-custom-header-not-found", "content");
        });
        assert_error_message(
            "Expected header 'x-custom-header-not-found' to be present in response, header was not found, for request GET /header",
            message,
        );
    }
}

#[cfg(test)]
mod test_assert_contains_header {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Router;
    use axum::http::HeaderMap;
    use axum::routing::get;

    async fn route_get_header() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-my-custom-header", "content".parse().unwrap());
        headers
    }

    #[tokio::test]
    async fn it_should_not_panic_if_contains_header() {
        let router = Router::new().route(&"/header", get(route_get_header));

        let server = TestServer::new(router);

        server
            .get(&"/header")
            .await
            .assert_contains_header("x-my-custom-header");
    }

    #[tokio::test]
    async fn it_should_panic_if_not_contains_header() {
        let router = Router::new().route(&"/header", get(route_get_header));
        let server = TestServer::new(router);

        let response = server.get(&"/header").await;
        let message = catch_panic_error_message(|| {
            response.assert_contains_header("x-custom-header-not-found");
        });
        assert_error_message(
            "Expected header 'x-custom-header-not-found' to be present in response, header was not found, for request GET /header",
            message,
        );
    }
}

#[cfg(test)]
mod test_assert_success {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Router;
    use axum::routing::get;
    use http::StatusCode;

    pub async fn route_get_pass() -> StatusCode {
        StatusCode::OK
    }

    pub async fn route_get_fail() -> StatusCode {
        StatusCode::SERVICE_UNAVAILABLE
    }

    #[tokio::test]
    async fn it_should_pass_when_200() {
        let router = Router::new()
            .route(&"/pass", get(route_get_pass))
            .route(&"/fail", get(route_get_fail));

        let server = TestServer::new(router);

        let response = server.get(&"/pass").await;

        response.assert_status_success();
    }

    #[tokio::test]
    async fn it_should_panic_when_not_200() {
        let router = Router::new()
            .route(&"/pass", get(route_get_pass))
            .route(&"/fail", get(route_get_fail));

        let server = TestServer::new(router);
        let response = server.get(&"/fail").expect_failure().await;

        let message = catch_panic_error_message(|| {
            response.assert_status_success();
        });
        assert_error_message(
            "Expect status code within 2xx range, received 503 (Service Unavailable), for request GET /fail, with body ''",
            message,
        );
    }
}

#[cfg(test)]
mod test_assert_failure {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Router;
    use axum::routing::get;
    use http::StatusCode;

    pub async fn route_get_pass() -> StatusCode {
        StatusCode::OK
    }

    pub async fn route_get_fail() -> StatusCode {
        StatusCode::SERVICE_UNAVAILABLE
    }

    #[tokio::test]
    async fn it_should_pass_when_not_200() {
        let router = Router::new()
            .route(&"/pass", get(route_get_pass))
            .route(&"/fail", get(route_get_fail));

        let server = TestServer::new(router);
        let response = server.get(&"/fail").expect_failure().await;

        response.assert_status_failure();
    }

    #[tokio::test]
    async fn it_should_panic_when_200() {
        let router = Router::new()
            .route(&"/pass", get(route_get_pass))
            .route(&"/fail", get(route_get_fail));

        let server = TestServer::new(router);
        let response = server.get(&"/pass").await;

        let message = catch_panic_error_message(|| {
            response.assert_status_failure();
        });
        assert_error_message(
            "Expect status code outside 2xx range, received 200 (OK), for request GET /pass, with body ''",
            message,
        );
    }
}

#[cfg(test)]
mod test_assert_status {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Router;
    use axum::routing::get;
    use http::StatusCode;

    pub async fn route_get_ok() -> StatusCode {
        StatusCode::OK
    }

    #[tokio::test]
    async fn it_should_pass_if_given_right_status_code() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router);

        server.get(&"/ok").await.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn it_should_panic_when_status_code_does_not_match() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router);

        let response = server.get(&"/ok").await;
        let message = catch_panic_error_message(|| {
            response.assert_status(StatusCode::ACCEPTED);
        });
        assert_error_message("assertion failed: `(left == right)`: Expected status code to be 202 (Accepted), received 200 (OK), for request GET /ok, with body ''

Diff < left / right > :
<202
>200

", message);
    }
}

#[cfg(test)]
mod test_assert_not_status {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Router;
    use axum::routing::get;
    use http::StatusCode;

    pub async fn route_get_ok() -> StatusCode {
        StatusCode::OK
    }

    #[tokio::test]
    async fn it_should_pass_if_status_code_does_not_match() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router);

        server
            .get(&"/ok")
            .await
            .assert_not_status(StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn it_should_panic_if_status_code_matches() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router);

        let response = server.get(&"/ok").await;
        let message = catch_panic_error_message(|| {
            response.assert_not_status(StatusCode::OK);
        });
        assert_error_message("assertion failed: `(left != right)`: Expected status code to not be 200 (OK), received 200 (OK), for request GET /ok, with body ''

Both sides:
200

", message);
    }
}

#[cfg(test)]
mod test_assert_status_in_range {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::routing::Router;
    use axum::routing::get;
    use http::StatusCode;
    use std::ops::RangeFull;

    #[tokio::test]
    async fn it_should_be_true_when_within_int_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_in_range(200..299);
    }

    #[tokio::test]
    async fn it_should_be_true_when_within_status_code_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_in_range(StatusCode::OK..StatusCode::IM_USED);
    }

    #[tokio::test]
    async fn it_should_be_false_when_outside_int_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_in_range(200..299);
        });
        assert_error_message(
            "Expected status to be in range 200..299, received 500 Internal Server Error, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_false_when_outside_status_code_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_in_range(StatusCode::OK..StatusCode::IM_USED);
        });
        assert_error_message(
            "Expected status to be in range 200..226, received 500 Internal Server Error, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_true_when_within_inclusive_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_in_range(200..=299);
    }

    #[tokio::test]
    async fn it_should_be_false_when_outside_inclusive_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_in_range(200..=299);
        });
        assert_error_message(
            "Expected status to be in range 200..=299, received 500 Internal Server Error, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_true_when_within_to_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_in_range(..299);
    }

    #[tokio::test]
    async fn it_should_be_false_when_outside_to_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_in_range(..299);
        });
        assert_error_message(
            "Expected status to be in range ..299, received 500 Internal Server Error, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_true_when_within_to_inclusive_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_in_range(..=299);
    }

    #[tokio::test]
    async fn it_should_be_false_when_outside_to_inclusive_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_in_range(..=299);
        });
        assert_error_message(
            "Expected status to be in range ..=299, received 500 Internal Server Error, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_true_when_within_from_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_in_range(200..);
    }

    #[tokio::test]
    async fn it_should_be_false_when_outside_from_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_in_range(500..);
        });
        assert_error_message(
            "Expected status to be in range 500.., received 203 Non Authoritative Information, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_true_for_rull_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_in_range::<RangeFull, StatusCode>(..);
    }
}

#[cfg(test)]
mod test_assert_status_not_in_range {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::routing::Router;
    use axum::routing::get;
    use http::StatusCode;
    use std::ops::RangeFull;

    #[tokio::test]
    async fn it_should_be_false_when_within_int_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_not_in_range(200..299);
        });
        assert_error_message(
            "Expected status is not in range 200..299, received 203 Non Authoritative Information, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_false_when_within_status_code_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_not_in_range(StatusCode::OK..StatusCode::IM_USED);
        });
        assert_error_message(
            "Expected status is not in range 200..226, received 203 Non Authoritative Information, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_true_when_outside_int_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_not_in_range(200..299);
    }

    #[tokio::test]
    async fn it_should_be_true_when_outside_status_code_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_not_in_range(StatusCode::OK..StatusCode::IM_USED);
    }

    #[tokio::test]
    async fn it_should_be_false_when_within_inclusive_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_not_in_range(200..=299);
        });
        assert_error_message(
            "Expected status is not in range 200..=299, received 203 Non Authoritative Information, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_true_when_outside_inclusive_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_not_in_range(200..=299);
    }

    #[tokio::test]
    async fn it_should_be_false_when_within_to_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_not_in_range(..299);
        });
        assert_error_message(
            "Expected status is not in range ..299, received 203 Non Authoritative Information, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_true_when_outside_to_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_not_in_range(..299);
    }

    #[tokio::test]
    async fn it_should_be_false_when_within_to_inclusive_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_not_in_range(..=299);
        });
        assert_error_message(
            "Expected status is not in range ..=299, received 203 Non Authoritative Information, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_true_when_outside_to_inclusive_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_not_in_range(..=299);
    }

    #[tokio::test]
    async fn it_should_be_false_when_within_from_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_not_in_range(200..);
        });
        assert_error_message(
            "Expected status is not in range 200.., received 203 Non Authoritative Information, for request GET /status, with body ''",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_be_true_when_outside_from_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        TestServer::new(app)
            .get(&"/status")
            .await
            .assert_status_not_in_range(500..);
    }

    #[tokio::test]
    async fn it_should_be_false_for_rull_range() {
        let app = Router::new().route(
            &"/status",
            get(|| async { StatusCode::NON_AUTHORITATIVE_INFORMATION }),
        );

        let response = TestServer::new(app).get(&"/status").await;
        let message = catch_panic_error_message(|| {
            response.assert_status_not_in_range::<RangeFull, StatusCode>(..);
        });
        assert_error_message(
            "Expected status is not in range .., received 203 Non Authoritative Information, for request GET /status, with body ''",
            message,
        );
    }
}

#[cfg(test)]
mod test_into_bytes {
    use crate::TestServer;
    use axum::Json;
    use axum::Router;
    use axum::routing::get;
    use serde_json::Value;
    use serde_json::json;

    async fn route_get_json() -> Json<Value> {
        Json(json!({
            "message": "it works?"
        }))
    }

    #[tokio::test]
    async fn it_should_deserialize_into_json() {
        let app = Router::new().route(&"/json", get(route_get_json));
        let server = TestServer::new(app);

        let bytes = server.get(&"/json").await.into_bytes();
        let text = String::from_utf8_lossy(&bytes);

        assert_eq!(text, r#"{"message":"it works?"}"#);
    }
}

#[cfg(test)]
mod test_content_type {
    use crate::TestServer;
    use axum::Json;
    use axum::Router;
    use axum::routing::get;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    #[tokio::test]
    async fn it_should_retrieve_json_content_type_for_json() {
        let app = Router::new().route(
            &"/json",
            get(|| async {
                Json(ExampleResponse {
                    name: "Joe".to_string(),
                    age: 20,
                })
            }),
        );

        let server = TestServer::new(app);

        let content_type = server.get(&"/json").await.content_type();
        assert_eq!(content_type, "application/json");
    }

    #[cfg(feature = "yaml")]
    #[tokio::test]
    async fn it_should_retrieve_yaml_content_type_for_yaml() {
        use axum_yaml::Yaml;

        let app = Router::new().route(
            &"/yaml",
            get(|| async {
                Yaml(ExampleResponse {
                    name: "Joe".to_string(),
                    age: 20,
                })
            }),
        );

        let server = TestServer::new(app);

        let content_type = server.get(&"/yaml").await.content_type();
        assert_eq!(content_type, "application/yaml");
    }
}

#[cfg(test)]
mod test_json {
    use crate::TestServer;
    use crate::testing::catch_panic_error_message;
    use axum::Json;
    use axum::Router;
    use axum::routing::get;
    use pretty_assertions::assert_eq;
    use pretty_assertions::assert_str_eq;
    use serde::Deserialize;
    use serde::Serialize;
    use serde_json::Value;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_json() -> Json<ExampleResponse> {
        Json(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    async fn route_get_fox() -> &'static str {
        "🦊"
    }

    #[tokio::test]
    async fn it_should_deserialize_into_json() {
        let app = Router::new().route(&"/json", get(route_get_json));
        let server = TestServer::new(app);

        let response = server.get(&"/json").await.json::<ExampleResponse>();

        assert_eq!(
            response,
            ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            }
        );
    }

    #[tokio::test]
    async fn it_should_display_the_body_when_deserializing_non_json() {
        let app = Router::new().route(&"/fox", get(route_get_fox));
        let server = TestServer::new(app);

        let response = server.get(&"/fox").await;
        let message = catch_panic_error_message(|| {
            let _ = response.json::<Value>();
        });

        assert_str_eq!(
            r#"Failed to deserialize Json response,
    for request GET /fox
    expected value at line 1 column 1

received:
    🦊
"#,
            message
        );
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_yaml {
    use crate::TestServer;
    use crate::testing::catch_panic_error_message;
    use axum::Router;
    use axum::routing::get;
    use axum_yaml::Yaml;
    use pretty_assertions::assert_eq;
    use pretty_assertions::assert_str_eq;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_yaml() -> Yaml<ExampleResponse> {
        Yaml(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    async fn route_get_fox() -> &'static str {
        "🦊"
    }

    #[tokio::test]
    async fn it_should_deserialize_into_yaml() {
        let app = Router::new().route(&"/yaml", get(route_get_yaml));

        let server = TestServer::new(app);

        let response = server.get(&"/yaml").await.yaml::<ExampleResponse>();

        assert_eq!(
            response,
            ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            }
        );
    }

    #[tokio::test]
    async fn it_should_display_the_body_when_deserializing_non_yaml() {
        let app = Router::new().route(&"/fox", get(route_get_fox));
        let server = TestServer::new(app);

        let response = server.get(&"/fox").await;
        let error_message = catch_panic_error_message(|| {
            let _ = response.yaml::<ExampleResponse>();
        });

        assert_str_eq!(
            r#"Failed to deserialize Yaml response,
    for request GET /fox
    invalid type: string "🦊", expected struct ExampleResponse

received:
    🦊
"#,
            error_message
        );
    }
}

#[cfg(feature = "msgpack")]
#[cfg(test)]
mod test_msgpack {
    use crate::TestServer;
    use axum::Router;
    use axum::routing::get;
    use axum_msgpack::MsgPack;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_msgpack() -> MsgPack<ExampleResponse> {
        MsgPack(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_deserialize_into_msgpack() {
        let app = Router::new().route(&"/msgpack", get(route_get_msgpack));

        let server = TestServer::new(app);

        let response = server.get(&"/msgpack").await.msgpack::<ExampleResponse>();

        assert_eq!(
            response,
            ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            }
        );
    }
}

#[cfg(test)]
mod test_form {
    use crate::TestServer;
    use axum::Form;
    use axum::Router;
    use axum::routing::get;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_form() -> Form<ExampleResponse> {
        Form(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_deserialize_into_form() {
        let app = Router::new().route(&"/form", get(route_get_form));

        let server = TestServer::new(app);

        let response = server.get(&"/form").await.form::<ExampleResponse>();

        assert_eq!(
            response,
            ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            }
        );
    }
}

#[cfg(test)]
mod test_from {
    use crate::TestServer;
    use axum::Router;
    use axum::routing::get;
    use bytes::Bytes;

    #[tokio::test]
    async fn it_should_turn_into_response_bytes() {
        let app = Router::new().route(&"/text", get(|| async { "This is some example text" }));
        let server = TestServer::new(app);

        let response = server.get(&"/text").await;
        let bytes: Bytes = response.into();
        let text = String::from_utf8_lossy(&bytes);
        assert_eq!(text, "This is some example text");
    }
}

#[cfg(test)]
mod test_assert_text {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Router;
    use axum::routing::get;

    fn new_test_server() -> TestServer {
        async fn route_get_text() -> &'static str {
            "This is some example text"
        }

        let app = Router::new().route(&"/text", get(route_get_text));
        TestServer::new(app)
    }

    #[tokio::test]
    async fn it_should_match_whole_text() {
        let server = new_test_server();

        server
            .get(&"/text")
            .await
            .assert_text("This is some example text");
    }

    #[tokio::test]
    async fn it_should_allow_chaining_direct_off_server() {
        let server = new_test_server();

        server
            .get(&"/text")
            .await
            .assert_status_ok()
            .assert_text("This is some example text");
    }

    #[tokio::test]
    async fn it_should_not_match_partial_text() {
        let server = new_test_server();

        let response = server.get(&"/text").await;
        let message = catch_panic_error_message(|| {
            response.assert_text("some example");
        });
        assert_error_message(
            "assertion failed: `(left == right)`

Diff < left / right > :
<some example
>This is some example text

",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_not_match_different_text() {
        let server = new_test_server();

        let response = server.get(&"/text").await;
        let message = catch_panic_error_message(|| {
            response.assert_text("🦊");
        });
        assert_error_message(
            "assertion failed: `(left == right)`

Diff < left / right > :
<🦊
>This is some example text

",
            message,
        );
    }
}

#[cfg(test)]
mod test_assert_text_contains {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Router;
    use axum::routing::get;

    fn new_test_server() -> TestServer {
        async fn route_get_text() -> &'static str {
            "This is some example text"
        }

        let app = Router::new().route(&"/text", get(route_get_text));
        TestServer::new(app)
    }

    #[tokio::test]
    async fn it_should_match_whole_text() {
        let server = new_test_server();

        server
            .get(&"/text")
            .await
            .assert_text_contains("This is some example text");
    }

    #[tokio::test]
    async fn it_should_match_partial_text() {
        let server = new_test_server();

        server
            .get(&"/text")
            .await
            .assert_text_contains("some example");
    }

    #[tokio::test]
    async fn it_should_not_match_different_text() {
        let server = new_test_server();

        let response = server.get(&"/text").await;
        let message = catch_panic_error_message(|| {
            response.assert_text_contains("🦊");
        });
        assert_error_message(
            "Failed to find '🦊', received 'This is some example text'",
            message,
        );
    }
}

#[cfg(test)]
mod test_assert_text_from_file {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::routing::Router;
    use axum::routing::get;

    #[tokio::test]
    async fn it_should_match_from_file() {
        let app = Router::new().route(&"/text", get(|| async { "hello!" }));
        let server = TestServer::new(app);

        server
            .get(&"/text")
            .await
            .assert_text_from_file("files/example.txt");
    }

    #[tokio::test]
    async fn it_should_panic_when_not_match_the_file() {
        let app = Router::new().route(&"/text", get(|| async { "🦊" }));
        let server = TestServer::new(app);

        let response = server.get(&"/text").await;
        let message = catch_panic_error_message(|| {
            response.assert_text_from_file("files/example.txt");
        });
        assert_error_message(
            "assertion failed: `(left == right)`

Diff < left / right > :
<hello!
>🦊

",
            message,
        );
    }
}

#[cfg(test)]
mod test_assert_json {
    use super::*;
    use crate::TestServer;
    use crate::testing::ExpectStrMinLen;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Form;
    use axum::Json;
    use axum::Router;
    use axum::routing::get;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_form() -> Form<ExampleResponse> {
        Form(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    async fn route_get_json() -> Json<ExampleResponse> {
        Json(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_match_json_returned() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app);

        server.get(&"/json").await.assert_json(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }

    #[tokio::test]
    async fn it_should_match_json_returned_using_json_value() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app);

        server.get(&"/json").await.assert_json(&json!({
            "name": "Joe",
            "age": 20,
        }));
    }

    #[tokio::test]
    async fn it_should_panic_if_response_is_different() {
        let app = Router::new().route(&"/json", get(route_get_json));
        let server = TestServer::new(app);

        let response = server.get(&"/json").await;
        let message = catch_panic_error_message(|| {
            response.assert_json(&ExampleResponse {
                name: "Julia".to_string(),
                age: 25,
            });
        });
        assert_error_message(
            "
Json integers at root.age are not equal:
    expected 25
    received 20
",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_panic_if_response_is_form() {
        let app = Router::new().route(&"/form", get(route_get_form));
        let server = TestServer::new(app);

        let response = server.get(&"/form").await;
        let message = catch_panic_error_message(|| {
            response.assert_json(&ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            });
        });
        assert_error_message(
            "Failed to deserialize Json response,
    for request GET /form
    expected ident at line 1 column 2

received:
    name=Joe&age=20
",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_work_with_custom_expect_op() {
        let app = Router::new().route(&"/json", get(route_get_json));
        let server = TestServer::new(app);

        server.get(&"/json").await.assert_json(&json!({
            "name": ExpectStrMinLen { min: 3 },
            "age": 20,
        }));
    }

    #[tokio::test]
    async fn it_should_panic_if_custom_expect_op_fails() {
        let app = Router::new().route(&"/json", get(route_get_json));
        let server = TestServer::new(app);

        let response = server.get(&"/json").await;
        let message = catch_panic_error_message(|| {
            response.assert_json(&json!({
                "name": ExpectStrMinLen { min: 10 },
                "age": 20,
            }));
        });
        assert_error_message("String is too short, received: Joe", message);
    }
}

#[cfg(test)]
mod test_assert_json_contains {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Form;
    use axum::Json;
    use axum::Router;
    use axum::routing::get;
    use serde::Deserialize;
    use serde::Serialize;
    use serde_json::json;
    use std::time::Instant;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        time: u64,
        name: String,
        age: u32,
    }

    async fn route_get_form() -> Form<ExampleResponse> {
        Form(ExampleResponse {
            time: Instant::now().elapsed().as_millis() as u64,
            name: "Joe".to_string(),
            age: 20,
        })
    }

    async fn route_get_json() -> Json<ExampleResponse> {
        Json(ExampleResponse {
            time: Instant::now().elapsed().as_millis() as u64,
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_match_subset_of_json_returned() {
        let app = Router::new().route(&"/json", get(route_get_json));
        let server = TestServer::new(app);

        server.get(&"/json").await.assert_json_contains(&json!({
            "name": "Joe",
            "age": 20,
        }));
    }

    #[tokio::test]
    async fn it_should_panic_if_response_is_different() {
        let app = Router::new().route(&"/json", get(route_get_json));
        let server = TestServer::new(app);

        let response = server.get(&"/json").await;
        let message = catch_panic_error_message(|| {
            response.assert_json_contains(&ExampleResponse {
                time: 1234,
                name: "Julia".to_string(),
                age: 25,
            });
        });
        assert_error_message(
            "
Json integers at root.age are not equal:
    expected 25
    received 20
",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_panic_if_response_is_form() {
        let app = Router::new().route(&"/form", get(route_get_form));
        let server = TestServer::new(app);

        let response = server.get(&"/form").await;
        let message = catch_panic_error_message(|| {
            response.assert_json_contains(&json!({
                "name": "Joe",
                "age": 20,
            }));
        });
        assert_error_message(
            "Failed to deserialize Json response,
    for request GET /form
    expected ident at line 1 column 2

received:
    time=0&name=Joe&age=20
",
            message,
        );
    }

    /// See: https://github.com/JosephLenton/axum-test/issues/151
    #[tokio::test]
    async fn it_should_propagate_contains_to_sub_objects() {
        let json_result = json!({ "a": {"prop1": "value1"} }).to_string();
        let app = Router::new().route(&"/json", get(|| async { json_result }));

        let server = TestServer::new(app);
        let response = server.get("/json").await;

        response.assert_json_contains(&json!({ "a": {} }));
    }
}

#[cfg(test)]
mod test_assert_json_from_file {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Form;
    use axum::Json;
    use axum::routing::Router;
    use axum::routing::get;
    use serde::Deserialize;
    use serde::Serialize;
    use serde_json::json;

    #[tokio::test]
    async fn it_should_match_json_from_file() {
        let app = Router::new().route(
            &"/json",
            get(|| async {
                Json(json!(
                    {
                        "name": "Joe",
                        "age": 20,
                    }
                ))
            }),
        );
        let server = TestServer::new(app);

        server
            .get(&"/json")
            .await
            .assert_json_from_file("files/example.json");
    }

    #[tokio::test]
    async fn it_should_panic_when_not_match_the_file() {
        let app = Router::new().route(
            &"/json",
            get(|| async {
                Json(json!(
                    {
                        "name": "Julia",
                        "age": 25,
                    }
                ))
            }),
        );
        let server = TestServer::new(app);

        let response = server.get(&"/json").await;
        let message = catch_panic_error_message(|| {
            response.assert_json_from_file("files/example.json");
        });
        assert_error_message(
            "
Json integers at root.age are not equal:
    expected 20
    received 25
",
            message,
        );
    }

    #[tokio::test]
    async fn it_should_panic_when_content_type_does_not_match() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct ExampleResponse {
            name: String,
            age: u32,
        }

        let app = Router::new().route(
            &"/form",
            get(|| async {
                Form(ExampleResponse {
                    name: "Joe".to_string(),
                    age: 20,
                })
            }),
        );
        let server = TestServer::new(app);

        let response = server.get(&"/form").await;
        let message = catch_panic_error_message(|| {
            response.assert_json_from_file("files/example.json");
        });
        assert_error_message(
            "Failed to deserialize Json response,
    for request GET /form
    expected ident at line 1 column 2

received:
    name=Joe&age=20
",
            message,
        );
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_assert_yaml {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Form;
    use axum::Router;
    use axum::routing::get;
    use axum_yaml::Yaml;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_form() -> Form<ExampleResponse> {
        Form(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    async fn route_get_yaml() -> Yaml<ExampleResponse> {
        Yaml(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_match_yaml_returned() {
        let app = Router::new().route(&"/yaml", get(route_get_yaml));
        let server = TestServer::new(app);

        server.get(&"/yaml").await.assert_yaml(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }

    #[tokio::test]
    async fn it_should_panic_if_response_is_different() {
        let app = Router::new().route(&"/yaml", get(route_get_yaml));
        let server = TestServer::new(app);

        let response = server.get(&"/yaml").await;
        let message = catch_panic_error_message(|| {
            response.assert_yaml(&ExampleResponse {
                name: "Julia".to_string(),
                age: 25,
            });
        });
        assert_error_message(
            r#"assertion failed: `(left == right)`

Diff < left / right > :
 ExampleResponse {
<    name: "Julia",
<    age: 25,
>    name: "Joe",
>    age: 20,
 }

"#,
            message,
        );
    }

    #[tokio::test]
    async fn it_should_panic_if_response_is_form() {
        let app = Router::new().route(&"/form", get(route_get_form));
        let server = TestServer::new(app);

        let response = server.get(&"/form").await;
        let message = catch_panic_error_message(|| {
            response.assert_yaml(&ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            });
        });
        assert_error_message(
            r#"Failed to deserialize Yaml response,
    for request GET /form
    invalid type: string "name=Joe&age=20", expected struct ExampleResponse

received:
    name=Joe&age=20
"#,
            message,
        );
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_assert_yaml_from_file {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Form;
    use axum::routing::Router;
    use axum::routing::get;
    use axum_yaml::Yaml;
    use serde::Deserialize;
    use serde::Serialize;
    use serde_json::json;

    #[tokio::test]
    async fn it_should_match_yaml_from_file() {
        let app = Router::new().route(
            &"/yaml",
            get(|| async {
                Yaml(json!(
                    {
                        "name": "Joe",
                        "age": 20,
                    }
                ))
            }),
        );
        let server = TestServer::new(app);

        server
            .get(&"/yaml")
            .await
            .assert_yaml_from_file("files/example.yaml");
    }

    #[tokio::test]
    async fn it_should_panic_when_not_match_the_file() {
        let app = Router::new().route(
            &"/yaml",
            get(|| async {
                Yaml(json!(
                    {
                        "name": "Julia",
                        "age": 25,
                    }
                ))
            }),
        );
        let server = TestServer::new(app);

        let response = server.get(&"/yaml").await;
        let message = catch_panic_error_message(|| {
            response.assert_yaml_from_file("files/example.yaml");
        });
        assert_error_message(
            r#"assertion failed: `(left == right)`

Diff < left / right > :
 Mapping {
<    "name": String("Joe"),
<    "age": Number(20),
>    "age": Number(25),
>    "name": String("Julia"),
 }

"#,
            message,
        );
    }

    #[tokio::test]
    async fn it_should_panic_when_content_type_does_not_match() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct ExampleResponse {
            name: String,
            age: u32,
        }

        let app = Router::new().route(
            &"/form",
            get(|| async {
                Form(ExampleResponse {
                    name: "Joe".to_string(),
                    age: 20,
                })
            }),
        );
        let server = TestServer::new(app);

        let response = server.get(&"/form").await;
        let message = catch_panic_error_message(|| {
            response.assert_yaml_from_file("files/example.yaml");
        });
        assert_error_message(
            r#"assertion failed: `(left == right)`

Diff < left / right > :
<Mapping {
<    "name": String("Joe"),
<    "age": Number(20),
<}
>String("name=Joe&age=20")

"#,
            message,
        );
    }
}

#[cfg(test)]
mod test_assert_form {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message;
    use axum::Form;
    use axum::Json;
    use axum::Router;
    use axum::routing::get;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_form() -> Form<ExampleResponse> {
        Form(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    async fn route_get_json() -> Json<ExampleResponse> {
        Json(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_match_form_returned() {
        let app = Router::new().route(&"/form", get(route_get_form));

        let server = TestServer::new(app);

        server.get(&"/form").await.assert_form(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }

    #[tokio::test]
    async fn it_should_panic_if_response_is_different() {
        let app = Router::new().route(&"/form", get(route_get_form));
        let server = TestServer::new(app);

        let response = server.get(&"/form").await;
        let message = catch_panic_error_message(|| {
            response.assert_form(&ExampleResponse {
                name: "Julia".to_string(),
                age: 25,
            });
        });

        assert_error_message(
            r#"assertion failed: `(left == right)`

Diff < left / right > :
 ExampleResponse {
<    name: "Julia",
<    age: 25,
>    name: "Joe",
>    age: 20,
 }

"#,
            message,
        );
    }

    #[tokio::test]
    async fn it_should_panic_if_response_is_json() {
        let app = Router::new().route(&"/json", get(route_get_json));
        let server = TestServer::new(app);

        let response = server.get(&"/json").await;
        let message = catch_panic_error_message(|| {
            response.assert_form(&ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            });
        });
        assert_error_message(
            r#"Failed to deserialize Form response,
    for request GET /json
    missing field `name`

received:
    {"name":"Joe","age":20}
"#,
            message,
        );
    }
}

#[cfg(test)]
mod test_text {
    use crate::TestServer;
    use axum::Router;
    use axum::routing::get;

    #[tokio::test]
    async fn it_should_deserialize_into_text() {
        async fn route_get_text() -> String {
            "hello!".to_string()
        }

        let app = Router::new().route(&"/text", get(route_get_text));

        let server = TestServer::new(app);

        let response = server.get(&"/text").await.text();

        assert_eq!(response, "hello!");
    }
}

#[cfg(feature = "ws")]
#[cfg(test)]
mod test_into_websocket {
    use crate::TestServer;
    use crate::testing::assert_error_message;
    use crate::testing::catch_panic_error_message_async;
    use axum::Router;
    use axum::extract::WebSocketUpgrade;
    use axum::extract::ws::WebSocket;
    use axum::response::Response;
    use axum::routing::get;

    fn new_test_router() -> Router {
        pub async fn route_get_websocket(ws: WebSocketUpgrade) -> Response {
            async fn handle_ping_pong(mut socket: WebSocket) {
                while let Some(_) = socket.recv().await {
                    // do nothing
                }
            }

            ws.on_upgrade(move |socket| handle_ping_pong(socket))
        }

        let app = Router::new().route(&"/ws", get(route_get_websocket));

        app
    }

    #[tokio::test]
    async fn it_should_upgrade_on_http_transport() {
        let router = new_test_router();
        let server = TestServer::builder().http_transport().build(router);

        let _ = server.get_websocket(&"/ws").await.into_websocket().await;

        assert!(true);
    }

    #[tokio::test]
    async fn it_should_fail_to_upgrade_on_mock_transport() {
        let router = new_test_router();
        let server = TestServer::builder().mock_transport().build(router);

        let response = server.get_websocket(&"/ws").await;
        let message = catch_panic_error_message_async(async {
            let _ = response.into_websocket().await;
        })
        .await;
        assert_error_message(
            "WebSocket requires a HTTP based transport layer, see `TestServerConfig::transport`",
            message,
        );
    }
}

#[cfg(test)]
mod test_fmt {
    use crate::TestServer;
    use axum::Json;
    use axum::Router;
    use axum::routing::get;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_json() -> Json<ExampleResponse> {
        Json(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_output_json_in_json_format() {
        let app = Router::new().route(&"/json", get(route_get_json));
        let server = TestServer::new(app);

        let response = server.get(&"/json").await;
        let output = format!("{response}");

        assert_eq!(
            output,
            r#"HTTP/1.1 200 OK
content-type: application/json
content-length: 23

{"name":"Joe","age":20}
"#
        );
    }
}

#[cfg(test)]
mod test_request_method {
    use super::*;
    use crate::TestServer;
    use axum::Router;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn it_should_return_same_method_as_the_request() {
        let server = TestServer::new(Router::new());

        let method = server.get("/").await.request_method();
        assert_eq!(Method::GET, method);

        let method = server.post("/").await.request_method();
        assert_eq!(Method::POST, method);

        let method = server.put("/").await.request_method();
        assert_eq!(Method::PUT, method);

        let method = server.patch("/").await.request_method();
        assert_eq!(Method::PATCH, method);

        let method = server.delete("/").await.request_method();
        assert_eq!(Method::DELETE, method);

        let method = server.method(Method::OPTIONS, "/").await.request_method();
        assert_eq!(Method::OPTIONS, method);
    }
}

#[cfg(test)]
mod test_request_uri {
    use crate::TestServer;
    use axum::Router;
    use pretty_assertions::assert_str_eq;

    #[tokio::test]
    async fn it_should_include_domain_for_random_http_transport() {
        let server = TestServer::builder().http_transport().build(Router::new());

        let uri = server.get("/my-path").await.request_uri();
        let expected = format!("http://127.0.0.1:{}/my-path", uri.port().unwrap());
        assert_str_eq!(expected, uri.to_string());
    }

    #[tokio::test]
    async fn it_should_not_include_domain_for_mock_transport() {
        let server = TestServer::builder().mock_transport().build(Router::new());

        let uri = server.get("/my-path").await.request_uri();
        assert_str_eq!("/my-path", uri.to_string());
    }

    #[tokio::test]
    async fn it_should_make_non_slash_path_into_slash_path_for_mock_transport() {
        let server = TestServer::builder().mock_transport().build(Router::new());

        let uri = server.get("my-path").await.request_uri();
        assert_str_eq!("/my-path", uri.to_string());
    }
}
