use anyhow::{Context, Error, Result, bail};
use serde::Deserialize;
use std::path::PathBuf;
use url::Url;
use tracing::debug;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub matrix: Matrix,
    pub gotify: Gotify,
}

impl Config {

    pub fn verify(&mut self) -> Result<()> {
        if self.gotify.threshold_high <= self.gotify.threshold_low {
            bail!("Thresholds must be strictly monotonous, defaults are [low, high] = 3, 8");
        }
        if self.gotify.threshold_low < 0 || self.gotify.threshold_high < 0 {
            bail!("Thresholds must be strictly positive.");
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct Gotify {
    pub url: Url,
    pub token: String,
    #[serde(default)]
    pub delete_sent: bool,

    // default vaules that can be overriden bei low, normal, high settings
    #[serde(default = "default_plain")]
    pub plain: String,
    #[serde(default = "default_html")]
    pub html: String,

    // settings for different priorities
    #[serde(default = "default_threshold_low")]
    pub threshold_low : i32,
    #[serde(default = "default_threshold_high")]
    pub threshold_high : i32,
    #[serde(default = "default_format")]
    pub low: GotifyFormat,
    #[serde(default = "default_format")]
    pub normal: GotifyFormat,
    #[serde(default = "default_format")]
    pub high: GotifyFormat,
}

#[derive(Deserialize, Debug)]
pub struct GotifyFormat {
    pub plain: Option<String>,
    pub html: Option<String>,
}

#[derive(Debug)]
pub enum GotifyPriority {
    Low,
    Normal,
    High
}

impl GotifyPriority {
    pub fn from_thresholds(threshold: i32, low: i32, high: i32) -> GotifyPriority {
        if threshold <= low {
            GotifyPriority::Low
        } else if threshold >= high {
            GotifyPriority::High
        }
        else {
            GotifyPriority::Normal
        }
    }
}


fn default_plain() -> String {
    "{{app}}: {{title}}\n{{message}}".to_string()
}

fn default_html() -> String {
    "<h4>{{app}}: {{title}}</h4>\n{{message}}".to_string()
}

fn default_format() -> GotifyFormat {
    GotifyFormat {
        plain: None,
        html: None,
    }
}

fn default_threshold_low() -> i32 {
    3
}
fn default_threshold_high() -> i32 {
    8
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

            let low = envy::prefixed("G2M_GOTIFY_LOW_").from_env::<GotifyFormat>()?;
            let normal = envy::prefixed("G2M_GOTIFY_NORMAL_").from_env::<GotifyFormat>()?;
            let high = envy::prefixed("G2M_GOTIFY_HIGH_").from_env::<GotifyFormat>()?;
            let mut gotify = envy::prefixed("G2M_GOTIFY_").from_env::<Gotify>()?;
            gotify.low = low;
            gotify.normal = normal;
            gotify.high = high;

            config = Some(Config { matrix, gotify });
        }

        let mut config = config.ok_or(Error::msg(
            "Could not read config file from either file nor environment",
        ))?;
        config.verify()?;
        debug!("Config: {:?}", config);
        Ok(config)
    }
}
