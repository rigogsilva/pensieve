---
date: 2026-03-15T04:39:19Z
git_commit: 19872a1
branch: main
repository: pensieve
topics:
  - rust-mcp-server
  - hybrid-retrieval
  - markdown-memory-storage
  - cross-agent-bootstrapping
  - migration-from-atlas
tags: [research, codebase, mcp, rust, fastembed, sqlite, memory]
status: complete
last_updated: 2026-03-15
last_updated_by: claude-code
---

# Research for 2026-03-14-cross-agent-memory / spec.cross-agent-memory.md

**Date**: 2026-03-15T04:39:19Z **Git Commit**: 19872a1 **Branch**: main
**Repository**: pensieve

## Summary

Research covered four areas: the current pensieve repo state, the atlas Python
memory server being replaced, Rust crate availability for MCP/embeddings/SQLite,
and the Claude memory files that need migration. All required Rust crates exist
and are mature. The official `rmcp` SDK is the clear choice for MCP. The atlas
server's bge-small-en-v1.5 model is available via `fastembed` in Rust. 10 Claude
memory files across 5 projects are documented and ready for migration mapping.

## Detailed Findings

### 1. Pensieve Repo — Current State

The repo is a clean Rust 2024 scaffold with zero external dependencies.

**Structure**:

```
pensieve/
├── .github/workflows/ci.yml    # cargo fmt, clippy, test, build (Linux + macOS)
├── .gitignore                   # /target, *.swp, .DS_Store, .env
├── .prettierrc                  # proseWrap: always, tabWidth: 2, printWidth: 80
├── AGENTS.md                    # Project description, commands, code style
├── Cargo.toml                   # edition 2024, clippy::pedantic, unsafe=forbid
├── Cargo.lock                   # Only pensieve 0.1.0, no deps
├── LICENSE                      # MIT, copyright 2026 rigo
├── clippy.toml                  # msrv = "1.85"
├── rustfmt.toml                 # max_width=100, use_small_heuristics=Max
├── specs/2026-03-14-cross-agent-memory/
│   └── spec.cross-agent-memory.md
└── src/
    └── main.rs                  # fn main() { println!("Hello, world!"); }
```

**Cargo.toml metadata**: name=pensieve, version=0.1.0, edition=2024,
license=MIT, repo=github.com/rigogsilva/pensieve, keywords=[mcp, ai, memory,
agents], categories=[command-line-utilities].

**CI**: Two GitHub Actions jobs — `check` (ubuntu, fmt+clippy+test+build) and
`build-macos` (macos-latest, build only).

### 2. Atlas Python Memory Server — What We're Replacing

**Location**: `atlas/mcps/memory/server.py`

**Architecture**: FastMCP server with Parquet storage + ONNX embeddings.

**Storage**:

- Primary: `~/.atlas/memory/memory_embeddings.parquet` — Parquet file with
  columns: key, value, embedding (384-dim float array), created_at, updated_at,
  metadata (JSON string)
- Legacy: `~/.atlas/memory/memory.json` — auto-migrated to Parquet on first run
- Models: `~/.atlas/models/bge-small-en-v1.5.onnx` + tokenizer.json

**Embedding model**: BAAI/bge-small-en-v1.5 via ONNX Runtime. 384 dimensions.
Embeddings generated for combined text `"{key}: {value}"`. L2-normalized. Mean
pooling with attention mask weighting.

**8 tools exposed**:

1. `search_memory(query, similarity_threshold=0.5, max_results=10)` — cosine
   similarity against all embeddings
2. `read_memory(key=None)` — exact key lookup or return all
3. `write_memory(key, value, metadata=None)` — upsert with embedding generation
4. `delete_memory(key)` — delete by exact key
5. `list_memory_keys(category=None, sort_by="created_at")` — list with optional
   category filter
6. `find_related_memories(key, max_results=5, similarity_threshold=0.6)` — find
   similar to a source memory
7. `memory_stats()` — count, size, dimension, date range, categories
8. `clear_memory(confirm=False)` — destructive reset

**Dependencies**: numpy, pandas, pyarrow, onnxruntime, sentence-transformers,
fastmcp.

**Key behaviors**: Atomic writes (temp file + rename). No duplicate key
checking. Metadata stored as JSON string. Singleton embedder instance.

**Shared embedder** (`atlas/mcps/shared/embedder.py`): ONNXEmbedder class with
batch encoding (batch_size=64), mean pooling, L2 normalization. Supports
chunking (not used by memory). CPU-only inference with all graph optimizations
enabled.

**Server launcher** (`run_server.py`): Maps server names to modules, runs via
subprocess. Memory server is `atlas.mcps.memory.server` on port 8001.

**CLAUDE.md**: Auto-generated, partially incomplete. References Python 3.13,
fastmcp, polars, pandas, jinja2, Chart.js. No AGENTS.md exists in atlas.

### 3. Rust Crate Ecosystem

#### MCP SDK: `rmcp` (official, recommended)

- **Crate**: `rmcp` v0.16.0 on crates.io
- **GitHub**: modelcontextprotocol/rust-sdk (~2.7k stars)
- **Last updated**: March 13, 2026
- **Transport**: stdio, SSE, Streamable HTTP
- **Ergonomics**: `#[tool]` proc macro for tool definitions, `#[tool_handler]`
  on ServerHandler impl

```rust
#[tool(description = "Save a memory")]
async fn save_memory(&self, #[tool(param)] content: String) -> Result<CallToolResult, McpError> {
    // ...
}
```

**Alternatives**:

- `rust-mcp-sdk` v0.8.0 — community SDK, 88k downloads, good but not official
- `pmcp` — performance-focused (16x faster than TypeScript SDK), newer

#### Embeddings: `fastembed` v5.12.0

- **GitHub**: Anush008/fastembed-rs (86 reverse deps)
- **Runtime**: ONNX via `ort` crate + HuggingFace `tokenizers`
- **Model download**: Automatic from HuggingFace on first use, fully local
- **Key models**:
  - `BGESmallENV15` — 384 dims, ~127MB (same model as atlas server)
  - `BGESmallENV15Q` — 384 dims, ~33MB (quantized, smallest practical)
  - `AllMiniLML6V2` — 384 dims, ~90MB

```rust
let model = TextEmbedding::try_new(InitOptions {
    model_name: EmbeddingModel::BGESmallENV15,
    ..Default::default()
})?;
let embeddings = model.embed(vec!["text to embed"], None)?;
```

**Note**: The quantized `BGESmallENV15Q` at ~33MB is recommended for pensieve to
keep binary + model size reasonable while matching atlas's existing model
family.

#### Vector Search: `sqlite-vec` v0.1.6

- FFI bindings for rusqlite (not standalone)
- Register as auto-extension, then use `vec0` virtual tables
- Supports float, int8, and binary vectors

```rust
use sqlite_vec::sqlite3_vec_init;
unsafe { sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ()))) };
conn.execute("CREATE VIRTUAL TABLE embeddings USING vec0(embedding float[384])", [])?;
```

#### Full-Text Search: `rusqlite` v0.38.0 with FTS5

- FTS5 enabled automatically with `bundled` feature (compiles SQLite 3.51.1)
- Built-in BM25 ranking via `bm25()` function
- Case-insensitive, supports prefix/phrase/boolean queries

```rust
conn.execute("CREATE VIRTUAL TABLE docs USING fts5(title, body)", [])?;
// Search with BM25 ranking:
conn.prepare("SELECT *, bm25(docs) FROM docs WHERE docs MATCH ?1 ORDER BY bm25(docs)")?;
```

#### Recommended Cargo.toml Dependencies

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
```

### 4. Claude Memory Files — Migration Inventory

**10 files across 5 projects** at `~/.claude/projects/*/memory/`:

| #   | Source Path                                      | Project | Type                    | Topic Key            | Content Summary                             |
| --- | ------------------------------------------------ | ------- | ----------------------- | -------------------- | ------------------------------------------- |
| 1   | atlas/memory/MEMORY.md                           | atlas   | gotcha + how-it-works   | gateway-cli-scope    | --scope flag must go AFTER subcommand       |
| 2   | atlas/memory/MEMORY.md                           | atlas   | how-it-works            | gateway-cli-config   | CLI path + config location                  |
| 3   | atlas/memory/MEMORY.md                           | atlas   | how-it-works            | project-runtime      | Use `poetry run` for Python commands        |
| 4   | prism/memory/feedback_notebook_cell_format.md    | global  | gotcha                  | notebook-cell-format | Jupyter cell source must be list of strings |
| 5   | compass/memory/feedback_modernize_scaffold_cd.md | global  | preference              | scaffold-cd-pattern  | Acme repo modernization checklist           |
| 6   | compass/memory/feedback_slack_pr_format.md       | global  | preference              | slack-pr-format      | Slack DM format for PR reviews              |
| 7   | forge/memory/MEMORY.md                           | forge   | decision + how-it-works | forge-vision         | Product vision, roadmap, infrastructure     |
| 8   | forge/memory/feedback_sentinel_ci_gate.md        | forge   | gotcha                  | sentinel-ci-gate     | CI gate enforcement for sentinel-loop       |
| 9   | vault/memory/feedback_dangerous_mode_hooks.md    | global  | preference              | dangerous-mode-hooks | Safety hooks for dangerous mode             |
| 10  | vault/memory/MEMORY.md                           | —       | index                   | —                    | Just references feedback file (skip)        |
| —   | prism/memory/MEMORY.md                           | —       | index                   | —                    | Just references feedback file (skip)        |
| —   | compass/memory/MEMORY.md                         | —       | index                   | —                    | Just references feedback files (skip)       |

**Migration notes**:

- 3 index files (MEMORY.md in prism, compass, vault) are just pointers — skip
  during migration, their content is in the feedback files
- atlas MEMORY.md contains 3 distinct memories that need to be split into
  separate files
- forge MEMORY.md is a single large memory (vision + infra)
- 5 feedback files have YAML frontmatter already (name, description, type)
- Total unique memories to migrate: ~10 (after splitting atlas MEMORY.md into 3)

## Code References

- Atlas memory server: `atlas/mcps/memory/server.py` (555 lines)
- Atlas embedder: `atlas/mcps/shared/embedder.py`
- Atlas run_server: `atlas/mcps/run_server.py`
- Atlas CLAUDE.md: `CLAUDE.md` (auto-generated, incomplete)
- Pensieve main: `src/main.rs` (3 lines, placeholder)
- Pensieve Cargo.toml: `Cargo.toml` (zero deps)

## Architecture Documentation

### Current Atlas Memory Architecture (being replaced)

```
Agent → FastMCP (stdio) → server.py → load_embeddings_df() → pandas DataFrame
                                    → get_embedder() → ONNXEmbedder (bge-small)
                                    → save_embeddings_df() → Parquet file
```

### Target Pensieve Architecture

```
Agent → CLI (clap) ─────────────→ core library → rusqlite (FTS5 + sqlite-vec)
      → MCP (rmcp, stdio) ──────→              → fastembed (BGESmallENV15Q)
      → Files (grep, cat) ──────→              → markdown files (source of truth)
```

## Open Questions

1. **Model bundling vs download**: fastembed downloads models from HuggingFace
   on first use (~33MB for quantized). Should pensieve bundle the model in the
   binary, or download on first `save`? Bundling increases binary size but
   eliminates first-run download. Downloading keeps binary small but requires
   network on first use.

2. **MCP edition**: `rmcp` v0.16.0 uses tokio async. The spec doesn't specify
   sync vs async — but rmcp requires async, so tokio is a hard dependency.

3. **Self-update mechanism**: The spec requires downloading from GitHub Releases
   with checksum verification. Rust crates like `self_update` exist for this
   pattern. Worth investigating vs rolling our own.
