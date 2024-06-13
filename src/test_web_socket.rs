use anyhow::anyhow;
use anyhow::Result;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::fmt::Display;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::WebSocketStream;

use crate::WsMessage;
use anyhow::Context;
use bytes::Bytes;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

#[cfg(feature = "pretty-assertions")]
use ::pretty_assertions::assert_eq;

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
        let text = format!("{}", raw_text);
        self.send_message(WsMessage::Text(text)).await;
    }

    pub async fn send_json<J>(&mut self, body: &J)
    where
        J: ?Sized + Serialize,
    {
        let raw_json =
            ::serde_json::to_string(body).expect("It should serialize the content into Json");

        self.send_message(WsMessage::Text(raw_json)).await;
    }

    #[cfg(feature = "yaml")]
    pub async fn send_yaml<Y>(&mut self, body: &Y)
    where
        Y: ?Sized + Serialize,
    {
        let raw_yaml =
            ::serde_yaml::to_string(body).expect("It should serialize the content into Yaml");

        self.send_message(WsMessage::Text(raw_yaml)).await;
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

    pub async fn assert_receive_json<T>(&mut self, expected: &T)
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*expected, self.receive_json::<T>().await);
    }

    pub async fn assert_receive_text<C>(&mut self, expected: C)
    where
        C: AsRef<str>,
    {
        let expected_contents = expected.as_ref();
        assert_eq!(expected_contents, &self.receive_text().await);
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

    #[must_use]
    pub async fn maybe_receive_message(&mut self) -> Option<WsMessage> {
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
}

fn message_to_text(message: WsMessage) -> Result<String> {
    let text = match message {
        WsMessage::Text(text) => text,
        WsMessage::Binary(data) => String::from_utf8(data).map_err(|err| err.utf8_error())?,
        WsMessage::Ping(data) => String::from_utf8(data).map_err(|err| err.utf8_error())?,
        WsMessage::Pong(data) => String::from_utf8(data).map_err(|err| err.utf8_error())?,
        WsMessage::Close(None) => String::new(),
        WsMessage::Close(Some(frame)) => frame.reason.into_owned(),
        WsMessage::Frame(_) => {
            return Err(anyhow!(
                "Unexpected Frame, did not expect Frame message whilst reading"
            ))
        }
    };

    Ok(text)
}

fn message_to_bytes(message: WsMessage) -> Result<Bytes> {
    let bytes = match message {
        WsMessage::Text(string) => string.into_bytes().into(),
        WsMessage::Binary(data) => data.into(),
        WsMessage::Ping(data) => data.into(),
        WsMessage::Pong(data) => data.into(),
        WsMessage::Close(None) => Bytes::new(),
        WsMessage::Close(Some(frame)) => frame.reason.into_owned().into_bytes().into(),
        WsMessage::Frame(_) => {
            return Err(anyhow!(
                "Unexpected Frame, did not expect Frame message whilst reading"
            ))
        }
    };

    Ok(bytes)
}
