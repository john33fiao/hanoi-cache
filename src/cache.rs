use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CacheEntry {
    pub value: String,
    pub expires_at: Option<Instant>,
}

pub type SharedCache = Arc<RwLock<HashMap<String, CacheEntry>>>;

pub fn new_cache() -> SharedCache {
    Arc::new(RwLock::new(HashMap::new()))
}

pub async fn get_fresh(cache: &SharedCache, key: &str) -> Option<String> {
    let cache = cache.read().await;
    let entry = cache.get(key)?;

    match entry.expires_at {
        None => Some(entry.value.clone()),
        Some(expires_at) if Instant::now() <= expires_at => Some(entry.value.clone()),
        Some(_) => None,
    }
}

pub async fn get_stale(cache: &SharedCache, key: &str, stale_window: Duration) -> Option<String> {
    let cache = cache.read().await;
    let entry = cache.get(key)?;
    let expires_at = entry.expires_at?;

    if Instant::now() <= expires_at + stale_window {
        Some(entry.value.clone())
    } else {
        None
    }
}

pub async fn set(cache: &SharedCache, key: String, value: String, ttl: Option<Duration>) {
    let expires_at = ttl.map(|ttl| Instant::now() + ttl);

    let mut cache = cache.write().await;
    cache.insert(key, CacheEntry { value, expires_at });
}
