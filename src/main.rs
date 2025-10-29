mod app_state;
mod handler;
mod user;
mod log;
mod websocket;
mod server_state;

use axum::{
    Router,
    extract::{Json, Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use hickory_resolver::{TokioResolver};
use regex::Regex;
use serde::{Deserialize,Serialize};
use std::env;
use std::net::SocketAddr;

use user::User;
use handler::Handler;
use app_state::{Peer, StatusInfo, AppState};
use log::{Command, Log, ToCommand, add_to_log};
use websocket::client::spawn_client;
use websocket::server::ws_handler;

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
    let mut next_id = state.next_id.lock().await;
    let id = *next_id;
    *next_id += 1;
    let user = User {
        id,
        name: req.name.clone(),
        email: req.email.clone(),
    };
    let term = state.current_term.lock().await;
    let mut users = state.users.lock().await;
    users.insert(id, user.clone());
    add_to_log(state.log, *term, &req);
    (StatusCode::CREATED, Json(user))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<User>, StatusCode> {
    state
        .users
        .lock()
        .await
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn list_users(State(state): State<AppState>) -> Json<Vec<User>> {
    let users: Vec<User> = state.users.lock().await.values().cloned().collect();

    Json(users)
}

async fn get_peers(State(state): State<AppState>) -> Json<Vec<Peer>> {
    let peers: Vec<Peer> = state.peers.lock().await.clone();

    Json(peers)
}

async fn get_status_info(State(state): State<AppState>) -> Json<StatusInfo> {
    let status_info = state.status_info.clone();

    Json(status_info)
}

fn peer_to_ws_addr(peer: Peer) -> String {
    let re = Regex::new(".:").unwrap();
    let ip = re.replace_all(&peer.ip, ":");
    format!("ws://{}/ws", ip)
}

async fn discover_peers() -> anyhow::Result<Vec<Peer>> {
    let service = env::var("SERVICE_NAME")?;
    let namespace = env::var("NAMESPACE")?;
    let port_name = env::var("SERVICE_PORT_NAME")?;
    println!("Service {}, NS {}, Port {}", service, namespace, port_name);

    let resolver = TokioResolver::builder_tokio()?.build();

    let srv_query = format!(
        "_{}._{}.{}.{}.svc.cluster.local",
        port_name, "tcp", service, namespace
    );
    println!("Using query: {}", srv_query);

    let records = resolver.srv_lookup(&srv_query).await?;
    println!("Found records: {:?}", records);

    let peers: Vec<Peer> = records
        .iter()
        .map(|srv| Peer { ip: format!("{}:{}", srv.target().to_utf8(), srv.port()) })
        .collect();
    println!("Found peers: {:?}", peers);

    let mut confirmed_peers = Vec::<Peer>::new();
    for peer in peers {
        let peer_c = peer.clone();
        match spawn_client(peer_to_ws_addr(peer)).await {
            Ok(_) => {
                println!("Connected to peer {:?}", peer_c.clone());
                confirmed_peers.push(peer_c);
            }
            Err(e) => {
                eprintln!("Error connecting to peer {peer_c:?}: {e}");
            }
        }
    }

    Ok(confirmed_peers)
}

async fn update_peers(state: AppState) -> anyhow::Result<()> {
    let peers = discover_peers().await?;
    let mut curr = state.peers.lock().await;
    curr.clear();
    curr.extend(peers);
    Ok(())
}

#[derive(Serialize)]
struct PeerResult {
    peers: Option<Vec<Peer>>,
    error: Option<String>,
}

async fn post_update_peers(State(state): State<AppState>) -> Json<PeerResult> {
    let sc = state.clone();
    let res = update_peers(sc).await;

    match res {
        Ok(_) => {
            let peers = state.peers.lock().await.clone();
            Json(PeerResult { peers: Some(peers), error: None })
        }
        Err(e) => {
            let e_msg = e.to_string();
            Json(PeerResult { peers: None, error: Some(e_msg) })
        }
    }
}

fn retrieve_status_info() -> anyhow::Result<StatusInfo> {
    let name = env::var("POD_NAME")?;
    let ip = env::var("POD_IP")?;
    println!("Name: {}, IP: {}", name, ip);
    Ok(StatusInfo {
        name, ip
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = AppState::new(retrieve_status_info());

    println!("App state initialized");

    let handler = Handler::spawn(&state);

    let app = Router::new()
        .without_v07_checks()
        .route("/users", post(create_user))
        .route("/users", get(list_users))
        .route("/users/:id", get(get_user))
        .route("/peers", get(get_peers))
        .route("/peers/update", post(post_update_peers))
        .route("/status", get(get_status_info))
        .route("/ws", get(|ws: WebSocketUpgrade| ws_handler(ws, handler)))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8090));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}
