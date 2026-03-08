use crate::internals::ErrorMessage;
use crate::internals::body_fmt::BodyTextFmt;
use bytes::Bytes;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

#[derive(Debug, Copy, Clone)]
pub struct BodyJsonFmt<'a>(pub &'a Bytes);

impl<'a> Display for BodyJsonFmt<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let result = serde_json::from_slice::<serde_json::Value>(self.0);

        match result {
            Err(_) => {
                write!(
                    f,
                    "!!! YOUR JSON IS MALFORMED !!!

{}",
                    BodyTextFmt(self.0)
                )
            }
            Ok(body) => {
                let pretty_raw = serde_json::to_string_pretty(&body)
                    .error_message("Failed to reserialise serde_json::Value of request body");

                write!(f, "{pretty_raw}")
            }
        }
    }
}

#[cfg(test)]
mod test_fmt {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    #[test]
    fn it_should_pretty_print_json() {
        let json_bytes = serde_json::to_vec(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
        .unwrap();

        let output = format!("{}", BodyJsonFmt(&json_bytes.into()));
        let expected = r###"{
  "age": 20,
  "name": "Joe"
}"###;

        assert_eq!(expected, output);
    }

    #[test]
    fn it_should_warn_malformed_json() {
        let json_bytes = r###"{ "name": "Joe" "###.to_string();

        let output = format!("{}", BodyJsonFmt(&json_bytes.into()));
        let expected = r###"!!! YOUR JSON IS MALFORMED !!!

'{ "name": "Joe" '"###;

        assert_eq!(expected, output);
    }
}
