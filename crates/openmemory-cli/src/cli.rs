use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(
    name = "openmemory",
    version,
    about = "Read and write the shared memory"
)]
pub(crate) struct Cli {
    #[arg(
        long,
        global = true,
        help = "Print JSON (automatic when stdout is not a terminal)"
    )]
    pub(crate) json: bool,
    #[arg(
        long,
        global = true,
        env = "OPENMEMORY_HOST",
        help = "Server address, for example https://openmemory.example.com"
    )]
    pub(crate) host: Option<String>,
    #[arg(long, global = true, help = "Never prompt or open an editor")]
    pub(crate) no_input: bool,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    #[command(subcommand, about = "Sign in, check or sign out")]
    Auth(AuthCommand),
    #[command(subcommand, about = "Read and change pages")]
    Page(PageCommand),
    #[command(subcommand, about = "Upload, download and read files")]
    File(FileCommand),
    #[command(about = "Search pages and files")]
    Search(SearchArgs),
    #[command(about = "Show a page's versions, or recent change groups without a path")]
    History {
        #[arg(help = "Page path")]
        path: Option<String>,
    },
    #[command(about = "Restore an earlier version of a page")]
    Revert {
        #[arg(help = "Page path")]
        path: String,
        #[arg(help = "Version number to restore")]
        version: i32,
    },
    #[command(about = "List changes since you last looked")]
    Changes {
        #[arg(long, help = "Mark everything listed as seen")]
        seen: bool,
    },
    #[command(about = "Call the REST API with your credentials")]
    Api(ApiArgs),
    #[command(about = "Print a shell completion script")]
    Completion {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum AuthCommand {
    #[command(about = "Sign in through the browser")]
    Login {
        #[arg(long, help = "Sign in with a code typed on another device")]
        device: bool,
    },
    #[command(about = "Show the server and whether you are signed in")]
    Status,
    #[command(about = "Sign out and revoke the connection")]
    Logout,
}

#[derive(Debug, Subcommand)]
pub(crate) enum PageCommand {
    #[command(about = "List the pages and folders in a folder")]
    List {
        #[arg(help = "Folder path; the top level when omitted")]
        folder: Option<String>,
    },
    #[command(about = "Print a page")]
    View {
        #[arg(help = "Page path")]
        path: String,
    },
    #[command(about = "Create a page from a file or stdin")]
    Create {
        #[arg(help = "Page path")]
        path: String,
        #[arg(help = "Markdown file, or `-` for stdin")]
        source: Option<String>,
    },
    #[command(about = "Edit a page in $VISUAL or $EDITOR")]
    Edit {
        #[arg(help = "Page path")]
        path: String,
    },
    #[command(about = "Add a timeline entry to a page")]
    Append {
        #[arg(help = "Page path")]
        path: String,
        #[arg(help = "Entry text")]
        text: String,
        #[arg(long, help = "Where the entry comes from")]
        source: Option<String>,
        #[arg(long, help = "Entry date, YYYY-MM-DD")]
        date: Option<String>,
    },
    #[command(name = "move", about = "Move a page")]
    Move { from: String, to: String },
    #[command(about = "Delete a page")]
    Delete {
        #[arg(help = "Page path")]
        path: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum FileCommand {
    #[command(about = "List the files and folders in a folder")]
    List {
        #[arg(help = "Folder path; the top level when omitted")]
        folder: Option<String>,
    },
    #[command(about = "Upload a local file")]
    Upload {
        #[arg(help = "Local file")]
        local: PathBuf,
        #[arg(help = "Path on the server")]
        path: String,
    },
    #[command(about = "Download a file")]
    Download {
        #[arg(help = "Path on the server")]
        path: String,
        #[arg(
            short,
            long,
            help = "Write here instead of the file's name; `-` is stdout"
        )]
        output: Option<String>,
    },
    #[command(about = "Print a file's extracted text")]
    View {
        #[arg(help = "Path on the server")]
        path: String,
        #[arg(long, help = "Text page number, from 1")]
        page: Option<u32>,
    },
}

#[derive(Debug, Args)]
pub(crate) struct SearchArgs {
    #[arg(help = "Text to find")]
    pub(crate) text: String,
    #[arg(
        long = "where",
        value_name = "EXPR",
        help = "Field condition, repeatable"
    )]
    pub(crate) conditions: Vec<String>,
    #[arg(long, help = "Only search below this folder")]
    pub(crate) folder: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct ApiArgs {
    #[arg(help = "Route such as /api/tree; a relative path is taken under /api/")]
    pub(crate) path: String,
    #[arg(short = 'X', long, help = "HTTP method")]
    pub(crate) method: Option<String>,
    #[arg(
        short = 'f',
        long = "field",
        value_name = "KEY=VALUE",
        help = "Text field"
    )]
    pub(crate) fields: Vec<String>,
    #[arg(
        short = 'F',
        long = "raw-field",
        value_name = "KEY=VALUE",
        help = "Field; a value like @file reads that file"
    )]
    pub(crate) raw_fields: Vec<String>,
    #[arg(
        long,
        value_name = "FILE",
        help = "Send this file as the body; `-` is stdin"
    )]
    pub(crate) input: Option<String>,
    #[arg(short = 'H', long = "header", value_name = "NAME: VALUE")]
    pub(crate) headers: Vec<String>,
    #[arg(short = 'i', long, help = "Print the status line and headers")]
    pub(crate) include: bool,
    #[arg(long, help = "Follow `next` cursors and print every page as one array")]
    pub(crate) paginate: bool,
}

#[cfg(test)]
mod tests;
