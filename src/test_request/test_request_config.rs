use ::http::Method;
use ::url::Url;

use crate::internals::ExpectedState;

#[derive(Debug, Clone)]
pub struct TestRequestConfig {
    pub is_saving_cookies: bool,
    pub expected_state: ExpectedState,
    pub content_type: Option<String>,
    pub method: Method,
    pub full_request_url: Url,
    pub path: String,
}
