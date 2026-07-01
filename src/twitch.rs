use serde::Deserialize;

#[derive(Deserialize)]
pub struct HelixData {
    data: Vec<HelixArray>,
}
#[derive(Deserialize)]
pub struct HelixArray {
    id: String,
    login: String,
}

pub async fn get_broadcaster_id(client_id: &str, token: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let get_request = client
        .get("https://api.twitch.tv/helix/users")
        .header("Authorization", format!("Bearer {}", token))
        .header("Client-Id", client_id)
        .send()
        .await?;

    let info = get_request.json::<HelixData>().await?;
    let id = info.data[0].id.to_string();

    Ok(id)
}
