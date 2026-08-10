use anyhow::Ok;
use rand::{RngExt, distr::Alphanumeric};
use std::sync::Arc;
use tokio::sync::broadcast;

mod auth;
mod configparser;
mod connect;
mod httpserver;
mod twitch;

struct SharedState {
    tx: broadcast::Sender<String>,
}

/**
 * Main method.
 */
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = configparser::parse_configuration_file()?;

    let token_state = auth::check_tokens().await?;

    // TODO: Implement OAuth preservation. Web browser should only open if both access and refresh token is not available.
    let broadcaster_user_id =
        twitch::get_broadcaster_id(&config.client_id, token_state.to_string())
            .await
            .expect("Failed to get broadcaster id.");
    // println!("Broadcaster id: {}", &broadcaster_user_id); // Testing purposes.

    let (tx, _rx) = broadcast::channel(10);

    let shared_state = Arc::new(SharedState { tx });

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
