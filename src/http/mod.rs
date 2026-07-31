use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Instant,
};

use ::http::{
    HeaderMap, HeaderValue, Method, Request, Response, StatusCode, Uri, Version,
    header::{ALLOW, HOST, LOCATION, USER_AGENT},
    uri::Authority,
};
use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, State},
    routing::any,
};
use chrono::Utc;
use ipnet::IpNet;
use tracing::error;
use url::Url;

use crate::domain::{
    CanonicalUrl, ClientInfo, EventId, RedirectEvent, RedirectEventSink, RedirectId,
    RedirectOutcome, RequestInfo, ResponseInfo,
};
use crate::runtime::RedirectRuntime;

#[derive(Debug, Clone, Copy)]
pub struct HeaderCaptureLimits {
    pub value_bytes: usize,
    pub total_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub trusted_proxies: Vec<IpNet>,
    pub record_client_addresses: bool,
}

impl ProxyConfig {
    fn is_trusted(&self, address: IpAddr) -> bool {
        self.trusted_proxies
            .iter()
            .any(|network| network.contains(&address))
    }
}

#[derive(Clone)]
pub struct AppState {
    runtime: Arc<RedirectRuntime>,
    sink: Arc<dyn RedirectEventSink>,
    proxy: ProxyConfig,
    header_limits: HeaderCaptureLimits,
}

impl AppState {
    pub fn new(
        runtime: Arc<RedirectRuntime>,
        sink: Arc<dyn RedirectEventSink>,
        proxy: ProxyConfig,
        header_limits: HeaderCaptureLimits,
    ) -> Self {
        Self {
            runtime,
            sink,
            proxy,
            header_limits,
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", any(handle_healthz))
        .fallback(handle_request)
        .with_state(state)
}

async fn handle_healthz(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request<Body>,
) -> Response<Body> {
    match *request.method() {
        Method::GET => Response::builder()
            .status(StatusCode::OK)
            .body(Body::from("ok\n"))
            .expect("static health response is valid"),
        Method::HEAD => Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .expect("static health response is valid"),
        _ => process_request(state, peer, request).await,
    }
}

async fn handle_request(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request<Body>,
) -> Response<Body> {
    process_request(state, peer, request).await
}

async fn process_request(
    state: AppState,
    peer: SocketAddr,
    request: Request<Body>,
) -> Response<Body> {
    let started = Instant::now();
    let peer = Some(peer);
    let method = request.method().clone();
    let is_head = method == Method::HEAD;
    let uri = request.uri().clone();
    let version = request.version();
    let headers = request.headers();

    let captured_headers = capture_headers(headers, state.header_limits);
    let user_agent = header_text(headers, USER_AGENT)
        .map(|value| truncate_utf8(value, state.header_limits.value_bytes));
    let direct_scheme = uri.scheme_str().unwrap_or("http").to_ascii_lowercase();
    let direct_host = uri
        .authority()
        .map(ToString::to_string)
        .or_else(|| header_text(headers, HOST).map(str::to_owned))
        .unwrap_or_default();

    let resolved =
        resolve_request_metadata(&direct_scheme, &direct_host, headers, peer, &state.proxy);
    let (scheme, host, client_address, metadata_valid) = match resolved {
        Ok(metadata) => (
            metadata.scheme,
            metadata.host,
            metadata.client_address,
            true,
        ),
        Err(message) => {
            error!(error = %message, "invalid request metadata");
            (
                direct_scheme,
                direct_host,
                peer.map(|value| value.ip()),
                false,
            )
        }
    };
    let client_address = state
        .proxy
        .record_client_addresses
        .then(|| client_address.map(|value| value.to_string()))
        .flatten();
    let request_info = RequestInfo {
        method: method.to_string(),
        scheme: scheme.clone(),
        host: host.clone(),
        path: uri.path().to_owned(),
        query: uri.query().map(str::to_owned),
        protocol: protocol_name(version).to_owned(),
        headers: captured_headers,
    };
    let client = ClientInfo {
        address: client_address,
        user_agent,
    };

    let result = if !metadata_valid {
        RequestResult::error(StatusCode::BAD_REQUEST, RedirectOutcome::InvalidRequest)
    } else if method != Method::GET && method != Method::HEAD {
        RequestResult::method_not_allowed()
    } else {
        resolve_redirect(&state, &scheme, &host, &uri).await
    };

    let event = RedirectEvent {
        event_id: EventId::generate(),
        redirect_id: result.redirect_id.clone(),
        occurred_at: Utc::now(),
        duration_ms: started.elapsed().as_secs_f64() * 1000.0,
        outcome: result.outcome,
        client,
        request: request_info,
        response: ResponseInfo {
            status_code: result.status.as_u16(),
            location: result.location.clone(),
        },
    };
    if let Err(error) = state.sink.emit(&event).await {
        error!(
            event_id = %event.event_id,
            error = %error,
            "could not persist redirect event"
        );
    }

    result.into_response(is_head)
}

async fn resolve_redirect(state: &AppState, scheme: &str, host: &str, uri: &Uri) -> RequestResult {
    if !matches!(scheme, "http" | "https") || Authority::from_str(host).is_err() {
        return RequestResult::error(StatusCode::BAD_REQUEST, RedirectOutcome::InvalidRequest);
    }
    let canonical = match CanonicalUrl::parse(&format!("{scheme}://{host}{}", uri.path())) {
        Ok(value) => value,
        Err(_) => {
            return RequestResult::error(StatusCode::BAD_REQUEST, RedirectOutcome::InvalidRequest);
        }
    };

    match state.runtime.resolve(&canonical).await {
        Ok(Some(definition)) => {
            let mut destination = definition.redirect_url.clone();
            append_query(&mut destination, uri.query());
            RequestResult {
                status: StatusCode::from_u16(definition.status_code.as_u16())
                    .expect("redirect status is validated"),
                outcome: RedirectOutcome::Redirected,
                redirect_id: Some(definition.id),
                location: Some(destination),
                response_headers: definition
                    .response_headers
                    .iter()
                    .map(|(name, value)| (name.clone(), value.clone()))
                    .collect(),
                allow: false,
            }
        }
        Ok(None) => RequestResult::error(StatusCode::NOT_FOUND, RedirectOutcome::NotFound),
        Err(error) => {
            error!(error = %error, "redirect source lookup failed");
            RequestResult::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                RedirectOutcome::SourceError,
            )
        }
    }
}

fn append_query(destination: &mut Url, incoming: Option<&str>) {
    let Some(incoming) = incoming else {
        return;
    };
    let combined = match destination.query() {
        Some(existing) if !existing.is_empty() && !incoming.is_empty() => {
            format!("{existing}&{incoming}")
        }
        Some(existing) if incoming.is_empty() => existing.to_owned(),
        _ => incoming.to_owned(),
    };
    destination.set_query(Some(&combined));
}

struct RequestResult {
    status: StatusCode,
    outcome: RedirectOutcome,
    redirect_id: Option<RedirectId>,
    location: Option<Url>,
    response_headers: Vec<(::http::HeaderName, HeaderValue)>,
    allow: bool,
}

impl RequestResult {
    fn error(status: StatusCode, outcome: RedirectOutcome) -> Self {
        Self {
            status,
            outcome,
            redirect_id: None,
            location: None,
            response_headers: Vec::new(),
            allow: false,
        }
    }

    fn method_not_allowed() -> Self {
        Self {
            allow: true,
            ..Self::error(
                StatusCode::METHOD_NOT_ALLOWED,
                RedirectOutcome::InvalidRequest,
            )
        }
    }

    fn into_response(self, _is_head: bool) -> Response<Body> {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = self.status;
        for (name, value) in self.response_headers {
            response.headers_mut().insert(name, value);
        }
        if let Some(location) = self.location {
            match HeaderValue::from_str(location.as_str()) {
                Ok(value) => {
                    response.headers_mut().insert(LOCATION, value);
                }
                Err(error) => {
                    error!(error = %error, "validated destination could not become Location");
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                }
            }
        }
        if self.allow {
            response
                .headers_mut()
                .insert(ALLOW, HeaderValue::from_static("GET, HEAD"));
        }
        response
    }
}

struct ResolvedMetadata {
    scheme: String,
    host: String,
    client_address: Option<IpAddr>,
}

fn resolve_request_metadata(
    direct_scheme: &str,
    direct_host: &str,
    headers: &HeaderMap,
    peer: Option<SocketAddr>,
    config: &ProxyConfig,
) -> Result<ResolvedMetadata, String> {
    let trusted_peer = peer
        .map(|address| config.is_trusted(address.ip()))
        .unwrap_or(false);
    if !trusted_peer {
        return Ok(ResolvedMetadata {
            scheme: direct_scheme.to_owned(),
            host: direct_host.to_owned(),
            client_address: peer.map(|address| address.ip()),
        });
    }

    let forwarded = parse_forwarded(headers)?;
    let fallback_scheme = comma_header_first(headers, "x-forwarded-proto")?;
    let scheme = forwarded
        .as_ref()
        .and_then(|value| value.scheme.clone())
        .or(fallback_scheme)
        .unwrap_or_else(|| direct_scheme.to_owned())
        .to_ascii_lowercase();
    if !matches!(scheme.as_str(), "http" | "https") {
        return Err(format!("forwarded scheme {scheme:?} is not http or https"));
    }
    let fallback_host = comma_header_first(headers, "x-forwarded-host")?;
    let host = forwarded
        .as_ref()
        .and_then(|value| value.host.clone())
        .or(fallback_host)
        .unwrap_or_else(|| direct_host.to_owned());
    Authority::from_str(&host).map_err(|error| format!("invalid forwarded host: {error}"))?;

    let forwarded_addresses = match forwarded {
        Some(value) if !value.addresses.is_empty() => value.addresses,
        _ => parse_x_forwarded_for(headers)?,
    };
    let mut chain = forwarded_addresses;
    if let Some(peer) = peer {
        chain.push(peer.ip());
    }
    while chain
        .last()
        .is_some_and(|address| config.is_trusted(*address))
    {
        chain.pop();
    }

    Ok(ResolvedMetadata {
        scheme,
        host,
        client_address: chain
            .last()
            .copied()
            .or_else(|| peer.map(|value| value.ip())),
    })
}

#[derive(Default)]
struct ForwardedValues {
    scheme: Option<String>,
    host: Option<String>,
    addresses: Vec<IpAddr>,
}

fn parse_forwarded(headers: &HeaderMap) -> Result<Option<ForwardedValues>, String> {
    let Some(value) = headers.get("forwarded") else {
        return Ok(None);
    };
    let value = value
        .to_str()
        .map_err(|_| "Forwarded header is not valid text".to_owned())?;
    if value.trim().is_empty() {
        return Err("Forwarded header must not be empty".into());
    }
    let mut parsed = ForwardedValues::default();
    for element in value.split(',') {
        for parameter in element.split(';') {
            let Some((name, raw_value)) = parameter.trim().split_once('=') else {
                return Err("malformed Forwarded parameter".into());
            };
            let value = parse_forwarded_value(raw_value)?;
            match name.trim().to_ascii_lowercase().as_str() {
                "for" => parsed.addresses.push(parse_forwarded_ip(value)?),
                "proto" if parsed.scheme.is_none() => parsed.scheme = Some(value.to_owned()),
                "host" if parsed.host.is_none() => parsed.host = Some(value.to_owned()),
                _ => {}
            }
        }
    }
    Ok(Some(parsed))
}

fn parse_forwarded_value(raw_value: &str) -> Result<&str, String> {
    let value = raw_value.trim();
    let starts_quote = value.starts_with('"');
    let ends_quote = value.ends_with('"');
    if starts_quote != ends_quote || (!starts_quote && value.contains('"')) {
        return Err("malformed quoted Forwarded value".into());
    }
    if starts_quote && value.len() < 2 {
        return Err("malformed quoted Forwarded value".into());
    }
    let value = if starts_quote {
        &value[1..value.len() - 1]
    } else {
        value
    };
    if value.is_empty() || value.contains(['"', '\\']) {
        return Err("unsupported quoted Forwarded value".into());
    }
    Ok(value)
}

fn parse_x_forwarded_for(headers: &HeaderMap) -> Result<Vec<IpAddr>, String> {
    let Some(value) = headers.get("x-forwarded-for") else {
        return Ok(Vec::new());
    };
    value
        .to_str()
        .map_err(|_| "X-Forwarded-For header is not valid text".to_owned())?
        .split(',')
        .map(|entry| parse_forwarded_ip(entry.trim()))
        .collect()
}

fn parse_forwarded_ip(value: &str) -> Result<IpAddr, String> {
    let value = value.trim_matches('"');
    if value.eq_ignore_ascii_case("unknown") || value.starts_with('_') {
        return Err(format!("unsupported forwarded client identifier {value:?}"));
    }
    if let Ok(address) = value.parse::<IpAddr>() {
        return Ok(address);
    }
    if let Ok(address) = value.parse::<SocketAddr>() {
        return Ok(address.ip());
    }
    if value.starts_with('[') && value.ends_with(']') {
        return value[1..value.len() - 1]
            .parse()
            .map_err(|_| format!("invalid forwarded client address {value:?}"));
    }
    Err(format!("invalid forwarded client address {value:?}"))
}

fn comma_header_first(headers: &HeaderMap, name: &'static str) -> Result<Option<String>, String> {
    let Some(value) = headers.get(name) else {
        return Ok(None);
    };
    let value = value
        .to_str()
        .map_err(|_| format!("{name} header is not valid text"))?;
    let value = value
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match value {
        Some(value) => Ok(Some(value.to_owned())),
        None => Err(format!("{name} header must not be empty")),
    }
}

fn capture_headers(headers: &HeaderMap, limits: HeaderCaptureLimits) -> BTreeMap<String, String> {
    let mut captured = BTreeMap::new();
    let mut total = 0;
    for name in ["referer", "accept", "accept-language", "x-request-id"] {
        let Some(value) = header_text(headers, name) else {
            continue;
        };
        let value = truncate_utf8(value, limits.value_bytes);
        if total + value.len() > limits.total_bytes {
            continue;
        }
        total += value.len();
        captured.insert(name.to_owned(), value);
    }
    captured
}

fn truncate_utf8(value: &str, maximum: usize) -> String {
    if value.len() <= maximum {
        return value.to_owned();
    }
    let mut boundary = maximum;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value[..boundary].to_owned()
}

fn header_text<N>(headers: &HeaderMap, name: N) -> Option<&str>
where
    N: ::http::header::AsHeaderName,
{
    headers.get(name)?.to_str().ok()
}

fn protocol_name(version: Version) -> &'static str {
    match version {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2",
        Version::HTTP_3 => "HTTP/3",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_append_preserves_order_and_duplicates() {
        let mut url = Url::parse("https://example.com/path?fixed=1").unwrap();
        append_query(&mut url, Some("tag=a&tag=b"));
        assert_eq!(url.as_str(), "https://example.com/path?fixed=1&tag=a&tag=b");
    }

    #[test]
    fn captured_headers_are_allowlisted_and_bounded() {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("123456"));
        headers.insert("cookie", HeaderValue::from_static("secret"));
        headers.insert("x-request-id", HeaderValue::from_static("abcdef"));
        let captured = capture_headers(
            &headers,
            HeaderCaptureLimits {
                value_bytes: 4,
                total_bytes: 6,
            },
        );
        assert_eq!(captured.get("accept").unwrap(), "1234");
        assert!(!captured.contains_key("cookie"));
        assert!(!captured.contains_key("x-request-id"));
    }

    #[test]
    fn captured_headers_truncate_on_utf8_boundaries_and_skip_invalid_text() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "accept",
            HeaderValue::from_bytes(&[0xff]).expect("opaque header bytes are valid"),
        );
        let captured = capture_headers(
            &headers,
            HeaderCaptureLimits {
                value_bytes: 3,
                total_bytes: 8,
            },
        );
        assert!(!captured.contains_key("accept"));
        assert_eq!(truncate_utf8("éclair", 2), "é");
    }

    #[test]
    fn trusted_proxy_uses_forwarded_values_and_walks_chain() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "forwarded",
            HeaderValue::from_static("for=203.0.113.9;proto=https;host=GO.example, for=10.0.0.2"),
        );
        let config = ProxyConfig {
            trusted_proxies: vec!["10.0.0.0/8".parse().unwrap()],
            record_client_addresses: true,
        };
        let resolved = resolve_request_metadata(
            "http",
            "internal:8080",
            &headers,
            Some("10.0.0.3:5000".parse().unwrap()),
            &config,
        )
        .unwrap();
        assert_eq!(resolved.scheme, "https");
        assert_eq!(resolved.host, "GO.example");
        assert_eq!(
            resolved.client_address,
            Some("203.0.113.9".parse().unwrap())
        );
    }

    #[test]
    fn untrusted_peer_ignores_forwarding_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "forwarded",
            HeaderValue::from_static("for=203.0.113.9;proto=https;host=go.example"),
        );
        let config = ProxyConfig {
            trusted_proxies: Vec::new(),
            record_client_addresses: true,
        };
        let peer = "192.0.2.4:5000".parse().unwrap();
        let resolved =
            resolve_request_metadata("http", "direct.example", &headers, Some(peer), &config)
                .unwrap();
        assert_eq!(resolved.scheme, "http");
        assert_eq!(resolved.host, "direct.example");
        assert_eq!(resolved.client_address, Some(peer.ip()));
    }

    #[test]
    fn trusted_proxy_falls_back_to_x_forwarded_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.4, 10.0.0.2"),
        );
        headers.insert("x-forwarded-proto", HeaderValue::from_static("HTTPS"));
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("go.example:443"),
        );
        let config = ProxyConfig {
            trusted_proxies: vec!["10.0.0.0/8".parse().unwrap()],
            record_client_addresses: true,
        };
        let resolved = resolve_request_metadata(
            "http",
            "internal:8080",
            &headers,
            Some("10.0.0.3:5000".parse().unwrap()),
            &config,
        )
        .unwrap();
        assert_eq!(resolved.scheme, "https");
        assert_eq!(resolved.host, "go.example:443");
        assert_eq!(
            resolved.client_address,
            Some("203.0.113.4".parse().unwrap())
        );
    }

    #[test]
    fn trusted_proxy_rejects_malformed_fallback_and_forwarded_quotes() {
        let config = ProxyConfig {
            trusted_proxies: vec!["10.0.0.0/8".parse().unwrap()],
            record_client_addresses: true,
        };
        let peer = Some("10.0.0.3:5000".parse().unwrap());

        let mut invalid_fallback = HeaderMap::new();
        invalid_fallback.insert(
            "x-forwarded-host",
            HeaderValue::from_bytes(&[0xff]).unwrap(),
        );
        assert!(
            resolve_request_metadata("http", "internal:8080", &invalid_fallback, peer, &config)
                .is_err()
        );

        let mut invalid_forwarded = HeaderMap::new();
        invalid_forwarded.insert(
            "forwarded",
            HeaderValue::from_static("for=\"203.0.113.4;proto=https"),
        );
        assert!(
            resolve_request_metadata("http", "internal:8080", &invalid_forwarded, peer, &config)
                .is_err()
        );
    }
}
