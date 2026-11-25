use std::path::{Path, PathBuf};

use clap::Args;

#[derive(Debug, Args)]
pub(crate) struct BackupConfig {
    #[arg(long, env = "OPENMEMORY_BACKUP_STATUS_FILE")]
    backup_status_file: Option<PathBuf>,
}

impl BackupConfig {
    pub(crate) fn status_file(&self) -> Option<&Path> {
        self.backup_status_file.as_deref()
    }
}
