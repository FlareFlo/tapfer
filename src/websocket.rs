use crate::error::TapferResult;
use crate::tapfer_id::TapferId;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, WebSocketUpgrade};
use axum::response::Response;
use dashmap::DashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::broadcast::WeakSender;
use tokio::sync::broadcast::channel;
use tracing::warn;
use uuid::Uuid;

static WS_MAP: LazyLock<DashMap<WsDestination, WeakSender<WsEvent>>> = LazyLock::new(DashMap::new);

#[derive(Eq, Hash, PartialEq)]
pub enum WsDestination {
    Id(TapferId),
    Deposit(u64),
}

impl From<TapferId> for WsDestination {
    fn from(value: TapferId) -> Self {
        WsDestination::Id(value)
    }
}

impl From<u64> for WsDestination {
    fn from(value: u64) -> Self {
        WsDestination::Deposit(value)
    }
}

// Public API for other handlers to use
pub async fn broadcast_event(dst: impl Into<WsDestination> + Copy, event: WsEvent) -> TapferResult<()> {
    // Check if someone's listening
    let Some(rx) = WS_MAP.get(&dst.into()) else {
        if matches!(dst.into(), WsDestination::Deposit(_)) {
            warn!("Event for deposit has no listeners");
        }
        return Ok(());
    };

    // If the weak sender did also not have anyone listening
    let Some(rx) = rx.value().upgrade() else {
        // TODO: Remove sender from map?
        warn!("WS sender was too weak");
        return Ok(());
    };
    rx.send(event).unwrap();
    Ok(())
}

pub fn wss_method(host: &str) -> &str {
    match host {
        "localhost:3000" => "ws",
        _ => "wss",
    }
}

// Impl

#[axum::debug_handler]
pub async fn start_ws(Path(id): Path<Uuid>, ws: WebSocketUpgrade) -> Response {
    let id = TapferId::from_id(id);
    ws.on_upgrade(move |socket| handle_socket(socket, id))
}

pub(crate) async fn handle_socket(mut socket: WebSocket, dst: impl Into<WsDestination> + Copy) {
    let mut tx_seq = 0;
    let (tx, mut rx) = if let Some(tx) = WS_MAP.get(&dst.into()).map(|rx| rx.upgrade()).flatten() {
        (tx.clone(), tx.subscribe())
    } else {
        let (tx, rx) = channel(100);
        WS_MAP.insert(dst.into(), tx.downgrade());
        (tx, rx)
    };
    let cooldown = Duration::from_millis(1000 / 30); // 30Hz
    let mut last_progress = Instant::now() - cooldown;
    while let Ok(msg) = rx.recv().await {
        // Rate-limit progress
        if matches!(msg, WsEvent::UploadProgress { .. }) && last_progress.elapsed() < cooldown {
            continue;
        }
        last_progress = Instant::now();

        let msg = WsPacket {
            seq: tx_seq,
            event: msg,
        };
        tx_seq += 1;

        socket
            .send(Message::text(serde_json::to_string(&msg).unwrap()))
            .await
            .unwrap();
    }
    // Channel closed
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WsPacket {
    seq: u64,
    event: WsEvent,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "key")]
pub enum WsEvent {
    DeleteAsset,
    UploadProgress { progress: u64, total: u64 },
    UploadComplete,
    DepositReady {
        id: TapferId,
    },
}
