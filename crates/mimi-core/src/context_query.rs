//! Context query helpers (M2.2.4).

use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};

pub fn query_context_id(query: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    query.hash(&mut hasher);
    format!("ctx:q:{:x}", hasher.finish())
}

/// In-memory BFS over undirected edges (used in tests / offline tools).
pub fn bfs_order(root_id: &str, edges: &[(String, String)], max_hops: u32, limit: usize) -> Vec<String> {
    let mut adj: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (from, to) in edges {
        adj.entry(from.clone()).or_default().push(to.clone());
        adj.entry(to.clone()).or_default().push(from.clone());
    }

    let mut seen = HashSet::new();
    seen.insert(root_id.to_string());
    let mut queue = VecDeque::from([(root_id.to_string(), 0u32)]);
    let mut order = Vec::new();

    while let Some((current, depth)) = queue.pop_front() {
        if depth >= max_hops {
            continue;
        }
        for next in adj.get(&current).into_iter().flatten() {
            if seen.insert(next.clone()) {
                order.push(next.clone());
                if order.len() >= limit {
                    return order;
                }
                queue.push_back((next.clone(), depth + 1));
            }
        }
    }

    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bfs_respects_hop_limit() {
        let edges = vec![
            ("a".into(), "b".into()),
            ("b".into(), "c".into()),
        ];
        let found = bfs_order("a", &edges, 1, 10);
        assert_eq!(found, vec!["b".to_string()]);
    }
}
