use ::http::Method;
use ::std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct RequestPathFormatter {
    method: Method,

    /// This is the path that the user requested.
    user_requested_path: String,
}

impl RequestPathFormatter {
    pub fn new(method: Method, user_requested_path: String) -> Self {
        Self {
            method,
            user_requested_path,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }
}

impl fmt::Display for RequestPathFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let method = &self.method;
        let user_requested_path = &self.user_requested_path;

        write!(f, "{method} {user_requested_path}")
    }
}

#[cfg(test)]
mod test_fmt {
    use super::*;

    #[test]
    fn it_should_format_with_path_given() {
        let debug = RequestPathFormatter::new(Method::GET, "/donkeys".to_string());
        let output = format!("{}", debug);

        assert_eq!(output, "GET /donkeys");
    }
}
