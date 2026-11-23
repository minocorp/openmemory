use clap::{Parser, Subcommand};
use memory_server::Config;

#[derive(Debug, Parser)]
#[command(name = "memory-server", version)]
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
            println!("{}", memory_server::openapi_json()?);
            Ok(())
        }
        Some(Command::Serve(config)) => serve(*config).await,
        None => serve(Config::parse_from(["memory-server"])).await,
    }
}

async fn serve(config: Config) -> anyhow::Result<()> {
    memory_server::init_tracing();
    memory_server::run(config).await
}
