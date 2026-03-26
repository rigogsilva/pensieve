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

## Before Pushing

Run these before committing/pushing to avoid CI failures:

```bash
cargo fmt
npx prettier --write "**/*.md"
cargo test
cargo clippy
```

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
- Core operations: `src/ops/` (save, read, delete, list, archive, recall, prime,
  inject (alias), reindex, configure, context, end_session, schema, setup,
  update)
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
- `prime` — prime context with relevant memories (for hook integration);
  `inject` is a hidden alias for backward compat
- `configure` — view/update config (includes `--prime-enabled`)
- `get-context` / `context` — session start bootstrap; writes
  `~/.pensieve/memory/MEMORY.md` (global-scoped memories only) and
  `projects/{project}/MEMORY.md` (per-project, when `--project` is given);
  returns `{global_index, project_index, sessions, notice}`
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
- `[prime]` — `enabled` (false), `relevance_threshold` (0.3), `max_results` (3),
  `format` ("compact")

## Memory Protocol

See `.ai/skills/pensieve-setup.md` for the canonical Memory Protocol and
first-time setup. See `.ai/skills/pensieve-import.md` for importing existing
memories. See `.ai/skills/nightly-extraction/SKILL.md` for session transcript
extraction. See `.ai/mcp-configs/README.md` for per-agent MCP configuration.

## README

Keep `README.md` in sync with the code. When adding or changing features,
commands, or skills, update the README to match. The README is the primary
documentation for users.

## Specs

- Feature specs live in `specs/YYYY-MM-DD-description/`
