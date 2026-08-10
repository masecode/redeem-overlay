use crate::auth;
use anyhow::Context;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct HelixData {
    data: Vec<HelixArray>,
}
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct HelixArray {
    id: String,
    login: String,
}

pub async fn get_broadcaster_id(client_id: &str, mut token: String) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    loop {
        println!("Sending broadcaster id post request...");
        let bearer = format!("Bearer {}", token);
        let get_request = client
            .get("https://api.twitch.tv/helix/users")
            .header("Authorization", bearer)
            .header("Client-Id", client_id)
            .send()
            .await?;

        if get_request.status().is_client_error() {
            println!("Broadcaster id client error! Saving new tokens...");
            auth::save_tokens(&auth::refresh_tokens().await?)?;
            token = auth::check_tokens().await?;
            continue;
        }

        let info = get_request.json::<HelixData>().await?;
        let entry = info
            .data
            .first()
            .context("Twitch returned no user data, check your authentication token.")?;
        let id = entry.id.to_string();
        println!("Broadcaster Id request was successful!");

        break Ok(id);
    }
}
