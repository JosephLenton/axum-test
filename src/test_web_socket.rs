use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::fmt::Display;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::WebSocketStream;

use crate::WsMessage;

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
      self.stream.close(None).await.expect("Failed to close WebSocket stream");
    }

    pub async fn send_text<T>(&mut self, raw_text: T)
    where
        T: Display,
    {
        let text = format!("{}", raw_text);
        self.send_message(WsMessage::Text(text)).await;
    }

    pub async fn send_message(&mut self, message: WsMessage) {
        self.stream.send(message).await.unwrap();
    }

    pub async fn receive_text(&mut self) -> String {
        let message = self.receive_message().await;
        message.is_text() {

        }
        self.stream.next().await.unwrap().unwrap()
    }

    pub async fn receive_message(&mut self) -> WsMessage {
        self.maybe_receive_message().await
          .expect("No message found on WebSocket stream")
    }

    pub async fn maybe_receive_message(&mut self) -> Option<WsMessage> {
        let maybe_message = self.stream.next().await;

        match maybe_message {
          None => None,
          Some(message_result) => {
            let message = message_result.expect("Failed to receive message from WebSocket stream");
            Some(message)
          },
        }
    }
}
