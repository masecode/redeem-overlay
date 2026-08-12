use crate::Arc;
use crate::SharedState;
use crate::auth;
use crate::configparser;
use crate::twitch;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::{
    Router, http::StatusCode, response::Html, response::Json, routing::any, routing::get,
    routing::post,
};
use futures_util::{SinkExt, StreamExt};
use std::fs;

#[derive(Debug)]
enum ApiError {
    Token(anyhow::Error),
    Http(reqwest::Error),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Token(e) => write!(f, "Token error has occured: {}", e),
            ApiError::Http(e) => write!(f, "HTTP error has occured: {}", e),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError::Token(e)
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError::Http(e)
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            ApiError::Token(_) => StatusCode::UNAUTHORIZED,
            ApiError::Http(_) => StatusCode::BAD_GATEWAY,
        };
        (status, self.to_string()).into_response()
    }
}

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
        .route("/config", get(serve_configuration_html))
        .route("/api/rewards", get(give_rewards_list))
        .route("/api/rewards", post(receive_saved_rewards))
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
async fn serve_configuration_html() -> (StatusCode, Html<String>) {
    match fs::read_to_string("html/config.html") {
        Ok(html) => (StatusCode::OK, Html::from(html)),
        Err(e) => {
            println!("Error reading file, {e}");
            (
                StatusCode::NOT_FOUND,
                Html::from(String::from(
                    "<p>Error reading configuration.html file in httpserver</p>",
                )),
            )
        }
    }
}
async fn give_rewards_list() -> Result<Json<serde_json::Value>, ApiError> {
    let client = reqwest::Client::new();
    let config_file = configparser::parse_configuration_file()?;

    let client_id = config_file.client_id;
    let bearer = auth::check_tokens().await?;
    let broadcaster_id = twitch::get_broadcaster_id(&client_id, bearer.clone()).await?;
    let params = [("broadcaster_id", broadcaster_id.as_str())];
    let get_response = client
        .get("https://api.twitch.tv/helix/channel_points/custom_rewards")
        .query(&params)
        .header("Authorization", format!("Bearer {}", bearer))
        .header("Client-Id", client_id)
        .send()
        .await?;

    if !get_response.status().is_success() {
        eprintln!("Could not get rewards list.")
    }
    let json: serde_json::Value = get_response.json().await?;
    Ok(Json(json))
}

async fn receive_saved_rewards(
    State(state): State<Arc<SharedState>>,
    Json(ids): Json<Vec<String>>,
) -> StatusCode {
    println!("{:?}", ids);
    let json_array = match serde_json::to_string_pretty(&ids) {
        Ok(array) => array,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    match fs::write("rewards.json", json_array) {
        Ok(e) => e,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut reward_list = match state.reward_list.write() {
        Ok(list) => list,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    *reward_list = ids;
    let mut reward_list_exists = match state.reward_list_exists.write() {
        Ok(bool) => bool,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    *reward_list_exists = true;

    StatusCode::OK
}
