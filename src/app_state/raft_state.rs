use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;

use super::super::websocket::shared::WSMessage;
use super::log::Log;
use super::shared::{Peer, ServerState, StatusInfo};
use super::state_machine::StateMachine;

#[derive(Clone)]
pub struct RaftState {
    pub log: Log,
    voted_for: Option<Peer>,
    commit_index: u32,
    last_applied: u32,
    next_index: HashMap<Peer, u32>,
    match_index: HashMap<Peer, u32>,
    current_term: u32,
    current_state: ServerState,
}

impl RaftState {
    pub fn new() -> Self {
        RaftState {
            log: Log::new(),
            voted_for: None,
            commit_index: 0,
            last_applied: 0,
            next_index: HashMap::new(),
            match_index: HashMap::new(),
            current_term: 0,
            current_state: ServerState::Follower,
        }
    }

    fn inc_term(&mut self) {
        (*self).current_term += 1;
    }

    fn set_voted_for(&mut self, vote: Peer) {
        (*self).voted_for = Some(vote);
    }

    fn clear_voted_for(&mut self) {
        (*self).voted_for = None;
    }

    fn append_entries(&self) -> WSMessage {
        WSMessage::AppendEntries {
            term: self.current_term,
            leader_id: todo!(),
            prev_log_index: todo!(),
            prev_log_term: todo!(),
            entries: todo!(),
        }
    }

    fn request_vote(&self, state_machine: &StateMachine) -> WSMessage {
        WSMessage::RequestVote {
            term: (*self).current_term,
            candidate_id: state_machine.status_info.to_peer(),
            last_log_index: todo!(),
            last_log_term: todo!(),
        }
    }

    async fn initiate_election(
        &mut self,
        response_tx: Sender<WSMessage>,
        state_machine: &StateMachine,
    ) {
        (*self).current_state = ServerState::Candidate;
        (*self).inc_term();
        let request_vote = self.request_vote(state_machine);
        let _ = response_tx.send(request_vote).await;
        let peer_ip = state_machine.status_info.ip.clone();
        (*self).set_voted_for(Peer { ip: peer_ip });
    }

    fn convert_to_leader(&mut self, new_term: u32, state_machine: &StateMachine) {
        (*self).current_state = ServerState::Leader;
        (*self).current_term = new_term;
        let log = (*self).log.clone();
        let peers = state_machine.peers.clone();
        let peer_map: HashMap<Peer, u32> = peers
            .iter()
            .map(|p| (p.clone(), log.latest_applied + 1))
            .collect();
        (*self).next_index = peer_map;
        let match_map: HashMap<Peer, u32> = peers.iter().map(|p| (p.clone(), 0)).collect();
        (*self).match_index = match_map;
    }

    fn convert_to_follower(&mut self, new_term: u32) {
        (*self).current_state = ServerState::Follower;
        (*self).current_term = new_term;
    }

    pub async fn handle_missed_heartbeat(
        &mut self,
        response_tx: Sender<WSMessage>,
        state_machine: &StateMachine,
    ) {
        match (*self).current_state {
            ServerState::Follower => self.initiate_election(response_tx, state_machine).await,
            ServerState::Candidate => todo!(),
            ServerState::Leader => {}
        }
    }

    pub async fn send_messages(
        &self,
        response_tx: Sender<WSMessage>,
        state_machine: &StateMachine,
    ) {
        match (*self).current_state {
            ServerState::Follower => todo!(),
            ServerState::Candidate => todo!(),
            ServerState::Leader => todo!(),
        }
    }
}
