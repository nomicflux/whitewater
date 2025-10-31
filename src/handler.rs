use tokio::sync::broadcast;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::time::{Duration, timeout};

use super::app_state::AppState;
use super::websocket::shared::WSMessage;

#[derive(Clone)]
pub struct Handler {
    server_tx: Sender<WSMessage>,
    client_tx: broadcast::Sender<WSMessage>,
}

impl Handler {
    pub fn spawn(app_state: &AppState) -> Self {
        let (heartbeat_tx, heartbeat_rx) = channel::<()>(1);
        let (server_tx, server_rx) = channel::<WSMessage>(100);
        let (client_tx, _) = broadcast::channel::<WSMessage>(100);
        Self::setup_process_loop(app_state, heartbeat_tx.clone(), server_rx);
        Self::setup_missed_heartbeat_loop(app_state, heartbeat_rx, client_tx.clone());
        Self {
            server_tx,
            client_tx,
        }
    }

    pub async fn send_msg_to_process(&self, msg: WSMessage) {
        let _ = self.server_tx.send(msg).await;
    }

    pub async fn send_broadcast_msg(&self, msg: WSMessage) {
        match self.client_tx.send(msg) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error sending broadcast message: {e}");
            }
        };
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WSMessage> {
        self.client_tx.subscribe()
    }

    fn setup_process_loop(
        app_state: &AppState,
        heartbeat_tx: Sender<()>,
        mut server_rx: Receiver<WSMessage>,
    ) {
        let app_state = app_state.clone();
        tokio::spawn(async move {
            while let Some(msg) = server_rx.recv().await {
                let _ = Self::process_msg(&app_state, heartbeat_tx.clone(), msg).await;
            }
        });
    }

    fn setup_missed_heartbeat_loop(
        app_state: &AppState,
        mut heartbeat_rx: Receiver<()>,
        client_tx: broadcast::Sender<WSMessage>,
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
                    Err(_) => {
                        let state_machine = app_state.state_machine.lock().await;
                        app_state
                            .raft_state
                            .lock()
                            .await
                            .handle_missed_heartbeat(client_tx.clone(), &state_machine)
                            .await;
                    }
                };
            }
        });
    }

    async fn process_msg(_app_state: &AppState, heartbeat_tx: Sender<()>, msg: WSMessage) {
        match msg {
            WSMessage::AppendEntries {
                term,
                leader_id,
                prev_log_index,
                prev_log_term,
                entries,
            } => {
                let _ = heartbeat_tx.send(()).await;
            }
            WSMessage::AppendEntriesResponse { term, success } => {}
            WSMessage::RequestVote {
                term,
                candidate_id,
                last_log_index,
                last_log_term,
            } => {}
            WSMessage::RequestVoteResponse { term, vote_granted } => {}
        }
    }
}
