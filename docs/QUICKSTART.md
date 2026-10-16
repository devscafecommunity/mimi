# Quick Start

Get the MiMi foundation stack running locally and run your first CLI commands.

## Prerequisites

- Rust toolchain (stable)
- Docker and Docker Compose
- [Ollama Cloud](https://docs.ollama.com/cloud) API key ([create key](https://ollama.com/settings/keys)) — no local Ollama install

### Arch Linux / CachyOS

```bash
sudo pacman -S --needed base-devel gcc pkgconf openssl rust docker docker-compose
sudo systemctl enable --now docker
```

Add your user to the `docker` group (log out and back in afterward):

```bash
sudo usermod -aG docker "$USER"
```

If `cargo build` fails with `linker x86_64-linux-gnu-gcc not found`, this repo sets the linker to `cc` in `.cargo/config.toml`. You still need `base-devel` / `gcc` installed.

For Pandora (C++ / M2 later):

```bash
sudo pacman -S --needed cmake clang
```

## 1. Clone and build

```bash
git clone https://github.com/devscafecommunity/mimi.git
cd mimi
cargo build --release
```

The `mimi` binary is produced from `crates/mimi-cli`.

## 2. Start infrastructure

```bash
cp .env.development.example .env   # optional: edit secrets
docker compose up -d zenoh neo4j redis
```

Services:

| Service | Ports | Purpose |
|---------|-------|---------|
| Zenoh | 7447–7449 | Message bus |
| Neo4j | 7474, 7687 | Graph memory (Pandora) |
| Redis | 6379 | Cache |

Apply Neo4j schema and seed (M2.1):

```bash
./scripts/neo4j-apply-schema.sh
```

## 3. First commands

```bash
cargo run -p mimi-cli -- --version
cargo run -p mimi-cli -- query "What is MiMi in one sentence?" --include-context --limit 10
cargo run -p mimi-cli -- debug adapter-test ollama
cargo run -p mimi-cli -- repl
```

With `OLLAMA_API_KEY` set (or a `.env` file in the project root), `query` and `exec` call Ollama Cloud directly.

## 4. Configuration

```bash
export OLLAMA_API_KEY="your_key"   # from ollama.com/settings/keys
export OLLAMA_MODEL="gemma4:31b"
cargo run -p mimi-cli -- config list
cargo run -p mimi-cli -- config init development
```

Defaults target **Ollama Cloud** (`https://ollama.com`). See [OLLAMA-CLOUD.md](./OLLAMA-CLOUD.md). Optional Gemini fallback: `GEMINI_API_KEY`.

## 5. Tests

```bash
cargo test -p mimi-core
cargo test -p mimi-cli
cargo test -p mimi-cli --test e2e_cli_workflow
```

## 6. Benchmarks

See [M1.6.2-BENCHMARKS.md](./M1.6.2-BENCHMARKS.md).

## Next steps

- [DEPLOYMENT.md](./DEPLOYMENT.md) for production compose
- [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) for common failures
- [M2.1-NEO4J-SCHEMA.md](./M2.1-NEO4J-SCHEMA.md) for Pandora graph model
