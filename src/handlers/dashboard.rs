use crate::services::Analytics;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::time::{interval, Duration};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(analytics): State<Arc<Analytics>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, analytics))
}

async fn handle_socket(socket: WebSocket, analytics: Arc<Analytics>) {
    let (mut sender, mut receiver) = socket.split();
    
    let mut interval = interval(Duration::from_secs(1));
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let stats = analytics.get_stats().await;
                
                if let Ok(msg) = serde_json::to_string(&stats) {
                    if sender.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
            }
            
            Some(Ok(msg)) = receiver.next() => {
                match msg {
                    Message::Close(_) => break,
                    Message::Ping(data) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    tracing::debug!("WebSocket connection closed");
}

