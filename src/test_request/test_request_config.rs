use crate::internals::AtomicCrossCookieJar;
use crate::internals::ExpectedState;
use crate::internals::QueryParamsStore;
use cookie::CookieJar;
use http::HeaderName;
use http::HeaderValue;
use http::Method;
use std::sync::Arc;
use url::Url;

#[derive(Debug, Clone)]
pub struct TestRequestConfig {
    pub atomic_cookie_jar: Arc<AtomicCrossCookieJar>,

    pub is_saving_cookies: bool,
    pub expected_state: ExpectedState,
    pub content_type: Option<String>,
    pub full_request_url: Url,
    pub method: Method,

    pub cookies: CookieJar,
    pub query_params: QueryParamsStore,
    pub headers: Vec<(HeaderName, HeaderValue)>,
}
