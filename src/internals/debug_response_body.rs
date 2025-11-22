use crate::TestResponse;
use bytesize::ByteSize;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

/// An arbituary limit to avoid printing gigabytes to the terminal.
const MAX_TEXT_PRINT_LEN: usize = 10_000;

#[derive(Debug)]
pub struct DebugResponseBody<'a>(pub &'a TestResponse);

impl Display for DebugResponseBody<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.0.maybe_content_type() {
            Some(content_type) => {
                match content_type.as_str() {
                    // Json
                    "application/json" | "text/json" => write_json(f, self.0),

                    // Msgpack
                    "application/msgpack" => write!(f, "<MsgPack>"),

                    // Yaml
                    #[cfg(feature = "yaml")]
                    "application/yaml" | "application/x-yaml" | "text/yaml" => {
                        write_yaml(f, self.0)
                    }

                    #[cfg(not(feature = "yaml"))]
                    "application/yaml" | "application/x-yaml" | "text/yaml" => {
                        write_text(f, &self.0.text())
                    }

                    // Text Content
                    s if s.starts_with("text/") => write_text(f, &self.0.text()),

                    // Byte Streams
                    "application/octet-stream" => {
                        let len = self.0.as_bytes().len();
                        write!(f, "<Bytes, with len {}>", ByteSize(len as u64))
                    }

                    // Unknown content type
                    _ => {
                        let len = self.0.as_bytes().len();
                        write!(
                            f,
                            "<Unknown content type, with len {}>",
                            ByteSize(len as u64)
                        )
                    }
                }
            }

            // We just default to text
            _ => write_text(f, &self.0.text()),
        }
    }
}

fn write_text(f: &mut Formatter<'_>, text: &str) -> FmtResult {
    let len = text.len();

    if len < MAX_TEXT_PRINT_LEN {
        write!(f, "'{text}'")
    } else {
        let text_start = text.chars().take(MAX_TEXT_PRINT_LEN);
        write!(f, "'")?;
        for c in text_start {
            write!(f, "{c}")?;
        }
        write!(f, "...'")?;

        Ok(())
    }
}

fn write_json(f: &mut Formatter<'_>, response: &TestResponse) -> FmtResult {
    let bytes = response.as_bytes();
    let result = serde_json::from_slice::<serde_json::Value>(bytes);

    match result {
        Err(_) => {
            write!(
                f,
                "!!! YOUR JSON IS MALFORMED !!!\nBody: '{}'",
                response.text()
            )
        }
        Ok(body) => {
            let pretty_raw = serde_json::to_string_pretty(&body)
                .expect("Failed to reserialise serde_json::Value of request body");
            write!(f, "{pretty_raw}")
        }
    }
}

#[cfg(feature = "yaml")]
fn write_yaml(f: &mut Formatter<'_>, response: &TestResponse) -> FmtResult {
    let response_bytes = response.as_bytes();
    let result = serde_yaml::from_slice::<serde_yaml::Value>(response_bytes);

    match result {
        Err(_) => {
            write!(
                f,
                "!!! YOUR YAML IS MALFORMED !!!\nBody: '{}'",
                response.text()
            )
        }
        Ok(body) => {
            let pretty_raw = serde_yaml::to_string(&body)
                .expect("Failed to reserialise serde_yaml::Value of request body");
            write!(f, "{pretty_raw}")
        }
    }
}

#[cfg(test)]
mod test_fmt {
    use super::*;
    use crate::TestServer;
    use axum::Json;
    use axum::Router;
    use axum::body::Body;
    use axum::response::IntoResponse;
    use axum::response::Response;
    use axum::routing::get;
    use http::HeaderValue;
    use http::header;
    use pretty_assertions::assert_eq;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    #[tokio::test]
    async fn it_should_display_text_response_as_text() {
        let router = Router::new().route("/text", get(|| async { "Blah blah" }));
        let response = TestServer::new(router).unwrap().get("/text").await;

        let debug_body = DebugResponseBody(&response);
        let output = format!("{debug_body}");

        assert_eq!(output, "'Blah blah'");
    }

    #[tokio::test]
    async fn it_should_cutoff_very_long_text() {
        let router = Router::new().route(
            "/text",
            get(|| async {
                let max_len = MAX_TEXT_PRINT_LEN + 100;
                (0..max_len).map(|_| "🦊").collect::<String>()
            }),
        );
        let response = TestServer::new(router).unwrap().get("/text").await;

        let debug_body = DebugResponseBody(&response);
        let output = format!("{debug_body}");

        let expected_content = (0..MAX_TEXT_PRINT_LEN).map(|_| "🦊").collect::<String>();
        let expected = format!("'{expected_content}...'");

        assert_eq!(output, expected);
    }

    #[tokio::test]
    async fn it_should_pretty_print_json() {
        let router = Router::new().route(
            "/json",
            get(|| async {
                Json(ExampleResponse {
                    name: "Joe".to_string(),
                    age: 20,
                })
            }),
        );
        let response = TestServer::new(router).unwrap().get("/json").await;

        let debug_body = DebugResponseBody(&response);
        let output = format!("{debug_body}");
        let expected = r###"{
  "age": 20,
  "name": "Joe"
}"###;

        assert_eq!(output, expected);
    }

    #[tokio::test]
    async fn it_should_warn_malformed_json() {
        let router = Router::new().route(
            "/json",
            get(|| async {
                let body = Body::new(r###"{ "name": "Joe" "###.to_string());

                Response::builder()
                    .header(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    )
                    .body(body)
                    .unwrap()
                    .into_response()
            }),
        );
        let response = TestServer::new(router).unwrap().get("/json").await;

        let debug_body = DebugResponseBody(&response);
        let output = format!("{debug_body}");
        let expected = r###"!!! YOUR JSON IS MALFORMED !!!
Body: '{ "name": "Joe" '"###;

        assert_eq!(output, expected);
    }

    #[cfg(feature = "yaml")]
    #[tokio::test]
    async fn it_should_pretty_print_yaml() {
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
        let response = TestServer::new(router).unwrap().get("/yaml").await;

        let debug_body = DebugResponseBody(&response);
        let output = format!("{debug_body}");
        let expected = r###"name: Joe
age: 20
"###;

        assert_eq!(output, expected);
    }

    #[cfg(feature = "yaml")]
    #[tokio::test]
    async fn it_should_warn_on_malformed_yaml() {
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
        let response = TestServer::new(router).unwrap().get("/yaml").await;

        let debug_body = DebugResponseBody(&response);
        let output = format!("{debug_body}");
        let expected = r###"!!! YOUR YAML IS MALFORMED !!!
Body: '🦊 🦊 🦊: : :🦊 🦊 🦊'"###;

        assert_eq!(output, expected);
    }
}
