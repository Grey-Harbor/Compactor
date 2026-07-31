use std::{
    num::NonZeroUsize,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tokio::{
    sync::{Mutex, oneshot},
    task::JoinHandle,
    time::Instant,
};
use tracing::error;

use crate::domain::{CanonicalUrl, RedirectDefinition, RedirectSource, RedirectSourceError};

mod cache;
#[cfg(test)]
mod tests;

use cache::{CacheState, ResidentLifecycle};

const REFRESH_RETRY_DELAY: Duration = Duration::from_secs(30);

type Resolution = Result<Option<RedirectDefinition>, RedirectSourceError>;

#[derive(Debug, Clone, Copy)]
pub struct RedirectCachePolicy {
    pub freshness_ttl: Duration,
    pub max_entries: NonZeroUsize,
}

impl RedirectCachePolicy {
    pub const fn new(freshness_ttl: Duration, max_entries: NonZeroUsize) -> Self {
        Self {
            freshness_ttl,
            max_entries,
        }
    }
}

pub struct RedirectRuntime {
    source: Arc<dyn RedirectSource>,
    policy: RedirectCachePolicy,
    cache: Mutex<CacheState>,
    tasks: StdMutex<Vec<JoinHandle<()>>>,
    closed: AtomicBool,
}

impl RedirectRuntime {
    pub fn new(source: Arc<dyn RedirectSource>, policy: RedirectCachePolicy) -> Self {
        Self {
            source,
            policy,
            cache: Mutex::new(CacheState::default()),
            tasks: StdMutex::new(Vec::new()),
            closed: AtomicBool::new(false),
        }
    }

    pub async fn resolve(self: &Arc<Self>, canonical_url: &CanonicalUrl) -> Resolution {
        if self.closed.load(Ordering::Acquire) {
            return Err(RedirectSourceError::new(
                "redirect runtime is shutting down",
            ));
        }

        let now = Instant::now();
        let decision = {
            let mut cache = self.cache.lock().await;
            let recency = cache.next_recency();
            match cache.entries.get_mut(canonical_url) {
                Some(entry) => {
                    entry.last_used = recency;
                    match entry.lifecycle {
                        ResidentLifecycle::Fresh { expires_at } if expires_at > now => {
                            LookupDecision::Return(entry.definition.clone())
                        }
                        ResidentLifecycle::Fresh { .. } => {
                            entry.lifecycle = ResidentLifecycle::Refreshing;
                            LookupDecision::Refresh(entry.definition.clone())
                        }
                        ResidentLifecycle::Stale { retry_after } if retry_after <= now => {
                            entry.lifecycle = ResidentLifecycle::Refreshing;
                            LookupDecision::Refresh(entry.definition.clone())
                        }
                        ResidentLifecycle::Stale { .. } | ResidentLifecycle::Refreshing => {
                            LookupDecision::Return(entry.definition.clone())
                        }
                    }
                }
                None => {
                    let (sender, receiver) = oneshot::channel();
                    match cache.loads.get_mut(canonical_url) {
                        Some(waiters) => {
                            waiters.push(sender);
                            LookupDecision::Wait(receiver)
                        }
                        None => {
                            cache.loads.insert(canonical_url.clone(), vec![sender]);
                            LookupDecision::Load(receiver)
                        }
                    }
                }
            }
        };

        match decision {
            LookupDecision::Return(definition) => Ok(Some(definition)),
            LookupDecision::Refresh(definition) => {
                self.start_refresh(canonical_url.clone());
                Ok(Some(definition))
            }
            LookupDecision::Load(receiver) => {
                self.start_cold_load(canonical_url.clone());
                receive_resolution(receiver).await
            }
            LookupDecision::Wait(receiver) => receive_resolution(receiver).await,
        }
    }

    pub async fn shutdown(&self) {
        self.closed.store(true, Ordering::Release);
        let handles = {
            let mut tasks = self
                .tasks
                .lock()
                .expect("runtime task lock is not poisoned");
            std::mem::take(&mut *tasks)
        };
        for handle in handles {
            if let Err(join_error) = handle.await {
                error!(error = %join_error, "redirect runtime task failed during shutdown");
            }
        }
    }

    fn start_cold_load(self: &Arc<Self>, canonical_url: CanonicalUrl) {
        let runtime = Arc::clone(self);
        self.track(tokio::spawn(async move {
            let result = runtime.source.resolve(&canonical_url).await;
            runtime.finish_cold_load(canonical_url, result).await;
        }));
    }

    async fn finish_cold_load(&self, canonical_url: CanonicalUrl, result: Resolution) {
        let result = validate_resolution(&canonical_url, result);
        let waiters = {
            let mut cache = self.cache.lock().await;
            if let Ok(Some(definition)) = &result {
                cache.insert(
                    canonical_url.clone(),
                    definition.clone(),
                    Instant::now() + self.policy.freshness_ttl,
                    self.policy.max_entries.get(),
                );
            }
            cache.loads.remove(&canonical_url).unwrap_or_default()
        };
        for waiter in waiters {
            let _ = waiter.send(result.clone());
        }
    }

    fn start_refresh(self: &Arc<Self>, canonical_url: CanonicalUrl) {
        let runtime = Arc::clone(self);
        self.track(tokio::spawn(async move {
            let result = runtime.source.resolve(&canonical_url).await;
            runtime.finish_refresh(canonical_url, result).await;
        }));
    }

    async fn finish_refresh(&self, canonical_url: CanonicalUrl, result: Resolution) {
        let result = validate_resolution(&canonical_url, result);
        let mut cache = self.cache.lock().await;
        let recency = cache.next_recency();
        match result {
            Ok(Some(definition)) => {
                if let Some(entry) = cache.entries.get_mut(&canonical_url) {
                    if matches!(entry.lifecycle, ResidentLifecycle::Refreshing) {
                        entry.definition = definition;
                        entry.lifecycle = ResidentLifecycle::Fresh {
                            expires_at: Instant::now() + self.policy.freshness_ttl,
                        };
                        entry.last_used = recency;
                    }
                }
            }
            Ok(None) => {
                if cache
                    .entries
                    .get(&canonical_url)
                    .is_some_and(|entry| matches!(entry.lifecycle, ResidentLifecycle::Refreshing))
                {
                    cache.entries.remove(&canonical_url);
                }
            }
            Err(source_error) => {
                if let Some(entry) = cache.entries.get_mut(&canonical_url) {
                    if matches!(entry.lifecycle, ResidentLifecycle::Refreshing) {
                        entry.lifecycle = ResidentLifecycle::Stale {
                            retry_after: Instant::now() + REFRESH_RETRY_DELAY,
                        };
                        entry.last_used = recency;
                    }
                }
                error!(
                    canonical_url = %canonical_url,
                    error = %source_error,
                    "could not refresh cached redirect"
                );
            }
        }
    }

    fn track(&self, handle: JoinHandle<()>) {
        let mut tasks = self
            .tasks
            .lock()
            .expect("runtime task lock is not poisoned");
        tasks.retain(|task| !task.is_finished());
        tasks.push(handle);
    }
}

fn validate_resolution(canonical_url: &CanonicalUrl, result: Resolution) -> Resolution {
    match result {
        Ok(Some(definition)) if definition.canonical_url != *canonical_url => {
            Err(RedirectSourceError::new(format!(
                "source returned canonical URL {} for lookup {canonical_url}",
                definition.canonical_url
            )))
        }
        result => result,
    }
}

async fn receive_resolution(receiver: oneshot::Receiver<Resolution>) -> Resolution {
    receiver.await.unwrap_or_else(|_| {
        Err(RedirectSourceError::new(
            "redirect source resolution ended unexpectedly",
        ))
    })
}

enum LookupDecision {
    Return(RedirectDefinition),
    Refresh(RedirectDefinition),
    Load(oneshot::Receiver<Resolution>),
    Wait(oneshot::Receiver<Resolution>),
}
