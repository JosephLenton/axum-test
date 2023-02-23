use ::hyper::http::Method;

#[derive(Debug, Clone)]
pub struct TestRequestDetails {
    pub method: Method,
    pub path: String,
}
