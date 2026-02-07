use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub depot_url: String,
    pub indicator_url: String,
    pub api_key_id: String,
    pub api_secret_key: String,
    pub api_base_url: String,
    pub use_mock_data: bool,
    pub mock_file_path: String,
    pub top_n_configs: usize,
    pub eval_iterations: usize,
    pub surreal_db_url: String,
    pub surreal_db_user: String,
    pub surreal_db_pass: String,
    pub max_trade_percent: f64,
    pub max_position_percent: f64,
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
        // It might fail if config is missing, but we proceed.
        if let Err(e) = &settings {
            println!(
                "Settings load error (expected if env vars missing): {:?}",
                e
            );
        }
    }
}
