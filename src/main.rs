use axum::{
    extract::{Path, State, Json},
    response::{IntoResponse},
    http::StatusCode,
    Router,
    routing::{get, post},
};
use serde::{Deserialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::net::SocketAddr;

mod user;
use user::{User};

mod log;
use log::{ToCommand, Command, Log, add_to_log};

mod server_state;
use server_state::{ServerState};

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<HashMap<u32, User>>>,
    next_id: Arc<Mutex<u32>>,
    log: Arc<Mutex<Log>>,
    current_term: Arc<Mutex<u32>>,
    current_state: Arc<Mutex<ServerState>>,
}

#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

impl ToCommand for CreateUserRequest {
    fn to_command(&self) -> Command {
        Command::AddUser {
            name: self.name.clone(),
            email: self.email.clone(),
        }
    }
}

async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let mut next_id = state.next_id.lock().unwrap();
    let id = *next_id;
    *next_id += 1;
    let user = User {
        id,
        name: req.name.clone(),
        email: req.email.clone(),
    };
    state.users.lock().unwrap().insert(id, user.clone());
    add_to_log(state.log, &req);
    (StatusCode::CREATED, Json(user))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<User>, StatusCode> {
    state.users
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn list_users(
    State(state): State<AppState>,
) -> Json<Vec<User>> {
    let users: Vec<User> = state.users
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect();

    Json(users)
}

#[tokio::main]
async fn main() {
    let state = AppState {
        users: Arc::new(Mutex::new(HashMap::new())),
        next_id: Arc::new(Mutex::new(1)),
        log: Arc::new(Mutex::new(Log::new())),
        current_term: Arc::new(Mutex::new(0)),
        current_state: Arc::new(Mutex::new(ServerState::Follower)),
    };

    let app = Router::new()
        .without_v07_checks()
        .route("/users", post(create_user))
        .route("/users", get(list_users))
        .route("/users/:id", get(get_user))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8090));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap()
}
