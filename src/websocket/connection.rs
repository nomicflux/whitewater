use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};

use super::super::handler::Handler;
use super::super::websocket::shared::WSMessage;

enum WSMessageResult {
    Deserialized(WSMessage),
    DeserializationError(String),
    CloseStream,
    Noop,
}

trait WSMessageExt {
    fn deserialize(&self) -> WSMessageResult;
}

impl WSMessageExt for AxumMessage {
    fn deserialize(&self) -> WSMessageResult {
        match self {
            AxumMessage::Text(text) => match serde_json::from_str(text) {
                Ok(msg) => WSMessageResult::Deserialized(msg),
                Err(e) => WSMessageResult::DeserializationError(e.to_string()),
            },
            AxumMessage::Close(_) => WSMessageResult::CloseStream,
            _ => WSMessageResult::Noop,
        }
    }
}

impl WSMessageExt for TungsteniteMessage {
    fn deserialize(&self) -> WSMessageResult {
        match self {
            TungsteniteMessage::Text(text) => match serde_json::from_str(text) {
                Ok(msg) => WSMessageResult::Deserialized(msg),
                Err(e) => WSMessageResult::DeserializationError(e.to_string()),
            },
            TungsteniteMessage::Close(_) => WSMessageResult::CloseStream,
            _ => WSMessageResult::Noop,
        }
    }
}

pub struct Connection;

impl Connection {
    pub async fn accept(ws: WebSocketUpgrade, handler: Handler) -> impl IntoResponse {
        ws.on_upgrade(move |socket| Self::handle_axum_socket(socket, handler))
    }

    pub async fn connect(ws_url: String, handler: Handler) {
        if let Ok((stream, _)) = connect_async(&ws_url).await {
            let (write, read) = stream.split();
            Self::handle_socket(write, read, handler).await;
        }
    }

    async fn handle_axum_socket(socket: WebSocket, handler: Handler) {
        let (write, read) = socket.split();
        Self::handle_socket(write, read, handler).await;
    }

    async fn handle_socket<W, R, M, E>(mut write: W, mut read: R, handler: Handler)
    where
        M: WSMessageExt + Unpin + Send + From<WSMessage>,
        W: SinkExt<M> + Unpin + Send + 'static,
        R: StreamExt<Item = Result<M, E>> + Unpin + Send + 'static,
        E: std::error::Error + Send,
    {
        let handler_clone = handler.clone();

        tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                match msg.deserialize() {
                    WSMessageResult::Deserialized(ws_msg) => {
                        let _ = handler_clone.send_msg_to_process(ws_msg.clone()).await;
                    }
                    WSMessageResult::DeserializationError(e) => {
                        eprint!("Couldn't deserialize: {e}");
                        continue;
                    }
                    WSMessageResult::CloseStream => break,
                    WSMessageResult::Noop => continue,
                }
            }
        });

        let mut from_broadcast = handler.subscribe();

        tokio::spawn(async move {
            while let Ok(msg) = from_broadcast.recv().await {
                let _ = write.send(msg.into()).await;
            }
        });
    }
}
