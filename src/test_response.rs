use ::anyhow::Context;
use ::cookie::Cookie;
use ::cookie::CookieJar;
use ::hyper::body::Bytes;
use ::hyper::http::header::AsHeaderName;
use ::hyper::http::header::HeaderName;
use ::hyper::http::header::SET_COOKIE;
use ::hyper::http::response::Parts;
use ::hyper::http::HeaderMap;
use ::hyper::http::HeaderValue;
use ::hyper::http::StatusCode;
use ::serde::Deserialize;
use ::std::convert::AsRef;
use ::std::fmt::Debug;
use ::std::fmt::Display;

///
/// The `TestResponse` represents the result of a `TestRequest`.
/// It is returned when you call await on a `TestRequest` object.
///
/// Inside are the contents of the response, the status code, and some
/// debugging information. You can use this to deserialise the data
/// returned into a specific format (i.e. deserialising from JSON),
/// and validating the response looks how you expect.
///
#[derive(Clone, Debug)]
pub struct TestResponse {
    request_url: String,
    headers: HeaderMap<HeaderValue>,
    status_code: StatusCode,
    response_body: Bytes,
}

impl TestResponse {
    pub(crate) fn new(request_url: String, parts: Parts, response_body: Bytes) -> Self {
        Self {
            request_url,
            headers: parts.headers,
            status_code: parts.status,
            response_body,
        }
    }

    /// The URL that was used to produce this response.
    #[must_use]
    pub fn request_url<'a>(&'a self) -> &'a str {
        &self.request_url
    }

    /// Returns the raw underlying response, as it's raw bytes.
    #[must_use]
    pub fn bytes<'a>(&'a self) -> &'a [u8] {
        &self.response_body
    }

    /// Returns the underlying response, as a raw UTF-8 string.
    #[must_use]
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.response_body).to_string()
    }

    /// The status_code of the response.
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    /// Finds a header with the given name.
    /// If there are multiple headers with the same name,
    /// then only the first will be returned.
    ///
    /// `None` is returned when no header was found.
    #[must_use]
    pub fn maybe_header<N>(&self, header_name: N) -> Option<HeaderValue>
    where
        N: AsHeaderName,
    {
        self.headers.get(header_name).map(|h| h.to_owned())
    }

    /// Returns the headers returned from the response.
    #[must_use]
    pub fn headers<'a>(&'a self) -> &'a HeaderMap<HeaderValue> {
        &self.headers
    }

    /// Finds a header with the given name.
    /// If there are multiple headers with the same name,
    /// then only the first will be returned.
    ///
    /// If no header is found, then this will panic.
    #[must_use]
    pub fn header<N>(&self, header_name: N) -> HeaderValue
    where
        N: AsHeaderName + Display + Clone,
    {
        let debug_header = header_name.clone();
        self.headers
            .get(header_name)
            .map(|h| h.to_owned())
            .with_context(|| {
                format!(
                    "Cannot find header {} for response {}",
                    debug_header, self.request_url
                )
            })
            .unwrap()
    }

    /// Iterates over all of the headers contained in the response.
    pub fn iter_headers<'a>(&'a self) -> impl Iterator<Item = (&'a HeaderName, &'a HeaderValue)> {
        self.headers.iter()
    }

    /// Iterates over all of the headers for a specific name, contained in the response.
    pub fn iter_headers_by_name<'a, N>(
        &'a self,
        header_name: N,
    ) -> impl Iterator<Item = &'a HeaderValue>
    where
        N: AsHeaderName,
    {
        self.headers.get_all(header_name).iter()
    }

    #[must_use]
    pub fn maybe_cookie(&self, cookie_name: &str) -> Option<Cookie<'static>> {
        for cookie in self.iter_cookies() {
            if cookie.name() == cookie_name {
                return Some(cookie.into_owned());
            }
        }

        None
    }

    #[must_use]
    pub fn cookie(&self, cookie_name: &str) -> Cookie<'static> {
        self.maybe_cookie(cookie_name)
            .with_context(|| {
                format!(
                    "Cannot find cookie {} for response {}",
                    cookie_name, self.request_url
                )
            })
            .unwrap()
    }

    /// Returns all of the cookies contained in the response,
    /// within a `CookieJar` object.
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
    #[must_use]
    pub fn iter_cookies<'a>(&'a self) -> impl Iterator<Item = Cookie<'a>> {
        self.iter_headers_by_name(SET_COOKIE).map(|header| {
            let header_str = header
                .to_str()
                .with_context(|| {
                    format!(
                        "Reading header 'Set-Cookie' as string for response {}",
                        self.request_url
                    )
                })
                .unwrap();

            Cookie::parse(header_str)
                .with_context(|| {
                    format!(
                        "Parsing 'Set-Cookie' header for response {}",
                        self.request_url
                    )
                })
                .unwrap()
        })
    }

    /// Reads the response from the server as JSON text,
    /// and then deserialise the contents into the structure given.
    #[must_use]
    pub fn json<T>(&self) -> T
    where
        for<'de> T: Deserialize<'de>,
    {
        serde_json::from_slice::<T>(&self.response_body)
            .with_context(|| {
                format!(
                    "Deserializing response from JSON for request {}",
                    self.request_url
                )
            })
            .unwrap()
    }

    /// This performs an assertion comparing the whole body of the response,
    /// against the text provided.
    pub fn assert_text<C>(self, other: C) -> Self
    where
        C: AsRef<str>,
    {
        let other_contents = other.as_ref();
        assert_eq!(&self.text(), other_contents);

        self
    }

    /// Deserializes the contents of the request,
    /// and asserts if it matches the value given.
    ///
    /// If `other` does not match, then this will panic.
    ///
    /// Other can be your own Serde model that you wish to deserialise
    /// the data into, or it can be a `json!` blob created using
    /// the `::serde_json::json` macro.
    pub fn assert_json<T>(self, other: &T) -> Self
    where
        for<'de> T: Deserialize<'de> + PartialEq<T> + Debug,
    {
        let own_json: T = self.json();
        assert_eq!(own_json, *other);

        self
    }

    pub fn assert_status_bad_request(self) -> Self {
        self.assert_status(StatusCode::BAD_REQUEST)
    }

    pub fn assert_status_not_found(self) -> Self {
        self.assert_status(StatusCode::NOT_FOUND)
    }

    pub fn assert_status_ok(self) -> Self {
        self.assert_status(StatusCode::OK)
    }

    pub fn assert_status_not_ok(self) -> Self {
        self.assert_not_status(StatusCode::OK)
    }

    pub fn assert_status(self, status_code: StatusCode) -> Self {
        assert_eq!(self.status_code(), status_code);

        self
    }

    pub fn assert_not_status(self, status_code: StatusCode) -> Self {
        assert_ne!(self.status_code(), status_code);

        self
    }
}
