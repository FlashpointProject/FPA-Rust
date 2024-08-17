use std::{env, fs};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub oauth_client_id: String,
    pub oauth_client_secret: String,
    pub oauth_auth_url: String,
    pub oauth_token_url: String,
    pub oauth_redirect_url: String,
    pub oauth_profile_url: String,
    pub oauth_provider: String,
    pub metadata_database: String,
    pub auth_database: String,
}

impl Config {
    pub fn from_env_or_file() -> Self {
        // Try loading from config.json
        let config_file_path = "config.json";
        let config_from_file: Option<Config> = fs::read_to_string(config_file_path)
            .ok()
            .and_then(|file_content| serde_json::from_str(&file_content).ok());

        // Load from environment variables
        let config_from_env = Config {
            oauth_client_id: env::var("OAUTH_CLIENT_ID").unwrap_or_else(|_| "".to_string()),
            oauth_client_secret: env::var("OAUTH_CLIENT_SECRET").unwrap_or_else(|_| "".to_string()),
            oauth_auth_url: env::var("OAUTH_AUTH_URL").unwrap_or_else(|_| "".to_string()),
            oauth_token_url: env::var("OAUTH_TOKEN_URL").unwrap_or_else(|_| "".to_string()),
            oauth_redirect_url: env::var("OAUTH_REDIRECT_URL").unwrap_or_else(|_| "".to_string()),
            oauth_profile_url: env::var("OAUTH_PROFILE_URL").unwrap_or_else(|_| "".to_string()),
            oauth_provider: env::var("OAUTH_PROVIDER").unwrap_or_else(|_| "".to_string()),
            metadata_database: env::var("METADATA_DATABASE")
                .unwrap_or_else(|_| "flashpoint.sqlite".to_string()),
            auth_database: env::var("AUTH_DB").unwrap_or_else(|_| "auth.db".to_string()),
        };

        // Merge configurations, prioritizing the file config
        config_from_file.unwrap_or(config_from_env)
    }
}
