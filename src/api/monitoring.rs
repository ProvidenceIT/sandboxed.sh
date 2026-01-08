//! WebSocket-based real-time system monitoring.
//!
//! Provides CPU, memory, and network usage metrics streamed
//! to connected clients via WebSocket.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sysinfo::{Networks, System};

use super::auth;
use super::routes::AppState;

/// Query parameters for the monitoring stream endpoint
#[derive(Debug, Deserialize)]
pub struct MonitoringParams {
    /// Update interval in milliseconds (default: 1000, min: 500, max: 5000)
    pub interval_ms: Option<u64>,
}

/// System metrics snapshot
#[derive(Debug, Clone, Serialize)]
pub struct SystemMetrics {
    /// CPU usage percentage (0-100)
    pub cpu_percent: f32,
    /// Per-core CPU usage percentages
    pub cpu_cores: Vec<f32>,
    /// Memory used in bytes
    pub memory_used: u64,
    /// Total memory in bytes
    pub memory_total: u64,
    /// Memory usage percentage (0-100)
    pub memory_percent: f32,
    /// Network bytes received per second
    pub network_rx_bytes_per_sec: u64,
    /// Network bytes transmitted per second
    pub network_tx_bytes_per_sec: u64,
    /// Timestamp in milliseconds since epoch
    pub timestamp_ms: u64,
}

/// Extract JWT from WebSocket subprotocol header
fn extract_jwt_from_protocols(headers: &HeaderMap) -> Option<String> {
    let raw = headers
        .get("sec-websocket-protocol")
        .and_then(|v| v.to_str().ok())?;
    // Client sends: ["openagent", "jwt.<token>"]
    for part in raw.split(',').map(|s| s.trim()) {
        if let Some(rest) = part.strip_prefix("jwt.") {
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// WebSocket endpoint for streaming system metrics
pub async fn monitoring_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(params): Query<MonitoringParams>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Enforce auth in non-dev mode
    if state.config.auth.auth_required(state.config.dev_mode) {
        let token = match extract_jwt_from_protocols(&headers) {
            Some(t) => t,
            None => return (StatusCode::UNAUTHORIZED, "Missing websocket JWT").into_response(),
        };
        if !auth::verify_token_for_config(&token, &state.config) {
            return (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response();
        }
    }

    ws.protocols(["openagent"])
        .on_upgrade(move |socket| handle_monitoring_stream(socket, params))
}

/// Client command for controlling the monitoring stream
#[derive(Debug, Deserialize)]
#[serde(tag = "t")]
enum ClientCommand {
    /// Pause streaming
    #[serde(rename = "pause")]
    Pause,
    /// Resume streaming
    #[serde(rename = "resume")]
    Resume,
    /// Change update interval
    #[serde(rename = "interval")]
    SetInterval { interval_ms: u64 },
}

/// Handle the WebSocket connection for system monitoring
async fn handle_monitoring_stream(socket: WebSocket, params: MonitoringParams) {
    let interval_ms = params.interval_ms.unwrap_or(1000).clamp(500, 5000);

    tracing::info!(
        interval_ms = interval_ms,
        "Starting system monitoring stream"
    );

    // Initialize sysinfo
    let mut sys = System::new_all();
    let mut networks = Networks::new_with_refreshed_list();

    // Track previous network stats for calculating rates
    let mut prev_rx_bytes: u64 = 0;
    let mut prev_tx_bytes: u64 = 0;
    let mut prev_time = std::time::Instant::now();

    // Initial refresh to get baseline readings
    sys.refresh_all();
    networks.refresh();

    // Get initial network totals
    for (_name, data) in networks.iter() {
        prev_rx_bytes += data.total_received();
        prev_tx_bytes += data.total_transmitted();
    }

    // Channel for control commands from client
    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<ClientCommand>();

    // Split the socket
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Spawn task to handle incoming messages
    let cmd_tx_clone = cmd_tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Text(t) => {
                    if let Ok(cmd) = serde_json::from_str::<ClientCommand>(&t) {
                        let _ = cmd_tx_clone.send(cmd);
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Streaming state
    let mut paused = false;
    let mut current_interval = Duration::from_millis(interval_ms);

    // Main streaming loop
    let mut stream_task = tokio::spawn(async move {
        loop {
            // Check for control commands (non-blocking)
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    ClientCommand::Pause => {
                        paused = true;
                        tracing::debug!("Monitoring stream paused");
                    }
                    ClientCommand::Resume => {
                        paused = false;
                        tracing::debug!("Monitoring stream resumed");
                    }
                    ClientCommand::SetInterval { interval_ms: new_interval } => {
                        let clamped = new_interval.clamp(500, 5000);
                        current_interval = Duration::from_millis(clamped);
                        tracing::debug!(interval_ms = clamped, "Interval changed");
                    }
                }
            }

            if paused {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            // Refresh system info
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            networks.refresh();

            // Calculate CPU usage
            let cpu_percent = sys.global_cpu_usage();
            let cpu_cores: Vec<f32> = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();

            // Calculate memory usage
            let memory_used = sys.used_memory();
            let memory_total = sys.total_memory();
            let memory_percent = if memory_total > 0 {
                (memory_used as f64 / memory_total as f64 * 100.0) as f32
            } else {
                0.0
            };

            // Calculate network rates
            let now = std::time::Instant::now();
            let elapsed_secs = now.duration_since(prev_time).as_secs_f64();

            let mut current_rx_bytes: u64 = 0;
            let mut current_tx_bytes: u64 = 0;
            for (_name, data) in networks.iter() {
                current_rx_bytes += data.total_received();
                current_tx_bytes += data.total_transmitted();
            }

            let rx_diff = current_rx_bytes.saturating_sub(prev_rx_bytes);
            let tx_diff = current_tx_bytes.saturating_sub(prev_tx_bytes);

            let network_rx_bytes_per_sec = if elapsed_secs > 0.0 {
                (rx_diff as f64 / elapsed_secs) as u64
            } else {
                0
            };
            let network_tx_bytes_per_sec = if elapsed_secs > 0.0 {
                (tx_diff as f64 / elapsed_secs) as u64
            } else {
                0
            };

            // Update previous values
            prev_rx_bytes = current_rx_bytes;
            prev_tx_bytes = current_tx_bytes;
            prev_time = now;

            // Build metrics
            let metrics = SystemMetrics {
                cpu_percent,
                cpu_cores,
                memory_used,
                memory_total,
                memory_percent,
                network_rx_bytes_per_sec,
                network_tx_bytes_per_sec,
                timestamp_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            };

            // Send as JSON text message
            let json = match serde_json::to_string(&metrics) {
                Ok(j) => j,
                Err(e) => {
                    tracing::error!("Failed to serialize metrics: {}", e);
                    continue;
                }
            };

            if ws_sender.send(Message::Text(json)).await.is_err() {
                tracing::debug!("Client disconnected from monitoring stream");
                break;
            }

            // Wait for next update
            tokio::time::sleep(current_interval).await;
        }

        tracing::info!("System monitoring stream ended");
    });

    // Wait for either task to complete
    tokio::select! {
        _ = &mut recv_task => {
            stream_task.abort();
        }
        _ = &mut stream_task => {
            recv_task.abort();
        }
    }
}
