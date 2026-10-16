# Deployment Guide (M1.6.4)

## Prerequisites

- Docker 24+ and Docker Compose v2
- 4 GB RAM minimum (Neo4j heap + Redis + Zenoh)
- TLS termination at reverse proxy (production)
- Secrets manager or sealed `.env.production` (never commit real secrets)

## Checklist

- [ ] Copy `.env.production.example` → `.env.production` and rotate all passwords
- [ ] `docker compose --env-file .env.production build mimi-core`
- [ ] `docker compose --env-file .env.production up -d zenoh neo4j redis`
- [ ] Wait until Zenoh, Neo4j, and Redis health checks pass
- [ ] `./scripts/neo4j-apply-schema.sh` (set `NEO4J_PASSWORD` if changed)
- [ ] `docker compose --env-file .env.production up -d mimi-core`
- [ ] Verify: `docker exec mimi-core /app/mimi --version`
- [ ] Run smoke tests: `cargo test -p mimi-cli --test e2e_full_stack` (from CI or build host)
- [ ] Configure log shipping (`RUST_LOG`, JSON subscriber when enabled)
- [ ] Document rollback: pin image tag / `docker compose down` + volume backup

## Build image

```bash
docker compose build mimi-core
```

The `Dockerfile` multi-stage build compiles `mimi-cli` and installs `/app/mimi`.

## Production compose

Default file: `docker-compose.yml`. Override ports and debug logging only in `docker-compose.override.yml` (local).

```bash
docker compose -f docker-compose.yml up -d
```

## Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ZENOH_ENDPOINT` | Yes | Zenoh router address |
| `NEO4J_URI` | Yes | Bolt URI for Pandora |
| `NEO4J_USER` / `NEO4J_PASSWORD` | Yes | Neo4j credentials |
| `REDIS_URL` | Yes | Redis with password |
| `RUST_LOG` | No | `info` recommended in prod |
| `OLLAMA_API_KEY` | Yes (default LLM) | Ollama Cloud key |
| `OLLAMA_ENDPOINT` | No | Default `https://ollama.com` |
| `OLLAMA_MODEL` | No | e.g. `gemma4:31b` |
| `GEMINI_API_KEY` | Optional | Fallback adapter |
| `MIMI_AUTH_SECRET` | Yes (multi-user) | JWT signing secret |

See `.env.production.example`.

## Neo4j backup

```bash
docker exec mimi-neo4j neo4j-admin database dump neo4j --to-path=/backups
docker cp mimi-neo4j:/backups ./neo4j-backups
```

Restore: use `neo4j-admin database load` per Neo4j 5 docs.

## Rollback

1. `docker compose stop mimi-core`
2. Deploy previous image digest
3. Restore Neo4j dump if schema migrations were applied

## Related

- [MONITORING.md](./MONITORING.md)
- [SCALING.md](./SCALING.md)
- [QUICKSTART.md](./QUICKSTART.md)
