//!
//! This is a list of example snapshots, using Axum Test with Insta.
//! The examples show how to take snapshots of responses,
//! and the body of their responses.
//!
//! In the subfolder `/snapshots` you'll find the saved snapshots of
//! tests. Looking at them can show you what the snapshot output looks
//! like.
//!
//! It is a tiny mock application that offers endpoints for:
//!  - `/todo/json`
//!  - `/todo/yaml`
//!  - `/example.png`
//!
//! To run you can use the following command:
//! ```bash
//! # Tests
//! cargo test --example=snapshots --features yaml
//!
//! # To run the example
//! cargo run --example=snapshots --features yaml
//! ```
//!

use anyhow::Result;
use axum::Json;
use axum::Router;
use axum::routing::get;
use axum::serve::serve;
use axum_yaml::Yaml;
use bytes::Bytes;
use http::header::CONTENT_TYPE;
use serde_json::json;
use std::fs;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::path::Path;
use tokio::net::TcpListener;

const PORT: u16 = 8080;

#[tokio::main]
async fn main() {
    let result: Result<()> = {
        let app = new_app();

        // Start!
        let ip_address = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        let address = SocketAddr::new(ip_address, PORT);
        let listener = TcpListener::bind(address).await.unwrap();
        serve(listener, app.into_make_service()).await.unwrap();

        Ok(())
    };

    match &result {
        Err(err) => eprintln!("{}", err),
        _ => {}
    };
}

pub(crate) fn new_app() -> Router {
    Router::new()
        .route(
            &"/todo/json",
            get(async || {
                Json(json!(
                    [
                        { "name": "shopping", "content": "buy eggs" },
                        { "name":"afternoon", "content": "buy shoes" }
                    ]
                ))
            }),
        )
        .route(
            &"/todo/yaml",
            get(async || {
                Yaml(json!(
                    [
                        { "name": "shopping", "content": "buy eggs" },
                        { "name":"afternoon", "content": "buy shoes" }
                    ]
                ))
            }),
        )
        .route(
            &"/example.png",
            get(async || {
                let path = Path::new(file!()).parent().unwrap().join("example.png");
                let data = fs::read(&path).unwrap();

                ([(CONTENT_TYPE, "image/png")], Bytes::from(data))
            }),
        )
}

#[cfg(test)]
mod test_response_snapshots {
    use super::*;
    use axum_test::TestServer;
    use serde_json::Value;

    #[tokio::test]
    async fn it_should_save_json_snapshots() {
        let server = TestServer::new(new_app()).unwrap();

        // Get all example todos out from the server.
        let response = server.get(&"/todo/json").await;
        insta::assert_snapshot!(response);
    }

    #[tokio::test]
    async fn it_should_save_json_snapshots_of_the_body() {
        let server = TestServer::new(new_app()).unwrap();

        // Get all example todos out from the server.
        let response = server.get(&"/todo/json").await;
        insta::assert_snapshot!(response.json::<Value>());
    }

    #[tokio::test]
    async fn it_should_save_yaml_snapshots() {
        let server = TestServer::new(new_app()).unwrap();

        // Get all example todos out from the server.
        let response = server.get(&"/todo/yaml").await;
        insta::assert_snapshot!(response);
    }

    #[tokio::test]
    async fn it_should_save_yaml_snapshots_of_the_body() {
        let server = TestServer::new(new_app()).unwrap();

        // Get all example todos out from the server.
        let response = server.get(&"/todo/yaml").await;
        insta::assert_snapshot!(response.yaml::<Value>());
    }

    #[tokio::test]
    async fn it_should_save_binary_snapshots() {
        let server = TestServer::new(new_app()).unwrap();

        // Get all example todos out from the server.
        let response = server.get(&"/example.png").await;
        insta::assert_snapshot!(response);
    }
}
