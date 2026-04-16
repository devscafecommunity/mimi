# Heatmap Memory Decay Algorithm

Technical specification for Pandora's Heatmap memory decay algorithm. This system manages memory importance in the knowledge graph using exponential decay.

## 1. Overview
The Heatmap algorithm provides a dynamic way to manage the assistant's context. Unlike standard RAG that relies solely on semantic similarity, the Heatmap tracks "temporal relevance." It prevents context flooding by filtering out nodes that haven't been touched recently. This optimizes token usage by ensuring only high-signal information enters the LLM prompt.

While RAG finds what is relevant to a query, the Heatmap finds what is active in the current session.

## 2. Temperature Formula
The temperature of a node represents its current relevance. It follows an exponential decay model.

### Mathematical Definition
**T(t) = T0 * e^(-λ * (now - last_accessed))**

| Variable | Definition |
| :--- | :--- |
| **T(t)** | Current temperature at time *t* |
| **T0** | Initial temperature (1.0) when the node is accessed |
| **λ (lambda)** | Decay constant that controls the cooling rate |
| **now** | Current system timestamp (seconds) |
| **last_accessed** | Timestamp of the most recent access (seconds) |

### Example Calculations
Using λ = 0.01:
*   **Immediate (0s):** T = 1.0 * e^(0) = 1.0
*   **70s later:** T = 1.0 * e^(-0.01 * 70) ≈ 0.496 (approx. half-life)
*   **230s later:** T = 1.0 * e^(-0.01 * 230) ≈ 0.100 (threshold reached)

## 3. Parameters & Tuning
*   **T0 (initial temperature):** Default 1.0. This is the maximum "heat" a node can have.
*   **Lambda (λ):** Default 0.01. This yields a half-life of roughly 70 seconds. This is tuned for active conversation cycles where context remains relevant for a minute or two of silence.
*   **Threshold:** Default 0.1. Any node with a calculated temperature below this value is ignored by queries.
*   **Max nodes per query:** 500. A hard limit to prevent oversized context injections.

## 4. Update on Access
When a node is created or read:
1.  Set `last_accessed = now`
2.  Set `temperature = 1.0` (Reset to hot)
3.  Increment `access_count`

This reset mechanism ensures that frequently used "hot spots" in the graph stay active regardless of their original creation date.

## 5. Query Algorithm (BFS with Temperature Filtering)
To retrieve relevant context:
1.  **Start** from a seed node (the current task or user query target).
2.  **Traverse** the graph using Breadth-First Search (BFS).
3.  **Filter** each encountered node: 
    *   Calculate current `T(now)`.
    *   Only include nodes where `T(now) > threshold`.
4.  **Sort** the remaining results by `temperature` in descending order.
5.  **Limit** to `max_nodes`.

## 6. Cypher Queries

### Retrieve hot context around a node
```cypher
MATCH (start {id: $node_id})-[r*1..3]-(neighbor)
WITH neighbor, 
     1.0 * exp(-0.01 * (timestamp()/1000 - neighbor.last_accessed)) AS current_temp
WHERE current_temp > 0.1
RETURN neighbor, current_temp
ORDER BY current_temp DESC
LIMIT 500
```

### Update temperature on access
```cypher
MATCH (n {id: $node_id})
SET n.last_accessed = timestamp()/1000,
    n.access_count = n.access_count + 1,
    n.temperature = 1.0
```

### Calculate current temperature for node
```cypher
MATCH (n {id: $node_id})
RETURN 1.0 * exp(-0.01 * (timestamp()/1000 - n.last_accessed)) AS current_temp
```

### Find cold nodes for archival
```cypher
MATCH (n)
WHERE 1.0 * exp(-0.01 * (timestamp()/1000 - n.last_accessed)) <= 0.1
RETURN n.id
```

### Checkpoint temperature state
Note: Temperature is calculated on the fly, but for indexing or external analysis, you can periodically flush values.
```cypher
MATCH (n)
SET n.temperature = 1.0 * exp(-0.01 * (timestamp()/1000 - n.last_accessed))
```

## 7. Performance Analysis
*   **Time complexity:** O(N log N) for sorting N results from the BFS traversal. The traversal itself is bounded by the graph density and limited depth.
*   **Space complexity:** O(max_nodes). We only keep the hottest nodes in the working set.
*   **Typical query time:** < 50ms for a 1M node graph when using indexed timestamps for the initial node lookup.
*   **Cache hit rate expectations:** > 70% with LRU L1 cache, as hot nodes are accessed repeatedly in tight loops.

## 8. Decay Calibration
*   **Choosing λ:** A smaller λ (0.001) keeps context for hours. A larger λ (0.1) clears context in seconds.
*   **Trade-offs:** 
    *   High retention: Better "long-term" short-term memory, but higher risk of context flooding.
    *   Low retention: Cleaner prompts, but requires the user to repeat themselves more often.
*   **Validation:** Use historical conversation logs to see which nodes were actually needed vs. which were just noise. Adjust λ until the "needed" nodes stay above the threshold for the duration of a typical task.

## 9. Edge Cases
*   **Very old nodes (T → 0):** These are naturally filtered out. They remain in the database but do not affect LLM reasoning. They are candidates for archival.
*   **Frequently accessed nodes (T → 1.0):** Nodes that are central to a task will be reset constantly, keeping them "red hot."
*   **New nodes (T = 1.0):** Immediately included.
*   **Node never accessed:** If a node was created but never read, use `created_at` as the fallback for `last_accessed` to allow for an initial "warm" period.

## 10. Integration with Skill Execution
*   **Skill Detection:** Hot clusters of nodes indicate the current work pattern, allowing Pandora to suggest relevant skills.
*   **Context Injection:** The hottest context is formatted and injected into the system prompt for every LLM call.
*   **Audit Trail:** Event nodes tagged as `Audit` ignore the temperature filter for compliance and logging, ensuring the history is never "forgotten" during a review.

## 11. Versioning & Changes
To change λ or the threshold:
1.  Update the global configuration.
2.  Queries immediately reflect the new "coolness."
3.  No data migration is needed because temperatures are calculated relative to `now`.

## 12. Testing Strategy
*   **Unit tests:** Verify the `exp()` calculation matches the expected decay curve for specific time deltas.
*   **Integration tests:** Run BFS on a mock graph and verify that the result set shrinks as simulated time passes.
*   **Benchmark:** Measure query latency with 10k, 100k, and 1M nodes to ensure performance remains within the 50ms window.
*   **Correctness:** Verify that temperature never increases unless an access event occurs (monotonicity check).

---
**See also:** 
* [REQUIREMENTS.md#RNF-1](../REQUIREMENTS.md#RNF-1)
* [milestones/M2-PANDORA.md](../milestones/M2-PANDORA.md)
* [modules/PANDORA.md](../modules/PANDORA.md)
