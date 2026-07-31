use async_trait::async_trait;
use http::header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT};

use crate::{
    adapter_http::{
        HttpAdapterConfigError, HttpEndpointConfig, HttpTransport, validate_reserved_headers,
    },
    domain::{RedirectEvent, RedirectEventSink, RedirectEventSinkError},
};

const USER_AGENT_VALUE: &str = concat!("Compactor/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct HttpRedirectEventSink {
    transport: HttpTransport,
    endpoint: HttpEndpointConfig,
}

impl HttpRedirectEventSink {
    pub fn new(
        transport: HttpTransport,
        endpoint: HttpEndpointConfig,
    ) -> Result<Self, HttpAdapterConfigError> {
        validate_reserved_headers(&endpoint.static_headers, &[CONTENT_TYPE, USER_AGENT])?;
        Ok(Self {
            transport,
            endpoint,
        })
    }

    pub fn endpoint_config(&self) -> &HttpEndpointConfig {
        &self.endpoint
    }
}

#[async_trait]
impl RedirectEventSink for HttpRedirectEventSink {
    async fn emit(&self, event: &RedirectEvent) -> Result<(), RedirectEventSinkError> {
        let body = serde_json::to_vec(event)
            .map_err(|_| RedirectEventSinkError::new("could not serialize redirect event"))?;
        let mut request = self
            .transport
            .client
            .post(self.endpoint.url.clone())
            .timeout(self.endpoint.request_timeout)
            .header(CONTENT_TYPE, "application/json")
            .header(USER_AGENT, USER_AGENT_VALUE)
            .headers(self.endpoint.static_headers.clone())
            .body(body);
        if let Some(authorization) = &self.endpoint.bearer_token {
            request = request.header(AUTHORIZATION, authorization);
        }
        let response = request.send().await.map_err(|error| {
            let category = if error.is_timeout() {
                "timeout"
            } else {
                "transport"
            };
            RedirectEventSinkError::new(format!("HTTP event sink {category} error"))
        })?;
        if !response.status().is_success() {
            return Err(RedirectEventSinkError::new(format!(
                "HTTP event sink returned unsuccessful status {}",
                response.status().as_u16()
            )));
        }
        Ok(())
    }
}
