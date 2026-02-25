use crate::TestResponse;
use crate::internals::ErrorMessageFormatter;
use bytes::Bytes;
use std::fmt::Display;

/// `ErrorMessage` adds a standard error formatting to [`Result`] and [`Option`] types,
/// for use over [`Result::unwrap`] and [`Option::unwrap`].
#[allow(dead_code, reason = "These are used under feature flags")]
pub trait ErrorMessage<V> {
    fn error_message(self, message: &str) -> V;
    fn error_message_fn<F>(self, message_func: F) -> V
    where
        F: FnOnce() -> String;

    fn error_message_with_body(self, message: &str, bytes: &Bytes) -> V;

    fn error_response(self, message: &str, response: &TestResponse) -> V;
    fn error_response_fn<F>(self, message_func: F, response: &TestResponse) -> V
    where
        F: FnOnce() -> String;
    fn error_response_with_body(self, message: &str, response: &TestResponse) -> V;
}

impl<V, E> ErrorMessage<V> for Result<V, E>
where
    E: Display,
{
    fn error_message(self, message: &str) -> V {
        self.unwrap_or_else(|err| {
            let err_message = ErrorMessageFormatter::new(message).error(err);

            panic!("{err_message}")
        })
    }

    fn error_message_fn<F>(self, message_func: F) -> V
    where
        F: FnOnce() -> String,
    {
        self.unwrap_or_else(|err| {
            let message = message_func();
            let err_message = ErrorMessageFormatter::new(&message).error(err);

            panic!("{err_message}")
        })
    }

    fn error_message_with_body(self, message: &str, bytes: &Bytes) -> V {
        self.unwrap_or_else(|err| {
            let err_message = ErrorMessageFormatter::new(message).body(bytes).error(err);

            panic!("{err_message}")
        })
    }

    fn error_response(self, message: &str, response: &TestResponse) -> V {
        self.unwrap_or_else(|err| {
            let debug_request_format = response.debug_request_format();
            let err_message = ErrorMessageFormatter::new(message)
                .request_path(debug_request_format)
                .error(err);

            panic!("{err_message}")
        })
    }

    fn error_response_fn<F>(self, message_func: F, response: &TestResponse) -> V
    where
        F: FnOnce() -> String,
    {
        self.unwrap_or_else(|err| {
            let message = message_func();
            let debug_request_format = response.debug_request_format();
            let err_message = ErrorMessageFormatter::new(&message)
                .request_path(debug_request_format)
                .error(err);

            panic!("{err_message}")
        })
    }

    fn error_response_with_body(self, message: &str, response: &TestResponse) -> V {
        self.unwrap_or_else(|err| {
            let debug_request_format = response.debug_request_format();
            let body = response.as_bytes();
            let err_message = ErrorMessageFormatter::new(message)
                .request_path(debug_request_format)
                .body(body)
                .error(err);

            panic!("{err_message}")
        })
    }
}

impl<V> ErrorMessage<V> for Option<V> {
    fn error_message(self, message: &str) -> V {
        self.unwrap_or_else(|| {
            let err_message = ErrorMessageFormatter::new(message);

            panic!("{err_message}")
        })
    }

    fn error_message_fn<F>(self, message_func: F) -> V
    where
        F: FnOnce() -> String,
    {
        self.unwrap_or_else(|| {
            let message = message_func();
            let err_message = ErrorMessageFormatter::new(&message);

            panic!("{err_message}")
        })
    }

    fn error_message_with_body(self, message: &str, bytes: &Bytes) -> V {
        self.unwrap_or_else(|| {
            let err_message = ErrorMessageFormatter::new(message).body(bytes);

            panic!("{err_message}")
        })
    }

    fn error_response(self, message: &str, response: &TestResponse) -> V {
        self.unwrap_or_else(|| {
            let debug_request_format = response.debug_request_format();
            let err_message =
                ErrorMessageFormatter::new(message).request_path(debug_request_format);

            panic!("{err_message}")
        })
    }

    fn error_response_fn<F>(self, message_func: F, response: &TestResponse) -> V
    where
        F: FnOnce() -> String,
    {
        self.unwrap_or_else(|| {
            let message = message_func();
            let debug_request_format = response.debug_request_format();
            let err_message =
                ErrorMessageFormatter::new(&message).request_path(debug_request_format);

            panic!("{err_message}")
        })
    }

    fn error_response_with_body(self, message: &str, response: &TestResponse) -> V {
        self.unwrap_or_else(|| {
            let debug_request_format = response.debug_request_format();
            let body = response.as_bytes();
            let err_message = ErrorMessageFormatter::new(message)
                .request_path(debug_request_format)
                .body(body);

            panic!("{err_message}")
        })
    }
}
