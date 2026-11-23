use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use futures_util::stream::{self, BoxStream, Stream, StreamExt};
use sqlx::PgPool;
use zip::CompressionMethod;
use zip::write::{SimpleFileOptions, StreamWriter, ZipWriter};

use crate::access::Actor;
use crate::files::{FileRecord, FileRef, Files};
use crate::pages::listing::{self, PageSource};
use crate::transfer::TransferError;
use crate::transfer::layout;

#[derive(Debug, Clone, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl SharedBuffer {
    fn take(&self) -> Bytes {
        let mut buffer = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Bytes::from(std::mem::take(&mut *buffer))
    }
}

impl Write for SharedBuffer {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Export {
    writer: Option<ZipWriter<StreamWriter<SharedBuffer>>>,
    buffer: SharedBuffer,
    pages: VecDeque<PageSource>,
    files: VecDeque<FileRecord>,
    current: Option<BoxStream<'static, Result<Bytes, object_store::Error>>>,
    files_service: Files,
    actor: Actor,
}

pub(crate) async fn export(
    pool: &PgPool,
    files_service: &Files,
    actor: &Actor,
) -> Result<impl Stream<Item = Result<Bytes, TransferError>> + Send + use<>, TransferError> {
    let pages = listing::sources(pool, actor).await?;
    let files = files_service.list(actor).await?;
    let buffer = SharedBuffer::default();
    let state = Export {
        writer: Some(ZipWriter::new_stream(buffer.clone())),
        buffer,
        pages: pages.into(),
        files: files.into(),
        current: None,
        files_service: files_service.clone(),
        actor: actor.clone(),
    };
    Ok(stream::unfold(state, |mut state| async move {
        match state.next_chunk().await {
            Ok(Some(chunk)) => Some((Ok(chunk), state)),
            Ok(None) => None,
            Err(error) => {
                state.abandon();
                Some((Err(error), state))
            }
        }
    }))
}

impl Export {
    fn abandon(&mut self) {
        self.writer = None;
        self.pages.clear();
        self.files.clear();
        self.current = None;
        self.buffer = SharedBuffer::default();
    }

    async fn next_chunk(&mut self) -> Result<Option<Bytes>, TransferError> {
        loop {
            let ready = self.buffer.take();
            if !ready.is_empty() {
                return Ok(Some(ready));
            }
            if let Some(stream) = self.current.as_mut() {
                match stream.next().await {
                    Some(chunk) => self.write(&chunk.map_err(|error| {
                        TransferError::Files(crate::files::FilesError::Storage(error))
                    })?)?,
                    None => self.current = None,
                }
            } else if let Some(page) = self.pages.pop_front() {
                self.start_entry(&layout::page_entry(&page.path))?;
                self.write(page.content.as_bytes())?;
            } else if let Some(file) = self.files.pop_front() {
                self.start_entry(&layout::file_entry(file.path.as_str()))?;
                let download = self
                    .files_service
                    .download(&self.actor, &FileRef::Id(file.id))
                    .await?;
                self.current = Some(download.stream);
            } else if let Some(writer) = self.writer.take() {
                writer.finish().map_err(TransferError::InvalidArchive)?;
            } else {
                return Ok(None);
            }
        }
    }

    fn start_entry(&mut self, name: &str) -> Result<(), TransferError> {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .large_file(true);
        self.writer
            .as_mut()
            .ok_or_else(|| {
                TransferError::Write(std::io::Error::other("archive is already finished"))
            })?
            .start_file(name, options)
            .map_err(TransferError::InvalidArchive)
    }

    fn write(&mut self, data: &[u8]) -> Result<(), TransferError> {
        self.writer
            .as_mut()
            .ok_or_else(|| {
                TransferError::Write(std::io::Error::other("archive is already finished"))
            })?
            .write_all(data)
            .map_err(TransferError::Write)
    }
}
