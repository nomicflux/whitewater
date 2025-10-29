use super::log::Log;
use super::server_state::ServerState;
use super::user::User;
use super::websocket::shared::WSMessage;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;

#[derive(Serialize, Clone, Debug)]
pub struct Peer {
    pub ip: String,
}

#[derive(Serialize, Clone)]
pub struct StatusInfo {
    pub name: String,
    pub ip: String,
}

impl Default for StatusInfo {
    fn default() -> Self {
        Self {
            name: String::from(""),
            ip: String::from(""),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub status_info: StatusInfo,
    pub peers: Arc<Mutex<Vec<Peer>>>,
    pub users: Arc<Mutex<HashMap<u32, User>>>,
    pub next_id: Arc<Mutex<u32>>,
    pub log: Arc<Mutex<Log>>,
    pub current_term: Arc<Mutex<u32>>,
    pub current_state: Arc<Mutex<ServerState>>,
}

impl AppState {
    pub fn new(status_info: anyhow::Result<StatusInfo>) -> Self {
        AppState {
            status_info: match status_info {
                Ok(status) => status,
                Err(_) => StatusInfo::default(),
            },
            peers: Arc::new(Mutex::new(Vec::new())),
            users: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            log: Arc::new(Mutex::new(Log::new())),
            current_term: Arc::new(Mutex::new(0)),
            current_state: Arc::new(Mutex::new(ServerState::Follower)),
        }
    }

    async fn is_majority(&self, num: usize) -> bool {
        (2 * num) > self.peers.lock().await.len()
    }

    async fn initiate_election(&self, response_tx: Sender<WSMessage>) {
        let mut state = self.current_state.lock().await;
        *state = ServerState::Candidate;
        let _ = response_tx.send(WSMessage::Heartbeat("".to_string())).await;
    }

    pub async fn handle_missed_heartbeat(
        &self,
        response_tx: Sender<WSMessage>,
    ) -> anyhow::Result<()> {
        let state = self.current_state.lock().await;
        match *state {
            ServerState::Follower => Ok(self.initiate_election(response_tx).await),
            ServerState::Candidate => todo!(),
            ServerState::Leader => Ok(()),
        }
    }
}
