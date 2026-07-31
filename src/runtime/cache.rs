use std::collections::HashMap;

use tokio::{sync::oneshot, time::Instant};

use super::Resolution;
use crate::domain::{CanonicalUrl, RedirectDefinition};

#[derive(Default)]
pub(super) struct CacheState {
    pub(super) entries: HashMap<CanonicalUrl, ResidentEntry>,
    pub(super) loads: HashMap<CanonicalUrl, Vec<oneshot::Sender<Resolution>>>,
    recency: u64,
}

impl CacheState {
    pub(super) fn next_recency(&mut self) -> u64 {
        if self.recency == u64::MAX {
            let mut ordered = self
                .entries
                .iter()
                .map(|(key, entry)| (key.clone(), entry.last_used))
                .collect::<Vec<_>>();
            ordered.sort_by_key(|(_, last_used)| *last_used);
            for (index, (key, _)) in ordered.into_iter().enumerate() {
                if let Some(entry) = self.entries.get_mut(&key) {
                    entry.last_used = index as u64;
                }
            }
            self.recency = self.entries.len() as u64;
        }
        self.recency += 1;
        self.recency
    }

    pub(super) fn insert(
        &mut self,
        canonical_url: CanonicalUrl,
        definition: RedirectDefinition,
        expires_at: Instant,
        max_entries: usize,
    ) {
        if !self.entries.contains_key(&canonical_url) && self.entries.len() >= max_entries {
            let eviction = self
                .entries
                .iter()
                .filter(|(_, entry)| !matches!(entry.lifecycle, ResidentLifecycle::Refreshing))
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone());
            let Some(eviction) = eviction else {
                return;
            };
            self.entries.remove(&eviction);
        }
        let last_used = self.next_recency();
        self.entries.insert(
            canonical_url,
            ResidentEntry {
                definition,
                lifecycle: ResidentLifecycle::Fresh { expires_at },
                last_used,
            },
        );
    }
}

pub(super) struct ResidentEntry {
    pub(super) definition: RedirectDefinition,
    pub(super) lifecycle: ResidentLifecycle,
    pub(super) last_used: u64,
}

#[derive(Clone, Copy)]
pub(super) enum ResidentLifecycle {
    Fresh { expires_at: Instant },
    Refreshing,
    Stale { retry_after: Instant },
}
