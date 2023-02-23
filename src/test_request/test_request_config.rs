#[derive(Debug, Clone)]
pub struct TestRequestConfig {
    pub save_cookies: bool,
    pub content_type: Option<String>,
}
