use http::Method;
use std::fmt;

use crate::internals::QueryParamsStore;

#[derive(Debug, Clone, PartialEq)]
pub struct RequestPathFormatter<'a> {
    method: &'a Method,

    /// This is the path that the user requested.
    user_requested_path: &'a str,
    query_params: Option<&'a QueryParamsStore>,
}

impl<'a> RequestPathFormatter<'a> {
    pub fn new(
        method: &'a Method,
        user_requested_path: &'a str,
        query_params: Option<&'a QueryParamsStore>,
    ) -> Self {
        Self {
            method,
            user_requested_path,
            query_params,
        }
    }
}

impl fmt::Display for RequestPathFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let method = &self.method;
        let user_requested_path = &self.user_requested_path;

        match self.query_params {
            None => {
                write!(f, "{method} {user_requested_path}")
            }
            Some(query_params) => {
                if query_params.is_empty() {
                    write!(f, "{method} {user_requested_path}")
                } else {
                    write!(f, "{method} {user_requested_path}?{query_params}")
                }
            }
        }
    }
}

#[cfg(test)]
mod test_fmt {
    use super::*;

    #[test]
    fn it_should_format_with_path_given() {
        let query_params = QueryParamsStore::new();
        let debug = RequestPathFormatter::new(&Method::GET, &"/donkeys", Some(&query_params));
        let output = debug.to_string();

        assert_eq!(output, "GET /donkeys");
    }

    #[test]
    fn it_should_format_with_path_given_and_no_query_params() {
        let debug = RequestPathFormatter::new(&Method::GET, &"/donkeys", None);
        let output = debug.to_string();

        assert_eq!(output, "GET /donkeys");
    }

    #[test]
    fn it_should_format_with_path_given_and_query_params() {
        let mut query_params = QueryParamsStore::new();
        query_params.add_raw("value=123".to_string());
        query_params.add_raw("another-value".to_string());

        let debug = RequestPathFormatter::new(&Method::GET, &"/donkeys", Some(&query_params));
        let output = debug.to_string();

        assert_eq!(output, "GET /donkeys?value=123&another-value");
    }
}
