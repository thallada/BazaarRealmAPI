use anyhow::Result;
use lru::LruCache;
use std::fmt::Debug;
use std::future::Future;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;
use warp::{Rejection, Reply};

use super::CachedResponse;
use crate::problem::{reject_anyhow, unpack_problem};

#[derive(Debug, Clone)]
pub struct Cache<K, V>
where
    K: Eq + Hash + Debug,
    V: Clone,
{
    pub name: String,
    pub lru_mutex: Arc<Mutex<LruCache<K, V>>>,
    pub log_keys: bool,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Debug,
    V: Clone,
{
    pub fn new(name: &str, capacity: usize) -> Self {
        Cache {
            name: name.to_string(),
            lru_mutex: Arc::new(Mutex::new(LruCache::new(capacity))),
            log_keys: true,
        }
    }

    pub fn log_keys(mut self, value: bool) -> Self {
        self.log_keys = value;
        self
    }

    pub fn log_with_key(&self, key: &K, message: &str) {
        if self.log_keys {
            debug!(cache = %self.name, key = ?key, message);
        } else {
            debug!(cache = %self.name, message);
        }
    }

    pub async fn get<G, F>(&self, key: K, getter: G) -> Result<V>
    where
        G: Fn() -> F,
        F: Future<Output = Result<V>>,
    {
        let mut guard = self.lru_mutex.lock().await;
        if let Some(value) = guard.get(&key) {
            self.log_with_key(&key, "get: hit");
            return Ok(value.clone());
        }
        drop(guard);

        self.log_with_key(&key, "get: miss");
        let value = getter().await?;
        let mut guard = self.lru_mutex.lock().await;
        guard.put(key, value.clone());

        Ok(value)
    }

    pub async fn delete(&self, key: K) -> Option<V> {
        let mut guard = self.lru_mutex.lock().await;
        let value = guard.pop(&key);
        self.log_with_key(&key, "delete");

        value
    }

    pub async fn clear(&self) {
        let mut guard = self.lru_mutex.lock().await;
        guard.clear();
        debug!(cache = %self.name, "cache clear");
    }
}

impl<K> Cache<K, CachedResponse>
where
    K: Eq + Hash + Debug,
{
    pub async fn get_response<G, F, R>(
        &self,
        key: K,
        getter: G,
    ) -> Result<CachedResponse, Rejection>
    where
        G: Fn() -> F,
        F: Future<Output = Result<R>>,
        R: Reply,
    {
        let mut guard = self.lru_mutex.lock().await;
        if let Some(value) = guard.get(&key) {
            self.log_with_key(&key, "get_response: hit");
            return Ok(value.clone());
        }
        drop(guard);

        self.log_with_key(&key, "get_response: miss");
        let reply = getter().await.map_err(reject_anyhow);
        Ok(match reply {
            Ok(reply) => {
                let cached_response = CachedResponse::from_reply(reply)
                    .await
                    .map_err(reject_anyhow)?;
                let mut guard = self.lru_mutex.lock().await;
                guard.put(key, cached_response.clone());
                cached_response
            }
            Err(rejection) => {
                self.log_with_key(&key, "get_response: getter returned rejection, not caching");
                let reply = unpack_problem(rejection).await?;
                CachedResponse::from_reply(reply)
                    .await
                    .map_err(reject_anyhow)?
            }
        })
    }

    pub async fn delete_response(&self, key: K) -> Option<CachedResponse> {
        let mut guard = self.lru_mutex.lock().await;
        let cached_response = guard.pop(&key);
        self.log_with_key(&key, "delete_response");

        cached_response
    }
}
