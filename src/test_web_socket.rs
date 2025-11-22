use crate::WsMessage;
use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use bytes::Bytes;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fmt::Debug;
use std::fmt::Display;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol::Role;

#[cfg(feature = "pretty-assertions")]
use pretty_assertions::assert_eq;

#[cfg(not(feature = "old-json-diff"))]
use expect_json::expect;
#[cfg(not(feature = "old-json-diff"))]
use expect_json::expect_json_eq;

#[derive(Debug)]
pub struct TestWebSocket {
    stream: WebSocketStream<TokioIo<Upgraded>>,
}

impl TestWebSocket {
    pub(crate) async fn new(upgraded: Upgraded) -> Self {
        let upgraded_io = TokioIo::new(upgraded);
        let stream = WebSocketStream::from_raw_socket(upgraded_io, Role::Client, None).await;

        Self { stream }
    }

    pub async fn close(mut self) {
        self.stream
            .close(None)
            .await
            .expect("Failed to close WebSocket stream");
    }

    pub async fn send_text<T>(&mut self, raw_text: T)
    where
        T: Display,
    {
        let text = raw_text.to_string();
        self.send_message(WsMessage::Text(text.into())).await;
    }

    pub async fn send_json<J>(&mut self, body: &J)
    where
        J: ?Sized + Serialize,
    {
        let raw_json =
            ::serde_json::to_string(body).expect("It should serialize the content into Json");

        self.send_message(WsMessage::Text(raw_json.into())).await;
    }

    #[cfg(feature = "yaml")]
    pub async fn send_yaml<Y>(&mut self, body: &Y)
    where
        Y: ?Sized + Serialize,
    {
        let raw_yaml =
            ::serde_yaml::to_string(body).expect("It should serialize the content into Yaml");

        self.send_message(WsMessage::Text(raw_yaml.into())).await;
    }

    #[cfg(feature = "msgpack")]
    pub async fn send_msgpack<M>(&mut self, body: &M)
    where
        M: ?Sized + Serialize,
    {
        let body_bytes =
            ::rmp_serde::to_vec(body).expect("It should serialize the content into MsgPack");

        self.send_message(WsMessage::Binary(body_bytes.into()))
            .await;
    }

    pub async fn send_message(&mut self, message: WsMessage) {
        self.stream.send(message).await.unwrap();
    }

    #[must_use]
    pub async fn receive_text(&mut self) -> String {
        let message = self.receive_message().await;

        message_to_text(message)
            .context("Failed to read message as a String")
            .unwrap()
    }

    #[must_use]
    pub async fn receive_json<T>(&mut self) -> T
    where
        T: DeserializeOwned,
    {
        let bytes = self.receive_bytes().await;
        serde_json::from_slice::<T>(&bytes)
            .context("Failed to deserialize message as Json")
            .unwrap()
    }

    #[cfg(feature = "yaml")]
    #[must_use]
    pub async fn receive_yaml<T>(&mut self) -> T
    where
        T: DeserializeOwned,
    {
        let bytes = self.receive_bytes().await;
        serde_yaml::from_slice::<T>(&bytes)
            .context("Failed to deserialize message as Yaml")
            .unwrap()
    }

    #[cfg(feature = "msgpack")]
    #[must_use]
    pub async fn receive_msgpack<T>(&mut self) -> T
    where
        T: DeserializeOwned,
    {
        let received_bytes = self.receive_bytes().await;
        rmp_serde::from_slice::<T>(&received_bytes)
            .context("Failed to deserializing message as MsgPack")
            .unwrap()
    }

    #[must_use]
    pub async fn receive_bytes(&mut self) -> Bytes {
        let message = self.receive_message().await;

        message_to_bytes(message)
            .context("Failed to read message as a Bytes")
            .unwrap()
    }

    #[must_use]
    pub async fn receive_message(&mut self) -> WsMessage {
        self.maybe_receive_message()
            .await
            .expect("No message found on WebSocket stream")
    }

    #[must_use]
    async fn maybe_receive_message(&mut self) -> Option<WsMessage> {
        let maybe_message = self.stream.next().await;

        match maybe_message {
            None => None,
            Some(message_result) => {
                let message =
                    message_result.expect("Failed to receive message from WebSocket stream");
                Some(message)
            }
        }
    }

    pub async fn assert_receive_json<T>(&mut self, expected: &T)
    where
        T: Serialize + DeserializeOwned + PartialEq<T> + Debug,
    {
        let received = self.receive_json::<T>().await;

        #[cfg(feature = "old-json-diff")]
        {
            assert_eq!(*expected, received);
        }

        #[cfg(not(feature = "old-json-diff"))]
        {
            if *expected != received {
                if let Err(error) = expect_json_eq(&received, &expected) {
                    panic!(
                        "
{error}
",
                    );
                }
            }
        }
    }

    pub async fn assert_receive_json_contains<T>(&mut self, expected: &T)
    where
        T: Serialize,
    {
        let received = self.receive_json::<Value>().await;

        #[cfg(feature = "old-json-diff")]
        {
            assert_json_diff::assert_json_include!(actual: received, expected: expected);
        }

        #[cfg(not(feature = "old-json-diff"))]
        {
            let expected_value = serde_json::to_value(expected).unwrap();
            let result = expect_json_eq(
                &received,
                &expect::object().propagated_contains(expected_value),
            );
            if let Err(error) = result {
                panic!(
                    "
{error}
",
                );
            }
        }
    }

    pub async fn assert_receive_text<C>(&mut self, expected: C)
    where
        C: AsRef<str>,
    {
        let expected_contents = expected.as_ref();
        assert_eq!(expected_contents, &self.receive_text().await);
    }

    pub async fn assert_receive_text_contains<C>(&mut self, expected: C)
    where
        C: AsRef<str>,
    {
        let expected_contents = expected.as_ref();
        let received = self.receive_text().await;
        let is_contained = received.contains(expected_contents);

        assert!(
            is_contained,
            "Failed to find '{expected_contents}', received '{received}'"
        );
    }

    #[cfg(feature = "yaml")]
    pub async fn assert_receive_yaml<T>(&mut self, expected: &T)
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*expected, self.receive_yaml::<T>().await);
    }

    #[cfg(feature = "msgpack")]
    pub async fn assert_receive_msgpack<T>(&mut self, expected: &T)
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*expected, self.receive_msgpack::<T>().await);
    }
}

fn message_to_text(message: WsMessage) -> Result<String> {
    let text = match message {
        WsMessage::Text(text) => text.to_string(),
        WsMessage::Binary(data) => {
            String::from_utf8(data.to_vec()).map_err(|err| err.utf8_error())?
        }
        WsMessage::Ping(data) => {
            String::from_utf8(data.to_vec()).map_err(|err| err.utf8_error())?
        }
        WsMessage::Pong(data) => {
            String::from_utf8(data.to_vec()).map_err(|err| err.utf8_error())?
        }
        WsMessage::Close(None) => String::new(),
        WsMessage::Close(Some(frame)) => frame.reason.to_string(),
        WsMessage::Frame(_) => {
            return Err(anyhow!(
                "Unexpected Frame, did not expect Frame message whilst reading"
            ));
        }
    };

    Ok(text)
}

fn message_to_bytes(message: WsMessage) -> Result<Bytes> {
    let bytes = match message {
        WsMessage::Text(string) => string.into(),
        WsMessage::Binary(data) => data,
        WsMessage::Ping(data) => data,
        WsMessage::Pong(data) => data,
        WsMessage::Close(None) => Bytes::new(),
        WsMessage::Close(Some(frame)) => frame.reason.into(),
        WsMessage::Frame(_) => {
            return Err(anyhow!(
                "Unexpected Frame, did not expect Frame message whilst reading"
            ));
        }
    };

    Ok(bytes)
}

#[cfg(test)]
mod test_assert_receive_text {
    use crate::TestServer;

    use axum::Router;
    use axum::extract::WebSocketUpgrade;
    use axum::extract::ws::Message;
    use axum::extract::ws::WebSocket;
    use axum::response::Response;
    use axum::routing::get;

    fn new_test_app() -> TestServer {
        pub async fn route_get_websocket_ping_pong(ws: WebSocketUpgrade) -> Response {
            async fn handle_ping_pong(mut socket: WebSocket) {
                while let Some(maybe_message) = socket.recv().await {
                    let message_text = maybe_message.unwrap().into_text().unwrap();

                    let encoded_text = format!("Text: {message_text}").try_into().unwrap();
                    let encoded_data = format!("Binary: {message_text}").into_bytes().into();

                    socket.send(Message::Text(encoded_text)).await.unwrap();
                    socket.send(Message::Binary(encoded_data)).await.unwrap();
                }
            }

            ws.on_upgrade(move |socket| handle_ping_pong(socket))
        }

        let app = Router::new().route(&"/ws-ping-pong", get(route_get_websocket_ping_pong));
        TestServer::builder().http_transport().build(app).unwrap()
    }

    #[tokio::test]
    async fn it_should_ping_pong_text_in_text_and_binary() {
        let server = new_test_app();

        let mut websocket = server
            .get_websocket(&"/ws-ping-pong")
            .await
            .into_websocket()
            .await;

        websocket.send_text("Hello World!").await;

        websocket.assert_receive_text("Text: Hello World!").await;
        websocket.assert_receive_text("Binary: Hello World!").await;
    }

    #[tokio::test]
    async fn it_should_ping_pong_large_text_blobs() {
        const LARGE_BLOB_SIZE: usize = 16777200; // Max websocket size (16mb) - 16 bytes for the 'Text: ' in the reply.
        let large_blob = (0..LARGE_BLOB_SIZE).map(|_| "X").collect::<String>();

        let server = new_test_app();
        let mut websocket = server
            .get_websocket(&"/ws-ping-pong")
            .await
            .into_websocket()
            .await;

        websocket.send_text(&large_blob).await;

        websocket
            .assert_receive_text(format!("Text: {large_blob}"))
            .await;
        websocket
            .assert_receive_text(format!("Binary: {large_blob}"))
            .await;
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_not_match_partial_text_match() {
        let server = new_test_app();

        let mut websocket = server
            .get_websocket(&"/ws-ping-pong")
            .await
            .into_websocket()
            .await;

        websocket.send_text("Hello World!").await;
        websocket.assert_receive_text("Hello World!").await;
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_not_match_different_text() {
        let server = new_test_app();

        let mut websocket = server
            .get_websocket(&"/ws-ping-pong")
            .await
            .into_websocket()
            .await;

        websocket.send_text("Hello World!").await;
        websocket.assert_receive_text("🦊").await;
    }
}

#[cfg(test)]
mod test_assert_receive_text_contains {
    use crate::TestServer;
    use axum::Router;
    use axum::extract::WebSocketUpgrade;
    use axum::extract::ws::Message;
    use axum::extract::ws::WebSocket;
    use axum::response::Response;
    use axum::routing::get;

    fn new_test_app() -> TestServer {
        pub async fn route_get_websocket_ping_pong(ws: WebSocketUpgrade) -> Response {
            async fn handle_ping_pong(mut socket: WebSocket) {
                while let Some(maybe_message) = socket.recv().await {
                    let message_text = maybe_message.unwrap().into_text().unwrap();
                    let encoded_text = format!("Text: {message_text}").try_into().unwrap();

                    socket.send(Message::Text(encoded_text)).await.unwrap();
                }
            }

            ws.on_upgrade(move |socket| handle_ping_pong(socket))
        }

        let app = Router::new().route(&"/ws-ping-pong", get(route_get_websocket_ping_pong));
        TestServer::builder().http_transport().build(app).unwrap()
    }

    #[tokio::test]
    async fn it_should_assert_whole_text_match() {
        let server = new_test_app();

        let mut websocket = server
            .get_websocket(&"/ws-ping-pong")
            .await
            .into_websocket()
            .await;

        websocket.send_text("Hello World!").await;
        websocket
            .assert_receive_text_contains("Text: Hello World!")
            .await;
    }

    #[tokio::test]
    async fn it_should_assert_partial_text_match() {
        let server = new_test_app();

        let mut websocket = server
            .get_websocket(&"/ws-ping-pong")
            .await
            .into_websocket()
            .await;

        websocket.send_text("Hello World!").await;
        websocket.assert_receive_text_contains("Hello World!").await;
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_not_match_different_text() {
        let server = new_test_app();

        let mut websocket = server
            .get_websocket(&"/ws-ping-pong")
            .await
            .into_websocket()
            .await;

        websocket.send_text("Hello World!").await;
        websocket.assert_receive_text_contains("🦊").await;
    }
}

#[cfg(test)]
mod test_assert_receive_json {
    use crate::TestServer;
    use axum::Router;
    use axum::extract::WebSocketUpgrade;
    use axum::extract::ws::Message;
    use axum::extract::ws::WebSocket;
    use axum::response::Response;
    use axum::routing::get;
    use serde_json::Value;
    use serde_json::json;

    #[cfg(not(feature = "old-json-diff"))]
    use crate::testing::ExpectStrMinLen;
    #[cfg(not(feature = "old-json-diff"))]
    use expect_json::expect;

    fn new_test_app() -> TestServer {
        pub async fn route_get_websocket_ping_pong(ws: WebSocketUpgrade) -> Response {
            async fn handle_ping_pong(mut socket: WebSocket) {
                while let Some(maybe_message) = socket.recv().await {
                    let message_text = maybe_message.unwrap().into_text().unwrap();
                    let decoded = serde_json::from_str::<Value>(&message_text).unwrap();

                    let encoded_text = serde_json::to_string(&json!({
                        "format": "text",
                        "message": decoded
                    }))
                    .unwrap()
                    .try_into()
                    .unwrap();
                    let encoded_data = serde_json::to_vec(&json!({
                        "format": "binary",
                        "message": decoded
                    }))
                    .unwrap()
                    .into();

                    socket.send(Message::Text(encoded_text)).await.unwrap();
                    socket.send(Message::Binary(encoded_data)).await.unwrap();
                }
            }

            ws.on_upgrade(move |socket| handle_ping_pong(socket))
        }

        let app = Router::new().route(&"/ws-ping-pong", get(route_get_websocket_ping_pong));
        TestServer::builder().http_transport().build(app).unwrap()
    }

    #[tokio::test]
    async fn it_should_ping_pong_json_in_text_and_binary() {
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

        // Once for text
        websocket
            .assert_receive_json(&json!({
                "format": "text",
                "message": {
                    "hello": "world",
                    "numbers": [1, 2, 3],
                },
            }))
            .await;

        // Again for binary
        websocket
            .assert_receive_json(&json!({
                "format": "binary",
                "message": {
                    "hello": "world",
                    "numbers": [1, 2, 3],
                },
            }))
            .await;
    }

    #[cfg(not(feature = "old-json-diff"))]
    #[tokio::test]
    async fn it_should_work_with_custom_expect_op() {
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

        // Once for text
        websocket
            .assert_receive_json(&json!({
                "format": "text",
                "message": {
                    "hello": ExpectStrMinLen { min: 3 },
                    "numbers": expect::array().len(3).all(expect::integer()),
                },
            }))
            .await;

        // Again for binary
        websocket
            .assert_receive_json(&json!({
                "format": "binary",
                "message": {
                    "hello": ExpectStrMinLen { min: 3 },
                    "numbers": expect::array().len(3).all(expect::integer()),
                },
            }))
            .await;
    }

    #[cfg(not(feature = "old-json-diff"))]
    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_custom_expect_op_fails() {
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

        // Once for text
        websocket
            .assert_receive_json(&json!({
                "format": "text",
                "message": {
                    "hello": ExpectStrMinLen { min: 10 },
                    "numbers": expect::array().len(3).all(expect::integer()),
                },
            }))
            .await;
    }
}

#[cfg(test)]
mod test_assert_receive_json_contains {
    use crate::TestServer;
    use axum::Router;
    use axum::extract::WebSocketUpgrade;
    use axum::extract::ws::Message;
    use axum::extract::ws::WebSocket;
    use axum::response::Response;
    use axum::routing::get;
    use serde_json::Value;
    use serde_json::json;

    fn new_test_app() -> TestServer {
        pub async fn route_get_websocket_ping_pong(ws: WebSocketUpgrade) -> Response {
            async fn handle_ping_pong(mut socket: WebSocket) {
                while let Some(maybe_message) = socket.recv().await {
                    let message_text = maybe_message.unwrap().into_text().unwrap();
                    let decoded = serde_json::from_str::<Value>(&message_text).unwrap();

                    let encoded_text = serde_json::to_string(&json!({
                        "format": "text",
                        "message": decoded
                    }))
                    .unwrap()
                    .try_into()
                    .unwrap();
                    let encoded_data = serde_json::to_vec(&json!({
                        "format": "binary",
                        "message": decoded
                    }))
                    .unwrap()
                    .into();

                    socket.send(Message::Text(encoded_text)).await.unwrap();
                    socket.send(Message::Binary(encoded_data)).await.unwrap();
                }
            }

            ws.on_upgrade(move |socket| handle_ping_pong(socket))
        }

        let app = Router::new().route(&"/ws-ping-pong", get(route_get_websocket_ping_pong));
        TestServer::builder().http_transport().build(app).unwrap()
    }

    #[tokio::test]
    async fn it_should_ping_pong_json_in_text_and_binary_with_root_content_missing_in_contains() {
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

        // Once for text
        websocket
            .assert_receive_json_contains(&json!({
                // "format" is missing here
                "message": {
                    "hello": "world",
                    "numbers": [1, 2, 3],
                },
            }))
            .await;

        // Again for binary
        websocket
            .assert_receive_json_contains(&json!({
                "format": "binary",
                // "message" is missing here
            }))
            .await;
    }

    #[tokio::test]
    async fn it_should_ping_pong_json_in_text_and_binary_with_nested_content_missing_in_contains() {
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

        // Once for text
        websocket
            .assert_receive_json_contains(&json!({
                "format": "text",
                "message": {
                    // "hello" is missing here
                    "numbers": [1, 2, 3],
                },
            }))
            .await;

        // Again for binary
        websocket
            .assert_receive_json_contains(&json!({
                "format": "binary",
                "message": {
                    "hello": "world",
                    // "numbers" is missing here
                },
            }))
            .await;
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_assert_receive_yaml {
    use crate::TestServer;

    use axum::Router;
    use axum::extract::WebSocketUpgrade;
    use axum::extract::ws::Message;
    use axum::extract::ws::WebSocket;
    use axum::response::Response;
    use axum::routing::get;
    use serde_json::Value;
    use serde_json::json;

    fn new_test_app() -> TestServer {
        pub async fn route_get_websocket_ping_pong(ws: WebSocketUpgrade) -> Response {
            async fn handle_ping_pong(mut socket: WebSocket) {
                while let Some(maybe_message) = socket.recv().await {
                    let message_text = maybe_message.unwrap().into_text().unwrap();
                    let decoded = serde_yaml::from_str::<Value>(&message_text).unwrap();

                    let encoded_text = serde_yaml::to_string(&json!({
                        "format": "text",
                        "message": decoded
                    }))
                    .unwrap()
                    .try_into()
                    .unwrap();
                    let encoded_data = serde_yaml::to_string(&json!({
                        "format": "binary",
                        "message": decoded
                    }))
                    .unwrap()
                    .into();

                    socket.send(Message::Text(encoded_text)).await.unwrap();
                    socket.send(Message::Binary(encoded_data)).await.unwrap();
                }
            }

            ws.on_upgrade(move |socket| handle_ping_pong(socket))
        }

        let app = Router::new().route(&"/ws-ping-pong", get(route_get_websocket_ping_pong));
        TestServer::builder().http_transport().build(app).unwrap()
    }

    #[tokio::test]
    async fn it_should_ping_pong_yaml_in_text_and_binary() {
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

        // Once for text
        websocket
            .assert_receive_yaml(&json!({
                "format": "text",
                "message": {
                    "hello": "world",
                    "numbers": [1, 2, 3],
                },
            }))
            .await;

        // Again for binary
        websocket
            .assert_receive_yaml(&json!({
                "format": "binary",
                "message": {
                    "hello": "world",
                    "numbers": [1, 2, 3],
                },
            }))
            .await;
    }
}

#[cfg(feature = "msgpack")]
#[cfg(test)]
mod test_assert_receive_msgpack {
    use crate::TestServer;

    use axum::Router;
    use axum::extract::WebSocketUpgrade;
    use axum::extract::ws::Message;
    use axum::extract::ws::WebSocket;
    use axum::response::Response;
    use axum::routing::get;
    use serde_json::Value;
    use serde_json::json;

    fn new_test_app() -> TestServer {
        pub async fn route_get_websocket_ping_pong(ws: WebSocketUpgrade) -> Response {
            async fn handle_ping_pong(mut socket: WebSocket) {
                while let Some(maybe_message) = socket.recv().await {
                    let message_data = maybe_message.unwrap().into_data();
                    let decoded = rmp_serde::from_slice::<Value>(&message_data).unwrap();

                    let encoded_data = ::rmp_serde::to_vec(&json!({
                        "format": "binary",
                        "message": decoded
                    }))
                    .unwrap()
                    .into();

                    socket.send(Message::Binary(encoded_data)).await.unwrap();
                }
            }

            ws.on_upgrade(move |socket| handle_ping_pong(socket))
        }

        let app = Router::new().route(&"/ws-ping-pong", get(route_get_websocket_ping_pong));
        TestServer::builder().http_transport().build(app).unwrap()
    }

    #[tokio::test]
    async fn it_should_ping_pong_msgpack_in_binary() {
        let server = new_test_app();

        let mut websocket = server
            .get_websocket(&"/ws-ping-pong")
            .await
            .into_websocket()
            .await;

        websocket
            .send_msgpack(&json!({
                "hello": "world",
                "numbers": [1, 2, 3],
            }))
            .await;

        websocket
            .assert_receive_msgpack(&json!({
                "format": "binary",
                "message": {
                    "hello": "world",
                    "numbers": [1, 2, 3],
                },
            }))
            .await;
    }
}
