use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::time::{Duration, timeout};

use super::app_state::AppState;
use super::websocket::shared::WSMessage;

#[derive(Clone)]
pub struct Handler {
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
        Self {
            msg_tx,
            response_rx: Arc::new(Mutex::new(response_rx)),
        }
    }

    fn setup_process_loop(
        app_state: &AppState,
        heartbeat_tx: Sender<()>,
        mut msg_rx: Receiver<WSMessage>,
    ) {
        let app_state = app_state.clone();
        tokio::spawn(async move {
            while let Some(msg) = msg_rx.recv().await {
                let _ = Self::process_msg(&app_state, heartbeat_tx.clone(), msg).await;
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
                let _ = match timeout(timeout_duration, heartbeat_rx.recv()).await {
                    Ok(Some(_)) => continue,
                    Ok(None) => break,
                    Err(_) => {
                        let state_machine = app_state.state_machine.lock().await;
                        app_state
                            .raft_state
                            .lock()
                            .await
                            .handle_missed_heartbeat(response_tx.clone(), &state_machine)
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
                heartbeat_tx.send(()).await;
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
