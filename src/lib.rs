mod adapter_http;
pub mod config;
pub mod domain;
pub mod http;
pub mod runtime;
pub mod sink;
pub mod source;

pub use adapter_http::{
    HttpAdapterConfigError, HttpEndpointConfig, HttpTransport, HttpTransportConfig,
};
pub use config::{Config, RedirectEventSinkConfig, RedirectSourceConfig};
pub use domain::{
    CanonicalUrl, ClientInfo, EventId, RedirectDefinition, RedirectEvent, RedirectEventSink,
    RedirectEventSinkError, RedirectId, RedirectOutcome, RedirectSource, RedirectSourceError,
    RedirectStatus, RequestInfo, ResponseHeaders, ResponseInfo,
};
pub use http::{AppState, HeaderCaptureLimits, ProxyConfig, router};
pub use runtime::{RedirectCachePolicy, RedirectRuntime};
pub use sink::{HttpRedirectEventSink, JsonlRedirectEventSink};
pub use source::{HttpRedirectSource, JsonRedirectSource};
