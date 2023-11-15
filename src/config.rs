use serde::Deserialize;
use std::path::PathBuf;
use url::Url;

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
   pub session_dir: PathBuf,
}

#[derive(Deserialize)]
pub struct Gotify {
   pub url: Url,
   pub token: String,
}
