# Ollama Cloud (MiMi default LLM)

MiMi uses **Ollama Cloud** by default: models run on `https://ollama.com` with an API key. You do **not** need to install the Ollama app or run `ollama serve` locally.

Official docs: [Ollama Cloud](https://docs.ollama.com/cloud).

## Setup

1. Create an API key at [ollama.com/settings/keys](https://ollama.com/settings/keys).
2. Export credentials (same terminal where you run `mimi`):

```bash
export OLLAMA_API_KEY="your_api_key"
export OLLAMA_ENDPOINT="https://ollama.com"
export OLLAMA_MODEL="gemma4:31b"   # API name from https://ollama.com/api/tags
export MIMI_ADAPTER_TYPE=ollama
```

Or copy `.env.development.example` to `.env` and load it.

3. Smoke test with curl:

```bash
curl https://ollama.com/api/chat \
  -H "Authorization: Bearer $OLLAMA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemma4:31b",
    "messages": [{"role": "user", "content": "Say hello in one sentence."}],
    "stream": false
  }'
```

## Model names

| Context | Model identifier |
|---------|------------------|
| REST API (`ollama.com`) | e.g. `gemma4:31b` (from `/api/tags`) |
| Ollama desktop app | often `gemma4:cloud` |

MiMi talks to the **HTTP API**, so use the cloud API tag (e.g. `gemma4:31b`).

## MiMi adapter behavior

- **Cloud**: `OLLAMA_API_KEY` set + endpoint `https://ollama.com` → `Authorization: Bearer` on `/api/chat`.
- **Local** (optional dev only): empty API key + `http://localhost:11434` — not required for normal MiMi setup.

Environment resolution is implemented in `AdapterConfig::from_env()` (`crates/mimi-cli/src/ai/config.rs`). The CLI loads a `.env` file from the project root on startup (`dotenvy`).

## MiMi CLI

```bash
# from repo root, with OLLAMA_API_KEY exported or in .env
cargo run -p mimi-cli -- query "Olá, MiMi!"
cargo run -p mimi-cli -- exec "Summarize Rust in one line"
cargo run -p mimi-cli -- debug adapter-test ollama
```

Use `--output json` for structured responses.

## Features (Ollama platform)

For agents built on top of MiMi, Ollama also documents:

- [Streaming](https://docs.ollama.com/streaming) — partial `content` / `thinking` chunks
- [Thinking models](https://docs.ollama.com/thinking) — `think` parameter
- [Tool calling](https://docs.ollama.com/tool-calling) — `tools` in chat API
- [Web search](https://docs.ollama.com/web-search) — requires `OLLAMA_API_KEY`

**Note:** [Structured outputs](https://docs.ollama.com/structured-outputs) are not supported on Ollama Cloud at the time of writing.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| 401 Unauthorized | Check `OLLAMA_API_KEY` |
| Model not found | List models: `curl https://ollama.com/api/tags` |
| Connection refused to `11434` | You are pointing at local Ollama; switch to `https://ollama.com` |

See also [TROUBLESHOOTING.md](./TROUBLESHOOTING.md).
