mod app_state;
mod handler;
mod websocket;

use axum::{
    Router,
    extract::{Json, Path, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
};
use hickory_resolver::TokioResolver;
use regex::Regex;
use serde::Serialize;
use std::env;
use std::net::SocketAddr;
use tokio::time::Duration;

use app_state::state_machine::user::CreateUserRequest;
use app_state::{
    AppState,
    shared::{Peer, StatusInfo},
};
use handler::Handler;
use websocket::connection::Connection;

async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> impl IntoResponse {
    state.create_user(req).await
}

async fn get_user(State(state): State<AppState>, Path(id): Path<u32>) -> impl IntoResponse {
    state.get_user(id).await
}

async fn list_users(State(state): State<AppState>) -> impl IntoResponse {
    state.list_users().await
}

fn peer_to_ws_addr(peer: Peer) -> String {
    let re = Regex::new(".:").unwrap();
    let ip = re.replace_all(&peer.ip, ":");
    format!("ws://{}/ws", ip)
}

async fn discover_peers(app_state: AppState, handler: Handler) -> anyhow::Result<()> {
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
        .map(|srv| Peer {
            ip: format!("{}:{}", srv.target().to_utf8(), srv.port()),
        })
        .collect();
    println!("Found peers: {:?}", peers);

    let status_info = app_state
        .state_machine
        .lock()
        .await
        .status_info
        .to_peer()
        .clone();
    for peer in peers {
        if peer.ip != status_info.ip {
            app_state.add_peer(peer.clone()).await;
            Connection::connect(peer.ip, handler.clone()).await;
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct PeerResult {
    peers: Option<Vec<Peer>>,
    error: Option<String>,
}

fn retrieve_status_info() -> anyhow::Result<StatusInfo> {
    let name = env::var("POD_NAME")?;
    let ip = env::var("POD_IP")?;
    println!("Name: {}, IP: {}", name, ip);
    Ok(StatusInfo { name, ip })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let status_info = retrieve_status_info().unwrap_or_default();
    let state = AppState::new(status_info.clone());

    println!("App state initialized");

    let handler = Handler::spawn(&state);

    let state_c = state.clone();
    let handler_c = handler.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let _ = discover_peers(state_c, handler_c).await;
    });

    let app = Router::new()
        .without_v07_checks()
        .route("/users", post(create_user))
        .route("/users", get(list_users))
        .route("/users/:id", get(get_user))
        .route(
            "/ws",
            get(|ws: WebSocketUpgrade| Connection::accept(ws, handler)),
        )
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8090));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}
