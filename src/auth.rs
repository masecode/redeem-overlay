use crate::generate_random_string;
use anyhow::Context;
use anyhow::bail;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use crate::configparser;

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String, // Authorization Bearer token, needed for POST responses.
    pub refresh_token: String,
    pub expires_in: u64,
}
#[derive(Serialize, Deserialize)]
pub struct SavedTokens {
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
}
#[derive(PartialEq, Eq)]
pub enum SavedTokensState {
    Valid(String),
    Expired { access: String, refresh: String },
    NeedsAuth,
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

pub fn save_tokens(tokens: &TokenResponse) -> anyhow::Result<(), anyhow::Error> {
    const FILE_PATH: &str = "token.json";
    const GRACE_PERIOD_SECONDS: Duration = Duration::seconds(60);
    let current_time: DateTime<Utc> = Utc::now();
    let expires_at_time: DateTime<Utc> =
        current_time + Duration::seconds(tokens.expires_in as i64) - GRACE_PERIOD_SECONDS;
    let saved_tokens = SavedTokens {
        access_token: tokens.access_token.to_owned(),
        refresh_token: tokens.refresh_token.to_owned(),
        expires_at: expires_at_time,
    };

    let json_string =
        serde_json::to_string_pretty(&saved_tokens).expect("Failed to write to oauth file.");

    #[warn(unused_variables)]
    let _token_file = match File::create_new(FILE_PATH) {
        Ok(mut file) => {
            println!("Creating new oauth file.");
            file.write_all(json_string.as_bytes())?;
        }
        Err(_already_exists) => {
            println!("OAuth JSON file already exists, overwriting tokens");
            let mut existing_file = File::create(FILE_PATH)?;
            existing_file.write_all(json_string.as_bytes())?;
        }
    };
    Ok(())
}
pub fn load_tokens() -> SavedTokensState {
    const FILE_PATH: &str = "token.json";
    let tokens_file = match File::open(FILE_PATH) {
        Ok(mut file) => {
            let mut json_contents: String = String::default();
            println!("Reading token file contents...");
            file.read_to_string(&mut json_contents)
                .expect("Could not read file and append it to buffer when loading tokens");
            let tokens: SavedTokens = match serde_json::from_str(&json_contents) {
                Ok(tokens) => tokens,
                Err(_) => return SavedTokensState::NeedsAuth,
            };
            if tokens.access_token.is_empty() || tokens.refresh_token.is_empty() {
                println!("Access token or refresh token in file is empty, ");
                return SavedTokensState::NeedsAuth;
            }
            if tokens.expires_at <= Utc::now() {
                println!("Tokens are expired, please refresh them via Twitch's API.");
                return SavedTokensState::Expired {
                    access: tokens.access_token,
                    refresh: tokens.refresh_token,
                };
            }
            SavedTokensState::Valid(tokens.access_token)
        }
        Err(e) => {
            println!("Unexplainable error occured when loading tokens. {}", e);
            return SavedTokensState::NeedsAuth;
        }
    };
    tokens_file
}

pub fn load_tokens_unchecked() -> SavedTokens {
    const FILE_PATH: &str = "token.json";
    let tokens_file: SavedTokens = match File::open(FILE_PATH) {
        Ok(mut file) => {
            let mut json_contents: String = String::default();
            file.read_to_string(&mut json_contents)
                .expect("Could not read file and append it to buffer when loading tokens");
            let tokens: SavedTokens = match serde_json::from_str(&json_contents) {
                Ok(tokens) => tokens,
                Err(_) => panic!("Cannot load tokens from file."),
            };
            tokens
        }
        Err(e) => {
            panic!("Can't read tokens file at all. {e}");
        }
    };
    tokens_file
}
pub async fn refresh_tokens() -> anyhow::Result<TokenResponse> {
    let config = configparser::parse_configuration_file()?;
    let tokens = load_tokens_unchecked();
    let params = [
        ("client_id", config.client_id.as_str()),
        ("client_secret", &config.client_secret),
        ("refresh_token", &tokens.refresh_token),
        ("grant_type", "refresh_token"),
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

pub async fn check_tokens() -> anyhow::Result<String, anyhow::Error> {
    let expected_state: &str = &generate_random_string(20);
    let config = configparser::parse_configuration_file()?;
    let initial_token_load: SavedTokensState = load_tokens();

    let token_state = match initial_token_load {
        SavedTokensState::NeedsAuth => {
            println!("Reached NeedsAuth in main method");
            webbrowser::open(&build_authorization(
                &config.client_id,
                3000,
                expected_state,
            ))?;
            let code = wait_for_code(3000, expected_state)?;

            let tokens = exchange_code(
                &config.client_id,
                &config.client_secret,
                &code,
                config.redirect_port,
            )
            .await?;

            save_tokens(&tokens)?;
            println!("{:?}", tokens);
            tokens.access_token
        }
        SavedTokensState::Expired {
            access: _,
            refresh: _,
        } => {
            println!("Expired!");
            let renewed_tokens = &refresh_tokens().await?;
            save_tokens(renewed_tokens)?;
            renewed_tokens.access_token.clone()
        }
        SavedTokensState::Valid(access) => access,
    };
    Ok(token_state)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_build_auth_url() {
        let client_id = "test_client";
        let port = 3000;
        let state = "temp_state";
        let url = build_authorization(client_id, port, state);

        assert!(
            url.contains("https://id.twitch.tv/oauth2/authorize?response_type=code&client_id=test_client&redirect_uri=http%3A%2F%2Flocalhost%3A3000&scope=channel%3Aread%3Aredemptions&state=temp_state"),
            "Authentication URL is not correct upon test."
        );
    }

    #[test]
    fn test_build_auth_url_includes_client_id() {
        let client_id = "test_client";
        let port = 3000;
        let state = "temp_state";
        let url = build_authorization(client_id, port, state);

        assert!(
            url.contains("client_id=test_client"),
            "Client id parameter must be in authentication url"
        );
    }
    #[test]
    fn test_build_auth_url_includes_port() {
        let client_id = "test_client";
        let port = 3000;
        let state = "temp_state";
        let url = build_authorization(client_id, port, state);

        assert!(
            url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A3000"),
            "Redirect URL Port parameter must be in authentication URL"
        );
    }
    #[test]
    fn test_build_auth_url_includes_state() {
        let client_id = "test_client";
        let port = 3000;
        let state = "temp_state";
        let url = build_authorization(client_id, port, state);

        assert!(
            url.contains("state=temp_state"),
            "State parameter must be in authentication URL"
        );
    }
}
