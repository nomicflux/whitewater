use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::super::handler::Handler;
use super::shared::WSMessage;
use serde_json;

pub async fn connect_to_peer(ws_uri: String, handler: Handler) -> impl IntoResponse {
    let Ok((ws_stream, _)) = connect_async(&ws_uri).await else {
        eprintln!("Failed to connect to peer {ws_uri}");
        return;
    };

    let (mut write, mut read) = ws_stream.split();
    let msg_tx = handler.msg_tx.clone();
    let response_rx = handler.response_rx.clone();

    tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = read.next().await {
            if let Ok(msg) = serde_json::from_str::<WSMessage>(&text) {
                let _ = msg_tx.send(msg).await;
            }
        }
    });

    tokio::spawn(async move {
        while let Some(msg) = response_rx.lock().await.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                let _ = write.send(Message::Text(json.into())).await;
            }
        }
    });
}
