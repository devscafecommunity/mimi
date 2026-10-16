//! L1 LRU context cache (M2.2.5), aligned with `pandora-memory` C++ `LRUCache`.

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::heatmap::{decayed_temperature, DEFAULT_LAMBDA};

pub const DEFAULT_L1_CAPACITY: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CachedContext {
    pub id: String,
    pub label: Option<String>,
    pub heat: f64,
    pub last_accessed_secs: u64,
    pub access_count: u64,
    /// Mirrors C++ `CachedNode::data` (URI, description, etc.).
    pub data: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct L1CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl L1CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// M2 acceptance target (see `docs/M2.7-ACCEPTANCE.md`).
pub const M2_L1_HIT_RATE_TARGET: f64 = 0.70;

pub struct L1ContextCache {
    capacity: usize,
    order: VecDeque<String>,
    entries: HashMap<String, CachedContext>,
    stats: L1CacheStats,
}

impl L1ContextCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            order: VecDeque::new(),
            entries: HashMap::new(),
            stats: L1CacheStats::default(),
        }
    }

    pub fn stats(&self) -> &L1CacheStats {
        &self.stats
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn get(&mut self, key: &str, now_secs: u64) -> Option<CachedContext> {
        if !self.entries.contains_key(key) {
            self.stats.misses += 1;
            return None;
        }

        self.stats.hits += 1;
        self.touch_order(key);

        let entry = self.entries.get_mut(key)?;
        entry.access_count += 1;
        entry.last_accessed_secs = now_secs;
        entry.heat = 1.0;

        Some(entry.clone())
    }

    pub fn put(&mut self, entry: CachedContext) {
        let key = entry.id.clone();
        if self.entries.contains_key(&key) {
            self.entries.insert(key.clone(), entry);
            self.touch_order(&key);
            return;
        }

        while self.entries.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
                self.stats.evictions += 1;
            } else {
                break;
            }
        }

        self.entries.insert(key.clone(), entry);
        self.order.push_back(key);
    }

    /// Hot entries for circuit-open fallback (no Neo4j).
    pub fn hot_snapshot(&self, limit: usize) -> Vec<CachedContext> {
        let mut entries: Vec<CachedContext> = self.entries.values().cloned().collect();
        entries.sort_by(|a, b| {
            b.heat
                .partial_cmp(&a.heat)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries.truncate(limit);
        entries
    }

    pub fn evict_expired(&mut self, now_secs: u64, min_heat: f64) {
        let keys: Vec<String> = self.entries.keys().cloned().collect();
        for key in keys {
            let Some(entry) = self.entries.get(&key) else {
                continue;
            };
            let age = now_secs.saturating_sub(entry.last_accessed_secs);
            let heat = decayed_temperature(entry.heat, DEFAULT_LAMBDA, age);
            if heat < min_heat {
                self.entries.remove(&key);
                self.order.retain(|k| k != &key);
                self.stats.evictions += 1;
            }
        }
    }

    fn touch_order(&mut self, key: &str) {
        self.order.retain(|k| k != key);
        self.order.push_back(key.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> CachedContext {
        CachedContext {
            id: id.to_string(),
            label: Some(id.to_string()),
            heat: 1.0,
            last_accessed_secs: 100,
            access_count: 1,
            data: String::new(),
        }
    }

    #[test]
    fn lru_evicts_oldest() {
        let mut cache = L1ContextCache::new(2);
        cache.put(sample("a"));
        cache.put(sample("b"));
        cache.put(sample("c"));
        assert_eq!(cache.len(), 2);
        assert!(!cache.entries.contains_key("a"));
        assert_eq!(cache.stats.evictions, 1);
    }

    #[test]
    fn acceptance_hit_rate_above_m2_target() {
        let mut cache = L1ContextCache::new(100);
        for i in 0..5 {
            cache.put(sample(&format!("hot-{}", i)));
        }
        for round in 0..100 {
            let key = format!("hot-{}", round % 5);
            cache.get(&key, 1_000);
        }
        let rate = cache.stats().hit_rate();
        assert!(
            rate >= M2_L1_HIT_RATE_TARGET,
            "L1 hit rate {:.2}% below M2 target {:.0}%",
            rate * 100.0,
            M2_L1_HIT_RATE_TARGET * 100.0
        );
    }

    #[test]
    fn tracks_hits_and_misses() {
        let mut cache = L1ContextCache::new(10);
        cache.put(sample("x"));
        assert!(cache.get("x", 200).is_some());
        assert!(cache.get("missing", 200).is_none());
        assert_eq!(cache.stats.hits, 1);
        assert_eq!(cache.stats.misses, 1);
    }
}
