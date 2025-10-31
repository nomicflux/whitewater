use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Peer {
    pub ip: String,
}

#[derive(Serialize, Clone, Default)]
pub struct StatusInfo {
    pub name: String,
    pub ip: String,
}

impl StatusInfo {
    pub fn to_peer(&self) -> Peer {
        Peer {
            ip: self.ip.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerState {
    Leader {
        next_index: HashMap<Peer, u32>,
        match_index: HashMap<Peer, u32>,
    },
    Follower,
    Candidate {
        voted_for: HashSet<Peer>,
    },
}

impl ServerState {
    pub fn follower() -> ServerState {
        ServerState::Follower
    }

    pub fn candidate(status_info: StatusInfo) -> ServerState {
        let mut voted_for = HashSet::new();
        voted_for.insert(status_info.to_peer());
        ServerState::Candidate { voted_for }
    }

    pub fn leader(peers: Vec<Peer>, latest_applied: u32) -> ServerState {
        let next_index: HashMap<Peer, u32> = peers
            .iter()
            .map(|p| (p.clone(), latest_applied + 1))
            .collect();
        let match_index: HashMap<Peer, u32> = peers.iter().map(|p| (p.clone(), 0)).collect();
        ServerState::Leader {
            next_index,
            match_index,
        }
    }
}
