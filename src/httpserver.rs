use crate::Arc;
use crate::SharedState;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::response::Response;
use axum::{Router, http::StatusCode, response::Html, routing::any, routing::get};
use futures_util::{SinkExt, StreamExt};
use std::fs;
use std::{fs::File, io::Read};

async fn hello() -> (StatusCode, Html<String>) {
    match fs::read_to_string("html/countdown.html") {
        Ok(html) => (StatusCode::OK, Html::from(html)),
        Err(e) => {
            println!("Error reading file, {e}");
            (
                StatusCode::NOT_FOUND,
                Html::from(String::from(
                    "<p>Error reading countdown.html file in httpserver</p>",
                )),
            )
        }
    }
}

async fn index() -> &'static str {
    "Hello, World!"
}

pub async fn run_server(port: u16, shared_state: Arc<SharedState>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(hello))
        .route("/ws", any(ws_handler))
        .with_state(shared_state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(shared_state): State<Arc<SharedState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, shared_state))
}

pub async fn handle_websocket(mut socket: WebSocket, state: Arc<SharedState>) {
    let mut rx = state.tx.subscribe();
    let (mut sender, mut receiver) = socket.split();
    while let Ok(msg) = rx.recv().await {
        println!("Message: {}", msg);
        sender.send(Message::Text("start".into())).await.ok();
    }
}
