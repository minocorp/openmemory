use std::process::Stdio;
use std::time::Duration;

use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::Semaphore;

const EXTRACT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_CONCURRENT_EXTRACTIONS: usize = 4;
const MAX_OUTPUT_BYTES: usize = 32 * 1024 * 1024;
const CPU_SECONDS: u32 = 30;
const ADDRESS_SPACE_KIB: u32 = 2 * 1024 * 1024;
static EXTRACTIONS: Semaphore = Semaphore::const_new(MAX_CONCURRENT_EXTRACTIONS);
const PAGE_BREAK: char = '\u{c}';

#[derive(Debug, thiserror::Error)]
pub(crate) enum PdfError {
    #[error("pdftotext could not start")]
    Unavailable(#[source] std::io::Error),
    #[error("pdftotext did not finish in time")]
    TimedOut,
    #[error("pdf is unreadable")]
    Unreadable,
    #[error("pdftotext wrote more than the allowed output")]
    OutputTooLarge,
}

pub(crate) async fn extract_pages(pdf: Bytes) -> Result<Vec<String>, PdfError> {
    extract_pages_limited(&EXTRACTIONS, pdf).await
}

async fn extract_pages_limited(slots: &Semaphore, pdf: Bytes) -> Result<Vec<String>, PdfError> {
    let extraction = async {
        let _slot = slots.acquire().await.map_err(|_| PdfError::Unreadable)?;
        run(pdf).await
    };
    tokio::time::timeout(EXTRACT_TIMEOUT, extraction)
        .await
        .map_err(|_| PdfError::TimedOut)?
        .map(|text| split_pages(&text))
}

async fn run(pdf: Bytes) -> Result<String, PdfError> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ulimit -t {CPU_SECONDS} 2>/dev/null; ulimit -v {ADDRESS_SPACE_KIB} 2>/dev/null; exec \"$@\""
        ))
        .args(["sh", "pdftotext", "-enc", "UTF-8", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(PdfError::Unavailable)?;
    let mut stdin = child.stdin.take().ok_or(PdfError::Unreadable)?;
    let feed = async move {
        // pdftotext exits early on a broken file, so a failed write is not the error to report
        let _ = stdin.write_all(&pdf).await;
    };
    let stdout = child.stdout.take().ok_or(PdfError::Unreadable)?;
    let (text, ()) = tokio::join!(read_capped(stdout, MAX_OUTPUT_BYTES), feed);
    let text = text?;
    let status = child.wait().await.map_err(PdfError::Unavailable)?;
    if !status.success() {
        return Err(PdfError::Unreadable);
    }
    String::from_utf8(text).map_err(|_| PdfError::Unreadable)
}

async fn read_capped(mut reader: impl AsyncRead + Unpin, cap: usize) -> Result<Vec<u8>, PdfError> {
    let mut output = Vec::new();
    let mut chunk = vec![0_u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut chunk)
            .await
            .map_err(PdfError::Unavailable)?;
        if read == 0 {
            return Ok(output);
        }
        if output.len() + read > cap {
            return Err(PdfError::OutputTooLarge);
        }
        output.extend_from_slice(chunk.get(..read).unwrap_or_default());
    }
}

pub(crate) fn split_pages(text: &str) -> Vec<String> {
    let mut pages: Vec<String> = text
        .split(PAGE_BREAK)
        .map(|page| page.replace('\0', ""))
        .collect();
    if pages.last().is_some_and(|page| page.trim().is_empty()) {
        pages.pop();
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn output_over_the_cap_is_refused_while_reading() {
        let endless = tokio::io::repeat(b'a');
        let result = read_capped(endless, 200_000).await;
        assert!(matches!(result, Err(PdfError::OutputTooLarge)));
        let small = read_capped(&b"abc"[..], 200_000).await.unwrap();
        assert_eq!(small, b"abc");
    }

    #[tokio::test]
    async fn extractions_wait_for_a_free_slot() {
        let slots = Semaphore::new(0);
        let waiting = tokio::time::timeout(
            Duration::from_millis(200),
            extract_pages_limited(&slots, Bytes::from_static(b"%PDF")),
        )
        .await;
        assert!(waiting.is_err());
    }

    #[test]
    fn form_feeds_separate_pages_and_the_trailing_one_is_dropped() {
        assert_eq!(split_pages("a\n\u{c}b\n\u{c}"), ["a\n", "b\n"]);
    }

    #[test]
    fn nul_bytes_are_removed() {
        assert_eq!(split_pages("a\0b\u{c}"), ["ab"]);
    }

    #[test]
    fn empty_output_has_no_pages() {
        assert_eq!(split_pages(""), Vec::<String>::new());
    }
}
