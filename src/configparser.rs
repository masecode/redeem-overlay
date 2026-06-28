use serde::Deserialize;
use std::fs;
use toml::Deserializer;

#[derive(Debug, Deserialize)]
pub struct Config {
    client_id: String,
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Config> {
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }
}

pub fn parse_configuration_file() -> String {
    let config = Config::load("config.toml").unwrap();
    let id = config.client_id;
    return id;
}
