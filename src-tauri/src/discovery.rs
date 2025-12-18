// Discovery server for P2P song library
// Can run as server (on VPS) or connect as client

use axum::{
    extract::State,
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tower_http::cors::CorsLayer;

const PEER_TIMEOUT_SECS: u64 = 45; // Remove peers after 45 seconds of no heartbeat

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSong {
    pub name: String,
    pub hash: String,
    pub duration: f64,
    pub bpm: u16,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    #[serde(default)]
    pub webrtc_id: Option<String>, // PeerJS ID for P2P connections
    pub name: String,
    pub songs: Vec<SharedSong>,
    #[serde(skip)]
    pub last_seen: Option<Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub peer_id: String,
    #[serde(default)]
    pub webrtc_id: Option<String>,
    pub name: String,
    pub songs: Vec<SharedSong>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerListResponse {
    pub peers: Vec<PeerInfo>,
    pub total_songs: usize,
}

// Server state
#[derive(Default)]
pub struct DiscoveryState {
    peers: HashMap<String, PeerInfo>,
}

type SharedState = Arc<RwLock<DiscoveryState>>;

// API handlers
async fn register_peer(
    State(state): State<SharedState>,
    Json(req): Json<RegisterRequest>,
) -> StatusCode {
    let mut state = state.write().unwrap();

    let peer = PeerInfo {
        peer_id: req.peer_id.clone(),
        webrtc_id: req.webrtc_id,
        name: req.name,
        songs: req.songs,
        last_seen: Some(Instant::now()),
    };

    state.peers.insert(req.peer_id, peer);
    println!(
        "[DISCOVERY] Peer registered: {} peers, {} songs total",
        state.peers.len(),
        state.peers.values().map(|p| p.songs.len()).sum::<usize>()
    );

    StatusCode::OK
}

async fn unregister_peer(
    State(state): State<SharedState>,
    Json(peer_id): Json<String>,
) -> StatusCode {
    let mut state = state.write().unwrap();
    state.peers.remove(&peer_id);
    println!(
        "[DISCOVERY] Peer unregistered, {} remaining",
        state.peers.len()
    );
    StatusCode::OK
}

async fn get_peers(State(state): State<SharedState>) -> Json<PeerListResponse> {
    let state = state.read().unwrap();

    let peers: Vec<PeerInfo> = state.peers.values().cloned().collect();
    let total_songs: usize = peers.iter().map(|p| p.songs.len()).sum();

    Json(PeerListResponse { peers, total_songs })
}

async fn heartbeat(
    State(state): State<SharedState>,
    Json(req): Json<RegisterRequest>,
) -> StatusCode {
    let mut state = state.write().unwrap();

    if let Some(peer) = state.peers.get_mut(&req.peer_id) {
        peer.last_seen = Some(Instant::now());
        peer.songs = req.songs;
        peer.name = req.name;
        peer.webrtc_id = req.webrtc_id; // Update WebRTC ID in case it changed
    } else {
        // Peer not found, register them
        let peer = PeerInfo {
            peer_id: req.peer_id.clone(),
            webrtc_id: req.webrtc_id,
            name: req.name,
            songs: req.songs,
            last_seen: Some(Instant::now()),
        };
        state.peers.insert(req.peer_id, peer);
    }

    StatusCode::OK
}

async fn health() -> &'static str {
    "OK"
}

// Cleanup task to remove stale peers
async fn cleanup_stale_peers(state: SharedState) {
    loop {
        tokio::time::sleep(Duration::from_secs(15)).await; // Check every 15 seconds

        let mut state = state.write().unwrap();
        let timeout = Duration::from_secs(PEER_TIMEOUT_SECS);
        let now = Instant::now();

        let before = state.peers.len();
        state.peers.retain(|_, peer| {
            peer.last_seen
                .map(|t| now.duration_since(t) < timeout)
                .unwrap_or(false)
        });

        let removed = before - state.peers.len();
        if removed > 0 {
            println!(
                "[DISCOVERY] Cleaned up {} stale peers, {} remaining",
                removed,
                state.peers.len()
            );
        }
    }
}

// Start the discovery server
#[allow(dead_code)]
pub async fn start_server(port: u16) -> Result<(), String> {
    let state: SharedState = Arc::new(RwLock::new(DiscoveryState::default()));

    // Start cleanup task
    let cleanup_state = state.clone();
    tokio::spawn(cleanup_stale_peers(cleanup_state));

    let app = Router::new()
        .route("/health", get(health))
        .route("/register", post(register_peer))
        .route("/unregister", delete(unregister_peer))
        .route("/peers", get(get_peers))
        .route("/heartbeat", post(heartbeat))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("[DISCOVERY] Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind: {}", e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    Ok(())
}

// Global server handle for shutdown
use std::sync::Mutex;
use tokio::sync::broadcast;

static SERVER_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SHUTDOWN_SENDER: Mutex<Option<broadcast::Sender<()>>> = Mutex::new(None);

pub fn is_server_running() -> bool {
    SERVER_RUNNING.load(std::sync::atomic::Ordering::SeqCst)
}

pub async fn start_discovery_server(port: u16) -> Result<(), String> {
    if is_server_running() {
        return Err("Server already running".to_string());
    }

    // Create shutdown channel
    let (tx, _) = broadcast::channel::<()>(1);
    {
        let mut sender = SHUTDOWN_SENDER.lock().unwrap();
        *sender = Some(tx.clone());
    }

    SERVER_RUNNING.store(true, std::sync::atomic::Ordering::SeqCst);

    let result = start_server_with_shutdown(port, tx).await;

    SERVER_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);

    // Clean up shutdown sender
    {
        let mut sender = SHUTDOWN_SENDER.lock().unwrap();
        *sender = None;
    }

    result
}

pub fn stop_discovery_server() -> Result<(), String> {
    if !is_server_running() {
        return Err("Server is not running".to_string());
    }

    let sender = SHUTDOWN_SENDER.lock().unwrap();
    if let Some(tx) = sender.as_ref() {
        let _ = tx.send(());
        println!("[DISCOVERY] Shutdown signal sent");
        Ok(())
    } else {
        Err("No shutdown handle available".to_string())
    }
}

// Start the discovery server with shutdown support
async fn start_server_with_shutdown(
    port: u16,
    shutdown_tx: broadcast::Sender<()>,
) -> Result<(), String> {
    let state: SharedState = Arc::new(RwLock::new(DiscoveryState::default()));

    // Start cleanup task
    let cleanup_state = state.clone();
    let mut cleanup_shutdown_rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        tokio::select! {
            _ = cleanup_stale_peers(cleanup_state) => {}
            _ = cleanup_shutdown_rx.recv() => {
                println!("[DISCOVERY] Cleanup task stopped");
            }
        }
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/register", post(register_peer))
        .route("/unregister", delete(unregister_peer))
        .route("/peers", get(get_peers))
        .route("/heartbeat", post(heartbeat))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("[DISCOVERY] Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind: {}", e))?;

    let mut shutdown_rx = shutdown_tx.subscribe();

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.recv().await;
            println!("[DISCOVERY] Server shutting down gracefully");
        })
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    println!("[DISCOVERY] Server stopped");
    Ok(())
}
