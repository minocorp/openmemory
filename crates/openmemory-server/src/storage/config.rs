use std::path::{Path, PathBuf};

use clap::Args;

use super::secret::Secret;

const S3_GROUP: [&str; 3] = ["s3_endpoint", "s3_access_key", "s3_secret_key"];

#[derive(Debug, Args)]
pub(crate) struct StorageConfig {
    #[arg(long, env = "OPENMEMORY_BLOB_DIR", default_value = "blobs")]
    blob_dir: PathBuf,
    #[arg(long, env = "OPENMEMORY_S3_BUCKET", requires_all = S3_GROUP)]
    s3_bucket: Option<String>,
    #[arg(long, env = "OPENMEMORY_S3_ENDPOINT")]
    s3_endpoint: Option<String>,
    #[arg(long, env = "OPENMEMORY_S3_REGION", default_value = "us-east-1")]
    s3_region: String,
    #[arg(long, env = "OPENMEMORY_S3_ACCESS_KEY")]
    s3_access_key: Option<String>,
    #[arg(long, env = "OPENMEMORY_S3_SECRET_KEY")]
    s3_secret_key: Option<Secret>,
}

pub(super) struct S3Settings<'a> {
    pub(super) endpoint: &'a str,
    pub(super) bucket: &'a str,
    pub(super) region: &'a str,
    pub(super) access_key: &'a str,
    pub(super) secret_key: &'a Secret,
}

impl StorageConfig {
    pub(super) fn blob_dir(&self) -> &Path {
        &self.blob_dir
    }

    pub(super) fn s3(&self) -> Option<S3Settings<'_>> {
        Some(S3Settings {
            endpoint: self.s3_endpoint.as_deref()?,
            bucket: self.s3_bucket.as_deref()?,
            region: &self.s3_region,
            access_key: self.s3_access_key.as_deref()?,
            secret_key: self.s3_secret_key.as_ref()?,
        })
    }
}

#[cfg(test)]
pub(super) fn parse_for_test(args: &[&str]) -> Result<StorageConfig, clap::Error> {
    use clap::Parser;

    #[derive(Debug, Parser)]
    struct Cli {
        #[command(flatten)]
        storage: StorageConfig,
    }

    Cli::try_parse_from(std::iter::once("test").chain(args.iter().copied())).map(|cli| cli.storage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_defaults_to_a_local_directory() {
        let config = parse_for_test(&[]).unwrap();
        assert!(config.s3().is_none());
        assert_eq!(config.blob_dir(), Path::new("blobs"));
    }

    #[test]
    fn s3_secret_key_is_redacted_in_debug_output() {
        let config = parse_for_test(&[
            "--s3-endpoint=https://s3.example",
            "--s3-bucket=openmemory",
            "--s3-access-key=key",
            "--s3-secret-key=very-secret",
        ])
        .unwrap();
        assert!(!format!("{config:?}").contains("very-secret"));
        assert_eq!(config.s3().unwrap().region, "us-east-1");
    }

    #[test]
    fn partial_s3_settings_are_rejected() {
        assert!(parse_for_test(&["--s3-bucket=openmemory"]).is_err());
    }
}
