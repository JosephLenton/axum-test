//!
//! This is a list of example snapshots, using Axum Test with Insta.
//! The examples show how to take snapshots of responses,
//! and the body of their responses.
//!
//! In the subfolder `/snapshots` you'll find the saved snapshots of
//! tests. Looking at them can show you what the snapshot output looks
//! like.
//!
//! To run the tests you can use the following command:
//! ```bash
//! cargo test --example=snapshots
//! ```
//!

fn main() {
    println!(
        r#"
This example is all tests.
Please run:
    cargo test --example=snapshots
"#
    )
}

#[cfg(test)]
mod test_response_snapshots {
    use axum::Json;
    use axum::Router;
    use axum::routing::get;
    use axum_test::TestServer;
    use axum_yaml::Yaml;
    use serde_json::json;

    #[tokio::test]
    async fn it_should_save_json_snapshots() {
        let app = Router::new().route(
            &"/todo/json",
            get(async || {
                Json(json!(
                    [
                        { "name": "shopping", "content": "buy eggs" },
                        { "name":"afternoon", "content": "buy shoes" }
                    ]
                ))
            }),
        );
        let server = TestServer::new(app).unwrap();

        // Get all example todos out from the server.
        let response = server.get(&"/todo/json").await;
        insta::assert_snapshot!(response);
    }

    #[tokio::test]
    async fn it_should_save_yaml_snapshots() {
        let app = Router::new().route(
            &"/todo/yaml",
            get(async || {
                Yaml(json!(
                    [
                        { "name": "shopping", "content": "buy eggs" },
                        { "name":"afternoon", "content": "buy shoes" }
                    ]
                ))
            }),
        );
        let server = TestServer::new(app).unwrap();

        // Get all example todos out from the server.
        let response = server.get(&"/todo/yaml").await;
        insta::assert_snapshot!(response);
    }
}
