---
date: 2026-03-25T03:33:45+0000
git_commit: 4a1cc3de6e65f21af070eb04acfd00cabda59304
branch: main
repository: pensieve
spec: spec.memory-md-index-generation.md
research: research.memory-md-index-generation.md
worktrees:
  - /Users/rigo/Documents/Projects/worktrees/memory-md-index-generation/implement/pensieve/
status: in-progress
last_updated: 2026-03-24
---

# Implementation Plan: MEMORY.md Index File Generation

**Spec**: spec.memory-md-index-generation.md **Research**:
research.memory-md-index-generation.md **Date**: 2026-03-25T03:33:45 UTC

## Overview

`get_context` currently dumps ~2,281 lines of JSON (preferences, gotchas,
decisions, sessions) into every agent session via the `SessionStart` hook. This
plan replaces that with lean `MEMORY.md` index files — one line per memory — at
global and per-project scope. `get_context` is slimmed to return these index
files + last 3 sessions. Nightly extraction regenerates the files after saves.
The `CONTEXT.md` file is deleted on first run.

## Progress

- [ ] Worktree setup
- [ ] Phase 1: Struct update (`ContextResponse`)
- [ ] Phase 2: Core implementation (`write_memory_index`, `get_context`)
- [ ] Independent steps (tests, skills)
- [ ] Build and verify
- [ ] AGENTS.md check
- [ ] Spec update
- [ ] Worktree teardown

## Worktree Setup

| Repo       | Worktree path                                                                             | Base branch |
| ---------- | ----------------------------------------------------------------------------------------- | ----------- |
| `pensieve` | `/Users/rigo/Documents/Projects/worktrees/memory-md-index-generation/implement/pensieve/` | `main`      |

_Created with
`git worktree add ../worktrees/memory-md-index-generation/implement/pensieve -b memory-md-index-generation`
run from the pensieve repo root._

All file edits must be made inside the worktree path. Never edit the original
repo at `/Users/rigo/Documents/Projects/pensieve/`.

---

## Implementation Steps

### Phase 1: Struct Update

_Must come first — the compiler will surface every callsite that references the
removed fields, making the blast radius visible before any logic changes._

#### Step 1 — Update `ContextResponse` in `src/types.rs`

**What changes**:
`/Users/rigo/Documents/Projects/worktrees/memory-md-index-generation/implement/pensieve/src/types.rs`
(at the `ContextResponse` struct, currently at `types.rs:8-17` per research)

**How to verify**: `cargo check` produces errors at every callsite referencing
the old fields — those become the todo list for Phase 2.

Replace the current struct:

```rust
// REMOVE these fields:
pub preferences: Vec<MemoryCompact>,
pub recent_gotchas: Vec<MemoryCompact>,
pub recent_decisions: Vec<MemoryCompact>,
pub stale_memories: Vec<MemoryCompact>,

// ADD these fields:
pub global_index: String,          // content of ~/.pensieve/memory/MEMORY.md
pub project_index: Option<String>, // content of projects/{project}/MEMORY.md
```

Keep unchanged: `pub sessions: Vec<SessionSummary>`,
`pub notice: Option<String>`

---

### Phase 2: Core Implementation

_Depends on Phase 1 — the new struct fields must exist before implementing the
functions that populate them._

#### Step 2 — Implement `write_memory_index()` in `src/ops/context.rs`

**What changes**:
`/Users/rigo/Documents/Projects/worktrees/memory-md-index-generation/implement/pensieve/src/ops/context.rs`
— replace `write_context_md()` (currently lines 164-221) with a new
`write_memory_index()` function.

**How to verify**: Function compiles; calling it produces correct files at the
expected paths.

New function:

```rust
fn write_memory_index(
    config: &PensieveConfig,
    project: Option<&str>,
) -> Result<(String, Option<String>)>
```

**Critical**: `list_memories(config, None, ...)` returns ALL memories (global +
all projects). The global MEMORY.md must contain ONLY globally-scoped memories
(where `project == None`). Filter after fetching:

```rust
// Global index: fetch all, filter to project==None only
let all_memories = list_memories(config, None, None, Some(&MemoryStatus::Active), None)?;
let global_memories: Vec<_> = all_memories.iter()
    .filter(|m| m.project.is_none())
    .collect();

// Project index: scoped fetch already returns only that project's memories
let project_memories = if let Some(proj) = project {
    Some(list_memories(config, Some(proj), None, Some(&MemoryStatus::Active), None)?)
} else {
    None
};
```

Line format for each entry (one-line summary = first non-empty line of
`preview`; fall back to `title` if preview is empty):

```rust
fn format_memory_line(memory: &MemoryCompact) -> String {
    let summary = memory.preview
        .lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .unwrap_or(memory.title.as_str());
    format!("- [{}] {}: {}", memory.memory_type, memory.topic_key, summary)
}
```

Sort order: `updated` descending — this is the default output from
`list_memories()`, no additional sort needed.

**File paths**:

- Global: `{config.memory_dir}/MEMORY.md`
- Per-project: `{config.memory_dir}/projects/{project}/MEMORY.md`

**CONTEXT.md deletion**: Delete `{config.memory_dir}/CONTEXT.md` if it exists.
Use `std::fs::remove_file()` — ignore `NotFound` errors.

Return value: `(global_index_content, project_index_content)` where each is the
full string written to disk.

#### Step 3 — Update `get_context()` in `src/ops/context.rs`

**What changes**: The same file — update `get_context()` (currently lines
82-161) to use the new function and return the new `ContextResponse` shape.

**How to verify**: `cargo check` passes; all old-field callsites are gone.

New flow (replaces the categorization logic — preferences, gotchas by recency,
decisions by recency, stale detection):

```
1. Call write_memory_index(config, project)
   → writes {memory_dir}/MEMORY.md (global-scoped only)
   → writes {memory_dir}/projects/{project}/MEMORY.md (if project given)
   → deletes {memory_dir}/CONTEXT.md if it exists
   → returns (global_index, project_index)
2. Call storage::list_sessions(config, 3)  ← unchanged
3. Optional version check                  ← unchanged
4. Return ContextResponse {
       global_index,
       project_index,
       sessions,
       notice,
   }
```

Remove all calls to the old `write_context_md()` function and delete it.

#### Step 4 — Update CLI output formatting in `src/main.rs` (or wherever context is displayed)

**What changes**: Find where `ContextResponse` is printed for the `context` /
`get-context` CLI command. Update the display to render `global_index` and
`project_index` content instead of the old categorized lists.

**How to verify**: `pensieve context` output looks correct and line count is
significantly reduced.

> **⚠ Research gap**: The CLI display path for `ContextResponse` was not covered
> in the research. Read `src/main.rs` and any display/formatting code for the
> `context` subcommand before implementing this step. Check how the current
> categorized output is printed and replace with the new fields.

---

### Independent Steps

_The following steps are independent of each other and may be completed in
parallel after Phase 2 is complete._

#### Step 5 — Update integration tests in `tests/integration_tests.rs`

**What changes**:
`/Users/rigo/Documents/Projects/worktrees/memory-md-index-generation/implement/pensieve/tests/integration_tests.rs`

**How to verify**: `cargo test test_get_context` and
`cargo test test_context_alias` both pass.

Update `test_get_context` (line 888):

- Remove assertions on `preferences`, `recent_gotchas`, `recent_decisions`,
  `stale_memories`
- Assert `global_index` is a non-empty `String`
- Assert `project_index` is `None` when no project was passed; `Some(_)` when
  project was passed
- Assert `~/.pensieve/memory/MEMORY.md` (or the temp dir equivalent) exists
- Assert `CONTEXT.md` does NOT exist in the temp dir

Update `test_context_alias` (line 1062):

- Same field updates as above
- Verify alias still returns the same result as `context` subcommand

Add edge-case tests:

- `test_get_context_empty_global`: no global-scoped memories → `global_index` is
  empty string (not an error)
- `test_get_context_no_project_scope`: project provided but no memories for it →
  `project_index` is `Some("")` or `None` (define which in spec; recommend
  `Some("")` to signal the file was written but is empty)
- `test_context_md_deletion`: `CONTEXT.md` pre-existing → after `get_context` it
  is deleted

#### Step 6 — Update nightly extraction skill

**What changes**:
`/Users/rigo/Documents/Projects/worktrees/memory-md-index-generation/implement/pensieve/.ai/skills/nightly-extraction/SKILL.md`

**How to verify**: File contains a step after the saves that calls
`pensieve context`.

After Step 4 (save sequentially), add a new final step:

```markdown
## Step 5 — Regenerate MEMORY.md index files

After all saves complete, regenerate the global and per-project MEMORY.md index
files:

\`\`\`bash pensieve context 2>/dev/null || true \`\`\`

This updates `~/.pensieve/memory/MEMORY.md` and any per-project `MEMORY.md`
files to reflect the memories saved in this extraction run. The command is
idempotent — safe to re-run.
```

#### Step 7 — Update pensieve-setup skill

**What changes**:
`/Users/rigo/Documents/Projects/worktrees/memory-md-index-generation/implement/pensieve/.ai/skills/pensieve-setup.md`

**How to verify**: The canonical Memory Protocol block includes the MEMORY.md
import instruction.

In Step 2 (Add Memory Protocol), inside the canonical block (between
`<!-- pensieve:start -->` and `<!-- pensieve:end -->`), add after the **Access**
section:

```markdown
### Global memory index

`get_context` writes `~/.pensieve/memory/MEMORY.md` at session start — a
one-line-per-memory index of all globally-scoped memories. For Claude Code, add
this import to `~/.claude/CLAUDE.md` to load the index into every session
automatically (even before `get_context` is called):

\`\`\` @~/.pensieve/memory/MEMORY.md \`\`\`

For other agents (Codex CLI, Gemini CLI, Cursor), the index is provided via
`SessionStart` hook output — no additional config needed.

Use `pensieve read --json '{"topic_key":"<key>"}' --output json` to fetch full
content for any entry in the index.
```

---

## Testing

- **`test_get_context` (updated)**: asserts new field names (`global_index`,
  `project_index`), MEMORY.md exists, CONTEXT.md absent → maps to Steps 2, 3, 5
- **`test_context_alias` (updated)**: asserts alias returns same new-shape
  response → maps to Steps 2, 3, 5
- **`test_get_context_empty_global`**: no global memories → `global_index == ""`
  → maps to Steps 2, 3, 5
- **`test_get_context_no_project_scope`**: project given, no project memories →
  `project_index` is defined (empty or None) → maps to Steps 2, 3, 5
- **`test_context_md_deletion`**: CONTEXT.md pre-exists → deleted after
  `get_context` → maps to Step 2
- **Build test**: `cargo build` compiles cleanly → maps to Steps 1-4
- **Full test suite**: `cargo test` passes → maps to all steps

---

## Risk Callouts

> **⚠ Research gap**: CLI display path for `ContextResponse` (Step 4) was not
> covered in the research. Read `src/main.rs` and any formatting code for the
> `context` subcommand before implementing Step 4.

> **⚠ Research gap**: The nightly extraction skill lives at
> `.ai/skills/nightly-extraction/SKILL.md` (not
> `.ai/skills/nightly-extraction.md` as the research stated). Confirmed by
> checking the actual file structure.

---

## Closing Steps

- [ ] **Check `AGENTS.md`**: Review `AGENTS.md` at the repository root. The
      `context` / `get-context` command behavior has changed (new response
      fields, new side effects). Update `AGENTS.md` if it documents the
      `ContextResponse` shape, the `context` command output, or the `CONTEXT.md`
      file. Also update if the `MEMORY.md` file convention is not documented
      there.
- [ ] **Update spec**: Once implementation is complete, update the
      `Implementation Notes` section of
      `specs/2026-03-24-memory-md-index-generation/spec.memory-md-index-generation.md`
      with: (a) how the global-only filter was implemented, (b) the
      `write_memory_index()` function signature and location, (c) how CONTEXT.md
      deletion was handled, (d) which tests were added vs updated.
- [ ] **Worktree teardown**: After all implementation work is verified:
  ```bash
  cd /Users/rigo/Documents/Projects/worktrees/memory-md-index-generation/implement/pensieve
  git add -A
  git commit -m "memory-md-index-generation: replace CONTEXT.md with lean MEMORY.md index files"
  git push --set-upstream origin memory-md-index-generation
  git -C /Users/rigo/Documents/Projects/pensieve worktree remove /Users/rigo/Documents/Projects/worktrees/memory-md-index-generation/implement/pensieve
  ```
