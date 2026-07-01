use anyhow::Ok;

mod auth;
mod configparser;
mod connect;

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
    println!(
        "Broadcaster id: {}",
        auth::get_broadcaster_id(&config.client_id, &tokens.access_token).await?
    );

    Ok(())
}
