use std::fmt::format;

use crate::Arc;
use crate::SharedState;
use anyhow::Context;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

// Envelope is a struct that is used for storing messages sent back from the Twitch API.
#[derive(Deserialize)]
struct Envelope {
    metadata: Metadata,
}
#[derive(Deserialize)]
struct Metadata {
    message_type: String,
}
// WelcomeMessage is a struct that is used to store data from Twitch's initial welcome message.
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

// Keep alive message structs
#[derive(Deserialize)]
struct KeepAliveMessage {
    metadata: Metadata,
}

// Notification message structs
#[derive(Deserialize)]
struct NotificationMessage {
    metadata: Metadata,
    payload: NotificationPayload,
}
#[derive(Deserialize)]
struct NotificationPayload {
    event: NotificationEvent,
}
#[derive(Deserialize)]
struct NotificationEvent {
    broadcaster_user_name: String,
}

// Request Body stores a request body used to send a POST request to Twitch to subscribe. Specifically, for the custom reward points subscription.
#[derive(Serialize, Deserialize)]
pub struct RequestBody {
    r#type: String,
    version: String,
    condition: Condition,
    transport: Transport,
}
#[derive(Serialize, Deserialize)]
struct Condition {
    broadcaster_user_id: String,
}
#[derive(Serialize, Deserialize)]
struct Transport {
    method: String,
    session_id: String,
}

// Response Body is used for storing data that Twitch sends back in response to our subscription request.
#[derive(Deserialize)]
pub struct ResponseBody {
    data: Vec<ResponseArray>,
}
#[derive(Deserialize)]
pub struct ResponseArray {
    status: String,
    r#type: String,
}

pub async fn connect(
    access_token: &str,
    client_id: &str,
    user_id: &str,
    shared_state_tx: Arc<SharedState>,
) -> anyhow::Result<()> {
    const TWITCH_WEBSOCKET_URL: &str = "ws://localhost:8080/ws";
    let (ws_stream, response) = connect_async(TWITCH_WEBSOCKET_URL)
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
                    let welcome_text: WelcomeMessage = serde_json::from_str(&text)?;
                    println!("welcome message id: {}", welcome_text.payload.session.id);
                    subscribe_to_channel_points(
                        access_token,
                        &welcome_text.payload.session.id,
                        client_id,
                        user_id,
                    )
                    .await?;
                }
                "session_keepalive" => {
                    let keepalive_text: KeepAliveMessage = serde_json::from_str(&text)?;
                    println!(
                        "Keep alive message: {}",
                        keepalive_text.metadata.message_type
                    )
                }
                "notification" => {
                    let notification_text: NotificationMessage = serde_json::from_str(&text)?;
                    println!(
                        "Notification: Broadcaster Name = {}",
                        notification_text.payload.event.broadcaster_user_name
                    );
                    shared_state_tx
                        .tx
                        .send("channel points twin".to_string())
                        .expect("Failed to send.");
                }
                other => println!("Unknown message type: {}", other),
            }
        }
    }

    todo!()
}
/// Method that subscribes to a broadcasters custom rewards notification endpoint.
pub async fn subscribe_to_channel_points(
    access_token: &str,
    session_id: &str,
    client_id: &str,
    broadcaster_user_id: &str,
) -> Result<ResponseBody, anyhow::Error> {
    const TWITCH_SUBSCRIPTIONS_URL: &str = "http://127.0.0.1:8080/eventsub/subscriptions";
    const CHANNEL_POINTS_TYPE: &str = "channel.channel_points_custom_reward_redemption.add";
    let client = reqwest::Client::new();

    let body = RequestBody {
        r#type: String::from(CHANNEL_POINTS_TYPE),
        version: String::from("1"),
        condition: Condition {
            broadcaster_user_id: broadcaster_user_id.to_string(),
        },
        transport: Transport {
            method: String::from("websocket"),
            session_id: session_id.to_string(),
        },
    };

    let post_request = client
        .post(TWITCH_SUBSCRIPTIONS_URL)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Client-Id", client_id)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = post_request.status();
    let body_text = post_request.text().await?;
    if !status.is_success() {
        println!("Subscription Request status code {}", status);
        eprintln!("Error body: {:?}", body_text);
    }

    // Receive data
    let post_response: ResponseBody = match serde_json::from_str(&body_text) {
        Ok(body) => body,
        Err(e) => {
            println!("Error parsing the JSON response when subscribing to Twitch. {e}");
            return Err(anyhow::anyhow!(
                "Error parsing the JSON response when subscribing to Twitch. {e}"
            ));
        }
    };
    println!(
        "Subscribed to {} with a status of {}",
        post_response.data[0].r#type, post_response.data[0].status
    );
    Ok(post_response)
}
