use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use bytes::Bytes;
use openmemory_ops::{MAX_INLINE_UPLOAD_BYTES, PagePath, UploadInput, UploadOutput};

use crate::access::Actor;
use crate::files::{FileContent, mime_for_path};
use crate::mcp::error::ToolError;
use crate::state::AppState;

pub(in crate::mcp) async fn upload(
    state: &AppState,
    actor: &Actor,
    input: UploadInput,
) -> Result<UploadOutput, ToolError> {
    let path = PagePath::parse(&input.path)?;
    let content = match (input.text, input.content_base64) {
        (Some(_), Some(_)) => {
            return Err(ToolError::rejected(
                "Give either `text` or `content_base64`, not both.",
            ));
        }
        (Some(text), None) => Some(FileContent::Text(text)),
        (None, Some(encoded)) => Some(FileContent::Bytes(decode(&encoded)?)),
        (None, None) => None,
    };
    let Some(content) = content else {
        let link = state
            .files()
            .create_upload_link(actor, path.as_str())
            .await?;
        return Ok(UploadOutput {
            path: path.to_string(),
            upload_link: Some(state.public_url().join(&format!("/upload/{}", link.token))),
        });
    };
    let mime = mime_for_path(&path).ok_or_else(|| {
        ToolError::rejected(
            "The file extension is not accepted. Use pdf, txt, md, csv, html, htm, xml, png, jpg, gif, webp, heic, avif, svg, xlsx, xls, ods, docx, mp3, m4a, aac, wav, ogg, flac or webm.",
        )
    })?;
    state
        .files()
        .upload(actor, path.as_str(), content, mime)
        .await?;
    Ok(UploadOutput {
        path: path.to_string(),
        upload_link: None,
    })
}

fn decode(encoded: &str) -> Result<Bytes, ToolError> {
    if encoded.len() / 4 * 3 > MAX_INLINE_UPLOAD_BYTES {
        return Err(ToolError::rejected(
            "The content is too large to send inline. Call `upload` with only `path` to get a one-time link for a person to upload it.",
        ));
    }
    STANDARD
        .decode(encoded)
        .map(Bytes::from)
        .map_err(|_| ToolError::rejected("`content_base64` is not valid standard base64."))
}
