//!
//! This is an example Todo Application using Web Sockets for communication.
//!
//! At the bottom of this file are a series of tests for using websockets.
//!

use ::anyhow::anyhow;
use ::anyhow::Result;
use ::axum::extract::ws::WebSocket;
use ::axum::extract::Json;
use ::axum::extract::State;
use ::axum::extract::WebSocketUpgrade;
use ::axum::response::Response;
use ::axum::routing::get;
use ::axum::routing::post;
use ::axum::routing::put;
use ::axum::serve::serve;
use ::axum::Router;
use ::axum_extra::extract::cookie::Cookie;
use ::axum_extra::extract::cookie::CookieJar;
use ::http::StatusCode;
use ::serde::Deserialize;
use ::serde::Serialize;
use ::serde_email::Email;
use ::std::collections::HashMap;
use ::std::net::IpAddr;
use ::std::net::Ipv4Addr;
use ::std::net::SocketAddr;
use ::std::result::Result as StdResult;
use ::std::sync::Arc;
use ::std::sync::RwLock;
use ::tokio::net::TcpListener;

#[cfg(test)]
use ::axum_test::TestServer;
#[cfg(test)]
use ::axum_test::TestServerConfig;

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

type SharedAppState = Arc<RwLock<AppState>>;

// This my poor mans in memory DB.
#[derive(Debug)]
pub struct AppState {}

pub async fn route_get_websocket(
    State(state): State<SharedAppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state.clone()))
}

async fn handle_socket(mut socket: WebSocket, state: SharedAppState) {
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
    let state = AppState {};
    let shared_state = Arc::new(RwLock::new(state));

    Router::new()
        .route(&"/ws", get(route_get_websocket))
        .with_state(shared_state)
}

#[cfg(test)]
fn new_test_app() -> TestServer {
    let app = new_app();
    let config = TestServerConfig::builder()
        .http_transport()
        // .mock_transport()
        .build();

    TestServer::new_with_config(app, config).unwrap()
}

#[cfg(test)]
mod test_websockets {
    use super::*;

    use ::serde_json::json;

    #[tokio::test]
    async fn it_should_start_a_websocket_connection() {
        let server = new_test_app();

        let response = server
            .get_websocket(&"/ws")
            .expect_switching_protocols()
            .await;

        response.assert_status_switching_protocols();
    }
}
