---
date: 2026-03-24T22:14:27-05:00
git_commit: 4a1cc3de6e65f21af070eb04acfd00cabda59304
branch: main
repository: pensieve
topics:
  - get_context
  - MEMORY.md generation
  - ContextResponse
  - nightly-extraction
  - pensieve-setup
  - memory storage structure
tags: research, codebase, context, memory-index, nightly-extraction
status: complete
last_updated: 2026-03-24
last_updated_by: claude-code
---

# Research for 2026-03-24-memory-md-index-generation / spec.memory-md-index-generation.md

**Date**: 2026-03-24T22:14:27 CDT **Git Commit**:
4a1cc3de6e65f21af070eb04acfd00cabda59304 **Branch**: main **Repository**:
pensieve

## Summary

The spec replaces the current `CONTEXT.md` side-effect of `get_context` with
lean `MEMORY.md` index files (one line per memory) at global and per-project
scope. `get_context` slims down to returning these index files + last 3
sessions. The nightly extraction skill gains a post-save regeneration step. The
setup skill is updated to document importing `MEMORY.md`.

All the infrastructure needed exists: `list_memories()` already returns the full
corpus, `ensure_dirs()` already creates the project directories, and the
existing `write_context_md()` function is the direct predecessor to replace.

## Detailed Findings

### `src/ops/context.rs` — get_context() and write_context_md()

**`get_context()` signature** (`context.rs:82-86`):

```rust
pub fn get_context(
    config: &PensieveConfig,
    project: Option<&str>,
    _source: Option<&str>,
) -> Result<ContextResponse>
```

**Sequence of operations** (`context.rs:87-161`):

1. `storage::list_sessions(config, 3)` — fetches 3 most recent session summaries
2. `storage::list_memory_files(config, project, None, Some(&MemoryStatus::Active))`
   — all active memories for project (or all projects if None)
3. Categorizes memories into: all preferences, gotchas updated ≥ 30 days ago,
   decisions updated ≥ 30 days ago, stale memories updated < 90 days ago
4. Optional version check against GitHub releases (2s timeout, non-blocking)
5. Calls `write_context_md()` as a side effect — writes `CONTEXT.md`
6. Returns `ContextResponse`

**`write_context_md()` function** (`context.rs:164-221`):

Writes `{memory_dir}/CONTEXT.md` with:

- `## Preferences` — all preferences, format: `- **{title}**: {preview}`
- `## Recent Gotchas` — gotchas ≥ 30 days, same format
- `## Recent Decisions` — decisions ≥ 30 days, same format
- `## Recent Sessions` — up to 3, format:
  `- **{date} ({project})**: {first line of summary}`
- Truncated to max 200 lines (`context.rs:214`)

**Current `ContextResponse` struct** (`context.rs:8-17`):

```rust
pub struct ContextResponse {
    pub sessions: Vec<SessionSummary>,
    pub preferences: Vec<MemoryCompact>,
    pub recent_gotchas: Vec<MemoryCompact>,
    pub recent_decisions: Vec<MemoryCompact>,
    pub stale_memories: Vec<MemoryCompact>,
    pub notice: Option<String>,
}
```

### `src/types.rs` — Key Structs

**`MemoryCompact`** (`types.rs:157-171`):

```rust
pub struct MemoryCompact {
    pub title: String,
    pub memory_type: MemoryType,
    pub topic_key: String,
    pub project: Option<String>,
    pub status: MemoryStatus,
    pub updated: DateTime<Utc>,
    pub score: Option<f64>,
    pub preview: String,   // first 2 lines of content
}
```

`preview` = `content.lines().take(2).collect().join("\n")` (`types.rs:175`).

**`SessionSummary`** (`types.rs:189-198`):

```rust
pub struct SessionSummary {
    pub summary: String,
    pub key_decisions: Vec<String>,
    pub source: String,
    pub project: Option<String>,
    pub created: DateTime<Utc>,
}
```

### `src/ops/list.rs` — list_memories()

**Signature** (`list.rs:7-13`):

```rust
pub fn list_memories(
    config: &PensieveConfig,
    project: Option<&str>,
    type_filter: Option<&MemoryType>,
    status_filter: Option<&MemoryStatus>,
    since: Option<&DateTime<Utc>>,
) -> Result<Vec<MemoryCompact>>
```

Calls `storage::list_memory_files()` → maps `Memory` → `MemoryCompact` →
optional `since` filter → returns sorted `Vec<MemoryCompact>`.

### `src/storage.rs` — Directory Structure

**`ensure_dirs()`** (`storage.rs:17-22`):

```rust
std::fs::create_dir_all(config.memory_dir.join("global"))?;
std::fs::create_dir_all(config.memory_dir.join("projects"))?;
std::fs::create_dir_all(config.memory_dir.join("sessions"))?;
```

**Path resolution** (`storage.rs:6-11`):

- Global: `{memory_dir}/global/{topic_key}.md`
- Project: `{memory_dir}/projects/{project}/{topic_key}.md`

**`list_memory_files()`** (`storage.rs:73-124`):

- If `project` set: scans only `{memory_dir}/projects/{project}/`
- If `project` is None: scans `global/` + all subdirs in `projects/`
- Applies type/status filters, sorts by `updated` descending

**Current disk structure**:

```
~/.pensieve/memory/
├── CONTEXT.md          ← side-effect of get_context (to be replaced by MEMORY.md)
├── index.sqlite
├── global/             ← 10 global memories
├── projects/           ← 15+ project directories
│   ├── pensieve/       ← 41 memories
│   ├── wearhouse/      ← 131 memories
│   ├── camber/         ← 75 memories
│   └── ...
└── sessions/
```

### `.ai/skills/nightly-extraction.md` — Where MEMORY.md Regeneration Fits

**Step 2** — Full Memory Inventory:

```bash
pensieve list --output json
```

Returns complete corpus (not ranked like `recall`). Passed to all subagents.

**Step 4** — Save sequentially:

```bash
pensieve save --json '{"type":"...","topic_key":"...","project":"...","content":"...","source":"extraction"}'
```

No current step regenerates `MEMORY.md` after saves. The regeneration step would
be added at the **end of Step 4**, after all saves complete.

### `.ai/skills/pensieve-setup.md` — Session Start Hook

**SessionStart hook command** (always installed):

```bash
__PENSIEVE_BIN__ context 2>/dev/null || true
```

No current reference to importing or loading `MEMORY.md` files. The setup skill
would need a new step instructing agents to add `@~/.pensieve/memory/MEMORY.md`
to their global instruction file.

**Memory Protocol canonical block** (`pensieve-setup.md:78-197`): Currently
documents `get_context(project, source)` as Step 1 — Session start. This remains
correct; the change is in what `get_context` returns, not how agents call it.

### Tests — `tests/integration_tests.rs`

28+ integration tests in `tests/integration_tests.rs`. Key test for this spec:

- **`test_get_context`** (`integration_tests.rs:888-913`): Tests current
  `ContextResponse` shape — will need updating to reflect new fields
  (`global_index`, `project_index` instead of `preferences`, `recent_gotchas`,
  `recent_decisions`)
- **`test_context_alias`** (`integration_tests.rs:1062-1107`): Tests `context`
  command alias — likely needs updating too

All tests use `TempDir` for isolation. No test currently verifies `CONTEXT.md`
is written to disk.

## Code References

- `src/ops/context.rs:82-161` — `get_context()` full implementation
- `src/ops/context.rs:164-221` — `write_context_md()` to be replaced by
  `write_memory_index()`
- `src/ops/context.rs:8-17` — `ContextResponse` struct definition
- `src/types.rs:157-171` — `MemoryCompact` struct (has `preview` = first 2
  lines)
- `src/types.rs:189-198` — `SessionSummary` struct
- `src/ops/list.rs:7-19` — `list_memories()` — reuse to build index
- `src/storage.rs:17-22` — `ensure_dirs()` — extend to create `MEMORY.md` dirs
- `src/storage.rs:73-124` — `list_memory_files()` — used by `list_memories()`
- `tests/integration_tests.rs:888-913` — `test_get_context` — needs updating
- `tests/integration_tests.rs:1062-1107` — `test_context_alias` — needs updating
- `.ai/skills/nightly-extraction.md` — Step 4 is where MEMORY.md regeneration is
  added
- `.ai/skills/pensieve-setup.md` — Step 2 (Memory Protocol block) needs
  MEMORY.md import instruction

## Architecture Documentation

**Current flow:**

```
get_context() → fetch all memories → categorize by type+recency
             → write_context_md() [side effect, writes CONTEXT.md]
             → return ContextResponse {preferences, gotchas, decisions, sessions}
```

**New flow:**

```
get_context() → list_memories(None) [global] → write MEMORY.md [global]
             → list_memories(project) [if project] → write MEMORY.md [project]
             → list_sessions(3)
             → return ContextResponse {global_index, project_index, sessions}
```

**`ContextResponse` new shape:**

```rust
pub struct ContextResponse {
    pub global_index: String,           // content of global MEMORY.md
    pub project_index: Option<String>,  // content of project MEMORY.md (if project given)
    pub sessions: Vec<SessionSummary>,  // last 3 sessions (unchanged)
    pub stale_memories: Vec<MemoryCompact>, // keep for now (useful signal)
    pub notice: Option<String>,         // keep (version check notice)
}
```

**`MEMORY.md` line format:**

```
- [gotcha] topic-key: First non-empty line of preview
- [decision] another-key: Summary of decision
```

**Where files live:**

- `{memory_dir}/MEMORY.md` — global index (replaces `CONTEXT.md`)
- `{memory_dir}/projects/{project}/MEMORY.md` — per-project index

## Open Questions

- Should `stale_memories` be kept in `ContextResponse`? It provides a useful
  signal but adds lines. Could be a separate flag/field rather than inline.
- Should the nightly extraction regenerate ALL project `MEMORY.md` files or only
  the ones touched in the current run? (All is simpler; touched-only is faster
  for large corpora.)
- Should `CONTEXT.md` be deleted during migration or just stop being generated?
  (Deleting avoids confusion; leaving it stale avoids breaking any existing
  `@import` references in user configs.)
