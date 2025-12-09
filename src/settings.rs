use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub depot_url: String,
    pub api_key_id: String,
    pub api_secret_key: String,
    pub api_base_url: String,
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {
        let builder = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(Environment::with_prefix("APP").separator("__"));

        builder.build()?.try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_load() {
        let settings = Settings::new();
        assert!(
            settings.is_ok(),
            "Settings failed to load: {:?}",
            settings.err()
        );
        let settings = settings.unwrap();

        // Check default values from config/default.toml
        assert_eq!(settings.depot_url, "http://[::1]:50051");
        assert_eq!(settings.api_base_url, "https://paper-api.alpaca.markets");
        assert_eq!(settings.api_key_id, "FAKE_API_KEY");
        assert_eq!(settings.api_secret_key, "FAKE_SECRET_KEY");
    }
}
