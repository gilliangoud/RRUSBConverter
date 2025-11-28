use crate::decoder::WsMessage;
use futures::{SinkExt, StreamExt};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use tokio::sync::broadcast;
use warp::Filter;

pub async fn start_server(
    tx: broadcast::Sender<WsMessage>,
    port: u16,
    is_connected: Arc<AtomicBool>,
) {
    let tx = warp::any().map(move || tx.clone());
    let is_connected = warp::any().map(move || is_connected.clone());

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(tx)
        .and(is_connected)
        .map(|ws: warp::ws::Ws, tx, is_connected| {
            ws.on_upgrade(move |socket| handle_connection(socket, tx, is_connected))
        })
        .boxed();

    println!("WebSocket server listening on 0.0.0.0:{}", port);
    warp::serve(ws_route).run(([0, 0, 0, 0], port)).await;
}

async fn handle_connection(
    ws: warp::ws::WebSocket,
    tx: broadcast::Sender<WsMessage>,
    is_connected: Arc<AtomicBool>,
) {
    let (mut ws_tx, _) = ws.split();
    let mut rx = tx.subscribe();

    println!("New WebSocket client connected");

    // Send initial status
    let status = if is_connected.load(Ordering::SeqCst) {
        "connected"
    } else {
        "disconnected"
    };
    
    let initial_msg = WsMessage::Status {
        event: status.to_string(),
    };

    if let Ok(json) = serde_json::to_string(&initial_msg) {
        if let Err(e) = ws_tx.send(warp::ws::Message::text(json)).await {
            eprintln!("WebSocket send error (initial status): {}", e);
            return;
        }
    }

    while let Ok(msg) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&msg) {
            if let Err(e) = ws_tx.send(warp::ws::Message::text(json)).await {
                eprintln!("WebSocket send error: {}", e);
                break;
            }
        }
    }
    println!("WebSocket client disconnected");
}
