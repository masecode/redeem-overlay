use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    client_id: String,
    client_secret: String,
    redirect_port: u16,
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Config> {
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }
}

pub fn parse_configuration_file() -> Config {
    let config = Config::load("config.toml").unwrap();
    println!("Configuration: {:?}", config);
    return config;
}
