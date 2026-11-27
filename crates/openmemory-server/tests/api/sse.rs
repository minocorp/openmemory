use std::time::Duration;

use axum::body::BodyDataStream;
use axum::response::Response;
use futures_util::StreamExt;
use serde_json::Value;

use crate::support::TestResult;

const WAIT: Duration = Duration::from_secs(5);

#[derive(Debug, PartialEq)]
pub(crate) struct SseEvent {
    pub(crate) name: String,
    pub(crate) id: String,
    pub(crate) data: Value,
}

pub(crate) struct SseReader {
    body: BodyDataStream,
    buffer: String,
}

impl SseReader {
    pub(crate) fn new(response: Response) -> Self {
        Self {
            body: response.into_body().into_data_stream(),
            buffer: String::new(),
        }
    }

    pub(crate) async fn next_event(&mut self) -> TestResult<SseEvent> {
        loop {
            if let Some(end) = self.buffer.find("\n\n") {
                let block: String = self.buffer.drain(..end + 2).collect();
                if let Some(event) = parse(&block)? {
                    return Ok(event);
                }
                continue;
            }
            let chunk = tokio::time::timeout(WAIT, self.body.next())
                .await?
                .ok_or("the stream ended")??;
            self.buffer.push_str(std::str::from_utf8(&chunk)?);
        }
    }
}

fn parse(block: &str) -> TestResult<Option<SseEvent>> {
    let mut name = None;
    let mut id = None;
    let mut data = None;
    for line in block.lines() {
        if let Some(value) = line.strip_prefix("event: ") {
            name = Some(value.to_owned());
        } else if let Some(value) = line.strip_prefix("id: ") {
            id = Some(value.to_owned());
        } else if let Some(value) = line.strip_prefix("data: ") {
            data = Some(serde_json::from_str(value)?);
        }
    }
    Ok(match (name, id, data) {
        (Some(name), Some(id), Some(data)) => Some(SseEvent { name, id, data }),
        _ => None,
    })
}
