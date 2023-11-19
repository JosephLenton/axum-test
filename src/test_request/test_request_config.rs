use ::url::Url;

use crate::internals::ExpectedState;
use crate::internals::RequestPathFormatter;

#[derive(Debug, Clone)]
pub struct TestRequestConfig {
    pub is_saving_cookies: bool,
    pub expected_state: ExpectedState,
    pub content_type: Option<String>,
    pub full_request_url: Url,
    pub request_format: RequestPathFormatter,
}
