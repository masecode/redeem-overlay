mod auth;
mod configparser;

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
}
