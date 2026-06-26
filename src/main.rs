mod auth;

/**
 * Main method.
 */
fn main() {
    webbrowser::open(&auth::build_authorization(
        "client_id", // Placeholder. TODO: Get config.toml parser working.
        3000,
        "temp_state_123",
    ));
    let code = auth::wait_for_code(3000, "temp_state_123");
    println!("Got code: {}", code);
}
