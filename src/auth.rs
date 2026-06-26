pub fn build_authorization(client_id: &str, port: u16, state: &str) -> String {
    // TODO: Format URL string.
    // https://id.twitch.tv/oauth2/authorize?[parameters]
    format!(
        "https://id.twitch.tv/oauth2/authorize?response_type=code&client_id={}&redirect_uri=http%3A%2F%2Flocalhost%3A{}&scope=channel%3Aread%3Aredemptions&state={}",
        client_id, port, state
    )
}
