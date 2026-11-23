use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_INLINE_UPLOAD_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = crate::schema::omit_null)]
pub struct UploadInput {
    #[schemars(
        description = "Path for the new file, for example `contracts/acme.pdf` or `transcripts/call-2026-11-20.txt`. The extension decides the file type: pdf, txt, md, csv, png, jpg, gif, webp, heic, avif, xlsx, xls, ods, mp3, m4a, aac, wav, ogg, flac, webm."
    )]
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "File content as plain text, for txt, md and csv files. Text holding a secret such as an API key or private key is rejected."
    )]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "File content as standard base64, for small binary files. Leave out both content fields to get a one-time upload link that a person opens to upload a large file."
    )]
    pub content_base64: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadOutput {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload_link: Option<String>,
}
