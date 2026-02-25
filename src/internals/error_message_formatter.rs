use crate::internals::RequestPathFormatter;
use bytes::Bytes;
use std::convert::Infallible;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

pub struct ErrorMessageFormatter<'a, E = Infallible> {
    message: &'a str,
    maybe_request_path: Option<RequestPathFormatter<'a>>,
    maybe_error: Option<E>,
    maybe_body_bytes: Option<&'a Bytes>,
}

impl<'a> ErrorMessageFormatter<'a> {
    pub fn new(message: &'a str) -> Self {
        Self {
            message,
            maybe_request_path: None,
            maybe_error: None,
            maybe_body_bytes: None,
        }
    }
}

impl<'a, E> ErrorMessageFormatter<'a, E> {
    pub fn request_path(mut self, path: RequestPathFormatter<'a>) -> Self {
        self.maybe_request_path = Some(path);
        self
    }

    pub fn error<E2>(self, error: E2) -> ErrorMessageFormatter<'a, E2>
    where
        E2: Display,
    {
        ErrorMessageFormatter::<'a, E2> {
            maybe_error: Some(error),

            message: self.message,
            maybe_request_path: self.maybe_request_path,
            maybe_body_bytes: self.maybe_body_bytes,
        }
    }

    pub fn body(mut self, body: &'a Bytes) -> Self {
        self.maybe_body_bytes = Some(body);
        self
    }
}

impl<'a, E> Display for ErrorMessageFormatter<'a, E>
where
    E: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let message = self.message;

        write!(f, "{message}")?;

        let has_trailing_comma = self.maybe_request_path.is_some() || self.maybe_error.is_some();
        if has_trailing_comma {
            write!(f, ",")?;
        }

        if let Some(request_path) = self.maybe_request_path {
            writeln!(f)?;
            write!(f, "    for request {request_path}")?;
        }

        if let Some(error) = self.maybe_error.as_ref() {
            writeln!(f)?;
            write!(f, "    {error}")?;
        }

        if let Some(body_bytes) = self.maybe_body_bytes {
            let body_str = String::from_utf8_lossy(body_bytes);
            let is_whitespace_trim_needed = body_str.ends_with("\n");
            let response_text_string = body_str.replace("\n", "\n    ");
            let response_text = if is_whitespace_trim_needed {
                &response_text_string[..response_text_string.len() - 4]
            } else {
                &response_text_string
            };

            writeln!(f)?;
            writeln!(f)?;
            write!(
                f,
                "received:
    {response_text}"
            )?;
        }

        writeln!(f)
    }
}

#[cfg(test)]
mod test_fmt {
    use super::*;
    use crate::internals::QueryParamsStore;
    use anyhow::anyhow;
    use http::Method;
    use pretty_assertions::assert_str_eq;
    use serde_json::json;

    #[test]
    fn it_should_format_error_message_on_its_own() {
        let message = ErrorMessageFormatter::new("this is an error").to_string();

        assert_str_eq!(
            "this is an error
",
            message
        )
    }

    #[test]
    fn it_should_format_error_message_with_error() {
        let error = anyhow!("some internal error");
        let message = ErrorMessageFormatter::new("this is an error")
            .error(error)
            .to_string();

        assert_str_eq!(
            "this is an error,
    some internal error
",
            message
        )
    }

    #[test]
    fn it_should_format_error_message_with_request_path() {
        let query_params = QueryParamsStore::new();
        let path = RequestPathFormatter::new(&Method::GET, &"/donkeys", Some(&query_params));
        let message = ErrorMessageFormatter::new("this is an error")
            .request_path(path)
            .to_string();

        assert_str_eq!(
            "this is an error,
    for request GET /donkeys
",
            message
        )
    }

    #[test]
    fn it_should_format_error_message_with_error_and_request_path() {
        let error = anyhow!("some internal error");
        let query_params = QueryParamsStore::new();
        let path = RequestPathFormatter::new(&Method::GET, &"/something", Some(&query_params));
        let message = ErrorMessageFormatter::new("this is an error")
            .error(error)
            .request_path(path)
            .to_string();

        assert_str_eq!(
            "this is an error,
    for request GET /something
    some internal error
",
            message
        )
    }

    #[test]
    fn it_should_format_error_message_with_request_path_and_json_body() {
        let query_params = QueryParamsStore::new();
        let path = RequestPathFormatter::new(&Method::GET, &"/json", Some(&query_params));
        let json_body = json!({
            "user_id": "abc123",
            "username": "MrUser",
        })
        .to_string()
        .into();

        let message = ErrorMessageFormatter::new("this is an error")
            .request_path(path)
            .body(&json_body)
            .to_string();

        assert_str_eq!(
            r#"this is an error,
    for request GET /json

received:
    {"user_id":"abc123","username":"MrUser"}
"#,
            message
        )
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn it_should_format_error_message_with_request_path_and_yaml_body() {
        let query_params = QueryParamsStore::new();
        let path = RequestPathFormatter::new(&Method::GET, &"/yaml", Some(&query_params));
        let yaml_body = serde_yaml::to_string(&json!({
            "user_id": "abc123",
            "username": "MrUser",
        }))
        .unwrap()
        .into();

        let message = ErrorMessageFormatter::new("this is an error")
            .request_path(path)
            .body(&yaml_body)
            .to_string();

        assert_str_eq!(
            r#"this is an error,
    for request GET /yaml

received:
    user_id: abc123
    username: MrUser

"#,
            message
        )
    }

    #[test]
    fn it_should_format_error_message_with_request_path_and_text_body() {
        let query_params = QueryParamsStore::new();
        let path = RequestPathFormatter::new(&Method::GET, &"/text", Some(&query_params));
        let text_body = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.
Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.
Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."
        .into();

        let message = ErrorMessageFormatter::new("this is an error")
            .request_path(path)
            .body(&text_body)
            .to_string();

        assert_str_eq!(
            r#"this is an error,
    for request GET /text

received:
    Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
    Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.
    Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.
    Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.
"#,
            message
        )
    }
}
