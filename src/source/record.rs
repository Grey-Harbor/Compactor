use std::collections::BTreeMap;

use serde::Deserialize;
use url::Url;

use crate::domain::{
    CanonicalUrl, RedirectDefinition, RedirectId, RedirectSourceError, RedirectStatus,
    ResponseHeaders,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawRedirect {
    pub id: String,
    pub canonical_url: String,
    pub redirect_url: String,
    pub status_code: u16,
    #[serde(default)]
    pub response_headers: BTreeMap<String, String>,
}

impl RawRedirect {
    pub(super) fn into_definition(self) -> Result<RedirectDefinition, RedirectSourceError> {
        let id = RedirectId::new(self.id)
            .map_err(|error| RedirectSourceError::new(format!("invalid redirect ID: {error}")))?;
        let canonical_url = CanonicalUrl::parse(&self.canonical_url).map_err(|error| {
            RedirectSourceError::new(format!("invalid canonical URL for redirect {id}: {error}"))
        })?;
        let redirect_url = Url::parse(&self.redirect_url).map_err(|error| {
            RedirectSourceError::new(format!(
                "invalid destination URL for redirect {id}: {error}"
            ))
        })?;
        let status_code = RedirectStatus::try_from(self.status_code)
            .map_err(|error| RedirectSourceError::new(format!("{error} for redirect {id}")))?;
        let response_headers =
            ResponseHeaders::try_from_strings(self.response_headers).map_err(|error| {
                RedirectSourceError::new(format!(
                    "invalid response headers for redirect {id}: {error}"
                ))
            })?;
        Ok(RedirectDefinition {
            id,
            canonical_url,
            redirect_url,
            status_code,
            response_headers,
        })
    }
}
