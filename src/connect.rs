use crate::Arc;
use crate::SharedState;
use crate::auth;
use anyhow::Context;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
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
    keepalive_timeout_seconds: u64,
}

// Keep alive message structs
#[derive(Deserialize)]
struct KeepAliveMessage {
    metadata: Metadata,
}

// Notification message structs
#[derive(Deserialize)]
#[allow(dead_code)] // Needed because metadata is required for the message to go through.
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
    reward: RewardEvent,
}
#[derive(Deserialize)]
struct RewardEvent {
    id: String,
    title: String,
}
// Session reconnect message structs
#[derive(Deserialize)]
struct ReconnectMessage {
    metadata: Metadata,
    payload: ReconnectPayload,
}
#[derive(Deserialize)]
struct ReconnectPayload {
    session: ReconnectSession,
}
#[derive(Deserialize)]
#[allow(dead_code)] // Needed because id is required for message to go through.
struct ReconnectSession {
    id: String,
    reconnect_url: String,
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
    const TWITCH_WEBSOCKET_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
    let mut twitch_websocket_url = TWITCH_WEBSOCKET_URL.to_string();
    let mut is_reconnect_event: bool = false;
    let mut timeout_seconds: Duration = Duration::from_secs(10);

    '_connection: loop {
        let (ws_stream, _response) = connect_async(&twitch_websocket_url)
            .await
            .context("Failed to connect to websocket.")?;

        let (mut _write, mut read) = ws_stream.split();

        loop {
            match timeout(timeout_seconds, read.next()).await {
                Ok(Some(Ok(message))) => {
                    let message = message;
                    if let Message::Text(text) = message {
                        let envelope: Envelope = serde_json::from_str(&text)?;
                        match envelope.metadata.message_type.as_str() {
                            "session_welcome" => {
                                let welcome_text: WelcomeMessage = serde_json::from_str(&text)?;
                                if !is_reconnect_event {
                                    subscribe_to_channel_points(
                                        access_token.to_string(),
                                        &welcome_text.payload.session.id,
                                        client_id,
                                        user_id,
                                    )
                                    .await?;
                                }
                                timeout_seconds = Duration::from_secs(
                                    welcome_text.payload.session.keepalive_timeout_seconds,
                                ) * 2;
                            }
                            "session_keepalive" => {
                                let keepalive_text: KeepAliveMessage = serde_json::from_str(&text)?;
                                println!(
                                    "Keep alive message: {}",
                                    keepalive_text.metadata.message_type
                                )
                            }
                            "notification" => {
                                let notification_text: NotificationMessage =
                                    serde_json::from_str(&text)?;
                                if !shared_state_tx
                                    .reward_list_exists
                                    .read()
                                    .expect("Could not read if the reward list exists.")
                                    .clone()
                                {
                                    println!(
                                        "Notification: Broadcaster Name = {}",
                                        notification_text.payload.event.broadcaster_user_name
                                    );
                                    let _ =
                                        shared_state_tx.tx.send("channel points twin".to_string());
                                    continue;
                                }
                                let reward_list = shared_state_tx.reward_list.read().expect("Could not read reward list from state when trying to do notification.").clone();
                                for id in reward_list {
                                    if notification_text.payload.event.reward.id == id {
                                        println!(
                                            "Notification: Broadcaster Name = {}, Redeemed Reward = {}",
                                            notification_text.payload.event.broadcaster_user_name,
                                            notification_text.payload.event.reward.title
                                        );
                                        let _ = shared_state_tx
                                            .tx
                                            .send("channel points twin".to_string());
                                    }
                                }
                            }
                            "session_reconnect" => {
                                let reconnect_text: ReconnectMessage = serde_json::from_str(&text)?;
                                println!(
                                    "Reconnect text: {}",
                                    reconnect_text.metadata.message_type
                                );
                                twitch_websocket_url = reconnect_text.payload.session.reconnect_url;
                                is_reconnect_event = true;
                                break;
                            }
                            other => println!("Unknown message type: {}", other),
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    eprintln!("Websocket Error: {e}");
                    break;
                }
                Ok(None) => {
                    println!("Connection closed.");
                    break;
                }
                Err(_elapsed) => {
                    println!("Connection to Twitch timed out - reconnecting.");
                    break;
                }
            }
        }
    }
}
/// Method that subscribes to a broadcasters custom rewards notification endpoint.
pub async fn subscribe_to_channel_points(
    mut access_token: String,
    session_id: &str,
    client_id: &str,
    broadcaster_user_id: &str,
) -> Result<ResponseBody, anyhow::Error> {
    const TWITCH_SUBSCRIPTIONS_URL: &str = "https://api.twitch.tv/helix/eventsub/subscriptions";
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

    '_post_request_loop: loop {
        let bearer = format!("Bearer {}", access_token);
        let post_request = client
            .post(TWITCH_SUBSCRIPTIONS_URL)
            .header("Authorization", bearer)
            .header("Client-Id", client_id)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        println!("Sending post request for channel points...");
        let status = post_request.status();
        let body_text = post_request.text().await?;
        if status.is_client_error() {
            println!("Channel points request sent back a client error! Refreshing tokens...");
            let new_access_token = auth::check_tokens().await?;
            access_token = new_access_token;
            continue;
        } else if !status.is_success() {
            println!(
                "Subscription Request sent back an error! Request status code: {}",
                status
            );
            eprintln!("Error body: {:?}", body_text);
        }

        // Receive data
        let post_response: ResponseBody = match serde_json::from_str(&body_text) {
            Ok(body) => body,
            Err(e) => {
                println!("Error parsing the JSON response when subscribing to Twitch. {e}");
                break Err(anyhow::anyhow!(
                    "Error parsing the JSON response when subscribing to Twitch. {e}"
                ));
            }
        };
        println!(
            "Subscribed to {} with a status of {}",
            post_response.data[0].r#type, post_response.data[0].status
        );
        break Ok(post_response);
    }
}
