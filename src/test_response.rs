use crate::internals::format_status_code_range;
use crate::internals::RequestPathFormatter;
use crate::internals::StatusCodeFormatter;
use crate::internals::TryIntoRangeBounds;
use anyhow::Context;
use assert_json_diff::assert_json_include;
use bytes::Bytes;
use cookie::Cookie;
use cookie::CookieJar;
use http::header::HeaderName;
use http::header::SET_COOKIE;
use http::response::Parts;
use http::HeaderMap;
use http::HeaderValue;
use http::Method;
use http::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::convert::AsRef;
use std::fmt::Debug;
use std::fmt::Display;
use std::fs::read_to_string;
use std::fs::File;
use std::io::BufReader;
use std::ops::RangeBounds;
use url::Url;

#[cfg(feature = "pretty-assertions")]
use pretty_assertions::{assert_eq, assert_ne};

#[cfg(feature = "ws")]
use crate::internals::TestResponseWebSocket;
#[cfg(feature = "ws")]
use crate::TestWebSocket;

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
/// use serde::Deserialize;
/// use serde::Serialize;
///
/// let app = Router::new()
///     .route(&"/test", get(|| async { "hello!" }));
///
/// let server = TestServer::new(app)?;
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
/// # let server = TestServer::new(app)?;
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
/// The result of a response can also be asserted using the many assertion functions.
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use axum::Json;
/// use axum::Router;
/// use axum_test::TestServer;
/// use axum::routing::get;
/// use serde::Deserialize;
/// use serde::Serialize;
///
/// let app = Router::new()
///     .route(&"/test", get(|| async { "hello!" }));
///
/// let server = TestServer::new(app)?;
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
#[derive(Clone, Debug)]
pub struct TestResponse {
    method: Method,

    /// This is the actual url that was used for the request.
    full_request_url: Url,
    headers: HeaderMap<HeaderValue>,
    status_code: StatusCode,
    response_body: Bytes,

    #[cfg(feature = "ws")]
    websockets: TestResponseWebSocket,
}

impl TestResponse {
    pub(crate) fn new(
        method: Method,
        full_request_url: Url,
        parts: Parts,
        response_body: Bytes,

        #[cfg(feature = "ws")] websockets: TestResponseWebSocket,
    ) -> Self {
        Self {
            method,
            full_request_url,
            headers: parts.headers,
            status_code: parts.status,
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
    /// let server = TestServer::new(app)?;
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
    /// let server = TestServer::new(app)?;
    /// let response = server.get(&"/todo").await;
    ///
    /// // Extract the response as a `Todo` item.
    /// let todo = response.json::<Todo>();
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn json<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice::<T>(self.as_bytes())
            .with_context(|| {
                let debug_request_format = self.debug_request_format();

                format!("Deserializing response from Json, for request {debug_request_format}")
            })
            .unwrap()
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
    /// let server = TestServer::new(app)?;
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
    pub fn yaml<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        serde_yaml::from_slice::<T>(self.as_bytes())
            .with_context(|| {
                let debug_request_format = self.debug_request_format();

                format!("Deserializing response from YAML, for request {debug_request_format}")
            })
            .unwrap()
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
    /// let server = TestServer::new(app)?;
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
    pub fn msgpack<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        rmp_serde::from_slice::<T>(self.as_bytes())
            .with_context(|| {
                let debug_request_format = self.debug_request_format();

                format!("Deserializing response from MsgPack, for request {debug_request_format}")
            })
            .unwrap()
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
    /// let server = TestServer::new(app)?;
    /// let response = server.get(&"/todo").await;
    ///
    /// // Extract the response as a `Todo` item.
    /// let todo = response.form::<Todo>();
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn form<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        serde_urlencoded::from_bytes::<T>(self.as_bytes())
            .with_context(|| {
                let debug_request_format = self.debug_request_format();

                format!("Deserializing response from Form, for request {debug_request_format}")
            })
            .unwrap()
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

    /// The full URL that was used to produce this response.
    #[must_use]
    pub fn request_url(&self) -> Url {
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

    /// Finds a header with the given name.
    /// If there are multiple headers with the same name,
    /// then only the first will be returned.
    ///
    /// If no header is found, then this will panic.
    #[must_use]
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
            .with_context(|| {
                let debug_request_format = self.debug_request_format();

                format!("Cannot find header {debug_header}, for request {debug_request_format}",)
            })
            .unwrap()
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
    pub fn assert_contains_header<N>(&self, name: N)
    where
        N: TryInto<HeaderName> + Display + Clone,
        N::Error: Debug,
    {
        let debug_header_name = name.clone();
        let debug_request_format = self.debug_request_format();
        let has_header = self.contains_header(name);

        assert!(has_header, "Expected header '{debug_header_name}' to be present in response, header was not found, for request {debug_request_format}");
    }

    #[track_caller]
    pub fn assert_header<N, V>(&self, name: N, value: V)
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
                panic!("Expected header '{debug_header_name}' to be present in response, header was not found, for request {debug_request_format}")
            }
            Some(found_header_value) => {
                assert_eq!(expected_header_value, found_header_value,)
            }
        }
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
    pub fn cookie(&self, cookie_name: &str) -> Cookie<'static> {
        self.maybe_cookie(cookie_name)
            .with_context(|| {
                let debug_request_format = self.debug_request_format();

                format!("Cannot find cookie {cookie_name}, for request {debug_request_format}")
            })
            .unwrap()
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
    pub fn iter_cookies(&self) -> impl Iterator<Item = Cookie<'_>> {
        self.iter_headers_by_name(SET_COOKIE).map(|header| {
            let header_str = header
                .to_str()
                .with_context(|| {
                    let debug_request_format = self.debug_request_format();

                    format!(
                        "Reading header 'Set-Cookie' as string, for request {debug_request_format}",
                    )
                })
                .unwrap();

            Cookie::parse(header_str)
                .with_context(|| {
                    let debug_request_format = self.debug_request_format();

                    format!("Parsing 'Set-Cookie' header, for request {debug_request_format}",)
                })
                .unwrap()
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
    #[must_use]
    pub async fn into_websocket(self) -> TestWebSocket {
        use crate::transport_layer::TransportLayerType;

        // Using the mock approach will just fail.
        if self.websockets.transport_type != TransportLayerType::Http {
            unimplemented!("WebSocket requires a HTTP based transport layer, see `TestServerConfig::transport`");
        }

        let debug_request_format = self.debug_request_format().to_string();

        let on_upgrade = self.websockets.maybe_on_upgrade.with_context(|| {
            format!("Expected WebSocket upgrade to be found, it is None, for request {debug_request_format}")
        })
        .unwrap();

        let upgraded = on_upgrade
            .await
            .with_context(|| {
                format!("Failed to upgrade connection for, for request {debug_request_format}")
            })
            .unwrap();

        TestWebSocket::new(upgraded).await
    }

    /// This performs an assertion comparing the whole body of the response,
    /// against the text provided.
    #[track_caller]
    pub fn assert_text<C>(&self, expected: C)
    where
        C: AsRef<str>,
    {
        let expected_contents = expected.as_ref();
        assert_eq!(expected_contents, &self.text());
    }

    /// This asserts if the text given is contained, somewhere, within the response.
    #[track_caller]
    pub fn assert_text_contains<C>(&self, expected: C)
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
    }

    /// Asserts the response from the server matches the contents of the file.
    #[track_caller]
    pub fn assert_text_from_file(&self, path: &str) {
        let expected = read_to_string(path).unwrap();
        self.assert_text(expected);
    }

    /// Deserializes the contents of the request as Json,
    /// and asserts it matches the value given.
    ///
    /// If `other` does not match, or the response is not Json,
    /// then this will panic.
    #[track_caller]
    pub fn assert_json<T>(&self, expected: &T)
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*expected, self.json::<T>());
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
    /// let server = TestServer::new(app)?;
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
    pub fn assert_json_contains<T>(&self, expected: &T)
    where
        T: Serialize,
    {
        let received = self.json::<Value>();
        assert_json_include!(actual: received, expected: expected);
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
    /// let server = TestServer::new(app).unwrap();
    /// server
    ///     .get(&"/json")
    ///     .await
    ///     .assert_json_from_file("files/example.json");
    /// #
    /// # Ok(()) }
    /// ```
    ///
    #[track_caller]
    pub fn assert_json_from_file(&self, path: &str) {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let expected = serde_json::from_reader::<_, serde_json::Value>(reader).unwrap();
        self.assert_json(&expected);
    }

    /// Deserializes the contents of the request as Yaml,
    /// and asserts it matches the value given.
    ///
    /// If `other` does not match, or the response is not Yaml,
    /// then this will panic.
    #[cfg(feature = "yaml")]
    #[track_caller]
    pub fn assert_yaml<T>(&self, other: &T)
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*other, self.yaml::<T>());
    }

    /// Read yaml file from given path and assert it with yaml response.
    #[cfg(feature = "yaml")]
    #[track_caller]
    pub fn assert_yaml_from_file(&self, path: &str) {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let expected = serde_yaml::from_reader::<_, serde_yaml::Value>(reader).unwrap();
        self.assert_yaml(&expected);
    }

    /// Deserializes the contents of the request as MsgPack,
    /// and asserts it matches the value given.
    ///
    /// If `other` does not match, or the response is not MsgPack,
    /// then this will panic.
    #[cfg(feature = "msgpack")]
    #[track_caller]
    pub fn assert_msgpack<T>(&self, other: &T)
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*other, self.msgpack::<T>());
    }

    /// Deserializes the contents of the request as an url encoded form,
    /// and asserts it matches the value given.
    ///
    /// If `other` does not match, or the response cannot be deserialized,
    /// then this will panic.
    #[track_caller]
    pub fn assert_form<T>(&self, other: &T)
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*other, self.form::<T>());
    }

    /// Assert the response status code matches the one given.
    #[track_caller]
    pub fn assert_status(&self, expected_status_code: StatusCode) {
        let received_debug = StatusCodeFormatter(self.status_code);
        let expected_debug = StatusCodeFormatter(expected_status_code);
        let debug_request_format = self.debug_request_format();

        assert_eq!(
            expected_status_code, self.status_code,
            "Expected status code to be {expected_debug}, received {received_debug}, for request {debug_request_format}",
        );
    }

    /// Assert the response status code does **not** match the one given.
    #[track_caller]
    pub fn assert_not_status(&self, expected_status_code: StatusCode) {
        let received_debug = StatusCodeFormatter(self.status_code);
        let expected_debug = StatusCodeFormatter(expected_status_code);
        let debug_request_format = self.debug_request_format();

        assert_ne!(
            expected_status_code,
            self.status_code,
            "Expected status code to not be {expected_debug}, received {received_debug}, for request {debug_request_format}"
        );
    }

    /// Assert that the status code is **within** the 2xx range.
    /// i.e. The range from 200-299.
    #[track_caller]
    pub fn assert_status_success(&self) {
        let status_code = self.status_code.as_u16();
        let received_debug = StatusCodeFormatter(self.status_code);
        let debug_request_format = self.debug_request_format();

        assert!(
            200 <= status_code && status_code <= 299,
            "Expect status code within 2xx range, received {received_debug}, for request {debug_request_format}"
        );
    }

    /// Assert that the status code is **outside** the 2xx range.
    /// i.e. A status code less than 200, or 300 or more.
    #[track_caller]
    pub fn assert_status_failure(&self) {
        let status_code = self.status_code.as_u16();
        let received_debug = StatusCodeFormatter(self.status_code);
        let debug_request_format = self.debug_request_format();

        assert!(
            status_code < 200 || 299 < status_code,
            "Expect status code outside 2xx range, received {received_debug}, for request {debug_request_format}",
        );
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
    /// let server = TestServer::new(app).unwrap();
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
    pub fn assert_status_in_range<R, S>(&self, expected_status_range: R)
    where
        R: RangeBounds<S> + TryIntoRangeBounds<StatusCode> + Debug,
        S: TryInto<StatusCode>,
    {
        let range = TryIntoRangeBounds::<StatusCode>::try_into_range_bounds(expected_status_range)
            .expect("Failed to convert status code");

        let status_code = self.status_code();
        let is_in_range = range.contains(&status_code);

        assert!(
            is_in_range,
            "Expected status to not in range {}, received {status_code}",
            format_status_code_range(range)
        );
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
    /// let server = TestServer::new(app).unwrap();
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
    pub fn assert_status_not_in_range<R, S>(&self, expected_status_range: R)
    where
        R: RangeBounds<S> + TryIntoRangeBounds<StatusCode> + Debug,
        S: TryInto<StatusCode>,
    {
        let range = TryIntoRangeBounds::<StatusCode>::try_into_range_bounds(expected_status_range)
            .expect("Failed to convert status code");

        let status_code = self.status_code();
        let is_not_in_range = !range.contains(&status_code);

        assert!(
            is_not_in_range,
            "Expected status is in range {}, received {status_code}",
            format_status_code_range(range)
        );
    }

    /// Assert the response status code is 200.
    #[track_caller]
    pub fn assert_status_ok(&self) {
        self.assert_status(StatusCode::OK)
    }

    /// Assert the response status code is **not** 200.
    #[track_caller]
    pub fn assert_status_not_ok(&self) {
        self.assert_not_status(StatusCode::OK)
    }

    /// Assert the response status code is 303.
    #[track_caller]
    pub fn assert_status_see_other(&self) {
        self.assert_status(StatusCode::SEE_OTHER)
    }

    /// Assert the response status code is 400.
    #[track_caller]
    pub fn assert_status_bad_request(&self) {
        self.assert_status(StatusCode::BAD_REQUEST)
    }

    /// Assert the response status code is 404.
    #[track_caller]
    pub fn assert_status_not_found(&self) {
        self.assert_status(StatusCode::NOT_FOUND)
    }

    /// Assert the response status code is 401.
    #[track_caller]
    pub fn assert_status_unauthorized(&self) {
        self.assert_status(StatusCode::UNAUTHORIZED)
    }

    /// Assert the response status code is 403.
    #[track_caller]
    pub fn assert_status_forbidden(&self) {
        self.assert_status(StatusCode::FORBIDDEN)
    }

    /// Assert the response status code is 413.
    ///
    /// The payload is too large.
    #[track_caller]
    pub fn assert_status_payload_too_large(&self) {
        self.assert_status(StatusCode::PAYLOAD_TOO_LARGE)
    }

    /// Assert the response status code is 422.
    #[track_caller]
    pub fn assert_status_unprocessable_entity(&self) {
        self.assert_status(StatusCode::UNPROCESSABLE_ENTITY)
    }

    /// Assert the response status code is 429.
    #[track_caller]
    pub fn assert_status_too_many_requests(&self) {
        self.assert_status(StatusCode::TOO_MANY_REQUESTS)
    }

    /// Assert the response status code is 101.
    ///
    /// This type of code is used in Web Socket connection when
    /// first request.
    #[track_caller]
    pub fn assert_status_switching_protocols(&self) {
        self.assert_status(StatusCode::SWITCHING_PROTOCOLS)
    }

    /// Assert the response status code is 500.
    #[track_caller]
    pub fn assert_status_internal_server_error(&self) {
        self.assert_status(StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Assert the response status code is 503.
    #[track_caller]
    pub fn assert_status_service_unavailable(&self) {
        self.assert_status(StatusCode::SERVICE_UNAVAILABLE)
    }

    fn debug_request_format(&self) -> RequestPathFormatter<'_> {
        RequestPathFormatter::new(&self.method, self.full_request_url.as_str(), None)
    }
}

impl From<TestResponse> for Bytes {
    fn from(response: TestResponse) -> Self {
        response.into_bytes()
    }
}

#[cfg(test)]
mod test_assert_header {
    use crate::TestServer;
    use axum::http::HeaderMap;
    use axum::routing::get;
    use axum::Router;

    async fn route_get_header() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-my-custom-header", "content".parse().unwrap());
        headers
    }

    #[tokio::test]
    async fn it_should_not_panic_if_contains_header_and_content_matches() {
        let router = Router::new().route(&"/header", get(route_get_header));

        let server = TestServer::new(router).unwrap();

        server
            .get(&"/header")
            .await
            .assert_header("x-my-custom-header", "content");
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_contains_header_and_content_does_not_match() {
        let router = Router::new().route(&"/header", get(route_get_header));

        let server = TestServer::new(router).unwrap();

        server
            .get(&"/header")
            .await
            .assert_header("x-my-custom-header", "different-content");
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_not_contains_header() {
        let router = Router::new().route(&"/header", get(route_get_header));

        let server = TestServer::new(router).unwrap();

        server
            .get(&"/header")
            .await
            .assert_header("x-custom-header-not-found", "content");
    }
}

#[cfg(test)]
mod test_assert_contains_header {
    use crate::TestServer;
    use axum::http::HeaderMap;
    use axum::routing::get;
    use axum::Router;

    async fn route_get_header() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-my-custom-header", "content".parse().unwrap());
        headers
    }

    #[tokio::test]
    async fn it_should_not_panic_if_contains_header() {
        let router = Router::new().route(&"/header", get(route_get_header));

        let server = TestServer::new(router).unwrap();

        server
            .get(&"/header")
            .await
            .assert_contains_header("x-my-custom-header");
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_not_contains_header() {
        let router = Router::new().route(&"/header", get(route_get_header));

        let server = TestServer::new(router).unwrap();

        server
            .get(&"/header")
            .await
            .assert_contains_header("x-custom-header-not-found");
    }
}

#[cfg(test)]
mod test_assert_success {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Router;
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

        let server = TestServer::new(router).unwrap();

        let response = server.get(&"/pass").await;

        response.assert_status_success()
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_when_not_200() {
        let router = Router::new()
            .route(&"/pass", get(route_get_pass))
            .route(&"/fail", get(route_get_fail));

        let server = TestServer::new(router).unwrap();

        let response = server.get(&"/fail").expect_failure().await;

        response.assert_status_success()
    }
}

#[cfg(test)]
mod test_assert_failure {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Router;
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

        let server = TestServer::new(router).unwrap();
        let response = server.get(&"/fail").expect_failure().await;

        response.assert_status_failure()
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_when_200() {
        let router = Router::new()
            .route(&"/pass", get(route_get_pass))
            .route(&"/fail", get(route_get_fail));

        let server = TestServer::new(router).unwrap();
        let response = server.get(&"/pass").await;

        response.assert_status_failure()
    }
}

#[cfg(test)]
mod test_assert_status {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Router;
    use http::StatusCode;

    pub async fn route_get_ok() -> StatusCode {
        StatusCode::OK
    }

    #[tokio::test]
    async fn it_should_pass_if_given_right_status_code() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router).unwrap();

        server.get(&"/ok").await.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_when_status_code_does_not_match() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router).unwrap();

        server.get(&"/ok").await.assert_status(StatusCode::ACCEPTED);
    }
}

#[cfg(test)]
mod test_assert_not_status {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Router;
    use http::StatusCode;

    pub async fn route_get_ok() -> StatusCode {
        StatusCode::OK
    }

    #[tokio::test]
    async fn it_should_pass_if_status_code_does_not_match() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router).unwrap();

        server
            .get(&"/ok")
            .await
            .assert_not_status(StatusCode::ACCEPTED);
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_status_code_matches() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router).unwrap();

        server.get(&"/ok").await.assert_not_status(StatusCode::OK);
    }
}

#[cfg(test)]
mod test_into_bytes {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Json;
    use axum::Router;
    use serde_json::json;
    use serde_json::Value;

    async fn route_get_json() -> Json<Value> {
        Json(json!({
            "message": "it works?"
        }))
    }

    #[tokio::test]
    async fn it_should_deserialize_into_json() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app).unwrap();

        let bytes = server.get(&"/json").await.into_bytes();
        let text = String::from_utf8_lossy(&bytes);

        assert_eq!(text, r#"{"message":"it works?"}"#);
    }
}

#[cfg(test)]
mod test_json {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Json;
    use axum::Router;
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
    async fn it_should_deserialize_into_json() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app).unwrap();

        let response = server.get(&"/json").await.json::<ExampleResponse>();

        assert_eq!(
            response,
            ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            }
        );
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_yaml {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Router;
    use axum_yaml::Yaml;
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

    #[tokio::test]
    async fn it_should_deserialize_into_yaml() {
        let app = Router::new().route(&"/yaml", get(route_get_yaml));

        let server = TestServer::new(app).unwrap();

        let response = server.get(&"/yaml").await.yaml::<ExampleResponse>();

        assert_eq!(
            response,
            ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            }
        );
    }
}

#[cfg(feature = "msgpack")]
#[cfg(test)]
mod test_msgpack {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Router;
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

        let server = TestServer::new(app).unwrap();

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
    use axum::routing::get;
    use axum::Form;
    use axum::Router;
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

        let server = TestServer::new(app).unwrap();

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
mod test_assert_text {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Router;

    fn new_test_server() -> TestServer {
        async fn route_get_text() -> &'static str {
            "This is some example text"
        }

        let app = Router::new().route(&"/text", get(route_get_text));
        TestServer::new(app).unwrap()
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
    #[should_panic]
    async fn it_should_not_match_partial_text() {
        let server = new_test_server();

        server.get(&"/text").await.assert_text("some example");
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_not_match_different_text() {
        let server = new_test_server();

        server.get(&"/text").await.assert_text("🦊");
    }
}

#[cfg(test)]
mod test_assert_text_contains {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Router;

    fn new_test_server() -> TestServer {
        async fn route_get_text() -> &'static str {
            "This is some example text"
        }

        let app = Router::new().route(&"/text", get(route_get_text));
        TestServer::new(app).unwrap()
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
    #[should_panic]
    async fn it_should_not_match_different_text() {
        let server = new_test_server();

        server.get(&"/text").await.assert_text_contains("🦊");
    }
}

#[cfg(test)]
mod test_assert_text_from_file {
    use crate::TestServer;
    use axum::routing::get;
    use axum::routing::Router;

    #[tokio::test]
    async fn it_should_match_from_file() {
        let app = Router::new().route(&"/text", get(|| async { "hello!" }));
        let server = TestServer::new(app).unwrap();

        server
            .get(&"/text")
            .await
            .assert_text_from_file("files/example.txt");
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_when_not_match_the_file() {
        let app = Router::new().route(&"/text", get(|| async { "🦊" }));
        let server = TestServer::new(app).unwrap();

        server
            .get(&"/text")
            .await
            .assert_text_from_file("files/example.txt");
    }
}

#[cfg(test)]
mod test_assert_json {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Form;
    use axum::Json;
    use axum::Router;
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
    async fn it_should_match_json_returned() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app).unwrap();

        server.get(&"/json").await.assert_json(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_different() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app).unwrap();

        server.get(&"/json").await.assert_json(&ExampleResponse {
            name: "Julia".to_string(),
            age: 25,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_form() {
        let app = Router::new().route(&"/form", get(route_get_form));

        let server = TestServer::new(app).unwrap();

        server.get(&"/form").await.assert_json(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }
}

#[cfg(test)]
mod test_assert_json_contains {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Form;
    use axum::Json;
    use axum::Router;
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
        let server = TestServer::new(app).unwrap();

        server.get(&"/json").await.assert_json_contains(&json!({
            "name": "Joe",
            "age": 20,
        }));
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_different() {
        let app = Router::new().route(&"/json", get(route_get_json));
        let server = TestServer::new(app).unwrap();

        server
            .get(&"/json")
            .await
            .assert_json_contains(&ExampleResponse {
                time: 1234,
                name: "Julia".to_string(),
                age: 25,
            });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_form() {
        let app = Router::new().route(&"/form", get(route_get_form));
        let server = TestServer::new(app).unwrap();

        server.get(&"/form").await.assert_json_contains(&json!({
            "name": "Joe",
            "age": 20,
        }));
    }
}

#[cfg(test)]
mod test_assert_json_from_file {
    use crate::TestServer;
    use axum::routing::get;
    use axum::routing::Router;
    use axum::Form;
    use axum::Json;
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
        let server = TestServer::new(app).unwrap();

        server
            .get(&"/json")
            .await
            .assert_json_from_file("files/example.json");
    }

    #[tokio::test]
    #[should_panic]
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
        let server = TestServer::new(app).unwrap();

        server
            .get(&"/json")
            .await
            .assert_json_from_file("files/example.json");
    }

    #[tokio::test]
    #[should_panic]
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
        let server = TestServer::new(app).unwrap();

        server
            .get(&"/form")
            .await
            .assert_json_from_file("files/example.json");
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_assert_yaml {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Form;
    use axum::Router;
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

        let server = TestServer::new(app).unwrap();

        server.get(&"/yaml").await.assert_yaml(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_different() {
        let app = Router::new().route(&"/yaml", get(route_get_yaml));

        let server = TestServer::new(app).unwrap();

        server.get(&"/yaml").await.assert_yaml(&ExampleResponse {
            name: "Julia".to_string(),
            age: 25,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_form() {
        let app = Router::new().route(&"/form", get(route_get_form));

        let server = TestServer::new(app).unwrap();

        server.get(&"/form").await.assert_yaml(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_assert_yaml_from_file {
    use crate::TestServer;
    use axum::routing::get;
    use axum::routing::Router;
    use axum::Form;
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
        let server = TestServer::new(app).unwrap();

        server
            .get(&"/yaml")
            .await
            .assert_yaml_from_file("files/example.yaml");
    }

    #[tokio::test]
    #[should_panic]
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
        let server = TestServer::new(app).unwrap();

        server
            .get(&"/yaml")
            .await
            .assert_yaml_from_file("files/example.yaml");
    }

    #[tokio::test]
    #[should_panic]
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
        let server = TestServer::new(app).unwrap();

        server
            .get(&"/form")
            .await
            .assert_yaml_from_file("files/example.yaml");
    }
}

#[cfg(test)]
mod test_assert_form {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Form;
    use axum::Json;
    use axum::Router;
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

        let server = TestServer::new(app).unwrap();

        server.get(&"/form").await.assert_form(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_different() {
        let app = Router::new().route(&"/form", get(route_get_form));

        let server = TestServer::new(app).unwrap();

        server.get(&"/form").await.assert_form(&ExampleResponse {
            name: "Julia".to_string(),
            age: 25,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_json() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app).unwrap();

        server.get(&"/json").await.assert_form(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }
}

#[cfg(test)]
mod test_text {
    use crate::TestServer;
    use axum::routing::get;
    use axum::Router;

    #[tokio::test]
    async fn it_should_deserialize_into_text() {
        async fn route_get_text() -> String {
            "hello!".to_string()
        }

        let app = Router::new().route(&"/text", get(route_get_text));

        let server = TestServer::new(app).unwrap();

        let response = server.get(&"/text").await.text();

        assert_eq!(response, "hello!");
    }
}

#[cfg(feature = "ws")]
#[cfg(test)]
mod test_into_websocket {
    use crate::TestServer;

    use axum::extract::ws::WebSocket;
    use axum::extract::WebSocketUpgrade;
    use axum::response::Response;
    use axum::routing::get;
    use axum::Router;

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
        let server = TestServer::builder()
            .http_transport()
            .build(router)
            .unwrap();

        let _ = server.get_websocket(&"/ws").await.into_websocket().await;

        assert!(true);
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_fail_to_upgrade_on_mock_transport() {
        let router = new_test_router();
        let server = TestServer::builder()
            .mock_transport()
            .build(router)
            .unwrap();

        let _ = server.get_websocket(&"/ws").await.into_websocket().await;
    }
}
