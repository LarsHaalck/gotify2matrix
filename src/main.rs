use std::path::PathBuf;
use structopt::StructOpt;

mod client;
pub mod config;
pub mod session;
mod verify;

#[derive(StructOpt, Clone)]
enum Command {
    #[structopt(about = "Wait for incoming device verifications")]
    Verify,
}

#[derive(StructOpt)]
struct Options {
    #[structopt(short = "c", long = "config")]
    config_file: Option<PathBuf>,
    #[structopt(subcommand)]
    command: Option<Command>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let options = Options::from_args();

    let config_file = match options.config_file {
        Some(file) => file,
        None => dirs::config_local_dir()
            .expect("Config dir does not exist")
            .join("gotify2matrix")
            .join("config.toml"),
    };
    let config = std::fs::read_to_string(config_file).expect("Could not read config file");
    let config: config::Config =
        toml::from_str(config.as_str()).expect("Could not parse config file");

    match options.command {
        Some(Command::Verify) => verify::run(config).await?,
        _ => client::run(config).await?,
    }
    Ok(())
}
