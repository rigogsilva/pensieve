---
date: 2026-03-21T07:36:31+0000
git_commit: 19e1c1f0b3dd1078c3d5a3c400f6419468cdb770
branch: main
repository: pensieve
topics:
  - list command CLI definition
  - since date parsing pattern
  - ops/list operation layer
  - MCP list_memories tool
  - skill references to pensieve list
tags: [research, codebase, cli, mcp, filtering]
status: complete
last_updated: 2026-03-21
last_updated_by: claude-code
---

# Research for 2026-03-21-list-since-filter / spec.list-since-filter.md

**Date**: 2026-03-21T07:36:31+0000 **Git Commit**: 19e1c1f **Branch**: main
**Repository**: pensieve

## Summary

The `list` command currently supports 3 optional filters (project, type, status)
but no date filter. The `recall` command already has a working `--since`
implementation that can be reused as a pattern. The change touches 4 Rust files
and 2-4 skill/protocol files.

## Detailed Findings

### 1. CLI Layer (`src/cli.rs`)

**List command** (lines 125-141): 4 fields — `output`, `project`, `r#type`,
`status`. No `since` field.

**Recall command** (lines 92-123): Has `since: Option<String>` at line 118 with
doc comment `/// Only memories updated after this date`. This is the exact
pattern to replicate.

### 2. Main Dispatch (`src/main.rs`)

**List handler** (lines 205-222): Destructures 4 fields, parses type/status
strings to enums via `.parse().expect()`, calls `ops::list::list_memories()`.

**Recall handler** (lines 167-203): Parses `since` at lines 172-178:

```rust
let since = since.map(|s| {
    chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
        .expect("invalid date, use YYYY-MM-DD")
        .and_hms_opt(0, 0, 0)
        .expect("invalid time")
        .and_utc()
});
```

Converts `Option<String>` → `Option<DateTime<Utc>>` at midnight UTC. This exact
block can be copied for the list handler.

### 3. Operations Layer (`src/ops/list.rs`)

**Full file** (lines 1-13):

```rust
pub fn list_memories(
    config: &PensieveConfig,
    project: Option<&str>,
    type_filter: Option<&MemoryType>,
    status_filter: Option<&MemoryStatus>,
) -> Result<Vec<MemoryCompact>>
```

Calls `storage::list_memory_files()` then maps each `Memory` to `MemoryCompact`.
The `since` filter should be applied after collecting from storage — filter
`MemoryCompact` by `updated >= since`.

### 4. Storage Layer (`src/storage.rs`)

**`list_memory_files`** (lines 73-124): Scans filesystem, parses YAML
frontmatter, applies type/status filters, sorts by `updated` descending. No date
filtering exists here. The `since` filter belongs in `ops/list.rs` (after
storage returns), not in storage itself — consistent with how recall applies
`since` post-retrieval.

### 5. Types (`src/types.rs`)

**`MemoryCompact`** (lines 157-171): Has `updated: DateTime<Utc>` field — the
filtering target. Also has `score: Option<f64>` (set to None by list, used by
recall).

**`Memory`** (lines 80-105): Has both `created: DateTime<Utc>` and
`updated: DateTime<Utc>`.

### 6. MCP Server (`src/mcp.rs`)

**`ListMemoriesParams`** (lines 91-99): 3 optional fields — `project`, `type`,
`status`. No `since`.

**`RecallParams`** (lines 49-70): Has `since: Option<String>` at line 62 with
doc `/// Only memories updated after this ISO date`. Parsed in recall method at
lines 237-246 with the same `NaiveDate` pattern.

**`list_memories` method** (lines 310-327): Tool description is
`"List all memories with title, type, project, topic_key, and updated date. No content."`

### 7. Skills and Protocol Files

**`pensieve list` appears as a CLI command in:**

- `~/.claude/skills/nightly-extraction/SKILL.md` line 61 —
  `pensieve list --output json` (Step 2 dedup inventory)
- `~/.claude/skills/pensieve-setup/SKILL.md` line 182 — usage example
- `~/.claude/CLAUDE.md` line 121 — usage example
- `~/.codex/AGENTS.md` line 105 — usage example

**`list_memories` MCP tool referenced in** the Access section of all protocol
files.

## Code References

- `src/cli.rs:125-141` — List command definition (add `since` field here)
- `src/cli.rs:117-118` — Recall's `since` field definition (pattern to copy)
- `src/main.rs:205-222` — List handler (add `since` parsing here)
- `src/main.rs:172-178` — Recall's `since` parsing (pattern to copy)
- `src/ops/list.rs:5-13` — `list_memories` function (add `since` parameter and
  filter)
- `src/mcp.rs:91-99` — `ListMemoriesParams` (add `since` field)
- `src/mcp.rs:310-327` — `list_memories` MCP method (parse and pass `since`)
- `src/mcp.rs:237-246` — Recall's `since` parsing in MCP (pattern to copy)

## Architecture Documentation

The `since` filter pattern in pensieve follows a consistent 3-layer flow:

1. **CLI layer**: `Option<String>` raw date string
2. **Main dispatch**: Parse to `Option<DateTime<Utc>>` via
   `NaiveDate::parse_from_str + and_utc()`
3. **Ops layer**: Receive parsed `DateTime<Utc>`, filter results by
   `updated >= since`

The MCP layer mirrors this: `Option<String>` in params, parsed inline in the
tool method, passed to ops.

No other command has a `since` parameter — only `recall` (CLI + MCP).
