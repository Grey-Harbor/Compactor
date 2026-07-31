use std::{
    collections::{BTreeMap, VecDeque},
    num::NonZeroUsize,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Query, Request},
    http::{HeaderMap, Response, StatusCode},
    routing::any,
};
use chrono::Utc;
use compactor::{
    CanonicalUrl, ClientInfo, EventId, HttpEndpointConfig, HttpRedirectEventSink,
    HttpRedirectSource, HttpTransport, HttpTransportConfig, RedirectCachePolicy, RedirectEvent,
    RedirectEventSink, RedirectOutcome, RedirectRuntime, RedirectSource, RequestInfo, ResponseInfo,
};
use tokio::{net::TcpListener, sync::Barrier, task::JoinHandle};

fn transport() -> HttpTransport {
    HttpTransport::new(HttpTransportConfig::new(Duration::from_millis(200)).unwrap()).unwrap()
}

fn endpoint(url: &str, timeout: Duration) -> HttpEndpointConfig {
    HttpEndpointConfig::new(url, timeout, BTreeMap::new(), None).unwrap()
}

fn definition(key: &str, id: &str) -> String {
    serde_json::json!({
        "id": id,
        "canonical_url": key,
        "redirect_url": format!("https://destination.example/{id}"),
        "status_code": 308,
        "response_headers": {"Cache-Control": "private"}
    })
    .to_string()
}

async fn serve(router: Router) -> (String, JoinHandle<Result<(), std::io::Error>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(axum::serve(listener, router).into_future());
    (format!("http://{address}"), task)
}

async fn wait_for_remaining_responses<T>(responses: &Arc<Mutex<VecDeque<T>>>, expected: usize) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while responses.lock().unwrap().len() != expected {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("mock response was consumed");
}

#[tokio::test]
async fn source_encodes_key_preserves_fixed_query_and_sends_owned_headers() {
    let seen = Arc::new(Mutex::new(None));
    let app = Router::new().fallback(any({
        let seen = Arc::clone(&seen);
        move |request: Request| {
            let seen = Arc::clone(&seen);
            async move {
                *seen.lock().unwrap() = Some((request.uri().clone(), request.headers().clone()));
                Response::builder()
                    .status(StatusCode::OK)
                    .header(
                        "content-type",
                        "application/vnd.compactor+json; charset=utf-8",
                    )
                    .body(Body::from(definition("https://go.example/a%20b", "one")))
                    .unwrap()
            }
        }
    }));
    let (base, _server) = serve(app).await;
    let mut headers = BTreeMap::new();
    headers.insert("X-Tenant".into(), "acme".into());
    let config = HttpEndpointConfig::new(
        format!("{base}/resolve?scope=published"),
        Duration::from_secs(1),
        headers,
        Some("secret".into()),
    )
    .unwrap();
    let source = HttpRedirectSource::new(transport(), config, 65_536).unwrap();
    let key = CanonicalUrl::parse("https://go.example/a%20b").unwrap();
    let result = source.resolve(&key).await.unwrap().unwrap();
    assert_eq!(result.id.as_str(), "one");

    let (uri, headers) = seen.lock().unwrap().take().unwrap();
    let parsed = url::Url::parse(&format!("{base}{uri}")).unwrap();
    assert_eq!(
        parsed.query_pairs().collect::<Vec<_>>(),
        vec![
            ("scope".into(), "published".into()),
            ("url".into(), key.as_str().into())
        ]
    );
    assert_eq!(headers["accept"], "application/json");
    assert_eq!(headers["user-agent"], "Compactor/0.2.0");
    assert_eq!(headers["x-tenant"], "acme");
    assert_eq!(headers["authorization"], "Bearer secret");
    assert!(!headers.contains_key("x-client-header"));
}

#[tokio::test]
async fn source_maps_status_content_and_contract_failures() {
    let responses = Arc::new(Mutex::new(VecDeque::from([
        (StatusCode::NOT_FOUND, None, "ignored".to_owned()),
        (StatusCode::NO_CONTENT, None, String::new()),
        (StatusCode::FOUND, None, String::new()),
        (StatusCode::OK, Some("text/plain"), "{}".to_owned()),
        (
            StatusCode::OK,
            Some("application/json"),
            "not json".to_owned(),
        ),
        (
            StatusCode::OK,
            Some("application/json"),
            definition("https://wrong.example/", "wrong"),
        ),
    ])));
    let app = Router::new().fallback(any({
        let responses = Arc::clone(&responses);
        move || {
            let (status, content_type, body) = responses.lock().unwrap().pop_front().unwrap();
            async move {
                let mut response = Response::builder().status(status);
                if let Some(content_type) = content_type {
                    response = response.header("content-type", content_type);
                }
                response.body(Body::from(body)).unwrap()
            }
        }
    }));
    let (base, _server) = serve(app).await;
    let source =
        HttpRedirectSource::new(transport(), endpoint(&base, Duration::from_secs(1)), 65_536)
            .unwrap();
    let key = CanonicalUrl::parse("https://go.example/").unwrap();
    assert!(source.resolve(&key).await.unwrap().is_none());
    for _ in 0..5 {
        assert!(source.resolve(&key).await.is_err());
    }
}

#[tokio::test]
async fn source_enforces_streamed_size_limit_and_total_timeout() {
    let app = Router::new()
        .route(
            "/large",
            any(|| async {
                Response::builder()
                    .header("content-type", "application/json")
                    .body(Body::from(vec![b'x'; 33]))
                    .unwrap()
            }),
        )
        .route(
            "/slow",
            any(|| async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Response::builder()
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap()
            }),
        );
    let (base, _server) = serve(app).await;
    let key = CanonicalUrl::parse("https://go.example/").unwrap();
    let large = HttpRedirectSource::new(
        transport(),
        endpoint(&format!("{base}/large"), Duration::from_secs(1)),
        32,
    )
    .unwrap();
    assert!(
        large
            .resolve(&key)
            .await
            .unwrap_err()
            .to_string()
            .contains("size limit")
    );
    let slow = HttpRedirectSource::new(
        transport(),
        endpoint(&format!("{base}/slow"), Duration::from_millis(20)),
        1024,
    )
    .unwrap();
    assert!(
        slow.resolve(&key)
            .await
            .unwrap_err()
            .to_string()
            .contains("timeout")
    );
}

#[tokio::test]
async fn source_supports_every_redirect_status_and_parallel_keys() {
    let barrier = Arc::new(Barrier::new(2));
    let statuses = Arc::new(Mutex::new(VecDeque::from([301, 302, 303, 307, 308])));
    let app = Router::new().fallback(any({
        let barrier = Arc::clone(&barrier);
        let statuses = Arc::clone(&statuses);
        move |Query(query): Query<BTreeMap<String, String>>| {
            let barrier = Arc::clone(&barrier);
            let status = statuses.lock().unwrap().pop_front().unwrap_or(308);
            async move {
                if query
                    .get("url")
                    .is_some_and(|url| url.ends_with("parallel"))
                {
                    barrier.wait().await;
                }
                let key = query.get("url").unwrap();
                Response::builder()
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "id": format!("redirect-{status}"),
                            "canonical_url": key,
                            "redirect_url": "https://destination.example/",
                            "status_code": status,
                            "response_headers": {}
                        })
                        .to_string(),
                    ))
                    .unwrap()
            }
        }
    }));
    let (base, _server) = serve(app).await;
    let source = Arc::new(
        HttpRedirectSource::new(transport(), endpoint(&base, Duration::from_secs(1)), 65_536)
            .unwrap(),
    );
    for expected in [301, 302, 303, 307, 308] {
        let key = CanonicalUrl::parse(&format!("https://go.example/status-{expected}")).unwrap();
        assert_eq!(
            source
                .resolve(&key)
                .await
                .unwrap()
                .unwrap()
                .status_code
                .as_u16(),
            expected
        );
    }
    let first = CanonicalUrl::parse("https://one.example/parallel").unwrap();
    let second = CanonicalUrl::parse("https://two.example/parallel").unwrap();
    let (first_result, second_result) =
        tokio::join!(source.resolve(&first), source.resolve(&second));
    assert!(first_result.unwrap().is_some());
    assert!(second_result.unwrap().is_some());
}

#[tokio::test]
async fn endpoint_and_adapter_validation_rejects_unsafe_configuration() {
    for url in [
        "ftp://example.com",
        "https://user@example.com",
        "https://example.com/#fragment",
    ] {
        assert!(
            HttpEndpointConfig::new(url, Duration::from_secs(1), BTreeMap::new(), None).is_err()
        );
    }
    let duplicate = endpoint(
        "https://example.com/resolve?url=one&url=two",
        Duration::from_secs(1),
    );
    assert!(HttpRedirectSource::new(transport(), duplicate, 100).is_err());
    let mut reserved = BTreeMap::new();
    reserved.insert("Accept".into(), "text/plain".into());
    let reserved = HttpEndpointConfig::new(
        "https://example.com",
        Duration::from_secs(1),
        reserved,
        None,
    )
    .unwrap();
    assert!(HttpRedirectSource::new(transport(), reserved, 100).is_err());
    let mut authorization = BTreeMap::new();
    authorization.insert("Authorization".into(), "custom".into());
    assert!(
        HttpEndpointConfig::new(
            "https://example.com",
            Duration::from_secs(1),
            authorization,
            Some("secret".into())
        )
        .is_err()
    );
}

fn event() -> RedirectEvent {
    RedirectEvent {
        event_id: EventId::generate(),
        redirect_id: None,
        occurred_at: Utc::now(),
        duration_ms: 2.5,
        outcome: RedirectOutcome::NotFound,
        client: ClientInfo {
            address: None,
            user_agent: None,
        },
        request: RequestInfo {
            method: "GET".into(),
            scheme: "https".into(),
            host: "go.example".into(),
            path: "/missing".into(),
            query: None,
            protocol: "HTTP/1.1".into(),
            headers: BTreeMap::new(),
        },
        response: ResponseInfo {
            status_code: 404,
            location: None,
        },
    }
}

#[tokio::test]
async fn sink_posts_the_exact_event_once_with_owned_headers() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new().fallback(any({
        let seen = Arc::clone(&seen);
        move |headers: HeaderMap, body: Bytes| {
            let seen = Arc::clone(&seen);
            async move {
                seen.lock().unwrap().push((headers, body));
                StatusCode::ACCEPTED
            }
        }
    }));
    let (base, _server) = serve(app).await;
    let mut headers = BTreeMap::new();
    headers.insert("X-Tenant".into(), "acme".into());
    let endpoint =
        HttpEndpointConfig::new(base, Duration::from_secs(1), headers, Some("secret".into()))
            .unwrap();
    let sink = HttpRedirectEventSink::new(transport(), endpoint).unwrap();
    let event = event();
    sink.emit(&event).await.unwrap();
    let captured = seen.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].0["content-type"], "application/json");
    assert_eq!(captured[0].0["user-agent"], "Compactor/0.2.0");
    assert_eq!(captured[0].0["authorization"], "Bearer secret");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&captured[0].1).unwrap(),
        serde_json::to_value(event).unwrap()
    );
}

#[tokio::test]
async fn sink_accepts_all_2xx_and_never_retries_failures() {
    let calls = Arc::new(Mutex::new(0));
    let statuses = Arc::new(Mutex::new(VecDeque::from([
        StatusCode::OK,
        StatusCode::NO_CONTENT,
        StatusCode::MULTI_STATUS,
        StatusCode::TOO_MANY_REQUESTS,
    ])));
    let app = Router::new().fallback(any({
        let calls = Arc::clone(&calls);
        let statuses = Arc::clone(&statuses);
        move || {
            *calls.lock().unwrap() += 1;
            let status = statuses.lock().unwrap().pop_front().unwrap();
            async move { status }
        }
    }));
    let (base, _server) = serve(app).await;
    let sink =
        HttpRedirectEventSink::new(transport(), endpoint(&base, Duration::from_secs(1))).unwrap();
    for _ in 0..3 {
        sink.emit(&event()).await.unwrap();
    }
    assert!(sink.emit(&event()).await.is_err());
    assert_eq!(*calls.lock().unwrap(), 4);
}

#[tokio::test]
async fn sink_timeout_is_bounded_and_reported() {
    let app = Router::new().fallback(any(|| async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        StatusCode::NO_CONTENT
    }));
    let (base, _server) = serve(app).await;
    let sink = HttpRedirectEventSink::new(transport(), endpoint(&base, Duration::from_millis(20)))
        .unwrap();
    assert!(
        sink.emit(&event())
            .await
            .unwrap_err()
            .to_string()
            .contains("timeout")
    );
}

#[tokio::test]
async fn http_source_obeys_runtime_cache_and_stale_failure_semantics() {
    let responses = Arc::new(Mutex::new(VecDeque::from([
        (StatusCode::OK, definition("https://go.example/docs", "old")),
        (StatusCode::INTERNAL_SERVER_ERROR, String::new()),
    ])));
    let app = Router::new().fallback(any({
        let responses = Arc::clone(&responses);
        move || {
            let (status, body) = responses.lock().unwrap().pop_front().unwrap();
            async move {
                Response::builder()
                    .status(status)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap()
            }
        }
    }));
    let (base, _server) = serve(app).await;
    let source = Arc::new(
        HttpRedirectSource::new(transport(), endpoint(&base, Duration::from_secs(1)), 65_536)
            .unwrap(),
    );
    let runtime = Arc::new(RedirectRuntime::new(
        source,
        RedirectCachePolicy::new(Duration::from_millis(10), NonZeroUsize::new(10).unwrap()),
    ));
    let key = CanonicalUrl::parse("https://go.example/docs").unwrap();
    assert_eq!(
        runtime.resolve(&key).await.unwrap().unwrap().id.as_str(),
        "old"
    );
    tokio::time::sleep(Duration::from_millis(15)).await;
    assert_eq!(
        runtime.resolve(&key).await.unwrap().unwrap().id.as_str(),
        "old"
    );
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(
        runtime.resolve(&key).await.unwrap().unwrap().id.as_str(),
        "old"
    );
    runtime.shutdown().await;
}

#[tokio::test]
async fn http_runtime_refreshes_changes_and_serves_remote_deletion_once() {
    let responses = Arc::new(Mutex::new(VecDeque::from([
        (StatusCode::OK, definition("https://go.example/docs", "old")),
        (StatusCode::OK, definition("https://go.example/docs", "new")),
        (StatusCode::NOT_FOUND, String::new()),
    ])));
    let app = Router::new().fallback(any({
        let responses = Arc::clone(&responses);
        move || {
            let (status, body) = responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or((StatusCode::NOT_FOUND, String::new()));
            async move {
                Response::builder()
                    .status(status)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap()
            }
        }
    }));
    let (base, _server) = serve(app).await;
    let source = Arc::new(
        HttpRedirectSource::new(transport(), endpoint(&base, Duration::from_secs(1)), 65_536)
            .unwrap(),
    );
    let runtime = Arc::new(RedirectRuntime::new(
        source,
        RedirectCachePolicy::new(Duration::from_millis(10), NonZeroUsize::new(10).unwrap()),
    ));
    let key = CanonicalUrl::parse("https://go.example/docs").unwrap();
    assert_eq!(
        runtime.resolve(&key).await.unwrap().unwrap().id.as_str(),
        "old"
    );
    tokio::time::sleep(Duration::from_millis(15)).await;
    assert_eq!(
        runtime.resolve(&key).await.unwrap().unwrap().id.as_str(),
        "old"
    );
    wait_for_remaining_responses(&responses, 1).await;
    assert_eq!(
        runtime.resolve(&key).await.unwrap().unwrap().id.as_str(),
        "new"
    );
    tokio::time::sleep(Duration::from_millis(15)).await;
    assert_eq!(
        runtime.resolve(&key).await.unwrap().unwrap().id.as_str(),
        "new"
    );
    wait_for_remaining_responses(&responses, 0).await;
    assert!(runtime.resolve(&key).await.unwrap().is_none());
    runtime.shutdown().await;
}
