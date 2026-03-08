#[cfg(feature = "yaml")]
use crate::internals::ErrorMessage;
use crate::internals::body_fmt::BodyTextFmt;
use bytes::Bytes;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

#[derive(Debug, Copy, Clone)]
pub struct BodyYamlFmt<'a>(pub &'a Bytes);

impl<'a> Display for BodyYamlFmt<'a> {
    #[cfg(not(feature = "yaml"))]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        BodyTextFmt(self.0).fmt(f)
    }

    #[cfg(feature = "yaml")]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let result = serde_yaml::from_slice::<serde_yaml::Value>(self.0);

        match result {
            Err(_) => {
                write!(
                    f,
                    "!!! YOUR YAML IS MALFORMED !!!

{}",
                    BodyTextFmt(self.0)
                )
            }
            Ok(body) => {
                let pretty_raw = serde_yaml::to_string(&body)
                    .error_message("Failed to reserialise serde_yaml::Value of request body");

                write!(f, "{pretty_raw}")
            }
        }
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_fmt {
    use super::*;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    #[test]
    fn it_should_pretty_print_yaml() {
        let yaml_bytes = serde_yaml::to_string(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
        .unwrap();

        let output = format!("{}", BodyYamlFmt(&yaml_bytes.into()));
        let expected = r###"name: Joe
age: 20
"###;

        assert_eq!(expected, output);
    }

    #[test]
    fn it_should_warn_on_malformed_yaml() {
        let yaml_bytes = "🦊 🦊 🦊: : :🦊 🦊 🦊".to_string();
        let output = format!("{}", BodyYamlFmt(&yaml_bytes.into()));
        let expected = r###"!!! YOUR YAML IS MALFORMED !!!

'🦊 🦊 🦊: : :🦊 🦊 🦊'"###;

        assert_eq!(expected, output);
    }
}
