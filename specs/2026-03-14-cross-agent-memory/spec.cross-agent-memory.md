# Pensieve — Cross-Agent Memory

## Overview

Every AI agent starts from zero. Claude Code has memory files locked in
`~/.claude/projects/*/memory/` — invisible to Codex, Cursor, Copilot, Gemini
CLI.

**Pensieve** is a local-first, cross-agent memory system. Markdown files as
source of truth, hybrid retrieval (BM25 + vector), and a single Rust binary that
any agent can use via CLI, MCP, or direct file access. Practical shared memory
for personal agent workflows — accessible to every AI tool, not locked to one.

**Goal**: One brain, every AI. A standalone Rust binary that any teammate can
install and any agent can use. Markdown files as storage (human-readable,
git-friendly, zero deps, browsable). `AGENTS.md` teaches every agent the Memory
Protocol. `CONTEXT.md` bootstraps every session with zero agent action.

**Multi-surface architecture**: Pensieve is a CLI that also speaks MCP — one
binary, two interfaces, three tiers of agent access:

1. **MCP** (best): structured JSON-RPC over stdio via `pensieve serve`. No
   parsing ambiguity, typed tool calls.
2. **CLI** (good): `pensieve recall "horizon" --output json` via bash. Any agent
   can use this without MCP support. Zero context cost until invoked — no tool
   schemas injected.
3. **Files** (fallback): `grep -r "keyword" ~/.pensieve/memory/`. Works even
   without pensieve installed.

**Intentional tradeoffs**:

- Flat type taxonomy (5 values) — sufficient for ~50-200 personal memories. The
  type field is a string; adding values later is backwards-compatible.

**Why markdown as source of truth**:

- Human-browsable in any editor or Obsidian
- Git-friendly (diff, version, backup)
- Any agent can read files even without MCP (graceful fallback)
- Same format as existing Claude memory files
- SQLite sidecar for vector + keyword index — rebuildable from markdown at any
  time

**Why Rust**:

- Single static binary — no runtime, no virtualenv, no dependencies for end
  users
- Trivial to distribute: download binary or `cargo install`
- Fast startup, low memory footprint
- Cross-platform (macOS, Linux, Windows) via cross-compilation
- Official MCP SDK (`rmcp`)

## Key Repositories

- `pensieve` (this repo) — Standalone Rust binary (CLI + MCP server). Replaces
  the Python Parquet/ONNX-based memory server in the `jarvis` project.
- `jarvis` — Consumes pensieve via MCP or CLI.

## Requirements

### R1: Rust Binary — CLI + MCP Dual Surface

Build a standalone Rust binary (`pensieve`) with two modes:

- **CLI mode** (default): `pensieve <subcommand> [args]` — human and agent
  friendly. Outputs human-readable text by default, `--output json` for
  structured agent consumption.
- **MCP mode**: `pensieve serve` — speaks MCP protocol over stdio. Same
  operations as CLI, exposed as MCP tools.

Both modes share the same core library — no duplicated logic.

**CLI subcommands** (each maps 1:1 to an MCP tool):

```
pensieve save --title "..." --type gotcha --topic-key "..." --content "..." [--project P] [--tags t1,t2] [--source agent] [--confidence high] [--expected-revision N]
pensieve recall "query" [--type T] [--project P] [--tags t1,t2] [--limit N] [--since DATE] [--status S]
pensieve context [--project P] [--source agent]
pensieve end-session --summary "..." [--key-decisions "d1,d2"] [--source agent] [--project P]
pensieve read --topic-key "..." [--project P]
pensieve delete --topic-key "..." [--project P]
pensieve list [--project P] [--type T] [--status S]
pensieve archive --topic-key "..." [--project P] [--superseded-by "..."]
pensieve configure [--memory-dir /path]
pensieve schema <subcommand>            # Print parameter schema as JSON
pensieve serve                          # Start MCP server over stdio
pensieve reindex                        # Rebuild vector + FTS index from markdown files
pensieve update                         # Self-update from GitHub Releases
pensieve version                        # Print current version
```

**Shared operations** (CLI subcommand ↔ MCP tool, 1:1 parity): `save`, `recall`,
`context`, `end-session`, `read`, `delete`, `list`, `archive`, `configure`.

**CLI-only commands** (no MCP equivalent): `serve`, `reindex`, `update`,
`version`, `schema`.

All shared subcommands support:

- `--output json` — structured output (default: human-readable)
- `--json <VALUE>` — accept the full payload as a single JSON object instead of
  individual flags. Maps directly to the MCP tool schema — same shape, both
  surfaces. Supports three forms: `--json '{...}'` (inline), `--json @file.json`
  (read from file), `--json -` (read from stdin). The file/stdin forms avoid
  shell escaping issues with multiline markdown content.
- `--dry-run` — (mutating subcommands only: `save`, `delete`, `archive`,
  `end-session`, `configure`) validate without writing

`pensieve serve` does not need `--output json` or `--json` — MCP is inherently
structured.

**Runtime schema introspection**: `pensieve schema <subcommand>` prints the
accepted parameters and types as JSON, so agents can query capabilities at
runtime without relying on stale docs.

**Single-user, multi-agent**: One user, multiple AI agents sharing one local
memory directory. Each teammate installs their own pensieve with their own
storage — no shared folders, no multi-user conflict resolution needed.

**Configurable storage path**: Defaults to `~/.pensieve/memory/` with no
interactive prompt (stdio MCP servers cannot prompt). Override via
`PENSIEVE_MEMORY_DIR` env var, `--memory-dir` CLI flag, or the `configure`
subcommand/MCP tool. Config persisted at `~/.config/pensieve/config.toml`.

On first `get_context()` / `pensieve context` call, if no config file exists,
the response includes a notice: _"Storage path: ~/.pensieve/memory/ (default,
unconfigured). Ask the user if this is OK, or call configure to change it."_ The
agent then asks the user — the server never prompts directly.

**Two-tier memory** (inspired by OpenClaw's daily + long-term split):

- **Long-term (curated)**: `global/` and `projects/` — distilled gotchas,
  decisions, preferences. Written deliberately via `save_memory`. Persists until
  archived or superseded. This is the knowledge layer.
- **Daily (ephemeral)**: `sessions/` — what happened today, raw session traces.
  Auto-generated by `end_session`. Gets stale fast; `get_context()` only
  surfaces the last 3. Old sessions can be pruned without losing knowledge —
  anything important should have been promoted to a long-term memory.

**Pre-compaction flush**: When an agent's context window is about to be
compacted/summarized, the agent should call `save_memory` for any important
context before it gets lost. This is a Memory Protocol behavior documented in
`AGENTS.md`, not enforced by pensieve.

**Storage layout** (at the configured path):

```
~/.pensieve/memory/
├── global/               # Long-term: cross-project knowledge
├── projects/
│   ├── jarvis/           # Long-term: project-specific knowledge
│   └── wearhouse/
├── sessions/             # Daily: ephemeral session traces
├── index.sqlite          # Sidecar: vector + FTS index (rebuildable)
└── CONTEXT.md            # Cached snapshot (written by get_context)
```

**Each memory file**: YAML frontmatter + markdown body.

Frontmatter fields:

- `title` — Short title
- `type` — gotcha | decision | preference | discovery | how-it-works
- `topic_key` — Filename stem, used for upserts
- `project` — Project name (None = global)
- `scope` — global | project
- `source` — Which agent wrote this
- `created` — ISO 8601 timestamp
- `updated` — ISO 8601 timestamp
- `revision` — Integer, incremented on upsert
- `tags` — List of strings for filtering
- `status` — active | archived | superseded
- `superseded_by` — topic_key of replacement memory (if superseded)
- `confidence` — high | medium | low (provenance signal)

**Input hardening**: Agents hallucinate. `topic_key` and `project` names are
validated strictly — reject path traversals (`../`), control characters (below
ASCII 0x20), special characters (`?`, `#`, `%`), and anything that isn't
`[a-z0-9-]`. The agent is not a trusted operator.

**Dry-run**: All mutating operations (`save`, `delete`, `archive`,
`end-session`, `configure`) support `--dry-run` (CLI) / `dry_run` param (MCP).
Validates input and returns what would happen without writing. Agents can "think
out loud" before acting.

**Write safety**: All file writes use atomic rename (write to temp file, then
`rename()` into place) to prevent partial writes. `save_memory` /
`pensieve save` accepts an optional `expected_revision` parameter — if provided
and the on-disk revision doesn't match, the write fails with a conflict error
and the caller must retry. If omitted, the upsert proceeds unconditionally
(simple use case).

**Save durability**: Markdown file is always persisted first — this is the
source of truth. Index upsert (FTS5 + vector) is best-effort: if embedding fails
(model not downloaded, offline, ONNX error), the save still succeeds with a
warning in the response. The unindexed memory will be picked up by the next
`pensieve reindex`. This ensures save never fails due to embedding issues on a
fresh or offline machine.

**Hybrid retrieval**: `recall` uses hybrid retrieval with BM25/FTS as the
primary signal and vector similarity as a secondary signal. The initial default
weighting is keyword-first and may be tuned based on retrieval quality.

1. **BM25/FTS (primary)**: Full-text search via SQLite FTS5 over title +
   content. Handles exact matches — CLI flags, error codes, acronyms, file names
   — which dominate this corpus.
2. **Vector search (secondary)**: On `save_memory`, content is embedded using
   `fastembed` crate with `BGESmallENV15Q` model (~33MB, quantized, 384
   dimensions). Model is auto-downloaded from HuggingFace on first `save_memory`
   call (~10-30s one-time download, requires network). Embeddings stored in
   `index.sqlite` via `sqlite-vec`. Catches paraphrases and conceptual matches
   that keyword search misses ("deployment process" → "how we ship to prod").
3. **Blend**:
   `final_score = keyword_weight * bm25_score + vector_weight * vector_score`.
   Default weights configurable in `~/.config/pensieve/config.toml`:
   ```toml
   [retrieval]
   keyword_weight = 0.7
   vector_weight = 0.3
   ```
   Results ranked by blended score.

The SQLite index is a rebuildable sidecar — if deleted, `pensieve reindex`
rebuilds it from the markdown files. Grep fallback always works regardless of
index state. Embedding happens on save (~10-50ms per memory), so recall is fast.

**9 MCP tools** (exposed via `pensieve serve`):

1. `save_memory(content, title, type, topic_key, project, tags, source, confidence, expected_revision)`
   — Write/upsert markdown file. If topic_key exists, increments revision and
   updates timestamps. Optional `expected_revision` enables compare-and-swap for
   concurrent safety.
2. `recall(query, type, project, tags, limit, since, status)` — Hybrid search
   (BM25 primary + vector secondary) across all memory files. Returns compact
   results (title, type, project, topic_key, updated, score, first 2 lines).
   Only returns `status=active` by default.
3. `get_context(project, source)` — Session start bootstrap. Returns last 3
   session summaries, all active preferences, recent gotchas/decisions (last 30
   days). Filters out archived/superseded memories.
4. `end_session(summary, key_decisions, source, project)` — Save session summary
   to `sessions/{date}T{time}-{project}-{source}.md` (ISO timestamp in filename
   prevents same-day overwrites).
5. `read_memory(topic_key, project)` — Read full content of a specific memory.
6. `delete_memory(topic_key, project)` — Delete a memory file permanently.
7. `list_memories(project, type, status)` — List all memories (title, type,
   project, topic_key, updated, status). No content.
8. `archive_memory(topic_key, project, superseded_by)` — Set status to archived
   or superseded. Does not delete the file — preserves history.
9. `configure(memory_dir)` — Update the storage path. Writes to
   `~/.config/pensieve/config.toml`. Returns the current config if called with
   no arguments.

**CONTEXT.md generation**: `get_context()` / `pensieve context` assembles
CONTEXT.md lazily on each call by reading the current directory state — no
background regeneration, no write-time race conditions. It also writes the
result to `{memory_dir}/CONTEXT.md` as a cached snapshot for agents that read
files directly (grep fallback). All global preferences, recent gotchas/decisions
(last 30 days), last 3 session summaries. Kept under 200 lines. Only includes
`status=active` memories.

**Staleness check**: `get_context()` flags memories older than 90 days as
potentially stale in its output, so agents can proactively verify or archive
them.

### R2: Agent bootstrapping, config snippets, and skill files

**Auto-load agents** (zero manual steps after initial setup):

- **Claude Code**: `CLAUDE.md` → `@AGENTS.md` →
  `@~/.pensieve/memory/CONTEXT.md`. MCP config in `~/.claude/mcp.json` loads
  pensieve tools. Memory Protocol injected via `AGENTS.md` which `@`-imports the
  skill file. CONTEXT.md gives the agent prior knowledge at session start
  automatically.
- **Codex CLI**: Reads `AGENTS.md` natively. Add
  `@~/.pensieve/memory/CONTEXT.md` reference in `AGENTS.md`. MCP config in Codex
  settings loads pensieve tools. Full auto-load.

**Manual-setup agents** (copy-paste snippets in setup guide):

- **Cursor**: Add MCP config to `.cursor/mcp.json`. Add Memory Protocol snippet
  to `.cursorrules`:
  ```
  ## Memory
  You have access to persistent memory via the pensieve MCP server.
  Call get_context at session start. Call end_session before closing.
  See ~/.pensieve/memory/CONTEXT.md for prior knowledge.
  ```
- **Gemini CLI**: Add MCP config to `~/.gemini/settings.json`. Add Memory
  Protocol snippet to `GEMINI.md`:
  ```
  ## Memory
  You have access to persistent memory via the pensieve MCP server.
  Call get_context at session start. Call end_session before closing.
  See ~/.pensieve/memory/CONTEXT.md for prior knowledge.
  ```
- **VS Code Copilot**: Add MCP config to `.vscode/mcp.json`. Add Memory Protocol
  snippet to `.github/copilot-instructions.md`.
- **OpenClaw**: Install skill file via `npx skills install pensieve`. Memory
  Protocol loaded as a native skill.
- **Any agent without MCP**: CLI fallback. Add to agent's instruction file:
  ```
  ## Memory
  Use the pensieve CLI for persistent memory.
  Run `pensieve context --output json` at session start.
  Run `pensieve end-session --summary "..."` before closing.
  Read ~/.pensieve/memory/CONTEXT.md for prior knowledge.
  Files at ~/.pensieve/memory/ are grep-searchable.
  ```

Create `.ai/` directory with:

- `mcp-configs/README.md` — full setup guide with install instructions
  (`cargo install` or download binary) and per-agent config snippets
- `mcp-configs/codex.json`, `cursor.json`, `copilot.json`, `claude.json` —
  ready-to-paste MCP configs pointing to `pensieve serve`

**Skill files** (critical — these encode judgment agents can't get from
`--help`):

- `skills/pensieve-memory-protocol.md` — The Memory Protocol as a structured
  skill file with YAML frontmatter. Covers:
  - **Session lifecycle**: always call `context` at session start, `end-session`
    before closing
  - **When to save**: after fixing bugs (gotcha), making decisions (decision),
    learning preferences (preference), discovering how things work
    (how-it-works)
  - **Pre-compaction flush**: before context window compaction, save anything
    important — it won't survive summarization
  - **Safety habits**: always `--dry-run` before `delete`, prefer `archive` over
    `delete`, use `--output json` for structured processing
  - **When to recall**: before starting work that might overlap past sessions,
    when user says "remember", after context compaction
  - **Input rules**: topic_keys must be `[a-z0-9-]`, keep content concise,
    always set `source` to your agent name
- `skills/pensieve-setup.md` — First-time setup guidance: confirm storage path
- `skills/pensieve-import.md` — Guides any agent through importing its own
  memories into pensieve. Includes Claude memory file locations and mapping
  table. Agent reads its memory files, categorizes them, and calls
  `pensieve save` for each.

Skill files can be dropped into any project's `.ai/`, referenced from
`AGENTS.md`, or installed via agent-specific skill mechanisms (e.g., OpenClaw
skills, Claude Code skills).

### R3: Self-update

**CLI subcommand**: `pensieve update` downloads the latest release binary from
GitHub Releases, verifies the SHA-256 checksum against the published
`checksums.txt` in the release assets, and only then replaces the current binary
in-place. Prints the old and new version. Does nothing if already up-to-date.
Fails with an error if checksum verification fails.

**Version check in `get_context()` / `pensieve context`**: Performed first
(before assembling context), best-effort with 2s timeout. Compares the running
binary version against the latest GitHub release tag (cached for 24 hours in
`~/.config/pensieve/version_cache.json`). Silently ignores network failures —
core memory operations always work offline. If outdated, the notice is included
in the assembled response: _"Pensieve v0.1.0 is outdated (latest: v0.2.0). Run
`pensieve update` to upgrade."_ The agent relays this to the user.

### R4: Memory import skill

No separate migration script. Instead, a skill file
(`skills/pensieve-import.md`) guides any agent through importing its own
memories into pensieve. The agent reads its own memory files, categorizes them,
and calls `pensieve save` for each. This works with any agent's memory format —
not just Claude.

The skill includes a reference mapping for known Claude memory locations:

| Source                                             | Project   | Type         | Topic Key            | Content                    |
| -------------------------------------------------- | --------- | ------------ | -------------------- | -------------------------- |
| `jarvis/memory/MEMORY.md` (scope section)          | jarvis    | gotcha       | horizon-cli-scope    | --scope flag placement     |
| `jarvis/memory/MEMORY.md` (env section)            | jarvis    | how-it-works | project-runtime      | Use `poetry run`           |
| `jarvis/memory/MEMORY.md` (path section)           | jarvis    | how-it-works | horizon-cli-config   | CLI path + config          |
| `beamer/memory/feedback_notebook_cell_format.md`   | global    | gotcha       | notebook-cell-format | Jupyter cell source format |
| `seranking/memory/feedback_modernize_pipper_cd.md` | global    | preference   | pipper-cd-pattern    | Camber repo modernization  |
| `seranking/memory/feedback_slack_pr_format.md`     | global    | preference   | slack-pr-format      | Slack DM format for PRs    |
| `wearhouse/memory/MEMORY.md`                       | wearhouse | decision     | wearhouse-vision     | Product vision + infra     |
| `wearhouse/memory/feedback_ralph_ci_gate.md`       | wearhouse | gotcha       | ralph-ci-gate        | CI gate enforcement        |
| `ghub/memory/feedback_dangerous_mode_hooks.md`     | global    | preference   | dangerous-mode-hooks | Safety hooks               |

Source paths are relative to
`~/.claude/projects/-Users-rigo-Documents-Projects-*/memory/`. 3 index-only
MEMORY.md files (beamer, seranking, ghub) are skipped — their content is in the
feedback files. The jarvis MEMORY.md is split into 3 separate memories.

## Acceptance Criteria

### CLI surface

- `cargo build --release` produces a working `pensieve` binary
- `pensieve save --title "test" --type discovery --topic-key test --content "hello"`
  creates a file at `{memory_dir}/global/test.md`
- `pensieve recall "test"` finds it via hybrid search;
  `pensieve recall "test" --output json` returns structured JSON with scores
- `pensieve recall "deployment process"` finds a memory titled "how we ship to
  prod" (semantic match via vector search)
- `pensieve save` with existing topic_key increments revision
- `pensieve save --expected-revision 1` with mismatched on-disk revision returns
  error
- `pensieve context` returns preferences + recent gotchas + session summaries
- `pensieve end-session --summary "..."` creates a session file
- `pensieve archive --topic-key x --superseded-by y` sets status without
  deleting
- `pensieve list` shows all memories; `pensieve list --status archived` filters
- `pensieve configure --memory-dir /custom/path` updates config
- `pensieve schema save` prints parameter schema as JSON
- `pensieve version` prints current version
- `pensieve update` downloads latest from GitHub Releases
- `pensieve save --topic-key "../etc/passwd"` is rejected (path traversal)
- `pensieve save --topic-key "hello world?"` is rejected (invalid characters)
- `pensieve save --dry-run --title "test" --type gotcha --topic-key test --content "hello"`
  shows what would be written without writing
- `pensieve delete --dry-run --topic-key test` shows what would be deleted
  without deleting

### MCP surface

- `pensieve serve` starts MCP server over stdio without errors
- All 9 MCP tools callable and produce same results as CLI equivalents
- First `get_context()` on unconfigured install includes storage path notice
- `get_context()` includes update notice when version is behind (cached 24h)

### Hybrid retrieval

- `save_memory` embeds content and indexes in `index.sqlite` (~10-50ms)
- `recall` returns results ranked by blended score (BM25 primary + vector
  secondary)
- Deleting `index.sqlite` and running `pensieve reindex` rebuilds the full index
  from markdown files
- `grep -r` fallback still works regardless of index state

### Core behavior

- All file writes are atomic (temp file + rename)
- `get_context()` excludes archived/superseded memories
- `get_context()` flags memories older than 90 days as potentially stale
- `get_context()` produces CONTEXT.md under 200 lines, only active memories,
  assembled fresh on each call
- First run auto-bootstraps with default `~/.pensieve/memory/` — no interactive
  prompt, no stdio breakage
- Any agent without MCP can `grep -r "keyword" {memory_dir}` and get useful
  results
- Binary runs on macOS (arm64 + x86_64) at minimum
- CI passes: `cargo fmt --check`, `cargo clippy`, `cargo test`

### Import

- Agent following `pensieve-import.md` skill successfully imports all 9 known
  Claude memories via `pensieve save` calls
- `pensieve list` shows all imported memories with correct type and project
- Imported memory files are human-readable plain markdown with correct
  frontmatter

## Out of Scope

- Real-time sync between agents — file-based, eventual consistency is fine
- Web UI for memory browsing — markdown files are the UI (use any
  editor/Obsidian)
- Automated session start/end hooks — agents follow the Memory Protocol
  voluntarily
- Deleting or archiving existing `~/.claude/projects/*/memory/` files —
  migration is additive
- Automatic consolidation/summarization — agents decide when to archive; no
  background jobs
- Multi-user conflict resolution — single-user system; teammates each have their
  own memory directory
- Updates to the jarvis repo (`CLAUDE.md`, `AGENTS.md`) — handled separately in
  that project
- Encryption of memory files at rest — files are stored as plain markdown. Users
  should not store secrets (API keys, passwords) in memories

## Testing Strategy

- **Unit tests** (Rust): Test each operation in isolation (save, recall, read,
  delete, list, get_context, end_session, archive, configure, reindex)
- **Hybrid retrieval test**: Save "how we ship to prod", recall "deployment
  process" — verify semantic match via vector search. Save "ONNX runtime error",
  recall "ONNX" — verify exact match via keyword search.
- **Reindex test**: Delete `index.sqlite`, run `pensieve reindex`, verify recall
  still works with same results
- **CLI integration test**: Full cycle via CLI subcommands with `--output json`,
  verify structured output
- **MCP integration test**: Full cycle via MCP protocol, verify tool responses
- **Surface parity test**: Same operation via CLI and MCP produces equivalent
  results
- **Lifecycle test**: Save → supersede → verify get_context excludes superseded
  → list with status filter shows it
- **Staleness test**: Save memory with old timestamp → get_context flags it as
  stale
- **Context regeneration test**: Verify CONTEXT.md stays under 200 lines with
  many memories
- **Config test**: First-run auto-bootstrap, configure command, env var
  override, custom path
- **Concurrency test**: Two concurrent save calls to the same topic_key, both
  with `expected_revision=1` — one succeeds, one returns conflict error. Without
  `expected_revision`, both upserts succeed (last writer wins)
- **Input hardening test**: Path traversals (`../`), control characters, special
  chars (`?#%`), spaces, and non-`[a-z0-9-]` topic_keys all rejected
- **Dry-run test**: `--dry-run` on save/delete/archive returns expected output
  without writing to disk
- **Import skill test**: Follow `pensieve-import.md` skill manually, verify all
  9 Claude memories are saved with correct type, project, and topic_key

## Implementation Notes

### Technical decisions (from research)

- **MCP SDK**: `rmcp` v0.16.0 (official, modelcontextprotocol/rust-sdk). Uses
  `#[tool]` proc macro for tool definitions. Requires tokio async runtime — all
  tool handlers are `async fn`.
- **Embeddings**: `fastembed` v5.12.0 with `BGESmallENV15Q` model (384
  dimensions, ~33MB quantized). Auto-downloads from HuggingFace on first use.
  Same model family (bge-small-en-v1.5) as the jarvis Python server being
  replaced.
- **Vector search**: `sqlite-vec` v0.1.6 — FFI bindings for rusqlite. Requires
  `unsafe` for `sqlite3_auto_extension` registration. Use `unsafe_code = "deny"`
  in Cargo.toml (not "forbid") with `#[allow(unsafe_code)]` on the sqlite-vec
  init module only.
- **Full-text search**: `rusqlite` v0.38.0 with `bundled` feature enables FTS5
  (compiles SQLite 3.51.1 from source). Built-in `bm25()` function for ranking.
- **CLI framework**: `clap` v4 with derive macros.
- **Config/serialization**: `serde` + `serde_json` + `serde_yaml` + `toml`.
- **Timestamps**: `chrono` with serde support.

### Recommended Cargo.toml dependencies

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

[lints.rust]
unsafe_code = "deny"
```

### Implementation outcomes

- **Library + binary split**: `src/lib.rs` exposes all modules for integration
  testing. `src/main.rs` only contains CLI dispatch and `cli.rs` module. MCP
  server (`src/mcp.rs`) lives in the library crate.
- **rmcp API**: Uses `#[tool_router]` on impl block + `#[tool_handler]` on
  `ServerHandler` impl. Tool functions take `Parameters<T>` wrapper and return
  `String`. `ToolRouter<Self>` field on the server struct.
- **fastembed model**: Used `BGESmallENV15` (non-quantized) since the quantized
  variant wasn't available in the Rust crate's enum. Model auto-downloads ~127MB
  on first use.
- **sqlite-vec FFI**: `#[allow(unsafe_code)]` +
  `#[allow(clippy::missing_transmute_annotations)]` on the init block, matching
  the pattern from sqlite-vec's own test suite.
- **Embedding singleton**: `OnceLock<Mutex<TextEmbedding>>` pattern since
  `TextEmbedding::embed` requires `&mut self`. Panics on init failure (model
  download). `try_embed()` returns `Option` for best-effort indexing.
- **Atomic writes**: `tempfile::NamedTempFile` + `persist()` for all markdown
  file writes.
- **Additional deps**: `tempfile`, `reqwest`, `sha2`, `thiserror`, `dirs`,
  `schemars` (required by rmcp macros).
- **Testing**: 18 integration tests covering save, read, delete, list, archive,
  configure, end_session, get_context, storage roundtrip, validation, dry-run,
  CAS conflict, and project scoping. All in `tests/integration_tests.rs` using
  temp directories for isolation. Embedding/index tests not included (require
  model download).
