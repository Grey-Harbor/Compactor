use std::{collections::BTreeMap, fmt};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;
use url::Url;

use super::{RedirectEventSinkError, RedirectId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(String);

impl EventId {
    pub fn generate() -> Self {
        Self(Ulid::new().to_string())
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedirectOutcome {
    Redirected,
    NotFound,
    InvalidRequest,
    SourceError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientInfo {
    pub address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestInfo {
    pub method: String,
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub query: Option<String>,
    pub protocol: String,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseInfo {
    pub status_code: u16,
    pub location: Option<Url>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedirectEvent {
    pub event_id: EventId,
    pub redirect_id: Option<RedirectId>,
    pub occurred_at: DateTime<Utc>,
    pub duration_ms: f64,
    pub outcome: RedirectOutcome,
    pub client: ClientInfo,
    pub request: RequestInfo,
    pub response: ResponseInfo,
}

#[async_trait]
pub trait RedirectEventSink: Send + Sync {
    async fn emit(&self, event: &RedirectEvent) -> Result<(), RedirectEventSinkError>;
}
