use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::Path,
};

use serde::Deserialize;
use url::Url;

use crate::domain::{
    CanonicalUrl, RedirectDefinition, RedirectId, RedirectSource, RedirectSourceError,
    RedirectStatus, ResponseHeaders,
};

#[derive(Debug)]
pub struct JsonRedirectSource {
    redirects: HashMap<CanonicalUrl, RedirectDefinition>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    redirects: Vec<RawRedirect>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRedirect {
    id: String,
    canonical_url: String,
    redirect_url: String,
    status_code: u16,
    #[serde(default)]
    response_headers: BTreeMap<String, String>,
}

impl JsonRedirectSource {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, RedirectSourceError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|error| {
            RedirectSourceError::new(format!(
                "could not read redirect source {}: {error}",
                path.display()
            ))
        })?;
        Self::from_json(&contents)
    }

    pub fn from_json(contents: &str) -> Result<Self, RedirectSourceError> {
        let document: Document = serde_json::from_str(contents).map_err(|error| {
            RedirectSourceError::new(format!("invalid redirect source JSON: {error}"))
        })?;
        let mut redirects = HashMap::with_capacity(document.redirects.len());
        let mut ids = HashSet::with_capacity(document.redirects.len());

        for raw in document.redirects {
            let id = RedirectId::new(raw.id).map_err(|error| {
                RedirectSourceError::new(format!("invalid redirect ID: {error}"))
            })?;
            if !ids.insert(id.clone()) {
                return Err(RedirectSourceError::new(format!(
                    "duplicate redirect ID {id}"
                )));
            }

            let canonical_url = CanonicalUrl::parse(&raw.canonical_url).map_err(|error| {
                RedirectSourceError::new(format!(
                    "invalid canonical URL for redirect {id}: {error}"
                ))
            })?;
            let redirect_url = Url::parse(&raw.redirect_url).map_err(|error| {
                RedirectSourceError::new(format!(
                    "invalid destination URL for redirect {id}: {error}"
                ))
            })?;
            if redirect_url.scheme().is_empty() {
                return Err(RedirectSourceError::new(format!(
                    "destination URL for redirect {id} must be absolute"
                )));
            }
            let status_code = RedirectStatus::try_from(raw.status_code)
                .map_err(|error| RedirectSourceError::new(format!("{error} for redirect {id}")))?;
            let response_headers = ResponseHeaders::try_from_strings(raw.response_headers)
                .map_err(|error| {
                    RedirectSourceError::new(format!(
                        "invalid response headers for redirect {id}: {error}"
                    ))
                })?;
            let definition = RedirectDefinition {
                id,
                canonical_url: canonical_url.clone(),
                redirect_url,
                status_code,
                response_headers,
            };
            if redirects
                .insert(canonical_url.clone(), definition)
                .is_some()
            {
                return Err(RedirectSourceError::new(format!(
                    "duplicate normalized canonical URL {canonical_url}"
                )));
            }
        }

        Ok(Self { redirects })
    }

    pub fn len(&self) -> usize {
        self.redirects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.redirects.is_empty()
    }
}

impl RedirectSource for JsonRedirectSource {
    fn resolve(
        &self,
        canonical_url: &CanonicalUrl,
    ) -> Result<Option<RedirectDefinition>, RedirectSourceError> {
        Ok(self.redirects.get(canonical_url).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(items: &str) -> String {
        format!(r#"{{"redirects":[{items}]}}"#)
    }

    fn valid(id: &str, canonical: &str) -> String {
        format!(
            r#"{{"id":"{id}","canonical_url":"{canonical}","redirect_url":"https://destination.example/path","status_code":308,"response_headers":{{}}}}"#
        )
    }

    #[test]
    fn loads_and_resolves_distinct_hosts() {
        let source = JsonRedirectSource::from_json(&document(&format!(
            "{},{}",
            valid("one", "https://one.example/help"),
            valid("two", "https://two.example/help")
        )))
        .unwrap();
        assert_eq!(source.len(), 2);
        let found = source
            .resolve(&CanonicalUrl::parse("https://TWO.example/help?q=1").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(found.id.as_str(), "two");
    }

    #[test]
    fn rejects_duplicate_ids() {
        let error = JsonRedirectSource::from_json(&document(&format!(
            "{},{}",
            valid("same", "https://one.example/"),
            valid("same", "https://two.example/")
        )))
        .unwrap_err();
        assert!(error.to_string().contains("duplicate redirect ID"));
    }

    #[test]
    fn rejects_duplicate_normalized_urls() {
        let error = JsonRedirectSource::from_json(&document(&format!(
            "{},{}",
            valid("one", "https://EXAMPLE.com:443/path?q=1"),
            valid("two", "https://example.com/path")
        )))
        .unwrap_err();
        assert!(error.to_string().contains("duplicate normalized canonical"));
    }

    #[test]
    fn rejects_relative_destination_status_and_headers() {
        let relative = valid("one", "https://example.com/")
            .replace("https://destination.example/path", "/relative");
        assert!(JsonRedirectSource::from_json(&document(&relative)).is_err());

        let status = valid("one", "https://example.com/").replace("308", "304");
        assert!(JsonRedirectSource::from_json(&document(&status)).is_err());

        let header = valid("one", "https://example.com/").replace(
            r#""response_headers":{}"#,
            r#""response_headers":{"Location":"https://evil.example"}"#,
        );
        assert!(JsonRedirectSource::from_json(&document(&header)).is_err());

        let malformed_header = valid("one", "https://example.com/").replace(
            r#""response_headers":{}"#,
            r#""response_headers":{"bad header":"value"}"#,
        );
        assert!(JsonRedirectSource::from_json(&document(&malformed_header)).is_err());

        let malformed_value = valid("one", "https://example.com/").replace(
            r#""response_headers":{}"#,
            r#""response_headers":{"X-Test":"line\u000Abreak"}"#,
        );
        assert!(JsonRedirectSource::from_json(&document(&malformed_value)).is_err());
    }

    #[test]
    fn preserves_distinct_dot_segments_and_percent_encodings() {
        let source = JsonRedirectSource::from_json(&document(&format!(
            "{},{},{}",
            valid("literal-dot", "https://example.com/a/../b"),
            valid("encoded-dot", "https://example.com/%2e%2e/b"),
            valid("tilde", "https://example.com/%7Euser")
        )))
        .unwrap();
        for (url, id) in [
            ("https://example.com/a/../b", "literal-dot"),
            ("https://example.com/%2e%2e/b", "encoded-dot"),
            ("https://example.com/%7Euser", "tilde"),
        ] {
            assert_eq!(
                source
                    .resolve(&CanonicalUrl::parse(url).unwrap())
                    .unwrap()
                    .unwrap()
                    .id
                    .as_str(),
                id
            );
        }
        assert!(
            source
                .resolve(&CanonicalUrl::parse("https://example.com/~user").unwrap())
                .unwrap()
                .is_none()
        );
    }
}
