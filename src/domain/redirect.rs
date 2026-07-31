use std::{collections::BTreeMap, fmt, str::FromStr};

use http::{
    HeaderName, HeaderValue,
    header::{CONNECTION, CONTENT_LENGTH, DATE, LOCATION, SERVER, TRANSFER_ENCODING},
    uri::PathAndQuery,
};
use serde::{Deserialize, Serialize};
use url::{Position, Url};

use async_trait::async_trait;

use super::RedirectSourceError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CanonicalUrl(String);

impl CanonicalUrl {
    pub fn parse(input: &str) -> Result<Self, String> {
        if input.contains('\\') {
            return Err("canonical URL must not contain backslashes".into());
        }
        let scheme_end = input
            .find("://")
            .ok_or_else(|| "canonical URL must be absolute".to_owned())?;
        let authority_start = scheme_end + 3;
        let remainder = &input[authority_start..];
        let authority_end = remainder
            .find(['/', '?', '#'])
            .map_or(input.len(), |offset| authority_start + offset);
        if authority_end == authority_start {
            return Err("canonical URL must include a host".into());
        }
        let path_end = input[authority_end..]
            .find(['?', '#'])
            .map_or(input.len(), |offset| authority_end + offset);
        let raw_path = match &input[authority_end..path_end] {
            "" => "/",
            path => path,
        };
        PathAndQuery::from_str(raw_path)
            .map_err(|error| format!("invalid canonical URL path: {error}"))?;
        validate_percent_encoding(raw_path)?;

        let url = Url::parse(input).map_err(|error| format!("invalid canonical URL: {error}"))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err("canonical URL scheme must be http or https".into());
        }
        if url.host().is_none() {
            return Err("canonical URL must include a host".into());
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err("canonical URL must not contain user information".into());
        }
        let authority = &url[Position::BeforeHost..Position::AfterPort];
        Ok(Self(format!("{}://{authority}{raw_path}", url.scheme())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn validate_percent_encoding(path: &str) -> Result<(), String> {
    let bytes = path.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'%'
            && (index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit())
        {
            return Err("canonical URL path contains malformed percent encoding".into());
        }
    }
    Ok(())
}

impl fmt::Display for CanonicalUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for CanonicalUrl {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl Serialize for CanonicalUrl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CanonicalUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RedirectId(String);

impl RedirectId {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("redirect ID must not be empty".into());
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RedirectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectStatus {
    MovedPermanently,
    Found,
    SeeOther,
    TemporaryRedirect,
    PermanentRedirect,
}

impl RedirectStatus {
    pub const fn as_u16(self) -> u16 {
        match self {
            Self::MovedPermanently => 301,
            Self::Found => 302,
            Self::SeeOther => 303,
            Self::TemporaryRedirect => 307,
            Self::PermanentRedirect => 308,
        }
    }
}

impl TryFrom<u16> for RedirectStatus {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            301 => Ok(Self::MovedPermanently),
            302 => Ok(Self::Found),
            303 => Ok(Self::SeeOther),
            307 => Ok(Self::TemporaryRedirect),
            308 => Ok(Self::PermanentRedirect),
            _ => Err(format!("unsupported redirect status code {value}")),
        }
    }
}

impl Serialize for RedirectStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u16(self.as_u16())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResponseHeaders(Vec<(HeaderName, HeaderValue)>);

impl ResponseHeaders {
    pub fn try_from_strings(headers: BTreeMap<String, String>) -> Result<Self, String> {
        let mut validated = Vec::new();
        for (name, value) in headers {
            let name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|error| format!("malformed response header name {name:?}: {error}"))?;
            if is_prohibited(&name) {
                return Err(format!("response header {name} is controlled by Compactor"));
            }
            let value = HeaderValue::from_str(&value)
                .map_err(|error| format!("malformed value for response header {name}: {error}"))?;
            validated.push((name, value));
        }
        Ok(Self(validated))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.0.iter().map(|(name, value)| (name, value))
    }
}

fn is_prohibited(name: &HeaderName) -> bool {
    [
        LOCATION,
        CONTENT_LENGTH,
        CONNECTION,
        TRANSFER_ENCODING,
        DATE,
        SERVER,
    ]
    .contains(name)
}

#[derive(Debug, Clone)]
pub struct RedirectDefinition {
    pub id: RedirectId,
    pub canonical_url: CanonicalUrl,
    pub redirect_url: Url,
    pub status_code: RedirectStatus,
    pub response_headers: ResponseHeaders,
}

#[async_trait]
pub trait RedirectSource: Send + Sync {
    async fn resolve(
        &self,
        canonical_url: &CanonicalUrl,
    ) -> Result<Option<RedirectDefinition>, RedirectSourceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_url_normalizes_only_authority_basics() {
        let url = CanonicalUrl::parse("HTTPS://EXAMPLE.COM:443/Docs/?ignored=1#fragment").unwrap();
        assert_eq!(url.as_str(), "https://example.com/Docs/");
        assert_ne!(
            url,
            CanonicalUrl::parse("https://example.com/Docs").unwrap()
        );
    }

    #[test]
    fn canonical_url_normalizes_empty_path_and_default_port() {
        assert_eq!(
            CanonicalUrl::parse("http://Example.com:80")
                .unwrap()
                .as_str(),
            "http://example.com/"
        );
    }

    #[test]
    fn canonical_url_excludes_query_but_distinguishes_trailing_slash() {
        assert_eq!(
            CanonicalUrl::parse("https://example.com/path?one=1").unwrap(),
            CanonicalUrl::parse("https://example.com/path?two=2").unwrap()
        );
        assert_ne!(
            CanonicalUrl::parse("https://example.com/path").unwrap(),
            CanonicalUrl::parse("https://example.com/path/").unwrap()
        );
        assert!(CanonicalUrl::parse("https://").is_err());
    }

    #[test]
    fn canonical_url_preserves_path_semantics_and_percent_spelling() {
        for path in ["/a/../b", "/a/./b", "/%2e%2e/secret", "/%7Euser", "/a//b"] {
            let canonical = CanonicalUrl::parse(&format!("https://example.com{path}")).unwrap();
            assert_eq!(canonical.as_str(), format!("https://example.com{path}"));
        }
        assert_ne!(
            CanonicalUrl::parse("https://example.com/%7Euser").unwrap(),
            CanonicalUrl::parse("https://example.com/~user").unwrap()
        );
        assert_ne!(
            CanonicalUrl::parse("https://example.com/%7Euser").unwrap(),
            CanonicalUrl::parse("https://example.com/%7euser").unwrap()
        );
        assert_ne!(
            CanonicalUrl::parse("https://example.com/%2e%2e/secret").unwrap(),
            CanonicalUrl::parse("https://example.com/secret").unwrap()
        );
    }

    #[test]
    fn canonical_url_rejects_ambiguous_or_malformed_inputs() {
        assert!(CanonicalUrl::parse("https://example.com/a b").is_err());
        assert!(CanonicalUrl::parse("https://example.com/%ZZ").is_err());
        assert!(CanonicalUrl::parse("https://example.com/%2").is_err());
        assert!(CanonicalUrl::parse("https://example.com\\other/path").is_err());
        assert!(CanonicalUrl::parse("relative/path").is_err());
        assert!(CanonicalUrl::parse("https:///missing-host").is_err());
    }

    #[test]
    fn response_headers_reject_protocol_owned_names_case_insensitively() {
        let error = ResponseHeaders::try_from_strings(BTreeMap::from([(
            "lOcAtIoN".into(),
            "https://example.com".into(),
        )]))
        .unwrap_err();
        assert!(error.contains("controlled by Compactor"));
    }
}
