const NEEDS_AUTH_EXIT: u8 = 4;
const CANCELLED_EXIT: u8 = 2;
const FAILED_EXIT: u8 = 1;

#[derive(Debug, thiserror::Error)]
pub(crate) enum CliError {
    #[error("not signed in: run `openmemory auth login` or set OPENMEMORY_TOKEN")]
    NeedsAuth,
    #[error(
        "no server: pass --host, set OPENMEMORY_HOST or run `openmemory auth login --host <url>`"
    )]
    NeedsHost,
    #[error("cancelled")]
    Cancelled,
    #[error("{0}")]
    Usage(String),
}

pub(crate) fn exit_code(error: &anyhow::Error) -> u8 {
    match error.downcast_ref::<CliError>() {
        Some(CliError::NeedsAuth) => NEEDS_AUTH_EXIT,
        Some(CliError::Cancelled | CliError::Usage(_)) => CANCELLED_EXIT,
        Some(CliError::NeedsHost) | None => FAILED_EXIT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_follow_the_error_kind() {
        assert_eq!(exit_code(&CliError::NeedsAuth.into()), 4);
        assert_eq!(exit_code(&CliError::Cancelled.into()), 2);
        assert_eq!(exit_code(&CliError::Usage("x".into()).into()), 2);
        assert_eq!(exit_code(&CliError::NeedsHost.into()), 1);
        assert_eq!(exit_code(&anyhow::anyhow!("boom")), 1);
    }
}
