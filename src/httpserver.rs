use crate::Arc;
use crate::SharedState;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::{Router, http::StatusCode, response::Html, routing::any, routing::get};
use futures_util::{SinkExt, StreamExt};
use std::fs;

async fn serve_countdown_html() -> (StatusCode, Html<String>) {
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

pub async fn run_server(port: u16, shared_state: Arc<SharedState>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(serve_countdown_html))
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

pub async fn handle_websocket(socket: WebSocket, state: Arc<SharedState>) {
    let mut rx = state.tx.subscribe();
    let (mut sender, mut _receiver) = socket.split();
    while let Ok(msg) = rx.recv().await {
        println!("Message: {}", msg);
        sender.send(Message::Text("start".into())).await.ok();
    }
}
