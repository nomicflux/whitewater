use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};

use tokio::net::TcpStream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use serde_json;

use super::shared::WSMessage;

type WSStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

fn create_send_task(
    mut sender: SplitSink<WSStream, Message>,
    mut rx: UnboundedReceiver<WSMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(m) => {
                    if let Err(e) = sender.send(Message::Text(m.into())).await {
                        eprintln!("Error sending message: {e}");
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Error serializing JSON: {e}");
                }
            }
        }
    })
}

fn create_recv_task(
    mut receiver: SplitStream<WSStream>,
    tx: UnboundedSender<WSMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<WSMessage>(&text) {
                    Ok(wsm) => {
                        println!("Received {:?}", wsm);
                        match wsm {
                            WSMessage::Connect => {
                                let _ = tx.send(WSMessage::Heartbeat("".to_string()));
                            }
                            WSMessage::Heartbeat(peer_id) => {
                                let tx = tx.clone();
                                tokio::spawn(async move {
                                    let _ = sleep(Duration::from_secs(2));
                                    let _ = tx.send(WSMessage::Heartbeat("".to_string()));
                                });
                            }
                            WSMessage::Close => {
                                eprintln!("Closing connection.");
                                let _ = tx.send(WSMessage::Close);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error parsing JSON: {e}");
                    }
                },
                Message::Close(_) => {
                    eprintln!("Client closed connection.");
                    break;
                }
                _ => {}
            }
        }
    })
}

pub async fn spawn_client(to_whom: String) -> anyhow::Result<()> {
    let to_whom_c = to_whom.clone();
    let ws_stream = match connect_async(to_whom).await {
        Ok((stream, _response)) => {
            println!("Handshake for {to_whom_c} has been completed");
            stream
        }
        Err(e) => {
            println!("Error setting up connection to {to_whom_c}: {e}");
            return Err(anyhow::Error::new(e));
        }
    };

    let (sender, receiver) = ws_stream.split();
    let (tx, rx) = unbounded_channel::<WSMessage>();

    let mut send_task = create_send_task(sender, rx);
    let mut recv_task = create_recv_task(receiver, tx.clone());

    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            drop(tx);
            let _ = send_task.await;
        }
    }

    Ok(())
}
