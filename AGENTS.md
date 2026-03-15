# Pensieve

A shared memory system for AI agents. One brain, every AI.

## Tech Stack

- Rust (2024 edition)
- MCP protocol over stdio (rmcp v0.16)
- SQLite (rusqlite + FTS5 + sqlite-vec) for hybrid retrieval
- fastembed (BGESmallENV15) for vector embeddings
- clap v4 for CLI
- Markdown files with YAML frontmatter as source of truth

## Commands

- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format Rust: `cargo fmt`
- Format Markdown: `npx prettier --write "**/*.md"`
- Run CLI: `cargo run -- <subcommand>`
- Run MCP server: `cargo run -- serve`

## Code Style

- `cargo fmt` enforces formatting (rustfmt.toml config)
- `clippy::pedantic` enabled — fix all warnings
- `unsafe_code = "deny"` — scoped `#[allow(unsafe_code)]` only for sqlite-vec
  FFI
- Concise, no over-engineering
- No unnecessary abstractions
- Prefer editing existing files over creating new ones

## Architecture

- Dual surface: CLI (`src/cli.rs`) + MCP server (`src/mcp.rs`)
- Library crate (`src/lib.rs`) exposes all modules
- Core operations: `src/ops/` (save, read, delete, list, archive, recall,
  inject, reindex, configure, context, end_session, schema, setup, update)
- Storage layer: `src/storage.rs` — markdown read/write with atomic temp+rename
- Index: `src/index.rs` — SQLite with FTS5 (BM25) + vec0 (vector similarity)
- Embedder: `src/embedder.rs` — fastembed singleton via OnceLock+Mutex
- Config: `src/config.rs` — TOML at `~/.config/pensieve/config.toml`
- Types: `src/types.rs` — Memory, MemoryCompact, SessionSummary, PensieveConfig
- Validation: `src/validation.rs` — input hardening for topic_key and project
  names
- Default storage: `~/.pensieve/memory/` (global/, projects/, sessions/)

## CLI Subcommands

- `save` — save/upsert a memory
- `read` — read a memory by topic key
- `recall` — hybrid search (BM25 + vector)
- `list` — list memories with filters
- `delete` — delete a memory
- `archive` — archive or supersede a memory
- `inject` — auto-inject relevant memories (for hook integration)
- `configure` — view/update config (includes `--inject-enabled`)
- `get-context` — session start bootstrap
- `end-session` — save session summary
- `reindex` — rebuild search index
- `schema` — introspect command schemas
- `serve` — start MCP server (stdio)
- `version` — print version
- `update` — self-update from GitHub releases
- `setup` — install setup skill for detected agents

## Configuration

Config at `~/.config/pensieve/config.toml`:

- `memory_dir` — storage location (default: `~/.pensieve/memory/`)
- `[retrieval]` — `keyword_weight` (0.7), `vector_weight` (0.3)
- `[inject]` — `enabled` (false), `relevance_threshold` (0.3), `max_results`
  (3), `format` ("compact")

## Memory Protocol

See `.ai/skills/pensieve-memory-protocol.md` for full protocol. See
`.ai/skills/pensieve-setup.md` for first-time setup. See
`.ai/skills/pensieve-import.md` for importing existing memories. See
`.ai/mcp-configs/README.md` for per-agent MCP configuration.

## Specs

- Feature specs live in `specs/YYYY-MM-DD-description/`
