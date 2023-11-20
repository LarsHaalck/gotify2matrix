use serde::Deserialize;
use std::path::PathBuf;
use url::Url;
use anyhow::{Result, Context};

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
        let config = std::fs::read_to_string(config_file).context("Could not read config file")?;
      toml::from_str(config.as_str()).context("Could not parse config file")
    }
}
