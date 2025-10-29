use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::sync::Mutex;
use tokio::time::{Duration, timeout};
use std::sync::Arc;

use super::app_state::{AppState};
use super::websocket::shared::WSMessage;

#[derive(Clone)]
pub struct Handler {
    pub heartbeat_tx: Sender<()>,
    pub msg_tx: Sender<WSMessage>,
    pub response_rx: Arc<Mutex<Receiver<WSMessage>>>,
}

impl Handler {
    pub fn spawn(app_state: &AppState) -> Self {
        let (heartbeat_tx, heartbeat_rx) = channel::<()>(1);
        let (msg_tx, msg_rx) = channel::<WSMessage>(100);
        let (response_tx, response_rx) = channel::<WSMessage>(100);
        Self::setup_process_loop(app_state, heartbeat_tx.clone(), msg_rx);
        Self::setup_missed_heartbeat_loop(app_state, response_tx.clone(), heartbeat_rx);
        Self { heartbeat_tx, msg_tx, response_rx: Arc::new(Mutex::new(response_rx)) }
    }

    fn setup_process_loop(
        app_state: &AppState,
        heartbeat_tx: Sender<()>,
        mut msg_rx: Receiver<WSMessage>,
    ) {
        let app_state = app_state.clone();
        tokio::spawn(async move {
            while let Some(msg) = msg_rx.recv().await {
                Self::process_msg(&app_state, heartbeat_tx.clone(), msg);
            }
        });
    }

    fn setup_missed_heartbeat_loop(
        app_state: &AppState,
        response_tx: Sender<WSMessage>,
        mut heartbeat_rx: Receiver<()>,
    ) {
        let app_state = app_state.clone();
        tokio::spawn(async move {
            loop {
                let timeout_ms = rand::random_range(100..=300);
                let timeout_duration = Duration::from_millis(timeout_ms);
                let app_state = app_state.clone();
                match timeout(timeout_duration, heartbeat_rx.recv()).await {
                    Ok(Some(_)) => continue,
                    Ok(None) => break,
                    Err(_) => app_state.handle_missed_heartbeat(response_tx.clone()),
                };
            }
        });
    }

    async fn process_msg(_app_state: &AppState, heartbeat_tx: Sender<()>, msg: WSMessage) {
        match msg {
            WSMessage::Heartbeat(_peer) => heartbeat_tx.send(()).await,
            _ => Ok(()),
        };
    }
}
