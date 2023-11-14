use anyhow;
use std::path::PathBuf;
use structopt::StructOpt;
use url::Url;

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
    let options = Options::from_args();

    match options.command {
        Some(Command::Verify) => {}
        _ => {}
    }
    Ok(())
}
