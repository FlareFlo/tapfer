use crate::error::TapferResult;
use crate::tapfer_id::TapferId;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, WebSocketUpgrade};
use axum::response::Response;
use dashmap::DashMap;
use std::sync::LazyLock;
use tokio::sync::broadcast::WeakSender;
use tokio::sync::broadcast::channel;
use tracing::warn;
use uuid::Uuid;

static WS_MAP: LazyLock<DashMap<TapferId, WeakSender<WsEvent>>> = LazyLock::new(DashMap::new);

// Public API for other handlers to use
pub async fn broadcast_event(id: TapferId, event: WsEvent) -> TapferResult<()> {
    // Check if someone's listening
    let Some(rx) = WS_MAP.get(&id) else {
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

// Impl

#[axum::debug_handler]
pub async fn start_ws(Path(id): Path<Uuid>, ws: WebSocketUpgrade) -> Response {
    let id = TapferId::from_id(id);
    ws.on_upgrade(move |socket| handle_socket(socket, id))
}

async fn handle_socket(mut socket: WebSocket, id: TapferId) {
    let mut tx_seq = 0;
    warn!("Handling socket");
    let (tx, mut rx) = if let Some(tx) = WS_MAP.get(&id).map(|rx| rx.upgrade()).flatten() {
        (tx.clone(), tx.subscribe())
    } else {
        let (tx, rx) = channel(100);
        WS_MAP.insert(id, tx.downgrade());
        (tx, rx)
    };
    while let Ok(msg) = rx.recv().await {
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
    UploadProgress {
        progress: u64,
        total: u64,
    },
    UploadComplete,
}

