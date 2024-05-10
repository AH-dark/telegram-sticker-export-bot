use std::hash::Hash;
use std::sync::Arc;

use governor::{Quota, RateLimiter};
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;

#[derive(Debug)]
pub struct Limiter<K: Hash + Clone + Eq>(RateLimiter<K, DefaultKeyedStateStore<K>, DefaultClock>);

impl<K: Hash + Clone + Eq> Limiter<K> {
    pub fn new(quota: Quota, clock: &DefaultClock) -> Arc<Self> {
        Arc::new(Self(RateLimiter::new(
            quota,
            DefaultKeyedStateStore::default(),
            clock,
        )))
    }

    pub fn check(&self, key: K) -> bool {
        self.0.check_key(&key).is_ok()
    }
}
