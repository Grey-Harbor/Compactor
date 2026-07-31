use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use axum::{
    body::{Body, to_bytes},
    extract::ConnectInfo,
};
use compactor::{
    AppState, CanonicalUrl, HeaderCaptureLimits, JsonRedirectSource, JsonlRedirectEventSink,
    ProxyConfig, RedirectCachePolicy, RedirectDefinition, RedirectEvent, RedirectEventSink,
    RedirectEventSinkError, RedirectId, RedirectRuntime, RedirectSource, RedirectSourceError,
    RedirectStatus, ResponseHeaders, router,
};
use http::{Method, Request, StatusCode, header};
use tempfile::tempdir;
use tokio::sync::Mutex;
use tower::ServiceExt;

#[derive(Default)]
struct RecordingSink {
    events: Mutex<Vec<RedirectEvent>>,
}

#[async_trait]
impl RedirectEventSink for RecordingSink {
    async fn emit(&self, event: &RedirectEvent) -> Result<(), RedirectEventSinkError> {
        self.events.lock().await.push(event.clone());
        Ok(())
    }
}

struct FailingSink;

#[async_trait]
impl RedirectEventSink for FailingSink {
    async fn emit(&self, _event: &RedirectEvent) -> Result<(), RedirectEventSinkError> {
        Err(RedirectEventSinkError::new("disk full"))
    }
}

#[derive(Default)]
struct DelayedRecordingSink {
    events: Mutex<Vec<RedirectEvent>>,
}

#[async_trait]
impl RedirectEventSink for DelayedRecordingSink {
    async fn emit(&self, event: &RedirectEvent) -> Result<(), RedirectEventSinkError> {
        tokio::time::sleep(Duration::from_millis(150)).await;
        self.events.lock().await.push(event.clone());
        Ok(())
    }
}

struct FailingSource;

#[async_trait]
impl RedirectSource for FailingSource {
    async fn resolve(
        &self,
        _canonical_url: &CanonicalUrl,
    ) -> Result<Option<compactor::RedirectDefinition>, RedirectSourceError> {
        Err(RedirectSourceError::new("source unavailable"))
    }
}

struct StaticSource {
    redirects: HashMap<CanonicalUrl, RedirectDefinition>,
}

struct SequenceSource {
    responses: Mutex<VecDeque<Result<Option<RedirectDefinition>, RedirectSourceError>>>,
}

#[async_trait]
impl RedirectSource for SequenceSource {
    async fn resolve(
        &self,
        _canonical_url: &CanonicalUrl,
    ) -> Result<Option<RedirectDefinition>, RedirectSourceError> {
        self.responses
            .lock()
            .await
            .pop_front()
            .expect("test source has a scripted response")
    }
}

#[async_trait]
impl RedirectSource for StaticSource {
    async fn resolve(
        &self,
        canonical_url: &CanonicalUrl,
    ) -> Result<Option<RedirectDefinition>, RedirectSourceError> {
        Ok(self.redirects.get(canonical_url).cloned())
    }
}

fn definition(
    id: &str,
    canonical_url: &str,
    redirect_url: &str,
    status_code: u16,
    response_headers: BTreeMap<String, String>,
) -> RedirectDefinition {
    RedirectDefinition {
        id: RedirectId::new(id).unwrap(),
        canonical_url: CanonicalUrl::parse(canonical_url).unwrap(),
        redirect_url: redirect_url.parse().unwrap(),
        status_code: RedirectStatus::try_from(status_code).unwrap(),
        response_headers: ResponseHeaders::try_from_strings(response_headers).unwrap(),
    }
}

fn static_source(definitions: Vec<RedirectDefinition>) -> Arc<dyn RedirectSource> {
    Arc::new(StaticSource {
        redirects: definitions
            .into_iter()
            .map(|definition| (definition.canonical_url.clone(), definition))
            .collect(),
    })
}

fn source() -> Arc<dyn RedirectSource> {
    static_source(vec![
        definition(
            "docs",
            "https://go.example/docs",
            "https://docs.example/current?fixed=1",
            308,
            BTreeMap::from([("Cache-Control".to_owned(), "public, max-age=300".to_owned())]),
        ),
        definition(
            "temporary",
            "https://go.example/temporary",
            "https://example.com/",
            302,
            BTreeMap::new(),
        ),
    ])
}

fn runtime(source: Arc<dyn RedirectSource>) -> Arc<RedirectRuntime> {
    Arc::new(RedirectRuntime::new(
        source,
        RedirectCachePolicy::new(Duration::from_secs(300), NonZeroUsize::new(100).unwrap()),
    ))
}

fn state(source: Arc<dyn RedirectSource>, sink: Arc<dyn RedirectEventSink>) -> AppState {
    AppState::new(
        runtime(source),
        sink,
        ProxyConfig {
            trusted_proxies: vec!["127.0.0.0/8".parse().unwrap()],
            record_client_addresses: true,
        },
        HeaderCaptureLimits {
            value_bytes: 32,
            total_bytes: 64,
        },
    )
}

fn request(method: Method, target: &str) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(target)
        .header("host", "internal:8080")
        .header("forwarded", "for=203.0.113.8;proto=https;host=go.example")
        .header("user-agent", "integration-test")
        .header("accept", "text/html")
        .header("authorization", "secret")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:54321".parse::<std::net::SocketAddr>().unwrap(),
    ));
    request
}

#[tokio::test]
async fn redirects_and_emits_a_sanitized_event() {
    let sink = Arc::new(RecordingSink::default());
    let app = router(state(source(), sink.clone()));
    let response = app
        .oneshot(request(Method::GET, "/docs?source=home&source=nav"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        response.headers()[header::LOCATION],
        "https://docs.example/current?fixed=1&source=home&source=nav"
    );
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "public, max-age=300"
    );

    let events = sink.events.lock().await;
    let event = &events[0];
    assert_eq!(event.redirect_id.as_ref().unwrap().as_str(), "docs");
    assert_eq!(event.client.address.as_deref(), Some("203.0.113.8"));
    assert_eq!(event.request.scheme, "https");
    assert_eq!(event.request.host, "go.example");
    assert_eq!(
        event.request.query.as_deref(),
        Some("source=home&source=nav")
    );
    assert!(event.request.headers.contains_key("accept"));
    assert!(!event.request.headers.contains_key("authorization"));
    assert_eq!(event.response.status_code, 308);
    assert!(event.duration_ms >= 0.0);
}

#[tokio::test]
async fn head_not_found_and_unsupported_methods_emit_expected_events() {
    let sink = Arc::new(RecordingSink::default());
    let app = router(state(source(), sink.clone()));

    let head = app
        .clone()
        .oneshot(request(Method::HEAD, "/temporary"))
        .await
        .unwrap();
    assert_eq!(head.status(), StatusCode::FOUND);
    assert_eq!(to_bytes(head.into_body(), 1024).await.unwrap().len(), 0);

    let missing = app
        .clone()
        .oneshot(request(Method::GET, "/missing"))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    let post = app.oneshot(request(Method::POST, "/docs")).await.unwrap();
    assert_eq!(post.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(post.headers()[header::ALLOW], "GET, HEAD");

    let events = sink.events.lock().await;
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].outcome, compactor::RedirectOutcome::Redirected);
    assert_eq!(events[1].outcome, compactor::RedirectOutcome::NotFound);
    assert_eq!(
        events[2].outcome,
        compactor::RedirectOutcome::InvalidRequest
    );
}

#[tokio::test]
async fn source_errors_become_500_events() {
    let sink = Arc::new(RecordingSink::default());
    let app = router(state(Arc::new(FailingSource), sink.clone()));
    let response = app
        .oneshot(request(Method::GET, "/anything"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let events = sink.events.lock().await;
    assert_eq!(events[0].outcome, compactor::RedirectOutcome::SourceError);
}

#[tokio::test(start_paused = true)]
async fn stale_refresh_errors_keep_successful_redirect_events() {
    let source: Arc<dyn RedirectSource> = Arc::new(SequenceSource {
        responses: Mutex::new(
            vec![
                Ok(Some(definition(
                    "docs",
                    "https://go.example/docs",
                    "https://docs.example/current",
                    308,
                    BTreeMap::new(),
                ))),
                Err(RedirectSourceError::new("source unavailable")),
            ]
            .into(),
        ),
    });
    let runtime = Arc::new(RedirectRuntime::new(
        source,
        RedirectCachePolicy::new(Duration::from_secs(1), NonZeroUsize::new(10).unwrap()),
    ));
    let sink = Arc::new(RecordingSink::default());
    let app = router(AppState::new(
        runtime,
        sink.clone(),
        ProxyConfig {
            trusted_proxies: vec!["127.0.0.0/8".parse().unwrap()],
            record_client_addresses: true,
        },
        HeaderCaptureLimits {
            value_bytes: 32,
            total_bytes: 64,
        },
    ));

    assert_eq!(
        app.clone()
            .oneshot(request(Method::GET, "/docs"))
            .await
            .unwrap()
            .status(),
        StatusCode::PERMANENT_REDIRECT
    );
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(
        app.oneshot(request(Method::GET, "/docs"))
            .await
            .unwrap()
            .status(),
        StatusCode::PERMANENT_REDIRECT
    );
    let events = sink.events.lock().await;
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|event| event.outcome == compactor::RedirectOutcome::Redirected)
    );
}

#[tokio::test]
async fn sink_failure_does_not_replace_redirect() {
    let app = router(state(source(), Arc::new(FailingSink)));
    let response = app.oneshot(request(Method::GET, "/docs")).await.unwrap();
    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
}

#[tokio::test]
async fn health_is_reserved_and_event_free() {
    let sink = Arc::new(RecordingSink::default());
    let app = router(state(source(), sink.clone()));
    let get_response = app
        .clone()
        .oneshot(request(Method::GET, "/healthz"))
        .await
        .unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(get_response.into_body(), 1024).await.unwrap(),
        "ok\n"
    );
    let head_response = app
        .clone()
        .oneshot(request(Method::HEAD, "/healthz"))
        .await
        .unwrap();
    assert_eq!(head_response.status(), StatusCode::OK);
    assert!(
        to_bytes(head_response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(sink.events.lock().await.is_empty());

    let post_response = app
        .oneshot(request(Method::POST, "/healthz"))
        .await
        .unwrap();
    assert_eq!(post_response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(post_response.headers()[header::ALLOW], "GET, HEAD");
    let events = sink.events.lock().await;
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].outcome,
        compactor::RedirectOutcome::InvalidRequest
    );
}

#[tokio::test]
async fn supports_every_redirect_status() {
    for status in [301, 302, 303, 307, 308] {
        let source = static_source(vec![definition(
            &format!("status-{status}"),
            "http://status.example/test",
            "https://destination.example/",
            status,
            BTreeMap::new(),
        )]);
        let sink = Arc::new(RecordingSink::default());
        let app = router(AppState::new(
            runtime(source),
            sink,
            ProxyConfig {
                trusted_proxies: Vec::new(),
                record_client_addresses: false,
            },
            HeaderCaptureLimits {
                value_bytes: 32,
                total_bytes: 64,
            },
        ));
        let mut direct = Request::builder()
            .uri("/test")
            .header("host", "status.example")
            .body(Body::empty())
            .unwrap();
        direct.extensions_mut().insert(ConnectInfo(
            "192.0.2.4:1234".parse::<std::net::SocketAddr>().unwrap(),
        ));
        let response = app.oneshot(direct).await.unwrap();
        assert_eq!(response.status().as_u16(), status);
    }
}

#[tokio::test]
async fn malformed_trusted_forwarding_is_invalid_and_address_can_be_omitted() {
    let sink = Arc::new(RecordingSink::default());
    let app = router(AppState::new(
        runtime(source()),
        sink.clone(),
        ProxyConfig {
            trusted_proxies: vec!["127.0.0.0/8".parse().unwrap()],
            record_client_addresses: false,
        },
        HeaderCaptureLimits {
            value_bytes: 32,
            total_bytes: 64,
        },
    ));
    let mut malformed = request(Method::GET, "/docs");
    malformed.headers_mut().insert(
        "forwarded",
        "for=not-an-ip;proto=https;host=go.example".parse().unwrap(),
    );
    let response = app.oneshot(malformed).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let events = sink.events.lock().await;
    assert_eq!(
        events[0].outcome,
        compactor::RedirectOutcome::InvalidRequest
    );
    assert!(events[0].client.address.is_none());
    assert_ne!(
        events[0].event_id.to_string(),
        events[0]
            .redirect_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default()
    );
    let serialized = serde_json::to_value(&events[0]).unwrap();
    assert!(serialized["occurred_at"].as_str().unwrap().ends_with('Z'));
}

#[tokio::test]
async fn lookup_uses_the_public_host_and_preserves_path_identity() {
    let source = static_source(vec![
        definition(
            "one",
            "http://one.example/a/../b",
            "https://destination.example/one",
            302,
            BTreeMap::new(),
        ),
        definition(
            "two",
            "http://two.example/a/../b",
            "https://destination.example/two",
            302,
            BTreeMap::new(),
        ),
        definition(
            "encoded",
            "http://one.example/%2e%2e/b",
            "https://destination.example/encoded",
            302,
            BTreeMap::new(),
        ),
    ]);
    let sink = Arc::new(RecordingSink::default());
    let app = router(AppState::new(
        runtime(source),
        sink,
        ProxyConfig {
            trusted_proxies: Vec::new(),
            record_client_addresses: true,
        },
        HeaderCaptureLimits {
            value_bytes: 32,
            total_bytes: 64,
        },
    ));

    for (host, path, location) in [
        ("one.example", "/a/../b", "https://destination.example/one"),
        ("two.example", "/a/../b", "https://destination.example/two"),
        (
            "one.example",
            "/%2e%2e/b",
            "https://destination.example/encoded",
        ),
    ] {
        let mut direct = Request::builder()
            .uri(path)
            .header("host", host)
            .body(Body::empty())
            .unwrap();
        direct.extensions_mut().insert(ConnectInfo(
            "192.0.2.4:1234".parse::<std::net::SocketAddr>().unwrap(),
        ));
        let response = app.clone().oneshot(direct).await.unwrap();
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers()[header::LOCATION], location);
    }
}

#[tokio::test]
async fn missing_host_is_rejected_before_source_lookup() {
    let sink = Arc::new(RecordingSink::default());
    let app = router(state(source(), sink.clone()));
    let mut missing_host = Request::builder().uri("/docs").body(Body::empty()).unwrap();
    missing_host.extensions_mut().insert(ConnectInfo(
        "192.0.2.4:1234".parse::<std::net::SocketAddr>().unwrap(),
    ));
    let response = app.oneshot(missing_host).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        sink.events.lock().await[0].outcome,
        compactor::RedirectOutcome::InvalidRequest
    );
}

#[tokio::test]
async fn event_duration_excludes_sink_latency() {
    let sink = Arc::new(DelayedRecordingSink::default());
    let app = router(state(source(), sink.clone()));
    let response = app.oneshot(request(Method::GET, "/docs")).await.unwrap();
    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    let events = sink.events.lock().await;
    assert!(
        events[0].duration_ms < 100.0,
        "duration included sink delay: {}ms",
        events[0].duration_ms
    );
}

#[tokio::test]
async fn json_source_http_service_and_jsonl_sink_work_together() {
    let directory = tempdir().unwrap();
    let redirects_path = directory.path().join("redirects.json");
    let events_path = directory.path().join("events.jsonl");
    std::fs::write(
        &redirects_path,
        r#"{"redirects":[{
            "id":"docs",
            "canonical_url":"https://go.example/docs",
            "redirect_url":"https://docs.example/current",
            "status_code":308,
            "response_headers":{}
        }]}"#,
    )
    .unwrap();
    let source: Arc<dyn RedirectSource> =
        Arc::new(JsonRedirectSource::open(&redirects_path).await.unwrap());
    let sink = Arc::new(JsonlRedirectEventSink::open(&events_path).await.unwrap());
    let app = router(state(source, sink.clone()));

    assert_eq!(
        app.clone()
            .oneshot(request(Method::GET, "/docs"))
            .await
            .unwrap()
            .status(),
        StatusCode::PERMANENT_REDIRECT
    );
    assert_eq!(
        app.clone()
            .oneshot(request(Method::GET, "/missing"))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let mut invalid = request(Method::GET, "/docs");
    invalid.headers_mut().insert(
        "forwarded",
        "for=invalid;proto=https;host=go.example".parse().unwrap(),
    );
    assert_eq!(
        app.oneshot(invalid).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );

    let failing_app = router(state(Arc::new(FailingSource), sink));
    assert_eq!(
        failing_app
            .oneshot(request(Method::GET, "/docs"))
            .await
            .unwrap()
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );

    let contents = tokio::fs::read_to_string(events_path).await.unwrap();
    let events: Vec<RedirectEvent> = contents
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.len(), 4);
    assert_eq!(
        events.iter().map(|event| event.outcome).collect::<Vec<_>>(),
        [
            compactor::RedirectOutcome::Redirected,
            compactor::RedirectOutcome::NotFound,
            compactor::RedirectOutcome::InvalidRequest,
            compactor::RedirectOutcome::SourceError,
        ]
    );
}
