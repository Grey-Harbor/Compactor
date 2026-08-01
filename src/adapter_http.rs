use std::{collections::BTreeMap, fmt, str::FromStr, time::Duration};

use http::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, redirect::Policy};
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, Copy)]
pub struct HttpTransportConfig {
    pub connect_timeout: Duration,
}

impl HttpTransportConfig {
    pub fn new(connect_timeout: Duration) -> Result<Self, HttpAdapterConfigError> {
        if connect_timeout.is_zero() {
            return Err(HttpAdapterConfigError::new(
                "HTTP connect timeout must be greater than zero",
            ));
        }
        Ok(Self { connect_timeout })
    }
}

#[derive(Clone)]
pub struct HttpEndpointConfig {
    pub(crate) url: Url,
    pub(crate) request_timeout: Duration,
    pub(crate) static_headers: HeaderMap,
    pub(crate) bearer_token: Option<HeaderValue>,
}

impl fmt::Debug for HttpEndpointConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpEndpointConfig")
            .field("scheme", &self.url.scheme())
            .field("host", &self.url.host_str())
            .field("request_timeout", &self.request_timeout)
            .field("static_header_count", &self.static_headers.len())
            .field(
                "bearer_token",
                &self.bearer_token.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

impl HttpEndpointConfig {
    pub fn new(
        url: impl AsRef<str>,
        request_timeout: Duration,
        static_headers: BTreeMap<String, String>,
        bearer_token: Option<String>,
    ) -> Result<Self, HttpAdapterConfigError> {
        let url = Url::parse(url.as_ref())
            .map_err(|_| HttpAdapterConfigError::new("HTTP endpoint URL is invalid"))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(HttpAdapterConfigError::new(
                "HTTP endpoint URL scheme must be http or https",
            ));
        }
        if url.host().is_none() {
            return Err(HttpAdapterConfigError::new(
                "HTTP endpoint URL must include a host",
            ));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(HttpAdapterConfigError::new(
                "HTTP endpoint URL must not contain user information",
            ));
        }
        if url.fragment().is_some() {
            return Err(HttpAdapterConfigError::new(
                "HTTP endpoint URL must not contain a fragment",
            ));
        }
        if request_timeout.is_zero() {
            return Err(HttpAdapterConfigError::new(
                "HTTP request timeout must be greater than zero",
            ));
        }

        let mut headers = HeaderMap::new();
        for (name, value) in static_headers {
            let name = HeaderName::from_str(&name)
                .map_err(|_| HttpAdapterConfigError::new("HTTP static header name is invalid"))?;
            let value = HeaderValue::from_str(&value).map_err(|_| {
                HttpAdapterConfigError::new(format!(
                    "value for HTTP static header {name} is invalid"
                ))
            })?;
            headers.insert(name, value);
        }

        let bearer_token = bearer_token
            .map(|token| {
                if token.is_empty() {
                    return Err(HttpAdapterConfigError::new(
                        "HTTP bearer token must not be empty",
                    ));
                }
                HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| {
                    HttpAdapterConfigError::new("HTTP bearer token contains invalid characters")
                })
            })
            .transpose()?;
        if bearer_token.is_some() && headers.contains_key(http::header::AUTHORIZATION) {
            return Err(HttpAdapterConfigError::new(
                "HTTP static Authorization header conflicts with bearer authentication",
            ));
        }

        Ok(Self {
            url,
            request_timeout,
            static_headers: headers,
            bearer_token,
        })
    }

    pub fn uses_plaintext_http(&self) -> bool {
        self.url.scheme() == "http"
    }

    pub fn endpoint_origin(&self) -> String {
        let host = self.url.host_str().unwrap_or_default();
        match self.url.port() {
            Some(port) => format!("{}://{host}:{port}", self.url.scheme()),
            None => format!("{}://{host}", self.url.scheme()),
        }
    }
}

#[derive(Clone)]
pub struct HttpTransport {
    pub(crate) client: Client,
}

impl HttpTransport {
    pub fn new(config: HttpTransportConfig) -> Result<Self, HttpAdapterConfigError> {
        HttpTransportConfig::new(config.connect_timeout)?;
        let client = Client::builder()
            .connect_timeout(config.connect_timeout)
            .redirect(Policy::none())
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(32)
            .build()
            .map_err(|_| HttpAdapterConfigError::new("could not build HTTP transport"))?;
        Ok(Self { client })
    }
}

impl fmt::Debug for HttpTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HttpTransport")
    }
}

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct HttpAdapterConfigError {
    message: String,
}

impl HttpAdapterConfigError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub(crate) fn validate_reserved_headers(
    headers: &HeaderMap,
    reserved: &[HeaderName],
) -> Result<(), HttpAdapterConfigError> {
    const CONNECTION_HEADERS: &[HeaderName] = &[
        http::header::HOST,
        http::header::CONTENT_LENGTH,
        http::header::CONNECTION,
        http::header::TRANSFER_ENCODING,
        http::header::TE,
        http::header::TRAILER,
        http::header::UPGRADE,
        http::header::PROXY_AUTHORIZATION,
    ];
    if let Some(name) = CONNECTION_HEADERS
        .iter()
        .chain(reserved)
        .find(|name| headers.contains_key(*name))
    {
        return Err(HttpAdapterConfigError::new(format!(
            "HTTP static header {name} is controlled by Compactor"
        )));
    }
    Ok(())
}
