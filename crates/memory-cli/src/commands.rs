mod api;
mod auth;
mod edit;
mod file;
mod folder;
mod history;
mod page;
mod search;

use clap::CommandFactory;
use clap_complete::Shell;

use crate::cli::{Cli, Command};
use crate::context::Context;

pub(crate) async fn run(command: Command, context: &Context) -> anyhow::Result<()> {
    match command {
        Command::Auth(command) => auth::run(command, context).await,
        Command::Page(command) => page::run(command, context).await,
        Command::File(command) => file::run(command, context).await,
        Command::Search(args) => search::run(&args, context).await,
        Command::History { path } => history::history(path.as_deref(), context).await,
        Command::Revert { path, version } => history::revert(&path, version, context).await,
        Command::Changes { seen } => history::changes(seen, context).await,
        Command::Api(args) => api::run(args, context).await,
        Command::Completion { shell } => {
            completion(shell);
            Ok(())
        }
    }
}

fn completion(shell: Shell) {
    clap_complete::generate(shell, &mut Cli::command(), "memory", &mut std::io::stdout());
}

#[cfg(test)]
mod tests;
