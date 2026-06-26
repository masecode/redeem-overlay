mod auth;

/**
 * Main method.
 */
fn main() {
    println!(
        "{}",
        auth::build_authorization("client_id", 3000, "temp_state_123")
    )
}
