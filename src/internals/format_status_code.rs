use ::http::StatusCode;
use ::std::fmt;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FormatStatusCode(pub StatusCode);

impl fmt::Display for FormatStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.0.as_u16();

        match self.0.canonical_reason() {
            Some(reason) => write!(f, "{code} ({reason})"),
            None => write!(f, "{code}"),
        }
    }
}

#[cfg(test)]
mod test_fmt {
    use super::*;

    #[test]
    fn it_should_format_with_reason_where_available() {
        let status_code = StatusCode::UNAUTHORIZED;
        let debug = FormatStatusCode(status_code);
        let output = format!("{}", debug);

        assert_eq!(output, "401 (Unauthorized)");
    }

    #[test]
    fn it_should_provide_only_number_where_reason_is_unavailable() {
        let status_code = StatusCode::from_u16(218).unwrap(); // Unofficial Apache status code.
        let debug = FormatStatusCode(status_code);
        let output = format!("{}", debug);

        assert_eq!(output, "218");
    }
}
