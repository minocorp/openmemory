use clap::{Parser, Subcommand};
use openmemory_server::Config;

#[derive(Debug, Parser)]
#[command(name = "openmemory-server", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve(Box<Config>),
    Openapi,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Some(Command::Openapi) => {
            println!("{}", openmemory_server::openapi_json()?);
            Ok(())
        }
        Some(Command::Serve(config)) => serve(*config).await,
        None => serve(Config::parse_from(["openmemory-server"])).await,
    }
}

async fn serve(config: Config) -> anyhow::Result<()> {
    openmemory_server::init_tracing();
    openmemory_server::run(config).await
}
