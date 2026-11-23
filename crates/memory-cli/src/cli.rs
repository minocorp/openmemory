use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use memory_ops::{
    DeleteInput, HistoryInput, MoveInput, ReadInput, RevertInput, SearchInput, WriteInput,
};

#[derive(Debug, Parser)]
#[command(name = "memory", version, about = "Read and write the shared notebook")]
pub(crate) struct Cli {
    #[arg(long, global = true, help = "Print the raw JSON the server returned")]
    pub(crate) json: bool,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    #[command(about = "Sign in to a memory server")]
    Login(LoginArgs),
    #[command(about = "Sign out and remove the connection from the server")]
    Logout,
    #[command(about = "Show the saved server and check the connection")]
    Status,
    #[command(about = "Read a page, file or folder")]
    Read(ReadArgs),
    #[command(about = "Search pages")]
    Search(SearchArgs),
    #[command(about = "Create or edit a page")]
    Write(WriteArgs),
    #[command(name = "move", about = "Move a page")]
    Move(MoveArgs),
    #[command(about = "Delete a page")]
    Delete(DeleteArgs),
    #[command(about = "List page versions")]
    History(HistoryArgs),
    #[command(about = "Undo a page version or a change group")]
    Revert(RevertArgs),
    #[command(about = "Upload a file")]
    Upload(UploadArgs),
}

#[derive(Debug, Args)]
pub(crate) struct LoginArgs {
    #[arg(help = "Server address, for example https://memory.example.com")]
    pub(crate) server: String,
    #[arg(long, help = "Sign in with a code typed on another device")]
    pub(crate) device: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ReadArgs {
    #[arg(help = "Page, file or folder path; `/` is the top level, a trailing slash is a folder")]
    path: String,
    #[arg(long, help = "Read an older version of a page")]
    version: Option<i32>,
    #[arg(long, help = "Page of a PDF or text file")]
    page: Option<u32>,
}

#[derive(Debug, Args)]
pub(crate) struct SearchArgs {
    #[arg(help = "Literal text to find")]
    text: Option<String>,
    #[arg(
        long = "where",
        value_name = "CONDITION",
        help = "Field condition such as status=active or renewal<+90d; repeat to combine"
    )]
    conditions: Vec<String>,
    #[arg(long, help = "Only this folder and below")]
    folder: Option<String>,
    #[arg(long, help = "Maximum pages to return")]
    limit: Option<u32>,
    #[arg(long, help = "Continue after the next_cursor of a previous search")]
    cursor: Option<i64>,
}

#[derive(Debug, Args)]
pub(crate) struct WriteArgs {
    #[arg(help = "Page path")]
    pub(crate) path: String,
    #[arg(
        long,
        conflicts_with_all = ["replace", "append", "overwrite"],
        help = "Create the page with this content; `-` reads standard input"
    )]
    pub(crate) create: Option<String>,
    #[arg(
        long,
        num_args = 2,
        value_names = ["OLD", "NEW"],
        help = "Replace OLD with NEW; OLD must match once"
    )]
    pub(crate) replace: Option<Vec<String>>,
    #[arg(long, help = "Add this line to the timeline")]
    pub(crate) append: Option<String>,
    #[arg(long, requires = "append", help = "Source of the appended line")]
    pub(crate) source: Option<String>,
    #[arg(
        long,
        requires = "append",
        value_name = "YYYY-MM-DD",
        help = "Date the appended fact happened, not after today; default today"
    )]
    pub(crate) date: Option<String>,
    #[arg(
        long,
        requires = "base_version",
        conflicts_with_all = ["create", "replace", "append"],
        help = "Replace the whole page with this content; `-` reads standard input"
    )]
    pub(crate) overwrite: Option<String>,
    #[arg(
        long,
        requires = "overwrite",
        help = "Version the overwrite is based on"
    )]
    pub(crate) base_version: Option<i32>,
    #[arg(long, help = "Label that groups related changes")]
    pub(crate) task: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct MoveArgs {
    from: String,
    to: String,
    #[arg(long, help = "Label that groups related changes")]
    task: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct DeleteArgs {
    path: String,
    #[arg(long, help = "Label that groups related changes")]
    task: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct HistoryArgs {
    #[arg(help = "Page path; leave out for changes everywhere")]
    path: Option<String>,
    #[arg(long, help = "Only this author, such as mino or mino/claude-code")]
    by: Option<String>,
    #[arg(long, help = "Only from this date, YYYY-MM-DD")]
    since: Option<String>,
    #[arg(long, help = "Maximum versions to return")]
    limit: Option<u32>,
}

#[derive(Debug, Args)]
pub(crate) struct RevertArgs {
    #[arg(help = "Page path, with --version")]
    path: Option<String>,
    #[arg(
        long,
        requires = "path",
        conflicts_with = "group",
        help = "Version to restore"
    )]
    version: Option<i32>,
    #[arg(long, conflicts_with_all = ["path", "version"], help = "Change group to undo")]
    group: Option<i64>,
    #[arg(long, help = "Label that groups related changes")]
    task: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct UploadArgs {
    #[arg(help = "File on this computer")]
    pub(crate) file: PathBuf,
    #[arg(help = "Path for the file in the notebook")]
    pub(crate) path: String,
}

impl From<ReadArgs> for ReadInput {
    fn from(args: ReadArgs) -> Self {
        Self {
            path: args.path,
            version: args.version,
            page: args.page,
        }
    }
}

impl From<SearchArgs> for SearchInput {
    fn from(args: SearchArgs) -> Self {
        Self {
            text: args.text,
            conditions: args.conditions,
            folder: args.folder,
            limit: args.limit,
            cursor: args.cursor,
        }
    }
}

impl From<MoveArgs> for MoveInput {
    fn from(args: MoveArgs) -> Self {
        Self {
            from: args.from,
            to: args.to,
            task: args.task,
        }
    }
}

impl From<DeleteArgs> for DeleteInput {
    fn from(args: DeleteArgs) -> Self {
        Self {
            path: args.path,
            task: args.task,
        }
    }
}

impl From<HistoryArgs> for HistoryInput {
    fn from(args: HistoryArgs) -> Self {
        Self {
            path: args.path,
            by: args.by,
            since: args.since,
            limit: args.limit,
        }
    }
}

impl From<RevertArgs> for RevertInput {
    fn from(args: RevertArgs) -> Self {
        Self {
            path: args.path,
            version: args.version,
            group: args.group,
            task: args.task,
        }
    }
}

impl WriteArgs {
    pub(crate) fn into_input(
        self,
        create: Option<String>,
        overwrite: Option<String>,
    ) -> WriteInput {
        let (replace_old, replace_new) = match self.replace.as_deref() {
            Some([old, new]) => (Some(old.clone()), Some(new.clone())),
            _ => (None, None),
        };
        WriteInput {
            path: self.path,
            task: self.task,
            content: create.or(overwrite),
            base_version: self.base_version,
            replace_old,
            replace_new,
            append_text: self.append,
            append_source: self.source,
            append_date: self.date,
        }
    }
}

#[cfg(test)]
mod tests;
