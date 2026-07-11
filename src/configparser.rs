use serde::Deserialize;
use std::error::Error;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_port: u16,
    pub http_port: u16,
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Config> {
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }
}

/// Parses TOML configuration files.
/// This function reads the specified file, parses its contents as a TOML document,
/// and returns the parsed configuration.
pub fn parse_configuration_file() -> Result<Config, anyhow::Error> {
    let config = Config::load("config.toml")?;
    println!("Configuration: {:?}", config);
    Ok(config)
}
