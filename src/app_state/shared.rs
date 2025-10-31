use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Peer {
    pub ip: String,
}

#[derive(Serialize, Clone)]
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

impl Default for StatusInfo {
    fn default() -> Self {
        StatusInfo {
            name: String::from(""),
            ip: String::from(""),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum ServerState {
    Leader,
    Follower,
    Candidate,
}
