use anyhow;
use rand::{RngExt, distr::Alphanumeric};
use serde_json;
use std::default;
use std::fs;
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::broadcast;
use webbrowser::Browser::Default;

mod auth;
mod configparser;
mod connect;
mod httpserver;
mod twitch;

struct SharedState {
    tx: broadcast::Sender<String>,
    reward_list: RwLock<Vec<String>>,
    reward_list_exists: RwLock<bool>,
}

/**
 * Main method.
 */
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = configparser::parse_configuration_file()?;
    let reward_file = fs::read_to_string("rewards.json")?;

    let token_state = auth::check_tokens().await?;

    let reward_list: RwLock<Vec<String>> = match serde_json::from_str(&reward_file) {
        Ok(list) => list,
        Err(_) => {
            println!("Could not read reward list file, possibly non-existent or empty.");
            let default: RwLock<Vec<String>> = RwLock::new(Vec::new());
            default
        }
    };

    let mut reward_list_exists: RwLock<bool> = match reward_list.read() {
        Ok(list) => {
            if (!list.is_empty()) {
                RwLock::new(true)
            } else {
                RwLock::new(false)
            }
        }
        Err(_) => RwLock::new(false),
    };

    let broadcaster_user_id =
        twitch::get_broadcaster_id(&config.client_id, token_state.to_string())
            .await
            .expect("Failed to get broadcaster id.");
    // println!("Broadcaster id: {}", &broadcaster_user_id); // Testing purposes.

    let (tx, _rx) = broadcast::channel(10);

    let shared_state = Arc::new(SharedState {
        tx,
        reward_list,
        reward_list_exists,
    });

    let twitch_state = shared_state.clone();

    tokio::spawn(async move {
        let _ = httpserver::run_server(config.http_port, shared_state).await;
    });
    // This has to be LAST. Do not put anything after connect.
    connect::connect(
        &token_state,
        &config.client_id,
        &broadcaster_user_id,
        twitch_state,
    )
    .await?;
    Ok(())
}

fn generate_random_string(length: usize) -> String {
    let mut rng = rand::rng();
    (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}
