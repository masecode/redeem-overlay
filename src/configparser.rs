use serde::Deserialize;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading_valid_config() -> anyhow::Result<()> {
        // Prepare contents for test
        let toml_contents = r#"
            client_id = "test_client"
            client_secret = "test_client_secret"
            redirect_port = 3000
            http_port = 8081
            "#;
        let path = "/tmp/test_config.toml";
        fs::write(path, toml_contents)?;

        // Load config
        let config = Config::load("/tmp/test_config.toml")?;

        // Assert it worked
        assert_eq!(config.client_id, "test_client");
        assert_eq!(config.client_secret, "test_client_secret");
        assert_eq!(config.redirect_port, 3000);
        assert_eq!(config.http_port, 8081);

        fs::remove_file("/tmp/test_config.toml")?;
        Ok(())
    }

    #[test]
    fn test_loading_invalid_config() -> anyhow::Result<()> {
        let toml_contents = r#"
            this is not valid
            "#;
        let path = "/tmp/test_invalid.toml";
        fs::write(path, toml_contents)?;

        let config = Config::load("/tmp/test_invalid.toml");

        assert!(config.is_err());
        let err = format!("{}", config.unwrap_err());
        assert!(
            err.contains("TOML") || err.contains("toml"),
            "expected TOML parse error, got: {err}"
        );

        fs::remove_file("/tmp/test_invalid.toml")?;
        Ok(())
    }

    #[test]
    fn test_loading_missing_file() {
        let config = Config::load("/tmp/missing_config.toml");

        assert!(config.is_err());
    }
}
