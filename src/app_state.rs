pub mod log;
mod raft_state;
pub mod shared;
pub mod state_machine;

use std::sync::Arc;
use tokio::sync::Mutex;

use axum::{extract::Json, http::StatusCode};
use raft_state::RaftState;
use shared::{Peer, StatusInfo};
use state_machine::{
    StateMachine,
    user::{CreateUserRequest, User},
};

#[derive(Clone)]
pub struct AppState {
    pub raft_state: Arc<Mutex<RaftState>>,
    pub state_machine: Arc<Mutex<StateMachine>>,
}

impl AppState {
    pub fn new(status_info: StatusInfo) -> Self {
        AppState {
            raft_state: Arc::new(Mutex::new(RaftState::new())),
            state_machine: Arc::new(Mutex::new(StateMachine::new(status_info.clone()))),
        }
    }

    async fn modify_state_machine<A>(&self, func: impl FnOnce(&mut StateMachine) -> A) -> A {
        let mut state_machine = self.state_machine.lock().await;
        func(&mut state_machine)
    }

    pub async fn add_peer(&self, peer: Peer) {
        self.modify_state_machine(|state| state.add_peer(peer))
            .await;
    }

    pub async fn create_user(&self, req: CreateUserRequest) -> (StatusCode, Json<Option<User>>) {
        let user = self
            .modify_state_machine(|state| state.create_user(req))
            .await;
        (StatusCode::CREATED, Json(Some(user.clone())))
    }

    pub async fn get_user(&self, id: u32) -> (StatusCode, Json<Option<User>>) {
        match self.state_machine.lock().await.get_user(id) {
            Some(user) => (StatusCode::OK, Json(Some(user.clone()))),
            None => (StatusCode::NOT_FOUND, Json(None::<User>)),
        }
    }

    pub async fn list_users(&self) -> (StatusCode, Json<Option<Vec<User>>>) {
        let users = self.state_machine.lock().await.list_users();
        (StatusCode::OK, Json(Some(users)))
    }
}
