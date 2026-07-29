use std::path::Path;

use async_trait::async_trait;
use tokio::{
    fs::OpenOptions,
    io::{AsyncWriteExt, BufWriter},
    sync::Mutex,
};

use crate::domain::{RedirectEvent, RedirectEventSink, RedirectEventSinkError};

pub struct JsonlRedirectEventSink {
    writer: Mutex<BufWriter<tokio::fs::File>>,
}

impl JsonlRedirectEventSink {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, RedirectEventSinkError> {
        let path = path.as_ref();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map_err(|error| {
                RedirectEventSinkError::new(format!(
                    "could not open event output {}: {error}",
                    path.display()
                ))
            })?;
        Ok(Self {
            writer: Mutex::new(BufWriter::new(file)),
        })
    }
}

#[async_trait]
impl RedirectEventSink for JsonlRedirectEventSink {
    async fn emit(&self, event: &RedirectEvent) -> Result<(), RedirectEventSinkError> {
        let mut record = serde_json::to_vec(event).map_err(|error| {
            RedirectEventSinkError::new(format!("could not serialize redirect event: {error}"))
        })?;
        record.push(b'\n');

        let mut writer = self.writer.lock().await;
        writer.write_all(&record).await.map_err(|error| {
            RedirectEventSinkError::new(format!("could not append redirect event: {error}"))
        })?;
        writer.flush().await.map_err(|error| {
            RedirectEventSinkError::new(format!("could not flush redirect event: {error}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc};

    use chrono::Utc;
    use tempfile::tempdir;
    use tokio::fs;

    use super::*;
    use crate::domain::{
        ClientInfo, EventId, RedirectEvent, RedirectOutcome, RequestInfo, ResponseInfo,
    };

    fn event() -> RedirectEvent {
        RedirectEvent {
            event_id: EventId::generate(),
            redirect_id: None,
            occurred_at: Utc::now(),
            duration_ms: 1.0,
            outcome: RedirectOutcome::NotFound,
            client: ClientInfo {
                address: None,
                user_agent: None,
            },
            request: RequestInfo {
                method: "GET".into(),
                scheme: "http".into(),
                host: "example.com".into(),
                path: "/missing".into(),
                query: None,
                protocol: "HTTP/1.1".into(),
                headers: BTreeMap::new(),
            },
            response: ResponseInfo {
                status_code: 404,
                location: None,
            },
        }
    }

    #[tokio::test]
    async fn creates_appends_and_serializes_concurrent_records() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("events.jsonl");
        let sink = Arc::new(JsonlRedirectEventSink::open(&path).await.unwrap());
        let mut tasks = Vec::new();
        for _ in 0..20 {
            let sink = Arc::clone(&sink);
            tasks.push(tokio::spawn(async move { sink.emit(&event()).await }));
        }
        for task in tasks {
            task.await.unwrap().unwrap();
        }

        let contents = fs::read_to_string(&path).await.unwrap();
        let lines: Vec<_> = contents.lines().collect();
        assert_eq!(lines.len(), 20);
        assert!(
            lines
                .iter()
                .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
        );

        drop(sink);
        let reopened = JsonlRedirectEventSink::open(&path).await.unwrap();
        reopened.emit(&event()).await.unwrap();
        assert_eq!(fs::read_to_string(&path).await.unwrap().lines().count(), 21);
    }

    #[tokio::test]
    async fn reports_open_errors() {
        let directory = tempdir().unwrap();
        let error = JsonlRedirectEventSink::open(directory.path().join("missing/events.jsonl"))
            .await
            .err()
            .expect("missing parent must fail");
        assert!(error.to_string().contains("could not open event output"));
    }
}
