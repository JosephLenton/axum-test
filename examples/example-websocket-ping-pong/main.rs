//!
//! This is a simple WebSocket example Application.
//! You send it data, and it will send it back.
//!
//! At the bottom of this file are a series of tests for using websockets.
//!
//! ```bash
//! # To run it's tests:
//! cargo test --example=example-websocket-ping-pong --features ws
//! ```
//!

use anyhow::Result;
use axum::extract::ws::WebSocket;
use axum::extract::WebSocketUpgrade;
use axum::response::Response;
use axum::routing::get;
use axum::serve::serve;
use axum::Router;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[cfg(test)]
use axum_test::TestServer;

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

pub async fn route_get_websocket_ping_pong(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| handle_ping_pong(socket))
}

async fn handle_ping_pong(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            // client disconnected
            return;
        };

        if socket.send(msg).await.is_err() {
            // client disconnected
            return;
        }
    }
}

pub(crate) fn new_app() -> Router {
    Router::new().route(&"/ws-ping-pong", get(route_get_websocket_ping_pong))
}

#[cfg(test)]
fn new_test_app() -> TestServer {
    let app = new_app();
    TestServer::builder()
        .http_transport() // Important! It must be a HTTP Transport here.
        .build(app)
        .unwrap()
}

#[cfg(test)]
mod test_websockets_ping_pong {
    use super::*;

    use serde_json::json;

    #[tokio::test]
    async fn it_should_start_a_websocket_connection() {
        let server = new_test_app();

        let response = server.get_websocket(&"/ws-ping-pong").await;

        response.assert_status_switching_protocols();
    }

    #[tokio::test]
    async fn it_should_ping_pong_text() {
        let server = new_test_app();

        let mut websocket = server
            .get_websocket(&"/ws-ping-pong")
            .await
            .into_websocket()
            .await;

        websocket.send_text("Hello!").await;
        websocket.assert_receive_text("Hello!").await;
    }

    #[tokio::test]
    async fn it_should_ping_pong_json() {
        let server = new_test_app();

        let mut websocket = server
            .get_websocket(&"/ws-ping-pong")
            .await
            .into_websocket()
            .await;

        websocket
            .send_json(&json!({
                "hello": "world",
                "numbers": [1, 2, 3],
            }))
            .await;
        websocket
            .assert_receive_json(&json!({
                "hello": "world",
                "numbers": [1, 2, 3],
            }))
            .await;
    }
}
