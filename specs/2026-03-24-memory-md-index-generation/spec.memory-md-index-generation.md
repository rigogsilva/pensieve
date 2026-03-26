# MEMORY.md Index File Generation

## Overview

Pensieve's `get_context` currently returns a large JSON payload (preferences,
gotchas, decisions, sessions) that the `SessionStart` hook injects into every
agent session — resulting in ~2,000+ lines of context before the user types
anything. Most of this is irrelevant to the current task.

The fix: generate lean `MEMORY.md` index files (one line per memory) at the
global and per-project level. Agents load these at session start to know what
memories exist, then fetch full content on demand via `pensieve read`.
`get_context` slims down to just these index files + recent session summaries.

## Impacted Repositories

- `pensieve` — the only repo affected; changes span `src/ops/context.rs`,
  `src/types.rs`, `.ai/skills/nightly-extraction.md`, and
  `.ai/skills/pensieve-setup.md`

## Requirements

- `get_context` generates and returns `~/.pensieve/memory/MEMORY.md` (global)
  and `~/.pensieve/memory/projects/{project}/MEMORY.md` (per-project) as
  markdown content
- **Global MEMORY.md** lists only globally-scoped memories (those stored in
  `{memory_dir}/global/`, where `project == None`) — NOT all memories across all
  projects
- **Project MEMORY.md** lists only memories scoped to that specific project
  (stored in `{memory_dir}/projects/{project}/`)
- Each `MEMORY.md` lists active memories one line per entry:
  `- [{type}] {topic_key}: {one-line summary}`
  - **`{type}`**: the `MemoryType` display value (e.g. `gotcha`, `decision`,
    `preference`, `how-it-works`, `discovery`)
  - **`{topic_key}`**: the memory's topic key
  - **`{one-line summary}`**: first non-empty line of `MemoryCompact.preview`;
    fall back to `title` if preview is empty. Newlines stripped. Colons and
    brackets in the summary are not escaped (agents parse the first colon after
    `topic_key` as the separator)
  - **Sort order**: by `updated` descending (most recent first)
  - **No line cap**: all active memories are listed, no truncation
- `get_context` response shape (the `ContextResponse` struct):
  - `global_index: String` — full content of global MEMORY.md
  - `project_index: Option<String>` — full content of project MEMORY.md (only if
    `--project` was provided)
  - `sessions: Vec<SessionSummary>` — last 3 sessions across all projects (not
    filtered by project)
  - `notice: Option<String>` — version check notice (unchanged)
  - **Removed**: `preferences`, `recent_gotchas`, `recent_decisions`,
    `stale_memories`
- `MEMORY.md` is regenerated on each `get_context` call. It is **not**
  automatically regenerated after individual `save`, `delete`, or `archive`
  operations (freshness is bounded by the next `get_context` call)
- The nightly extraction skill regenerates `MEMORY.md` files by calling
  `pensieve context` after all saves complete
- `CONTEXT.md` generation is removed and the file is deleted on the first
  `get_context` run after upgrade (if it exists at `{memory_dir}/CONTEXT.md`)
- The `pensieve-setup` skill adds an `@~/.pensieve/memory/MEMORY.md` import
  instruction to the canonical Memory Protocol block for Claude Code
  (`~/.claude/CLAUDE.md`). For agents that don't support file imports (Codex
  CLI, Gemini CLI), the setup skill relies on `SessionStart` hook output only

## Acceptance Criteria

- `~/.pensieve/memory/MEMORY.md` exists after running `pensieve context` and
  lists only globally-scoped memories (project == None), one per line
- `~/.pensieve/memory/projects/{project}/MEMORY.md` exists after running
  `pensieve context --project {project}` and lists only that project's memories
- Each line in a `MEMORY.md` matches: `- [{type}] {topic_key}: {summary}`
- `pensieve context --project pensieve` returns:
  - `global_index`: content of global MEMORY.md (global-scoped only)
  - `project_index`: content of pensieve project MEMORY.md
  - `sessions`: last 3 sessions (not filtered by project)
  - `notice`: version check result (may be null)
- `pensieve context` CLI output is significantly reduced from ~2,281 lines
  (target: ≤ total active memory count + 20 lines for session summaries)
- `CONTEXT.md` does not exist after running `pensieve context` post-upgrade
- Nightly extraction skill calls `pensieve context` at end; MEMORY.md files
  reflect saves from the current run
- Existing integration tests `test_get_context` and `test_context_alias` pass
  with updated field assertions

## Out of Scope

- Changing how `pensieve recall`, `list`, or `read` work
- Auto-importing `MEMORY.md` into `CLAUDE.md`/`AGENTS.md` (setup skill update
  only documents how; user adds the import manually)
- Per-project auto-detection from git remote (manual `--project` flag)
- Regenerating MEMORY.md after every `save`/`delete`/`archive` (only on
  `get_context` and at end of nightly extraction)
- Adding a `pensieve index` subcommand (not needed; `get_context` handles
  regeneration)

## Testing Strategy

- Build: `cargo build` must compile cleanly after struct and function changes
- Run `pensieve context` — verify output is significantly shorter than current
  2,281 lines; verify MEMORY.md exists with correct format
- Run `pensieve context --project pensieve` — verify `global_index` contains
  only globally-scoped memories, `project_index` contains only pensieve memories
- Verify `CONTEXT.md` does not exist after running context
- `cargo test` — `test_get_context` (integration_tests.rs:888) and
  `test_context_alias` (integration_tests.rs:1062) must pass with updated field
  assertions
- Run nightly extraction skill on a short test transcript — verify
  `pensieve context` is called at end and MEMORY.md reflects the saves

## Implementation Notes

**Global-only filter**: `write_memory_index()` calls
`list_memories(config, None, ...)` which returns ALL memories across all scopes,
then filters with `.filter(|m| m.project.is_none())` before building the global
index. This ensures only memories without a project scope appear in the global
`MEMORY.md`.

**`write_memory_index()` location**: `src/ops/context.rs`, private function.
Signature:

```rust
fn write_memory_index(config: &PensieveConfig, project: Option<&str>) -> Result<(String, Option<String>)>
```

Returns `(global_content, Option<project_content>)` where each string is the
full text written to disk.

**`ContextResponse` location**: Defined inline in `src/ops/context.rs` (not
`types.rs`). Fields: `global_index: String`, `project_index: Option<String>`,
`sessions: Vec<SessionSummary>`, `notice: Option<String>`.

**CONTEXT.md deletion**: `std::fs::remove_file()` with explicit
`io::ErrorKind::NotFound` check — non-NotFound errors are also silently ignored
so deletion never blocks context generation.

**Project MEMORY.md directory**: `create_dir_all(parent)` called before writing
per-project `MEMORY.md` to handle projects that have never had a memory saved
(their directory may not exist yet). This was a bug caught during review.

**Tests added**:

- `test_get_context` — updated: asserts `global_index` non-empty, contains
  topic_key, `MEMORY.md` exists, `CONTEXT.md` absent
- `test_context_alias` — updated: asserts `global_index == ""` when no memories,
  `project_index` is None
- `test_staleness_flag` — updated: removed `stale_memories` assertion (field
  removed)
- `test_get_context_empty_global` — new: project-scoped memories don't leak into
  global_index
- `test_get_context_with_project_scope` — new: project_index is Some and
  contains project memory
- `test_context_md_deletion` — new: pre-existing CONTEXT.md is deleted by
  get_context
