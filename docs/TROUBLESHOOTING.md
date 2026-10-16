# Troubleshooting

## Build

**Linker `x86_64-linux-gnu-gcc` not found (Arch / CachyOS)**

Arch provides `/usr/bin/gcc`, not the Debian triplet name. Install tooling and use the project linker config:

```bash
sudo pacman -S --needed base-devel gcc pkgconf openssl
```

This repository uses `.cargo/config.toml` with `linker = "cc"`. If the error persists, check for overrides:

```bash
echo "$RUSTFLAGS"
```

Remove any `-C linker=x86_64-linux-gnu-gcc` from `~/.zshrc` or `/etc/environment`.

**Linker / `gcc` not found (other distros)**

Install a C toolchain (`build-essential` on Debian/Ubuntu) and retry `cargo build`.

**OpenSSL errors**

On Debian-based images, install `pkg-config` and `libssl-dev` (see root `Dockerfile` builder stage).

## Docker Compose

**Neo4j unhealthy**

- Wait for `start_period` (30s) on first boot.
- Check logs: `docker logs mimi-neo4j`
- Default auth: `neo4j` / `mimi-password` (change in production).

**`neo4j-apply-schema.sh`: Connection refused**

Neo4j was still starting. Wait ~30–60s after `docker compose up -d neo4j`, then re-run `./scripts/neo4j-apply-schema.sh` (the script now waits for Bolt automatically).

**Redis health check fails**

Redis requires password `mimi-redis-password` in compose. Use the same in `REDIS_URL`.

**`mimi-core` container exits**

The image runs `/app/mimi --help` by default. Override `command` in compose for long-running modes (e.g. REPL). HTTP health on port 8080 is planned when the HTTP server entrypoint is wired.

## Neo4j schema

**`cypher-shell` not found in container**

Use the apply script from the host: `./scripts/neo4j-apply-schema.sh` (requires running `mimi-neo4j`).

**Constraint already exists**

Scripts use `IF NOT EXISTS`; safe to re-run `schema.cypher`.

## CLI

**`mimi exec` hangs after the AI response**

An Odlaguna background supervisor must not run for one-shot `mimi exec` (it keeps the Tokio runtime alive). Current behavior: only `mimi daemon` and scoped `debug skill-run` start the supervisor loop. If you still see duplicate `register`/`complete` logs, ensure you are on a build that does not call `start_odlaguna_supervisor` from `handle_exec`. For continuous watchdog + flush, use `mimi daemon`.

**Adapter errors / timeouts**

- Ollama Cloud: set `OLLAMA_API_KEY` and `OLLAMA_ENDPOINT=https://ollama.com` (not `localhost:11434`).
- Test the key: see [OLLAMA-CLOUD.md](./OLLAMA-CLOUD.md).
- Optional Gemini fallback: `GEMINI_API_KEY`.
- Run `mimi debug status` (when backend is connected).
- Increase adapter timeout in config; see `M1.5.5-PERFORMANCE-MONITORING.md`.

**Invalid state transition**

The FSM rejects illegal transitions. Ensure the core is not forcing `Degraded`/`Recovering` via failed health checks.

## Tests

**E2E tests flaky**

Some e2e suites use mocks and temp config; run with:

```bash
cargo test -p mimi-cli --test e2e_cli_workflow -- --test-threads=1
```

**Zenoh integration**

Live Zenoh tests need `docker compose up -d zenoh`. Mock adapter tests do not require Zenoh.

## Getting help

- Architecture: [ARCHITECTURE-OVERVIEW.md](./ARCHITECTURE-OVERVIEW.md)
- Deploy: [DEPLOYMENT.md](./DEPLOYMENT.md)
- GitHub issues: project repository
