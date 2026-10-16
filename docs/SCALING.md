# Scaling Guide

MiMi M1 is designed as a modular stack; scale each tier independently.

## Zenoh

- Run dedicated Zenoh routers for cross-host peers.
- Increase connection limits and partition topics (`mimi/commands/*`, `mimi/events/*`).
- Keep publishers close to producers (same AZ / host network).

## Neo4j

- **Vertical**: raise heap and page cache (`NEO4J_dbms_memory_*` in compose).
- **Horizontal**: Neo4j Enterprise clustering (out of scope for community compose); for M2 start with read replicas or sharded graphs by tenant.
- Index hot properties (`heat`, `last_accessed`) — see `docker/neo4j/schema.cypher`.

## Redis

- Increase `maxmemory` and choose `allkeys-lru` (default in compose).
- Use Redis Sentinel/Cluster for HA in production.

## Mimi core / Beatrice

- Stateless CLI: scale by running more client instances.
- Long-running REPL/API: one process per instance; sticky sessions for WebSocket.
- AI adapters: registry supports multiple backends; route by priority and health.

## Benchmark-driven capacity

Run before scaling decisions:

```bash
cargo bench -p mimi-core
cargo bench -p mimi-cli --bench startup_benchmarks
```

Compare Criterion reports over time; targets from M1.4.8 auth/load tests remain valid baselines for the CLI auth path.
