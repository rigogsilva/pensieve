---
date: 2026-03-21T07:54:03+0000
git_commit: 19e1c1f0b3dd1078c3d5a3c400f6419468cdb770
branch: feat/list-since-filter
repository: pensieve
spec: spec.list-since-filter.md
research: research.list-since-filter.md
worktrees: []
status: in-progress
last_updated: 2026-03-21
---

# Implementation Plan: Add `--since` filter to `pensieve list`

**Spec**: spec.list-since-filter.md **Research**: research.list-since-filter.md
**Date**: 2026-03-21T07:54:03+0000

## Overview

Add a `--since` date filter to the `pensieve list` command (CLI and MCP) so
users and agents can query memories by recency without needing a search query.
The implementation reuses the existing `--since` pattern from `recall` and
extends it with flexible date parsing (`yesterday`, `today`, ISO datetime).

## Progress

- [ ] Phase 1: Shared date parser
- [ ] Phase 2: Wire `--since` into list (CLI, ops, MCP)
- [ ] Phase 3: Update recall to use shared parser
- [ ] Phase 4: Testing
- [ ] Phase 5: Skill updates
- [ ] Closing steps

---

## Phase 1: Shared date parser

_Must come first — phases 2 and 3 depend on this function._

### Step 1.1: Create `parse_since_date` helper

**What changes**: New file `src/date_utils.rs` **How to verify**: `cargo build`
compiles; unit test passes

Create `src/date_utils.rs` with a single public function:

```rust
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

pub fn parse_since_date(input: &str) -> Result<DateTime<Utc>, String> {
    // try keywords first
    // "today" → today at 00:00:00 UTC
    // "yesterday" → yesterday at 00:00:00 UTC
    // try YYYY-MM-DD
    // try YYYY-MM-DDTHH:MM:SS
    // error with: "Invalid date '{}'. Accepted formats: YYYY-MM-DD, YYYY-MM-DDTHH:MM:SS, 'yesterday', 'today'"
}
```

Parse order:

1. `"today"` → `Utc::now().date_naive().and_hms_opt(0,0,0).unwrap().and_utc()`
2. `"yesterday"` → same but `Utc::now().date_naive().pred_opt().unwrap()...`
3. `NaiveDate::parse_from_str(input, "%Y-%m-%d")` →
   `.and_hms_opt(0,0,0).unwrap().and_utc()`
4. `NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S")` → `.and_utc()`
5. Return `Err` with clear message

Register the module in `src/lib.rs` with `pub mod date_utils;`.

---

## Phase 2: Wire `--since` into list

_Depends on Phase 1. The 3 steps within this phase are independent of each
other._

### Step 2.1: Add `--since` flag to CLI

**What changes**: `src/cli.rs` — `List` variant (lines 125-141) **How to
verify**: `cargo build` compiles; `pensieve list --help` shows `--since`

Add after the `status` field:

```rust
/// Only memories updated after this date (YYYY-MM-DD, yesterday, today)
#[arg(long)]
since: Option<String>,
```

### Step 2.2: Update `ops/list.rs` to accept and filter by `since`

**What changes**: `src/ops/list.rs` (lines 5-13) **How to verify**: Function
signature accepts `since` parameter

Add `since: Option<&DateTime<Utc>>` parameter. After mapping to `MemoryCompact`,
filter:

```rust
let memories: Vec<MemoryCompact> = memories
    .iter()
    .map(MemoryCompact::from)
    .filter(|m| since.map_or(true, |s| m.updated >= *s))
    .collect();
```

### Step 2.3: Update `main.rs` List handler

**What changes**: `src/main.rs` — `Command::List` match arm (lines 205-222)
**How to verify**: `cargo build` compiles

- Add `since` to destructuring
- Parse with `parse_since_date`, handle `Err` with `eprintln!` + `exit(1)`
- Pass `since.as_ref()` to `ops::list::list_memories()`

### Step 2.4: Update MCP `ListMemoriesParams` and method

**What changes**: `src/mcp.rs` — `ListMemoriesParams` (lines 91-99) and
`list_memories` method (lines 310-327) **How to verify**: MCP tool accepts
`since` parameter

- Add `/// Only memories updated after this ISO date` and
  `pub since: Option<String>` to `ListMemoriesParams`
- In `list_memories` method, parse `since` with `parse_since_date` (silently
  ignore parse errors for MCP, same as type/status)
- Pass to `ops::list::list_memories()`

---

## Phase 3: Update recall to use shared parser

_Independent of Phase 2. Depends on Phase 1._

### Step 3.1: Refactor recall CLI handler to use `parse_since_date`

**What changes**: `src/main.rs` — `Command::Recall` match arm (lines 172-178)
**How to verify**: `pensieve recall "test" --since yesterday` works

Replace the inline `chrono::NaiveDate::parse_from_str` block with:

```rust
let since = since.map(|s| {
    date_utils::parse_since_date(&s).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    })
});
```

### Step 3.2: Refactor recall MCP handler to use `parse_since_date`

**What changes**: `src/mcp.rs` — recall method (lines 237-246) **How to
verify**: MCP recall with `since` parameter still works

Replace inline parsing with `parse_since_date`. Silently ignore errors (`.ok()`)
to match existing MCP behavior.

---

## Phase 4: Testing

### Step 4.1: Build and run manual CLI tests

**What changes**: None — verification only **How to verify**: All commands
produce expected output

```bash
cargo build --release
# Date format tests
./target/release/pensieve list --since today --output json
./target/release/pensieve list --since yesterday --output json
./target/release/pensieve list --since 2026-03-20 --output json
./target/release/pensieve list --since 2026-03-20T12:00:00 --output json

# Filter composition
./target/release/pensieve list --since 2026-03-20 --project augmentt --output json

# Error handling
./target/release/pensieve list --since invalid-date

# Unchanged behavior
./target/release/pensieve list --output json

# Recall still works with new parser
./target/release/pensieve recall "test" --since yesterday --output json
```

### Step 4.2: Run clippy

**What changes**: None — verification only **How to verify**: No warnings

```bash
cargo clippy -- -D warnings
```

---

## Phase 5: Skill updates

_Independent of Phases 2-4 but should reflect the final CLI behavior._

### Step 5.1: Update pensieve-setup skill

**What changes**: `~/.claude/skills/pensieve-setup/SKILL.md` — canonical Memory
Protocol block **How to verify**: `list --since` example visible in the CLI
usage section

Add `list --since` example alongside the existing `list --output json` example
in the canonical block:

```bash
/Users/rigo/bin/pensieve list --since yesterday --output json
```

### Step 5.2: Run pensieve setup

**What changes**: `~/.claude/CLAUDE.md`, `~/.codex/AGENTS.md` **How to verify**:
Both files contain the updated Memory Protocol block

Run `pensieve setup` to sync the canonical block to all agents.

---

## Closing Steps

- [ ] **Check `AGENTS.md`**: No new CLI commands or env vars introduced — just a
      new flag on an existing command. No update needed.
- [ ] **Update spec**: Add implementation decisions and testing outcomes to
      `Implementation Notes` section of the spec.
