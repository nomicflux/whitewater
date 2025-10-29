use serde::{Deserialize, Serialize};

pub type PeerId = String;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum WSMessage {
    Connect,
    Heartbeat(PeerId),
    Close,
}
