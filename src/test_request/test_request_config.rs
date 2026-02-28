use crate::internals::AtomicCrossCookieJar;
use crate::internals::ExpectedState;
use crate::internals::Uri2;
use cookie::CookieJar;
use http::HeaderName;
use http::HeaderValue;
use http::Method;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TestRequestConfig {
    pub atomic_cookie_jar: Arc<AtomicCrossCookieJar>,

    pub is_saving_cookies: bool,
    pub expected_state: ExpectedState,
    pub content_type: Option<String>,
    pub request_uri: Uri2,
    pub method: Method,

    pub cookies: CookieJar,
    pub headers: Vec<(HeaderName, HeaderValue)>,
}
