use anyhow::Context;
use anyhow::bail;
use serde::Deserialize;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String, // Authorization Bearer token, needed for POST responses.
    pub refresh_token: String,
    pub expires_in: u64,
}

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
pub fn wait_for_code(port: u16, expected_state: &str) -> Result<String, anyhow::Error> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(listener) => listener,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            anyhow::bail!(
                "Port {port} are already in use, Close whatever is using it and run again."
            );
        }
        Err(e) => anyhow::bail!("Unable to bind TCP Listener: {e}"),
    };
    let mut code = "";
    let mut state = "";

    println!("Listening on {} for redirect..", port);

    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;

        let path = match request_line.split_whitespace().nth(1) {
            Some(path) => path,
            None => {
                eprintln!("Cannot parse request line.");
                anyhow::bail!(
                    "Cannot parse request line when waiting for authentication redirect code."
                )
            }
        };

        let parameters = path
            .split_once("?")
            .with_context(|| "URL from Twitch Auth redirect has no query parameters")?
            .1
            .split("&");

        for parameter in parameters {
            let current_parameter = match parameter.split_once("=") {
                Some(parameter) => parameter,
                None => {
                    eprintln!(
                        "Cannot split key and value from parameters in authentication code redirect URL."
                    );
                    return Err(anyhow::anyhow!(
                        "Cannot split key and value from parameters in authentication code redirect URL."
                    ));
                }
            };
            if current_parameter.0 == "code" {
                code = current_parameter.1;
            } else if current_parameter.0.starts_with("state") {
                state = current_parameter.1;
            }
        }

        if state != expected_state {
            bail!("Returned state was not the expected state.");
        }

        let body = "You can close this tab now, received.";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        match stream.write_all(response.as_bytes()) {
            Ok(_) => (),
            Err(_) => {
                eprintln!("Couldn't write HTML response back to browser after auth code redirect.")
            }
        }
        return Ok(code.to_string());
    }
    bail!("OAuth redirect listener received no connections before shutting down.");
}

pub async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    port: u16,
) -> anyhow::Result<TokenResponse> {
    let redirect_url = format!("http://localhost:{}", port.to_string());
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", &redirect_url),
        ("code", code),
        ("grant_type", "authorization_code"),
    ];

    let client = reqwest::Client::new();
    let post_response = client
        .post("https://id.twitch.tv/oauth2/token")
        .form(&params)
        .send()
        .await?;
    let tokens = post_response.json::<TokenResponse>().await?;

    Ok(tokens)
}
