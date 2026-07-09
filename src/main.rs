use anyhow::Ok;
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
    let config = configparser::parse_configuration_file();
    webbrowser::open(&auth::build_authorization(
        &config.client_id,
        3000,
        "temp_state_123",
    ))?;
    let code = auth::wait_for_code(3000, "temp_state_123");
    println!("Got code: {}", code);

    let tokens = auth::exchange_code(
        &config.client_id,
        &config.client_secret,
        &code,
        config.redirect_port,
    )
    .await?;

    println!("{:?}", tokens);
    let broadcaster_user_id = twitch::get_broadcaster_id(&config.client_id, &tokens.access_token)
        .await
        .expect("Failed to get broadcaster id.");
    println!("Broadcaster id: {}", &broadcaster_user_id);

    // Need to:
    // 1. create a channel,
    // 2. put the sending side into a "shared task" using an Arc wrapper.
    // 3. Get that "Shared task" to both consumers (Twitch notification handler and the httpserver that needs to send a message down.)
    // 4. Get the shared stated attached to the Router
    let (tx, _rx) = broadcast::channel(10);

    let shared_state = Arc::new(SharedState { tx });

    let twitch_state = shared_state.clone();

    tokio::spawn(async move {
        httpserver::run_server(config.http_port, shared_state).await;
    });
    // This has to be LAST. Do not put anything after connect.
    connect::connect(
        &tokens.access_token,
        &config.client_id,
        &broadcaster_user_id,
        twitch_state,
    )
    .await;
    Ok(())
}
