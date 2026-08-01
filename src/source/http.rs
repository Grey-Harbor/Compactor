use async_trait::async_trait;
use http::header::{ACCEPT, AUTHORIZATION, CONTENT_ENCODING, CONTENT_TYPE, USER_AGENT};

use crate::{
    adapter_http::{
        HttpAdapterConfigError, HttpEndpointConfig, HttpTransport, validate_reserved_headers,
    },
    domain::{CanonicalUrl, RedirectDefinition, RedirectSource, RedirectSourceError},
};

use super::record::RawRedirect;

const USER_AGENT_VALUE: &str = concat!("Compactor/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct HttpRedirectSource {
    transport: HttpTransport,
    endpoint: HttpEndpointConfig,
    max_response_bytes: usize,
}

impl HttpRedirectSource {
    pub fn new(
        transport: HttpTransport,
        endpoint: HttpEndpointConfig,
        max_response_bytes: usize,
    ) -> Result<Self, HttpAdapterConfigError> {
        if max_response_bytes == 0 {
            return Err(HttpAdapterConfigError::new(
                "HTTP source response limit must be greater than zero",
            ));
        }
        validate_reserved_headers(&endpoint.static_headers, &[ACCEPT, USER_AGENT])?;
        if endpoint.url.query_pairs().any(|(name, _)| name == "url") {
            return Err(HttpAdapterConfigError::new(
                "HTTP source endpoint must not contain a url query parameter",
            ));
        }
        Ok(Self {
            transport,
            endpoint,
            max_response_bytes,
        })
    }

    pub fn endpoint_config(&self) -> &HttpEndpointConfig {
        &self.endpoint
    }
}

#[async_trait]
impl RedirectSource for HttpRedirectSource {
    async fn resolve(
        &self,
        canonical_url: &CanonicalUrl,
    ) -> Result<Option<RedirectDefinition>, RedirectSourceError> {
        let mut url = self.endpoint.url.clone();
        url.query_pairs_mut()
            .append_pair("url", canonical_url.as_str());
        let mut request = self
            .transport
            .client
            .get(url)
            .timeout(self.endpoint.request_timeout)
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, USER_AGENT_VALUE)
            .headers(self.endpoint.static_headers.clone());
        if let Some(authorization) = &self.endpoint.bearer_token {
            request = request.header(AUTHORIZATION, authorization);
        }
        let mut response = request.send().await.map_err(source_transport_error)?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status != reqwest::StatusCode::OK {
            return Err(RedirectSourceError::new(format!(
                "HTTP source returned unsupported status {}",
                status.as_u16()
            )));
        }
        validate_content_headers(response.headers())?;

        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(source_transport_error)? {
            if body.len().saturating_add(chunk.len()) > self.max_response_bytes {
                return Err(RedirectSourceError::new(
                    "HTTP source response exceeded the configured size limit",
                ));
            }
            body.extend_from_slice(&chunk);
        }
        let raw: RawRedirect = serde_json::from_slice(&body).map_err(|error| {
            RedirectSourceError::new(format!("invalid HTTP source JSON: {error}"))
        })?;
        let definition = raw.into_definition()?;
        if definition.canonical_url != *canonical_url {
            return Err(RedirectSourceError::new(
                "HTTP source response canonical URL does not match the requested key",
            ));
        }
        Ok(Some(definition))
    }
}

fn validate_content_headers(headers: &http::HeaderMap) -> Result<(), RedirectSourceError> {
    if let Some(encoding) = headers.get(CONTENT_ENCODING) {
        if encoding
            .to_str()
            .map_or(true, |value| !value.eq_ignore_ascii_case("identity"))
        {
            return Err(RedirectSourceError::new(
                "HTTP source response uses an unsupported content encoding",
            ));
        }
    }
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .map(str::to_ascii_lowercase);
    let supported = content_type.as_deref().is_some_and(|value| {
        value == "application/json"
            || (value.starts_with("application/") && value.ends_with("+json"))
    });
    if !supported {
        return Err(RedirectSourceError::new(
            "HTTP source response has an unsupported content type",
        ));
    }
    Ok(())
}

fn source_transport_error(error: reqwest::Error) -> RedirectSourceError {
    let category = if error.is_timeout() {
        "timeout"
    } else {
        "transport"
    };
    RedirectSourceError::new(format!("HTTP source {category} error"))
}
