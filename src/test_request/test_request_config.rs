use ::http::Method;

#[derive(Debug, Clone)]
pub struct TestRequestConfig {
    pub is_saving_cookies: bool,
    pub is_expecting_success_by_default: bool,
    pub content_type: Option<String>,
    pub method: Method,
    pub full_request_path: String,
    pub path: String,
}
