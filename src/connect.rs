use futures_util::StreamExt;
use serde::Deserialize;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Deserialize)]
struct Metadata {
    message_type: String,
}
#[derive(Deserialize)]
struct Envelope {
    metadata: Metadata,
}
#[derive(Deserialize)]
struct WelcomeMessage {
    payload: WelcomePayload,
}
#[derive(Deserialize)]
struct WelcomePayload {
    session: Session,
}
#[derive(Deserialize)]
struct Session {
    id: String,
}

pub async fn connect(access_token: &str) -> anyhow::Result<()> {
    let (ws_stream, response) = connect_async("wss://eventsub.wss.twitch.tv/ws")
        .await
        .expect("Failed to connect to websocket.");

    println!("Connected with status: {}", response.status());

    let (mut write, mut read) = ws_stream.split();

    while let Some(message) = read.next().await {
        let message = message.expect("Error reading from websocket.");
        if let Message::Text(text) = message {
            let envelope: Envelope = serde_json::from_str(&text)?;
            match envelope.metadata.message_type.as_str() {
                "session_welcome" => {
                    let mut welcome_text: WelcomeMessage = serde_json::from_str(&text)?;
                    println!("welcome message id: {}", welcome_text.payload.session.id)
                }
                "session_keepalive" => {}
                "notification" => {}
                other => println!("Unknown message type: {}", other),
            }
        }
    }

    todo!()
}
