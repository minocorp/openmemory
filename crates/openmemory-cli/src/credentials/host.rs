use url::Url;

use crate::error::CliError;

pub(crate) fn normalize_host(input: &str) -> Result<String, CliError> {
    let invalid = || {
        CliError::Usage(format!(
            "`{input}` is not a server address like https://openmemory.example.com"
        ))
    };
    let url = Url::parse(input.trim()).map_err(|_| invalid())?;
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        return Err(invalid());
    }
    Ok(url.origin().ascii_serialization())
}
