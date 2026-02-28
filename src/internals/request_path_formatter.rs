use http::Method;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RequestPathFormatter<'a, U> {
    method: &'a Method,

    /// This is the path that the user requested.
    user_requested_path: &'a U,
}

impl<'a, U> RequestPathFormatter<'a, U> {
    pub fn new(method: &'a Method, user_requested_path: &'a U) -> Self {
        Self {
            method,
            user_requested_path,
        }
    }
}

impl<U> Display for RequestPathFormatter<'_, U>
where
    U: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let method = &self.method;
        let user_requested_path = &self.user_requested_path;

        write!(f, "{method} {user_requested_path}")
    }
}

#[cfg(test)]
mod test_fmt {
    use super::*;
    use crate::internals::Uri2;

    #[test]
    fn it_should_format_with_path_given() {
        let uri = Uri2::from_str("/donkeys");
        let debug = RequestPathFormatter::new(&Method::GET, &uri);
        let output = debug.to_string();

        assert_eq!(output, "GET /donkeys");
    }

    #[test]
    fn it_should_format_with_path_given_and_no_query_params() {
        let uri = Uri2::from_str("/donkeys");
        let debug = RequestPathFormatter::new(&Method::GET, &uri);
        let output = debug.to_string();

        assert_eq!(output, "GET /donkeys");
    }

    #[test]
    fn it_should_format_with_path_given_and_query_params() {
        let mut uri = Uri2::from_str("/donkeys");
        uri.add_raw_query_param("value=123");
        uri.add_raw_query_param("another-value");

        let debug = RequestPathFormatter::new(&Method::GET, &uri);
        let output = debug.to_string();

        assert_eq!(output, "GET /donkeys?value=123&another-value");
    }
}
