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

/// This contains the response from the server.
///
/// Inside are the contents of the response, the status code, and some
/// debugging information.
///
/// You can get the contents out as it's raw string, or deserialise it.
/// One can also also use the `assert_*` functions to test against the
/// response.
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
    pub fn request_url<'a>(&'a self) -> &'a str {
        &self.request_url
    }

    /// Returns the raw underlying response, as it's raw bytes.
    pub fn bytes<'a>(&'a self) -> &'a [u8] {
        &self.response_body
    }

    /// Returns the underlying response, as a raw UTF-8 string.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.response_body).to_string()
    }

    /// The status_code of the response.
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    pub fn header<N>(&self, header_name: N) -> Option<HeaderValue>
    where
        N: AsHeaderName,
    {
        self.headers.get(header_name).map(|h| h.to_owned())
    }

    pub fn iter_headers<'a>(&'a self) -> impl Iterator<Item = (&'a HeaderName, &'a HeaderValue)> {
        self.headers.iter()
    }

    pub fn iter_headers_by_name<'a, N>(
        &'a self,
        header_name: N,
    ) -> impl Iterator<Item = &'a HeaderValue>
    where
        N: AsHeaderName,
    {
        self.headers.get_all(header_name).iter()
    }

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

    pub fn maybe_cookie(&self, cookie_name: &str) -> Option<Cookie<'static>> {
        for cookie in self.iter_cookies() {
            if cookie.name() == cookie_name {
                return Some(cookie.into_owned());
            }
        }

        None
    }

    pub fn cookies(&self) -> CookieJar {
        let mut cookies = CookieJar::new();

        for cookie in self.iter_cookies() {
            cookies.add(cookie.into_owned());
        }

        cookies
    }

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
