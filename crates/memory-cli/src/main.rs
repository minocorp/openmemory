mod cli;
mod client;
mod commands;
mod context;
mod credentials;
mod error;
mod login;
mod oauth;
mod output;
mod render;

#[cfg(test)]
mod mock;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::Cli;
use crate::context::Context;
use crate::credentials::Vault;
use crate::output::Output;

const USAGE_EXIT: u8 = 2;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = if error.use_stderr() { USAGE_EXIT } else { 0 };
            error.print().ok();
            return ExitCode::from(code);
        }
    };
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("memory: {error:#}");
            ExitCode::from(error::exit_code(&error))
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let directory = credentials::config_directory()?;
    // The Linux keychain store starts its own runtime, which cannot happen inside ours.
    let vault = Vault::system(&directory);
    let context = Context {
        output: Output::detect(cli.json),
        vault,
        http: context::http_client()?,
        host: cli.host,
        token: std::env::var("MEMORY_TOKEN")
            .ok()
            .filter(|token| !token.is_empty()),
        editor: std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .ok()
            .filter(|editor| !editor.is_empty()),
        no_input: cli.no_input,
    };
    tokio::runtime::Runtime::new()?.block_on(commands::run(cli.command, &context))
}
