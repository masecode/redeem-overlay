mod auth;
mod configparser;
mod connect;

/**
 * Main method.
 */
fn main() {
    webbrowser::open(&auth::build_authorization(
        &configparser::parse_configuration_file(),
        3000,
        "temp_state_123",
    ));
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

    Ok(())
}
