use ::hyper::http::Method;

#[derive(Debug, Clone)]
pub struct TestRequestConfig {
    pub method: Method,
    pub path: String,
    pub save_cookies: bool,
}
