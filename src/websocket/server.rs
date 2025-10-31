use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};

use super::super::handler::Handler;
use super::shared::WSMessage;
use serde_json;

pub async fn ws_handler(ws: WebSocketUpgrade, handler: Handler) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, handler))
}

async fn handle_socket(socket: WebSocket, handler: Handler) {
    let (mut msg_tx, mut msg_rx) = socket.split();

    tokio::spawn(async move {
        while let Some(Ok(msg)) = msg_rx.next().await {
            match msg {
                Message::Text(text) => {
                    println!("Received: {text}");
                    if let Ok(m) = serde_json::from_str::<WSMessage>(&text) {
                        let _ = handler.msg_tx.send(m).await;
                    }
                }
                Message::Close(_) => {
                    println!("Closing connection.");
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::spawn(async move {
        while let Some(msg) = handler.response_rx.lock().await.recv().await {
            if let Ok(m) = serde_json::to_string(&msg) {
                let _ = msg_tx.send(Message::Text(m.into())).await;
            }
        }
    });
}
