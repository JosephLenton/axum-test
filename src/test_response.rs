use ::anyhow::Context;
use ::axum::http::StatusCode;
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
    pub request_url: String,
    pub contents: String,
    pub status_code: StatusCode,
}

impl TestResponse {
    pub(crate) fn new(request_url: String, contents: String, status_code: StatusCode) -> Self {
        Self {
            request_url,
            contents,
            status_code,
        }
    }

    pub fn request_url(&self) -> String {
        self.request_url.clone()
    }

    pub fn contents(&self) -> String {
        self.contents.clone()
    }

    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    pub fn json<T>(&self) -> T
    where
        for<'de> T: Deserialize<'de>,
    {
        serde_json::from_str::<T>(&self.contents)
            .with_context(|| {
                format!(
                    "Deserializing response from JSON for request {}",
                    self.request_url
                )
            })
            .unwrap()
    }

    pub fn assert_contents<C>(self, other: C) -> Self
    where
        C: AsRef<str>,
    {
        let other_contents = other.as_ref();
        assert_eq!(&self.contents, other_contents);

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
