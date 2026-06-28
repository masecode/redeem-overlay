use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

/// Builds the authorization URL for the Twitch application.
pub fn build_authorization(client_id: &str, port: u16, state: &str) -> String {
    format!(
        "https://id.twitch.tv/oauth2/authorize?response_type=code&client_id={}&redirect_uri=http%3A%2F%2Flocalhost%3A{}&scope=channel%3Aread%3Aredemptions&state={}",
        client_id, port, state
    )
}

/// Function that will bind a TcpListener to a localhost address and look for the OAuth redirect.
///
/// # Panics
///
/// Panics if:
/// - the state from redirect is not the expected state.
pub fn wait_for_code(port: u16, expected_state: &str) -> String {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).unwrap();
    let mut code = "";
    let mut state = "";

    println!("Listening on {} for redirect..", port);

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();

        let path = request_line.split_whitespace().nth(1).unwrap();

        let parameters = path.split_once("?").unwrap().1.split("&");

        for parameter in parameters {
            let current_parameter = parameter.split_once("=").unwrap();
            if current_parameter.0 == "code" {
                code = current_parameter.1;
            } else if current_parameter.0.starts_with("state") {
                state = current_parameter.1;
            }
        }

        if state != expected_state {
            panic!("Returned state was not the expected state.");
        }

        let body = "You can close this tab now, received.";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {} \r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
        return code.to_string();
    }
    unreachable!();
}
