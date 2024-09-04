//!
//! This is an example Todo Application using Web Sockets for communication.
//!
//! At the bottom of this file are a series of tests for using websockets.
//!
//! ```bash
//! # To run it's tests:
//! cargo test --example=example-websocket-chat --features ws
//! ```
//!

use anyhow::Result;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::Path;
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::response::Response;
use axum::routing::get;
use axum::serve::serve;
use axum::Router;
use futures_util::SinkExt;
use futures_util::StreamExt;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

#[cfg(test)]
use axum_test::TestServer;
#[cfg(test)]
use axum_test::TestServerConfig;

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

/// This my poor mans chat system.
///
/// It holds a map of User ID to Messages.
#[derive(Debug)]
pub struct AppState {
    user_messages: HashMap<String, Vec<ChatReceivedMessage>>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct ChatSendMessage {
    pub to: String,
    pub message: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct ChatReceivedMessage {
    pub from: String,
    pub message: String,
}

pub async fn route_get_websocket_chat(
    State(state): State<SharedAppState>,
    Path(username): Path<String>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_chat(socket, username, state.clone()))
}

async fn handle_chat(socket: WebSocket, username: String, state: SharedAppState) {
    let (mut sender, mut receiver) = socket.split();

    // Spawn a task that will push several messages to the client (does not matter what client does)
    let send_state = state.clone();
    let send_username = username.clone();
    let mut send_task = tokio::spawn(async move {
        loop {
            let mut state_locked = send_state.write().await;
            let maybe_messages = state_locked.user_messages.get_mut(&send_username);

            if let Some(messages) = maybe_messages {
                while let Some(message) = messages.pop() {
                    let json_text = serde_json::to_string(&message)
                        .expect("Failed to build JSON message for sending");

                    sender
                        .send(Message::Text(json_text))
                        .await
                        .expect("Failed to send message to socket");
                }
            }

            ::tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });

    // This second task will receive messages from client and print them on server console
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            let raw_text = message
                .into_text()
                .expect("Failed to read text from incoming message");
            let decoded = serde_json::from_str::<ChatSendMessage>(&raw_text)
                .expect("Failed to decode incoming JSON message");

            let mut state_locked = state.write().await;
            let maybe_messages = state_locked.user_messages.entry(decoded.to);
            maybe_messages.or_default().push(ChatReceivedMessage {
                from: username.clone(),
                message: decoded.message,
            });
        }
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(_) => println!("Messages sent"),
                Err(a) => println!("Error sending messages {a:?}")
            }
            recv_task.abort();
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(_) => println!("Received messages"),
                Err(b) => println!("Error receiving messages {b:?}")
            }
            send_task.abort();
        }
    }
}

pub(crate) fn new_app() -> Router {
    let state = AppState {
        user_messages: HashMap::new(),
    };
    let shared_state = Arc::new(RwLock::new(state));

    Router::new()
        .route(&"/ws-chat/:name", get(route_get_websocket_chat))
        .with_state(shared_state)
}

#[cfg(test)]
fn new_test_app() -> TestServer {
    let app = new_app();
    TestServerConfig::builder()
        .http_transport() // Important! It must be a HTTP Transport here.
        .build_server(app)
        .unwrap()
}

#[cfg(test)]
mod test_websockets_chat {
    use super::*;

    #[tokio::test]
    async fn it_should_start_a_websocket_connection() {
        let server = new_test_app();

        let response = server.get_websocket(&"/ws-chat/john").await;

        response.assert_status_switching_protocols();
    }

    #[tokio::test]
    async fn it_should_send_messages_back_and_forth() {
        let server = new_test_app();

        let mut alice_chat = server
            .get_websocket(&"/ws-chat/alice")
            .await
            .into_websocket()
            .await;

        let mut bob_chat = server
            .get_websocket(&"/ws-chat/bob")
            .await
            .into_websocket()
            .await;

        bob_chat
            .send_json(&ChatSendMessage {
                to: "alice".to_string(),
                message: "How are you Alice?".to_string(),
            })
            .await;

        alice_chat
            .assert_receive_json(&ChatReceivedMessage {
                from: "bob".to_string(),
                message: "How are you Alice?".to_string(),
            })
            .await;
        alice_chat
            .send_json(&ChatSendMessage {
                to: "bob".to_string(),
                message: "I am good".to_string(),
            })
            .await;
        alice_chat
            .send_json(&ChatSendMessage {
                to: "bob".to_string(),
                message: "How are you?".to_string(),
            })
            .await;

        bob_chat
            .assert_receive_json(&ChatReceivedMessage {
                from: "alice".to_string(),
                message: "I am good".to_string(),
            })
            .await;
        bob_chat
            .assert_receive_json(&ChatReceivedMessage {
                from: "alice".to_string(),
                message: "How are you?".to_string(),
            })
            .await;
    }
}
