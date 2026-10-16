# Monitoring & Health

## Docker health checks

| Service | Check |
|---------|--------|
| Zenoh | HTTP `GET /info` on 7447 |
| Neo4j | HTTP 7474 |
| Redis | `redis-cli incr ping` |
| mimi-core | `/app/mimi --version` |

Inspect: `docker compose ps` and `docker inspect --format='{{json .State.Health}}' <container>`.

## Application metrics (M1)

Implemented in-tree (not yet exported to Prometheus by default):

- **State machine / tasks**: logging via `tracing` and `log` in `mimi-core`
- **Adapter performance**: `PerformanceTracker`, `PerformanceMetrics` (p50/p95/p99, throughput) in `mimi-cli`
- **Health monitor**: `HealthMonitor` in `mimi-core`

Integration tests: `crates/mimi-cli/tests/ai_performance_integration.rs`.

## Recommended production signals

| Signal | Source | Action |
|--------|--------|--------|
| Container restart count | Docker / orchestrator | Investigate logs |
| Neo4j heap pressure | Neo4j metrics | Increase `NEO4J_dbms_memory_heap_max__size` |
| Redis evictions | `INFO stats` | Increase memory or tune TTL |
| Adapter error rate | App logs / tracker | Failover via registry |
| Task queue depth | Core metrics (when wired) | Scale workers |

## Log levels

```bash
RUST_LOG=mimi_core=debug,mimi_cli=info docker compose up mimi-core
```

## Future (optional)

- Prometheus exporter for task latency and adapter SLIs
- Grafana dashboards for Zenoh + Neo4j (see compose labels)
