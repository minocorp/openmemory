mod cli;
mod commands;
mod credentials;
mod device;
mod login;
mod render;
mod server;
mod session;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    commands::run(cli::Cli::parse()).await
}
