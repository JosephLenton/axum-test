#[cfg(feature = "yaml")]
use crate::internals::ErrorMessage;
use bytes::Bytes;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

#[derive(Debug, Copy, Clone)]
pub struct BodyYamlFmt<'a>(pub &'a Bytes);

impl<'a> Display for BodyYamlFmt<'a> {
    #[cfg(not(feature = "yaml"))]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use crate::internals::body_fmt::BodyTextFmt;

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
        use axum_yaml::Yaml;

        let router = Router::new().route(
            "/yaml",
            get(|| async {
                Yaml(ExampleResponse {
                    name: "Joe".to_string(),
                    age: 20,
                })
            }),
        );
        let response = TestServer::new(router).get("/yaml").await;

        let debug_body = DebugResponseBody(&response);
        let output = format!("{debug_body}");
        let expected = r###"name: Joe
age: 20
"###;

        assert_eq!(expected, output);
    }

    #[test]
    fn it_should_warn_on_malformed_yaml() {
        let router = Router::new().route(
            "/yaml",
            get(|| async {
                let body = Body::new("🦊 🦊 🦊: : :🦊 🦊 🦊".to_string());

                Response::builder()
                    .header(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/yaml"),
                    )
                    .body(body)
                    .unwrap()
                    .into_response()
            }),
        );
        let response = TestServer::new(router).get("/yaml").await;

        let debug_body = DebugResponseBody(&response);
        let output = format!("{debug_body}");
        let expected = r###"!!! YOUR YAML IS MALFORMED !!!

'🦊 🦊 🦊: : :🦊 🦊 🦊'"###;

        assert_eq!(expected, output);
    }
}
