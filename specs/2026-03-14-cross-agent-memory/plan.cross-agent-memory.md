---
date: 2026-03-15T04:58:01Z
git_commit: 19872a1
branch: main
repository: pensieve
spec: spec.cross-agent-memory.md
research: research.cross-agent-memory.md
worktrees:
  - /Users/rigo/Documents/Projects/worktrees/cross-agent-memory/implement/pensieve/
status: complete
last_updated: 2026-03-15
---

# Implementation Plan: Pensieve — Cross-Agent Memory

**Spec**: spec.cross-agent-memory.md **Research**:
research.cross-agent-memory.md **Date**: 2026-03-15T04:58:01Z

## Overview

Build a standalone Rust binary (`pensieve`) that provides cross-agent memory via
CLI + MCP dual surface. Markdown files are the source of truth with a SQLite
sidecar for hybrid BM25 + vector retrieval. The plan is structured in 5 phases:
foundation, core operations, retrieval engine, surfaces (CLI + MCP), and
documentation/skill files.

## Progress

- [x] Worktree setup
- [x] Phase 1: Foundation (deps, config, storage, types)
- [x] Phase 2: Core operations (save, read, delete, list, archive)
- [x] Phase 3: Hybrid retrieval engine (FTS5 + vector + reindex)
- [x] Phase 4: Surfaces (CLI with clap, MCP with rmcp)
- [x] Phase 5: Context, sessions, and advanced features
- [x] Phase 6: Documentation, skill files, and configs
- [x] Testing
- [x] AGENTS.md check
- [x] Spec update
- [x] Worktree teardown

## Worktree Setup

| Repo       | Worktree path                                                                     | Base branch |
| ---------- | --------------------------------------------------------------------------------- | ----------- |
| `pensieve` | `/Users/rigo/Documents/Projects/worktrees/cross-agent-memory/implement/pensieve/` | `main`      |

_Worktree created with
`git worktree add ../worktrees/cross-agent-memory/implement/pensieve -b cross-agent-memory`
from the pensieve repo root._

---

## Phase 1: Foundation

_Why first: Everything else depends on the data types, config system, storage
layout, and dependencies compiling together._

### 1.1 Add dependencies to Cargo.toml

**What changes**: `Cargo.toml` — add all dependencies from the spec's
Implementation Notes. **How to verify**: `cargo build` succeeds with all deps.
`cargo clippy` passes.

Update `Cargo.toml` with:

```toml
[dependencies]
rmcp = { version = "0.16", features = ["server", "transport-io"] }
fastembed = "5.12"
rusqlite = { version = "0.38", features = ["bundled"] }
sqlite-vec = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
toml = "0.8"
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
tempfile = "3"
reqwest = { version = "0.12", features = ["json"] }
sha2 = "0.10"
thiserror = "2"

[lints.rust]
unsafe_code = "deny"

[lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
```

Note: `tempfile` for atomic writes, `reqwest` for self-update and version check,
`sha2` for checksum verification, `thiserror` for error types.

### 1.2 Define core types and error handling

**What changes**: Create `src/types.rs` and `src/error.rs` **How to verify**:
`cargo build` passes. Types are importable.

`src/types.rs` — Rust structs for:

- `MemoryType` enum: `Gotcha`, `Decision`, `Preference`, `Discovery`,
  `HowItWorks`
- `MemoryStatus` enum: `Active`, `Archived`, `Superseded`
- `Confidence` enum: `High`, `Medium`, `Low`
- `Memory` struct: all frontmatter fields + `content: String`
- `MemoryCompact` struct: title, type, project, topic_key, updated, status,
  score, first 2 lines (for recall results)
- `SessionSummary` struct: summary, key_decisions, source, project, date
- `PensieveConfig` struct: `memory_dir`, `retrieval.keyword_weight`,
  `retrieval.vector_weight`

All types derive `Serialize`, `Deserialize`, `Debug`, `Clone`.

`src/error.rs` — `PensieveError` enum using `thiserror`:

- `Io(#[from] std::io::Error)`
- `Sqlite(#[from] rusqlite::Error)`
- `Yaml(#[from] serde_yaml::Error)`
- `Json(#[from] serde_json::Error)`
- `Config(String)`
- `InvalidInput(String)` — for input hardening rejections
- `RevisionConflict { expected: u32, actual: u32 }`
- `NotFound(String)`
- `EmbeddingError(String)`

### 1.3 Config system

**What changes**: Create `src/config.rs` **How to verify**: Config loads from
`~/.config/pensieve/config.toml`. Missing config creates defaults. Env var
`PENSIEVE_MEMORY_DIR` overrides. CLI `--memory-dir` overrides both.

Implementation:

- `load_config()` — reads `~/.config/pensieve/config.toml` if exists, else
  returns defaults (`~/.pensieve/memory/`, keyword_weight=0.7,
  vector_weight=0.3)
- `save_config(config)` — writes TOML to `~/.config/pensieve/config.toml`,
  creates parent dirs
- Priority: CLI flag > env var > config file > defaults
- `is_unconfigured()` — returns true if no config file exists (for first-run
  notice)

### 1.4 Input validation module

**What changes**: Create `src/validation.rs` **How to verify**:
`validate_topic_key("good-key")` passes. `validate_topic_key("../etc/passwd")`
returns `InvalidInput` error. `validate_topic_key("hello world?")` returns
error.

Implementation:

- `validate_topic_key(s: &str) -> Result<(), PensieveError>` — only `[a-z0-9-]`
- `validate_project_name(s: &str) -> Result<(), PensieveError>` — same rules
- Both reject: `../`, control chars (< 0x20), `?`, `#`, `%`, spaces, uppercase

### 1.5 Storage layer (markdown read/write)

**What changes**: Create `src/storage.rs` **How to verify**: Write a Memory to
disk, read it back, verify all frontmatter fields roundtrip correctly.

Implementation:

- `resolve_path(config, topic_key, project) -> PathBuf` — returns
  `{memory_dir}/projects/{project}/{topic_key}.md` or
  `{memory_dir}/global/{topic_key}.md`
- `write_memory(config, memory) -> Result<()>` — serialize to YAML frontmatter +
  markdown body, write to temp file, atomic rename into place
- `read_memory(config, topic_key, project) -> Result<Memory>` — parse YAML
  frontmatter + markdown body
- `delete_memory(config, topic_key, project) -> Result<()>` — remove file
- `list_memory_files(config, project, type_filter, status_filter) -> Result<Vec<Memory>>`
  — walk directories, parse frontmatter only (skip body for speed)
- `ensure_dirs(config)` — create `global/`, `projects/`, `sessions/` if missing

Frontmatter parsing: use `serde_yaml` to parse the YAML between `---` markers.
Body is everything after the second `---`.

---

## Phase 2: Core Operations

_Why after Phase 1: These depend on types, config, validation, and storage.
Phase 2 implements markdown-only operations (no index calls). Index integration
is added in Phase 3 after the index module exists._

### 2.1 `save_memory` operation

**What changes**: Create `src/ops/save.rs` **How to verify**: Save creates file.
Save again with same topic_key increments revision. `--expected-revision`
mismatch returns conflict error. `--dry-run` returns what would be written
without writing.

Implementation:

- Validate topic_key and project via `validation.rs`
- If file exists: read current revision. If `expected_revision` provided and
  doesn't match, return `RevisionConflict`. Otherwise increment revision,
  preserve `created` timestamp, update `updated`.
- If file doesn't exist: revision=1, set `created` and `updated` to now.
- If `dry_run`: return the Memory that would be written, don't write.
- Call `storage::write_memory` for the markdown file — this always succeeds
  (source of truth).
- Attempt `index::upsert` to update SQLite index — best-effort. If embedding
  fails (model not downloaded, offline), save still succeeds with a warning.
  Unindexed memories are picked up by `pensieve reindex`.
- Note: In Phase 2, implement save without index calls (markdown only). Index
  integration is added in Phase 3 when the index module exists.

### 2.2 `read_memory` operation

**What changes**: Create `src/ops/read.rs` **How to verify**: Read by topic_key
returns full memory content. Missing topic_key returns `NotFound` error.

### 2.3 `delete_memory` operation

**What changes**: Create `src/ops/delete.rs` **How to verify**: Delete removes
file. Delete with `--dry-run` shows what would be deleted. In Phase 2, deletes
markdown only. Index cleanup added in Phase 3.

### 2.4 `list_memories` operation

**What changes**: Create `src/ops/list.rs` **How to verify**: List returns all
memories. `--project`, `--type`, `--status` filters work. Default excludes
nothing (shows all statuses).

### 2.5 `archive_memory` operation

**What changes**: Create `src/ops/archive.rs` **How to verify**: Archive sets
status=archived. With `--superseded-by`, sets status=superseded and
superseded_by field. `--dry-run` works.

### 2.6 `configure` operation

**What changes**: Create `src/ops/configure.rs` **How to verify**: With
`--memory-dir`, updates config file and ensures the new directory exists.
Without args, returns current config. In MCP serve mode, after
`configure(memory_dir)`, the server closes the old index and reopens at the new
`{memory_dir}/index.sqlite` path. If no index exists at the new path, a fresh
one is created (user should run `pensieve reindex` to populate it).

---

## Phase 3: Hybrid Retrieval Engine

_Why after Phase 2: Retrieval depends on the index being populated by save
operations._

### 3.1 SQLite index initialization

**What changes**: Create `src/index.rs` **How to verify**: Opening a new index
creates FTS5 and vec0 tables. Opening an existing index reuses them.

Implementation:

- `#[allow(unsafe_code)]` on this module only for sqlite-vec init
- `Index::open(memory_dir) -> Result<Index>` — open or create
  `{memory_dir}/index.sqlite` (index lives inside memory_dir, moves with
  reconfiguration)
- Register sqlite-vec extension via `sqlite3_auto_extension`
- Create tables if not exist:
  ```sql
  CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
    memory_id, title, content, project, tags
  );
  CREATE VIRTUAL TABLE IF NOT EXISTS memory_vec USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding float[384]
  );
  ```
  `memory_id` is the composite key `"{project_or_global}/{topic_key}"` (e.g.
  `"projects/jarvis/horizon-cli-scope"` or `"global/notebook-cell-format"`).
  This prevents collisions between `global/foo` and `projects/jarvis/foo`.

### 3.2 Index upsert and delete

**What changes**: Add `upsert` and `delete` methods to `Index` **How to
verify**: After save_memory, both FTS5 and vec0 tables contain the entry. After
delete_memory, both tables are cleaned.

Implementation:

- `Index::upsert(memory_id, memory, embedding)` — delete old rows by `memory_id`
  if exist, insert into both FTS5 and vec0. `memory_id` is the composite
  `"{project_or_global}/{topic_key}"`.
- `Index::delete(memory_id)` — remove from both tables by composite key
- Embedding comes from fastembed (3.3)

### 3.3 Embedding with fastembed

**What changes**: Create `src/embedder.rs` **How to verify**: Embedding a string
returns a `Vec<f32>` of length 384. First call triggers model download (~33MB,
~10-30s).

Implementation:

- `Embedder::new() -> Result<Embedder>` — init `TextEmbedding` with
  `BGESmallENV15Q`
- `Embedder::embed(text: &str) -> Result<Vec<f32>>` — embed single text
- Singleton pattern: `get_embedder() -> &Embedder` via `OnceLock`
- Embed combined text: `"{title}: {content}"` (matching jarvis server pattern)

### 3.4 Hybrid recall

**What changes**: Add `recall` method to `Index` **How to verify**: Keyword
search finds "ONNX" when content contains "ONNX". Vector search finds "how we
ship to prod" when query is "deployment process". Blended scores are computed
correctly.

Implementation:

- `Index::recall(query, filters, config) -> Result<Vec<MemoryCompact>>`
- BM25 search:
  `SELECT topic_key, bm25(memory_fts) AS score FROM memory_fts WHERE memory_fts MATCH ?`
- Vector search: embed query, then
  `SELECT topic_key, distance FROM memory_vec WHERE embedding MATCH ?`
- Normalize both score sets to [0, 1]
- Blend:
  `final = keyword_weight * bm25_norm + vector_weight * (1 - vec_distance)`
- Apply filters (type, project, tags, since, status) — read frontmatter from
  markdown files for filtered fields not in the index
- Return top `limit` results sorted by final score

### 3.5 Integrate index into save/delete operations

**What changes**: Update `src/ops/save.rs` and `src/ops/delete.rs` from Phase 2
**How to verify**: After save, memory appears in both FTS5 and vec0. After
delete, memory is removed from both. If embedding fails, save still succeeds
(markdown persisted, warning returned).

Add best-effort `index::upsert` call to save, `index::delete` call to delete.
Wrap in error handling that logs warnings but doesn't fail the operation.

### 3.6 Reindex command

**What changes**: Create `src/ops/reindex.rs` **How to verify**: Delete
`index.sqlite`, run reindex, recall still works.

Implementation:

- Delete existing `index.sqlite`
- Walk all markdown files via `storage::list_memory_files`
- For each: parse content, embed, insert into new index
- Print progress: `Reindexed {n}/{total} memories`

---

## Phase 4: Surfaces (CLI + MCP)

_Why after Phase 3: Both surfaces call the same core operations and need
retrieval working._

### 4.1 CLI with clap

**What changes**: Rewrite `src/main.rs`, create `src/cli.rs` **How to verify**:
All CLI subcommands from the spec work. `--output json` returns valid JSON.
`--json` accepts payloads. `--dry-run` works on mutating commands.

Implementation:

- `src/cli.rs` — clap derive structs:
  - `#[derive(Parser)] struct Cli` with subcommands enum
  - Each subcommand struct has flags matching the spec
  - Global flags: `--output json`, `--memory-dir`
  - `--json <VALUE>` handling: if starts with `@`, read file; if `-`, read
    stdin; else parse as inline JSON
- `src/main.rs` — `#[tokio::main] async fn main()`:
  - Parse CLI args
  - Load config
  - Match on subcommand, call appropriate `src/ops/` function
  - Format output (human or JSON based on `--output`)

### 4.2 MCP server with rmcp

**What changes**: Create `src/mcp.rs` **How to verify**: `pensieve serve` starts
MCP server. All 9 tools callable via MCP protocol. Results match CLI
equivalents.

Implementation:

- `PensieveServer` struct holds config + index + embedder references
- `#[tool_handler] impl ServerHandler for PensieveServer`
- 9 tool methods with `#[tool]` macro:
  - `save_memory`, `recall`, `get_context`, `end_session`, `read_memory`,
    `delete_memory`, `list_memories`, `archive_memory`, `configure`
  - Each tool calls the same `src/ops/` functions as CLI
  - `dry_run` parameter on mutating tools
- `serve` subcommand: `PensieveServer::new().serve(stdio_transport).await`

### 4.3 Schema introspection

**What changes**: Create `src/ops/schema.rs` **How to verify**:
`pensieve schema save` prints JSON with all parameters, types, and descriptions.

Implementation:

- For each subcommand, output a JSON object with: name, description, parameters
  (name, type, required, description, default)
- Derive from clap's `CommandFactory` trait or build manually from the arg
  definitions

### 4.4 Version command

**What changes**: Add to `src/cli.rs` **How to verify**: `pensieve version`
prints `pensieve 0.1.0`.

Use `env!("CARGO_PKG_VERSION")`.

---

## Phase 5: Context, Sessions, and Advanced Features

_Why after Phase 4: These are higher-level features that compose the core ops._

### 5.1 `end_session` operation

**What changes**: Create `src/ops/end_session.rs` **How to verify**: Creates
file at `sessions/{date}-{project}-{source}.md`. Content includes summary and
key decisions.

Implementation:

- Validate project if provided
- Generate filename: `{YYYY-MM-DD}T{HHMMSS}-{project}-{source}.md` (ISO
  timestamp prevents same-day overwrites; project and source default to
  "unknown")
- Session files use a reduced frontmatter set: `title` (auto-generated from
  date), `source`, `project`, `created`. Body is the summary + key decisions.
  Sessions are not part of the 5-type memory taxonomy — they are a separate
  storage tier in `sessions/` and are not indexed in FTS5/vec0.

### 5.2 `get_context` operation

**What changes**: Create `src/ops/context.rs` **How to verify**: Returns last 3
sessions, all active preferences, recent gotchas/decisions (30 days). Excludes
archived/superseded. Flags stale (>90 days). Writes CONTEXT.md. If unconfigured,
includes notice. Version check included (best-effort).

Implementation:

- Read all memory files via `storage::list_memory_files`
- Filter: status=active only
- Collect: all preferences, gotchas/decisions updated in last 30 days
- Read last 3 session files from `sessions/` sorted by date
- Flag any memory with `updated` > 90 days ago as "potentially stale"
- Assemble into structured response (and CONTEXT.md text)
- Write CONTEXT.md to `{memory_dir}/CONTEXT.md` (truncate to 200 lines)
- If `config.is_unconfigured()`, prepend notice
- Version check: performed first, before assembling context. Check GitHub
  releases API with 2s timeout. Cache result for 24h in
  `~/.config/pensieve/version_cache.json`. If outdated, include notice in the
  assembled response. If network fails or times out, silently skip — never block
  context assembly.

### 5.3 Self-update

**What changes**: Create `src/ops/update.rs` **How to verify**:
`pensieve update` checks GitHub Releases, downloads binary if newer, verifies
SHA-256 checksum, replaces self.

Implementation:

- Fetch latest release from
  `https://api.github.com/repos/rigogsilva/pensieve/releases/latest`
- Compare version tag to `env!("CARGO_PKG_VERSION")`
- If newer: download platform-appropriate binary asset + `checksums.txt`
- Compute SHA-256 of downloaded binary, compare to checksums.txt entry
- If match: replace current binary (write to temp, rename into place)
- If mismatch: error, do not replace

---

## Phase 6: Documentation, Skill Files, and Configs

_The following steps are independent and may be completed in parallel._

### 6.1 MCP config snippets

**What changes**: Create `.ai/mcp-configs/` directory **How to verify**: Each
JSON file is valid and points to `pensieve serve`.

Files to create:

- `.ai/mcp-configs/README.md` — setup guide with install instructions and
  per-agent snippets
- `.ai/mcp-configs/claude.json`
- `.ai/mcp-configs/codex.json`
- `.ai/mcp-configs/cursor.json`
- `.ai/mcp-configs/copilot.json`

Each config JSON points command to `pensieve` binary with args `["serve"]`.

### 6.2 Skill files

**What changes**: Create `.ai/skills/` directory **How to verify**: Each skill
file has valid YAML frontmatter and covers the topics listed in the spec.

Files to create:

- `.ai/skills/pensieve-memory-protocol.md` — full Memory Protocol
- `.ai/skills/pensieve-setup.md` — first-time setup guidance
- `.ai/skills/pensieve-import.md` — agent-driven memory import with Claude
  mapping table from spec R4

### 6.3 Update AGENTS.md

**What changes**: Update `AGENTS.md` at repo root **How to verify**: Contains
accurate tech stack, commands, architecture, and Memory Protocol reference.

Add: specific crate names, module structure, memory protocol summary, link to
skill files.

---

## Testing

All tests in `tests/` directory using Rust's built-in test framework. Each test
creates a temp directory for isolation.

- **Unit: save** → maps to step 2.1. Create, upsert (revision increment),
  expected_revision conflict, dry-run.
- **Unit: read** → maps to step 2.2. Read existing, read missing (NotFound).
- **Unit: delete** → maps to step 2.3. Delete existing, delete missing, dry-run.
- **Unit: list** → maps to step 2.4. List all, filter by project/type/status.
- **Unit: archive** → maps to step 2.5. Archive, supersede with superseded_by,
  dry-run.
- **Unit: configure** → maps to step 2.6. Set memory_dir, read config, env var
  override.
- **Unit: validation** → maps to step 1.4. Path traversals, control chars,
  special chars, valid keys.
- **Unit: storage roundtrip** → maps to step 1.5. Write memory, read back,
  verify all fields.
- **Hybrid retrieval** → maps to steps 3.1-3.4. Save "how we ship to prod",
  recall "deployment process" (semantic). Save "ONNX runtime error", recall
  "ONNX" (keyword).
- **Reindex** → maps to step 3.5. Delete index.sqlite, reindex, verify recall
  works.
- **CLI integration** → maps to step 4.1. Full save→recall→list→archive cycle
  via CLI with `--output json`.
- **MCP integration** → maps to step 4.2. Full cycle via MCP protocol.
- **Surface parity** → maps to steps 4.1 + 4.2. Same operation via CLI and MCP,
  compare JSON output.
- **Lifecycle** → maps to steps 2.1 + 2.5 + 5.2. Save → supersede → verify
  get_context excludes → list with status filter shows it.
- **Staleness** → maps to step 5.2. Save memory with old timestamp, get_context
  flags it.
- **Context regeneration** → maps to step 5.2. Create many memories, verify
  CONTEXT.md stays under 200 lines.
- **Config** → maps to steps 1.3 + 2.6. First-run auto-bootstrap, configure
  command, env var, custom path.
- **Concurrency** → maps to step 2.1. Two saves with same expected_revision=1 —
  one succeeds, one conflicts. Without expected_revision, both succeed.
- **Dry-run** → maps to steps 2.1 + 2.3 + 2.5. Verify no files written.
- **Input hardening** → maps to step 1.4. Full suite of invalid inputs.
- **Schema** → maps to step 4.3. `pensieve schema save` returns valid JSON.
- **Import skill** → maps to step 6.2. Manual walkthrough of pensieve-import.md.

---

## Risk Callouts

> **⚠ Research gap**: `sqlite-vec` requires `unsafe` FFI code, but the project
> currently has `unsafe_code = "forbid"` in Cargo.toml. The spec resolves this
> by changing to `unsafe_code = "deny"` with scoped `#[allow(unsafe_code)]`.
> Verify this pattern works with clippy::pedantic during step 3.1.

> **⚠ Research gap**: `fastembed` downloads a ~33MB model on first use. If the
> CI environment has no network or HuggingFace is unreachable, embedding tests
> will fail. Consider caching the model in CI or mocking the embedder in unit
> tests.

> **⚠ Research gap**: The `self_update` crate was mentioned in research open
> questions. Step 5.3 rolls a custom implementation. If this proves complex,
> consider using the `self_update` crate instead.

---

## Closing Steps

- [ ] **Check `AGENTS.md`**: Review `AGENTS.md` at the repository root. This
      change introduces new CLI commands, architecture details, and development
      patterns. Update AGENTS.md to reflect the current state: crate
      dependencies, module structure (`src/types.rs`, `src/ops/`,
      `src/index.rs`, etc.), all CLI commands, Memory Protocol reference.

- [ ] **Update spec**: Once implementation is complete, update the
      `Implementation Notes` section of spec.cross-agent-memory.md with: (a)
      implementation decisions made and their rationale, (b) new patterns or
      conventions introduced, (c) the testing approach used and any coverage
      notes, (d) anything that would change how a future reader interprets or
      extends this spec.

- [ ] **Worktree teardown**: After all implementation work is verified:
  ```bash
  cd /Users/rigo/Documents/Projects/worktrees/cross-agent-memory/implement/pensieve
  git add -A
  git commit -m "cross-agent-memory: implement pensieve CLI + MCP memory server"
  git push --set-upstream origin cross-agent-memory
  git -C /Users/rigo/Documents/Projects/pensieve worktree remove /Users/rigo/Documents/Projects/worktrees/cross-agent-memory/implement/pensieve
  ```
