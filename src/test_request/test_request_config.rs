use ::cookie::CookieJar;
use ::http::HeaderName;
use ::http::HeaderValue;
use ::http::Method;
use ::url::Url;

use crate::internals::ExpectedState;
use crate::internals::QueryParamsStore;

#[derive(Debug, Clone)]
pub struct TestRequestConfig {
    pub is_saving_cookies: bool,
    pub expected_state: ExpectedState,
    pub content_type: Option<String>,
    pub full_request_url: Url,
    pub method: Method,

    pub cookies: CookieJar,
    pub query_params: QueryParamsStore,
    pub headers: Vec<(HeaderName, HeaderValue)>,
}
