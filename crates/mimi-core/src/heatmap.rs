//! In-memory heatmap (M2.2) — exponential decay aligned with `pandora-memory` C++.

use std::collections::HashMap;

pub const DEFAULT_LAMBDA: f64 = 0.01;
pub const DEFAULT_THRESHOLD: f64 = 0.1;
pub const INITIAL_TEMPERATURE: f64 = 1.0;
pub const DEFAULT_MAX_NODES: usize = 500;

#[derive(Debug, Clone, PartialEq)]
pub struct HeatmapNode {
    pub id: String,
    pub temperature: f64,
    pub last_accessed_secs: u64,
    pub access_count: u64,
}

#[derive(Debug, Clone)]
struct StoredNode {
    base_temperature: f64,
    last_accessed_secs: u64,
    access_count: u64,
}

#[derive(Debug, Clone)]
pub struct HeatmapManager {
    lambda: f64,
    threshold: f64,
    max_nodes: usize,
    nodes: HashMap<String, StoredNode>,
}

impl Default for HeatmapManager {
    fn default() -> Self {
        Self::new(DEFAULT_LAMBDA, DEFAULT_THRESHOLD, DEFAULT_MAX_NODES)
    }
}

impl HeatmapManager {
    pub fn new(lambda: f64, threshold: f64, max_nodes: usize) -> Self {
        Self {
            lambda,
            threshold,
            max_nodes,
            nodes: HashMap::new(),
        }
    }

    pub fn touch(&mut self, id: &str, now_secs: u64) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.base_temperature = INITIAL_TEMPERATURE;
            node.last_accessed_secs = now_secs;
            node.access_count += 1;
            return;
        }

        if self.nodes.len() >= self.max_nodes {
            self.prune_cold(now_secs);
        }

        if self.nodes.len() >= self.max_nodes {
            if let Some(oldest_key) = self
                .nodes
                .iter()
                .min_by_key(|(_, n)| n.last_accessed_secs)
                .map(|(k, _)| k.clone())
            {
                self.nodes.remove(&oldest_key);
            }
        }

        self.nodes.insert(
            id.to_string(),
            StoredNode {
                base_temperature: INITIAL_TEMPERATURE,
                last_accessed_secs: now_secs,
                access_count: 1,
            },
        );
    }

    pub fn temperature_at(&self, id: &str, now_secs: u64) -> f64 {
        match self.nodes.get(id) {
            Some(node) => {
                let age = now_secs.saturating_sub(node.last_accessed_secs);
                decayed_temperature(node.base_temperature, self.lambda, age)
            },
            None => 0.0,
        }
    }

    pub fn hot_nodes(&self, now_secs: u64) -> Vec<HeatmapNode> {
        let mut result: Vec<HeatmapNode> = self
            .nodes
            .iter()
            .filter_map(|(id, node)| {
                let age = now_secs.saturating_sub(node.last_accessed_secs);
                let heat = decayed_temperature(node.base_temperature, self.lambda, age);
                if heat >= self.threshold {
                    Some(HeatmapNode {
                        id: id.clone(),
                        temperature: heat,
                        last_accessed_secs: node.last_accessed_secs,
                        access_count: node.access_count,
                    })
                } else {
                    None
                }
            })
            .collect();

        result.sort_by(|a, b| {
            b.temperature
                .partial_cmp(&a.temperature)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result
    }

    pub fn prune_cold(&mut self, now_secs: u64) {
        self.nodes.retain(|_, node| {
            let age = now_secs.saturating_sub(node.last_accessed_secs);
            decayed_temperature(node.base_temperature, self.lambda, age) >= self.threshold
        });
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn snapshot_for_persist(&self, now_secs: u64) -> Vec<HeatmapNode> {
        self.nodes
            .iter()
            .map(|(id, node)| {
                let age = now_secs.saturating_sub(node.last_accessed_secs);
                HeatmapNode {
                    id: id.clone(),
                    temperature: decayed_temperature(node.base_temperature, self.lambda, age),
                    last_accessed_secs: node.last_accessed_secs,
                    access_count: node.access_count,
                }
            })
            .collect()
    }
}

pub fn decayed_temperature(t0: f64, lambda: f64, age_secs: u64) -> f64 {
    t0 * (-lambda * age_secs as f64).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_half_life_near_70s() {
        let mut hm = HeatmapManager::default();
        hm.touch("a", 0);
        let t = hm.temperature_at("a", 70);
        assert!(t > 0.45 && t < 0.55);
    }

    #[test]
    fn touch_resets_heat() {
        let mut hm = HeatmapManager::default();
        hm.touch("b", 0);
        assert!(hm.temperature_at("b", 250) < 0.1);
        hm.touch("b", 250);
        assert_eq!(hm.temperature_at("b", 250), 1.0);
    }
}
