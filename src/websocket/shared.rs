use super::super::app_state::log::LogEntry;
use super::super::app_state::shared::Peer;
use axum::extract::ws::Message as AxumMessage;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum WSMessage {
    AppendEntries {
        term: u32,
        leader_id: Peer,
        prev_log_index: u32,
        prev_log_term: u32,
        entries: Vec<LogEntry>,
    },
    AppendEntriesResponse {
        term: u32,
        success: bool,
    },
    RequestVote {
        term: u32,
        candidate_id: Peer,
        last_log_index: u32,
        last_log_term: u32,
    },
    RequestVoteResponse {
        term: u32,
        vote_granted: bool,
    },
}

impl From<WSMessage> for AxumMessage {
    fn from(msg: WSMessage) -> AxumMessage {
        AxumMessage::Text(serde_json::to_string(&msg).unwrap().into())
    }
}

impl From<WSMessage> for TungsteniteMessage {
    fn from(msg: WSMessage) -> TungsteniteMessage {
        TungsteniteMessage::Text(serde_json::to_string(&msg).unwrap().into())
    }
}
