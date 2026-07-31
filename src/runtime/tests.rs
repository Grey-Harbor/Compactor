
use std::{
    collections::{BTreeMap, VecDeque},
    sync::atomic::AtomicUsize,
};

use async_trait::async_trait;
use tokio::sync::Semaphore;
use url::Url;

use super::*;
use crate::domain::{RedirectId, RedirectStatus, ResponseHeaders};

struct ScriptedSource {
    responses: Mutex<VecDeque<Resolution>>,
    calls: AtomicUsize,
    blocked: AtomicBool,
    permits: Semaphore,
}

struct AllRefreshingSource {
    a_calls: AtomicUsize,
    b_calls: AtomicUsize,
    release_refresh: Semaphore,
}

#[async_trait]
impl RedirectSource for AllRefreshingSource {
    async fn resolve(&self, canonical_url: &CanonicalUrl) -> Resolution {
        if *canonical_url == canonical("a") {
            let call = self.a_calls.fetch_add(1, Ordering::AcqRel);
            if call > 0 {
                self.release_refresh.acquire().await.unwrap().forget();
            }
            Ok(Some(definition(
                "a",
                if call == 0 {
                    "https://destination.example/a-old"
                } else {
                    "https://destination.example/a-new"
                },
            )))
        } else {
            self.b_calls.fetch_add(1, Ordering::AcqRel);
            Ok(Some(definition("b", "https://destination.example/b")))
        }
    }
}

impl ScriptedSource {
    fn new(responses: Vec<Resolution>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into()),
            calls: AtomicUsize::new(0),
            blocked: AtomicBool::new(false),
            permits: Semaphore::new(0),
        })
    }

    fn block(&self) {
        self.blocked.store(true, Ordering::Release);
    }

    fn release(&self) {
        self.permits.add_permits(1);
    }

    async fn wait_for_calls(&self, expected: usize) {
        while self.calls.load(Ordering::Acquire) < expected {
            tokio::task::yield_now().await;
        }
    }
}

#[async_trait]
impl RedirectSource for ScriptedSource {
    async fn resolve(&self, _: &CanonicalUrl) -> Resolution {
        self.calls.fetch_add(1, Ordering::AcqRel);
        if self.blocked.load(Ordering::Acquire) {
            self.permits.acquire().await.unwrap().forget();
        }
        self.responses
            .lock()
            .await
            .pop_front()
            .expect("test source has a scripted response")
    }
}

fn canonical(path: &str) -> CanonicalUrl {
    CanonicalUrl::parse(&format!("https://example.com/{path}")).unwrap()
}

fn definition(path: &str, destination: &str) -> RedirectDefinition {
    RedirectDefinition {
        id: RedirectId::new(path).unwrap(),
        canonical_url: canonical(path),
        redirect_url: Url::parse(destination).unwrap(),
        status_code: RedirectStatus::PermanentRedirect,
        response_headers: ResponseHeaders::try_from_strings(BTreeMap::new()).unwrap(),
    }
}

fn runtime(
    source: Arc<dyn RedirectSource>,
    ttl: Duration,
    capacity: usize,
) -> Arc<RedirectRuntime> {
    Arc::new(RedirectRuntime::new(
        source,
        RedirectCachePolicy::new(ttl, NonZeroUsize::new(capacity).unwrap()),
    ))
}

#[tokio::test]
async fn cold_results_cache_only_found_definitions() {
    let found = definition("found", "https://destination.example/one");
    let source = ScriptedSource::new(vec![
        Ok(Some(found.clone())),
        Ok(None),
        Ok(None),
        Err(RedirectSourceError::new("offline")),
        Err(RedirectSourceError::new("offline")),
    ]);
    let runtime = runtime(source.clone(), Duration::from_secs(60), 10);

    assert!(
        runtime
            .resolve(&canonical("found"))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        runtime
            .resolve(&canonical("found"))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        runtime
            .resolve(&canonical("missing"))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        runtime
            .resolve(&canonical("missing"))
            .await
            .unwrap()
            .is_none()
    );
    assert!(runtime.resolve(&canonical("error")).await.is_err());
    assert!(runtime.resolve(&canonical("error")).await.is_err());
    assert_eq!(source.calls.load(Ordering::Acquire), 5);
}

#[tokio::test]
async fn concurrent_cold_lookups_share_each_result_shape() {
    for result in [
        Ok(Some(definition("key", "https://destination.example/one"))),
        Ok(None),
        Err(RedirectSourceError::new("offline")),
    ] {
        let source = ScriptedSource::new(vec![result]);
        source.block();
        let runtime = runtime(source.clone(), Duration::from_secs(60), 10);
        let key = canonical("key");
        let first = tokio::spawn({
            let runtime = Arc::clone(&runtime);
            let key = key.clone();
            async move { runtime.resolve(&key).await }
        });
        source.wait_for_calls(1).await;
        let second = tokio::spawn({
            let runtime = Arc::clone(&runtime);
            let key = key.clone();
            async move { runtime.resolve(&key).await }
        });
        tokio::task::yield_now().await;
        assert_eq!(source.calls.load(Ordering::Acquire), 1);
        source.release();
        let first = first.await.unwrap();
        let second = second.await.unwrap();
        assert_eq!(first.is_ok(), second.is_ok());
        assert_eq!(
            first.ok().flatten().is_some(),
            second.ok().flatten().is_some()
        );
    }
}

#[tokio::test(start_paused = true)]
async fn stale_lookup_returns_immediately_and_one_refresh_replaces_it() {
    let old = definition("key", "https://destination.example/old");
    let new = definition("key", "https://destination.example/new");
    let source = ScriptedSource::new(vec![Ok(Some(old)), Ok(Some(new))]);
    let runtime = runtime(source.clone(), Duration::from_secs(10), 10);
    let key = canonical("key");

    assert_eq!(
        runtime
            .resolve(&key)
            .await
            .unwrap()
            .unwrap()
            .redirect_url
            .as_str(),
        "https://destination.example/old"
    );
    tokio::time::advance(Duration::from_secs(10)).await;
    assert_eq!(
        runtime
            .resolve(&key)
            .await
            .unwrap()
            .unwrap()
            .redirect_url
            .as_str(),
        "https://destination.example/old"
    );
    assert_eq!(
        runtime
            .resolve(&key)
            .await
            .unwrap()
            .unwrap()
            .redirect_url
            .as_str(),
        "https://destination.example/old"
    );
    source.wait_for_calls(2).await;
    tokio::task::yield_now().await;
    assert_eq!(source.calls.load(Ordering::Acquire), 2);
    assert_eq!(
        runtime
            .resolve(&key)
            .await
            .unwrap()
            .unwrap()
            .redirect_url
            .as_str(),
        "https://destination.example/new"
    );
}

#[tokio::test(start_paused = true)]
async fn refresh_not_found_serves_once_then_removes_the_entry() {
    let source = ScriptedSource::new(vec![
        Ok(Some(definition("key", "https://destination.example/old"))),
        Ok(None),
        Ok(None),
    ]);
    let runtime = runtime(source.clone(), Duration::from_secs(10), 10);
    let key = canonical("key");
    runtime.resolve(&key).await.unwrap().unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;

    assert!(runtime.resolve(&key).await.unwrap().is_some());
    source.wait_for_calls(2).await;
    tokio::task::yield_now().await;
    assert!(runtime.resolve(&key).await.unwrap().is_none());
    assert_eq!(source.calls.load(Ordering::Acquire), 3);
}

#[tokio::test(start_paused = true)]
async fn refresh_errors_keep_stale_and_apply_retry_cooldown() {
    let source = ScriptedSource::new(vec![
        Ok(Some(definition("key", "https://destination.example/old"))),
        Err(RedirectSourceError::new("offline")),
        Err(RedirectSourceError::new("still offline")),
    ]);
    let runtime = runtime(source.clone(), Duration::from_secs(10), 10);
    let key = canonical("key");
    runtime.resolve(&key).await.unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;
    assert!(runtime.resolve(&key).await.unwrap().is_some());
    source.wait_for_calls(2).await;
    tokio::task::yield_now().await;

    assert!(runtime.resolve(&key).await.unwrap().is_some());
    assert_eq!(source.calls.load(Ordering::Acquire), 2);
    tokio::time::advance(Duration::from_secs(30)).await;
    assert!(runtime.resolve(&key).await.unwrap().is_some());
    source.wait_for_calls(3).await;
    assert_eq!(source.calls.load(Ordering::Acquire), 3);
}

#[tokio::test(start_paused = true)]
async fn least_recently_used_non_refreshing_entry_is_evicted() {
    let source = ScriptedSource::new(vec![
        Ok(Some(definition("a", "https://destination.example/a"))),
        Ok(Some(definition("b", "https://destination.example/b"))),
        Ok(Some(definition("c", "https://destination.example/c"))),
        Ok(Some(definition("b", "https://destination.example/b"))),
    ]);
    let runtime = runtime(source.clone(), Duration::from_secs(60), 2);
    runtime.resolve(&canonical("a")).await.unwrap();
    runtime.resolve(&canonical("b")).await.unwrap();
    runtime.resolve(&canonical("a")).await.unwrap();
    runtime.resolve(&canonical("c")).await.unwrap();
    runtime.resolve(&canonical("b")).await.unwrap();
    assert_eq!(source.calls.load(Ordering::Acquire), 4);
}

#[test]
fn insertion_does_not_evict_a_refreshing_entry() {
    let now = Instant::now();
    let mut cache = CacheState::default();
    cache.insert(
        canonical("a"),
        definition("a", "https://destination.example/a"),
        now,
        1,
    );
    cache.entries.get_mut(&canonical("a")).unwrap().lifecycle = ResidentLifecycle::Refreshing;
    cache.insert(
        canonical("b"),
        definition("b", "https://destination.example/b"),
        now,
        1,
    );
    assert!(cache.entries.contains_key(&canonical("a")));
    assert!(!cache.entries.contains_key(&canonical("b")));
}

#[tokio::test(start_paused = true)]
async fn found_result_is_returned_uncached_when_every_resident_is_refreshing() {
    let source = Arc::new(AllRefreshingSource {
        a_calls: AtomicUsize::new(0),
        b_calls: AtomicUsize::new(0),
        release_refresh: Semaphore::new(0),
    });
    let runtime = runtime(source.clone(), Duration::from_secs(10), 1);
    runtime.resolve(&canonical("a")).await.unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;
    runtime.resolve(&canonical("a")).await.unwrap();
    while source.a_calls.load(Ordering::Acquire) < 2 {
        tokio::task::yield_now().await;
    }

    assert!(runtime.resolve(&canonical("b")).await.unwrap().is_some());
    assert!(runtime.resolve(&canonical("b")).await.unwrap().is_some());
    assert_eq!(source.b_calls.load(Ordering::Acquire), 2);
    source.release_refresh.add_permits(1);
    runtime.shutdown().await;
}

#[tokio::test(start_paused = true)]
async fn shutdown_waits_for_an_in_flight_refresh() {
    let source = ScriptedSource::new(vec![
        Ok(Some(definition("key", "https://destination.example/old"))),
        Ok(Some(definition("key", "https://destination.example/new"))),
    ]);
    let runtime = runtime(source.clone(), Duration::from_secs(10), 10);
    let key = canonical("key");
    runtime.resolve(&key).await.unwrap();
    source.block();
    tokio::time::advance(Duration::from_secs(10)).await;
    runtime.resolve(&key).await.unwrap();
    source.wait_for_calls(2).await;

    let shutdown = tokio::spawn({
        let runtime = Arc::clone(&runtime);
        async move { runtime.shutdown().await }
    });
    tokio::task::yield_now().await;
    assert!(!shutdown.is_finished());
    source.release();
    shutdown.await.unwrap();
}
