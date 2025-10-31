pub mod user;

use std::collections::HashMap;

use super::shared::{Peer, StatusInfo};
use user::{CreateUserRequest, User};

#[derive(Clone)]
pub struct StateMachine {
    pub status_info: StatusInfo,
    pub peers: Vec<Peer>,
    pub users: HashMap<u32, User>,
    pub next_id: u32,
}

impl StateMachine {
    pub fn new(status_info: StatusInfo) -> Self {
        StateMachine {
            status_info: status_info.clone(),
            peers: Vec::new(),
            users: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn add_peer(&mut self, peer: Peer) {
        self.peers.push(peer);
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn create_user(&mut self, req: CreateUserRequest) -> User {
        let id = self.next_id();
        let user = User {
            id,
            name: req.name.clone(),
            email: req.email.clone(),
        };
        self.users.insert(id, user.clone());
        user
    }

    pub fn get_user(&self, id: u32) -> Option<User> {
        self.users.get(&id).cloned()
    }

    pub fn list_users(&self) -> Vec<User> {
        self.users.values().cloned().collect()
    }
}
