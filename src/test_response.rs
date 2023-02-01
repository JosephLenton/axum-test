use ::anyhow::Context;
use ::hyper::body::Bytes;
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
pub struct TestResponse {
    request_url: String,
    response_body: Bytes,
    status_code: StatusCode,
}

impl TestResponse {
    pub(crate) fn new(request_url: String, response_body: Bytes, status_code: StatusCode) -> Self {
        Self {
            request_url,
            response_body,
            status_code,
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
