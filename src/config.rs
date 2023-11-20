use anyhow::{Context, Error, Result};
use serde::Deserialize;
use std::path::PathBuf;
use url::Url;
use tracing::debug;

#[derive(Deserialize)]
pub struct Config {
    pub matrix: Matrix,
    pub gotify: Gotify,
}

#[derive(Deserialize)]
pub struct Matrix {
    pub homeserver: Url,
    pub username: String,
    pub password: String,
    pub room_id: String,
    #[serde(default = "default_session")]
    pub session_dir: PathBuf,
}

fn default_session() -> PathBuf {
    PathBuf::from("./session")
}

#[derive(Deserialize)]
pub struct Gotify {
    pub url: Url,
    pub token: String,
    #[serde(default)] // default false
    pub delete_sent: bool,
    #[serde(default = "default_plain")]
    pub format_plain: String,
    #[serde(default = "default_html")]
    pub format_html: String,
}

fn default_plain() -> String {
    "{{title}} ({{app}}) \n{{message}}".to_string()
}

fn default_html() -> String {
    "<h4>{{title}} (<i>{{app}}</i>)</h4>\n{{message}}".to_string()
}

impl Config {
    pub fn read(config_file: std::path::PathBuf) -> Result<Config> {
        let config: Option<Config>;
        // try to read file first
        if config_file.is_file() {
            debug!("Trying to read config from file: {}", config_file.display());
            let config_str =
                std::fs::read_to_string(config_file).context("Could not read config file")?;
            config =
                Some(toml::from_str(config_str.as_str()).context("Could not parse config file")?);
        } else {
            debug!("Trying to read config from env");
            let matrix = envy::prefixed("G2M_MATRIX_").from_env::<Matrix>()?;
            let gotify = envy::prefixed("G2M_GOTIFY_").from_env::<Gotify>()?;
            config = Some(Config { matrix, gotify });
        }

        config.ok_or(Error::msg(
            "Could not read config file from either file nor environment",
        ))
    }
}
