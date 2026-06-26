mod auth;

/**
 * Main method.
 */
fn main() {
    println!(
        "{}",
        auth::build_authorization("t7im3rh1c8rxv7dmrlfjhioh31jxmh", 3000, "temp_state_123")
    )
}
