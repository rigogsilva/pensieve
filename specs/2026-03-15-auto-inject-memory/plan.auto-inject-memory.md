---
date: 2026-03-15T09:29:19Z
git_commit: 07c6a16
branch: auto-inject-memory
repository: pensieve
spec: spec.auto-inject-memory.md
research: research.auto-inject-memory.md
worktrees:
  - /Users/rigo/Documents/Projects/worktrees/auto-inject-memory/implement/pensieve/
status: complete
last_updated: 2026-03-15
---

# Implementation Plan: Auto-Inject Memory Recall

**Spec**: spec.auto-inject-memory.md **Research**:
research.auto-inject-memory.md **Date**: 2026-03-15T09:29:19Z

## Overview

Add opt-in automatic memory injection to Pensieve. A new `pensieve inject`
command reads the user's prompt from stdin JSON, runs hybrid recall, and outputs
compact results for hook injection. The setup skill is updated to optionally
wire pre-prompt hooks for Claude Code, Cursor, Gemini CLI, and Codex CLI.
Configuration via `inject.enabled` in config.toml gates the feature.

## Progress

- [x] Worktree setup
- [x] Phase 1: inject command + config
- [x] Phase 2: update setup skill with opt-in hooks
- [x] Phase 3: README auto-inject section
- [x] Testing
- [x] AGENTS.md check
- [x] Spec update
- [x] Worktree teardown

## Worktree Setup

| Repo       | Worktree path                                                                     | Base branch          |
| ---------- | --------------------------------------------------------------------------------- | -------------------- |
| `pensieve` | `/Users/rigo/Documents/Projects/worktrees/auto-inject-memory/implement/pensieve/` | `auto-inject-memory` |

---

## Phase 1: inject command + config

_Why first: the inject command must exist before hooks can call it, and config
must support the inject settings before the command can check them._

### 1.1 Add inject config to types

**What changes**: `src/types.rs` — add `InjectConfig` struct to `PensieveConfig`
**How to verify**: `cargo build` passes. Config deserialization handles missing
`[inject]` section with defaults.

Add to `PensieveConfig`:

```rust
#[serde(default)]
pub inject: InjectConfig,
```

`InjectConfig` struct:

```rust
pub struct InjectConfig {
    pub enabled: bool,           // default: false
    pub relevance_threshold: f64, // default: 0.3
    pub max_results: usize,      // default: 3
    pub format: String,          // default: "compact"
}
```

### 1.2 Add --inject-enabled to configure CLI + ops

**What changes**: `src/cli.rs` (Configure command), `src/ops/configure.rs`,
`src/main.rs` **How to verify**: `pensieve configure --inject-enabled true`
updates config.toml with `[inject] enabled = true`.
`pensieve configure --inject-enabled false` sets it back. `pensieve configure`
(no args) shows inject settings.

### 1.3 Implement pensieve inject command

**What changes**: Create `src/ops/inject.rs`, add `Inject` to CLI commands in
`src/cli.rs`, dispatch in `src/main.rs`, add to `src/ops/mod.rs` **How to
verify**: `echo '{"prompt":"test"}' | pensieve inject` returns compact results.
`pensieve inject --query "test"` works as fallback.

Implementation:

- **Stdin parsing**: Read stdin. Try JSON parse → extract `"prompt"` field. If
  not JSON, use entire stdin as query text. If stdin is empty and `--query` not
  provided, output nothing and exit 0.
- **Config gate**: If `inject.enabled == false`, output nothing, exit 0
  immediately.
- **Recall**: Call `ops::recall::recall()` with query, limit from config
  (`inject.max_results`), no filters.
- **Threshold filter**: Drop results with score below
  `inject.relevance_threshold`.
- **Compact formatter**: For each result above threshold:
  ```
  [Pensieve: N relevant memories]
  - (type) title — project:name
  - (type) title — project:name
  ```
  If no results, output nothing (empty stdout).
- **Error handling**: No stderr output. All errors → silent exit 0. Never block
  the agent.
- **JSON format**: If `inject.format == "json"`, output JSON array instead of
  compact text.

CLI definition:

```rust
Inject {
    #[arg(long)]
    query: Option<String>,
    #[arg(long)]
    project: Option<String>,
    #[arg(long)]
    limit: Option<usize>,
    #[arg(long)]
    format: Option<String>,
    #[arg(long)]
    output: Option<OutputFormat>,
}
```

### 1.4 Add inject to MCP server

**What changes**: `src/mcp.rs` — add `inject` tool **How to verify**: MCP
`inject` tool callable via `pensieve serve`, returns same results as CLI.

Add a `inject` tool that accepts `query`, `project`, `limit`, `format` params.
Calls the same `ops::inject` logic. This gives MCP-connected agents access to
the inject functionality without hooks.

---

## Phase 2: Update setup skill with opt-in hooks

_Why after Phase 1: the skill references the inject command which must exist._

### 2.1 Update unified skill content in setup.rs

**What changes**: `src/ops/setup.rs` — update `unified_skill_content()` **How to
verify**: `pensieve setup` generates skill with Step 3 (hooks) that asks the
user and includes per-agent hook configs.

The skill's Step 3 (auto-inject hooks) must:

1. Ask the user: "Would you like to enable auto-inject? This automatically
   recalls relevant memories before every prompt."
2. If yes: run `<bin> configure --inject-enabled true`
3. If yes: add hooks to the agent's config file (per-agent format from spec R2)
4. If no: skip hooks, continue to Step 4 (verify)

The hook command for all agents is: `<bin> inject --limit 3`

The SessionStart hook command is:
`<bin> context --output json 2>/dev/null || true`

SessionStart hook is always wired (not gated by opt-in question) since it's
session bootstrap, not per-prompt injection.

Include per-agent hook configs for:

- Claude Code (`~/.claude/settings.json`)
- Cursor (`~/.cursor/hooks.json`)
- Gemini CLI (`~/.gemini/settings.json`)
- Codex CLI (`.codex/hooks.json` — SessionStart only)

The skill must instruct the agent to:

- Read existing config JSON, merge hooks (don't replace)
- Check for "pensieve" in existing commands to avoid duplicates
- Use the full binary path

---

## Phase 3: README auto-inject section

_Independent of Phase 1 and 2 but should reference the inject command._

### 3.1 Add Auto-inject section to README

**What changes**: `README.md` **How to verify**: README has "Auto-inject"
section with platform table, opt-in explanation, and disable instructions.

Add after the "How agents use Pensieve" section:

- What it does
- Why it matters (research-backed: OpenClaw, Mem0, CrewAI, LangGraph all do it)
- It's opt-in (default off)
- Platform support table from spec R6
- How to enable/disable

---

## Testing

All tests in `tests/integration_tests.rs`.

- **test_inject_stdin_json**: Pipe `{"prompt":"test"}` to inject subprocess,
  verify compact output contains matching memory → maps to step 1.3
- **test_inject_query_flag**: `pensieve inject --query "test"`, verify results →
  maps to step 1.3
- **test_inject_plain_stdin**: Pipe plain text to inject, verify it's used as
  query → maps to step 1.3
- **test_inject_empty_result**: Pipe query with no matches, verify empty stdout
  → maps to step 1.3
- **test_inject_disabled**: Set `inject.enabled = false`, pipe query, verify
  empty stdout → maps to steps 1.1, 1.3
- **test_inject_threshold**: Save memories, inject with high threshold, verify
  only high-score results returned → maps to step 1.3
- **test_inject_no_stderr**: Pipe query to inject, capture stderr, verify empty
  → maps to step 1.3
- **test_configure_inject_enabled**: `pensieve configure --inject-enabled true`,
  verify config file updated → maps to step 1.2
- **test_inject_offline**: Run inject without embedding model, verify
  keyword-only results (no crash) → maps to step 1.3

---

## Risk Callouts

> **⚠ Research gap**: Cursor and Gemini CLI stdin JSON format is unverified. The
> spec defers this — implement Claude Code format first. The inject command's
> stdin parsing (try JSON → fall back to raw text) should handle different
> formats gracefully, but the hook configs for Cursor and Gemini in the skill
> may need adjustment once verified.

---

## Closing Steps

- [ ] **Check `AGENTS.md`**: This change adds a new CLI command (`inject`) and
      new config options (`[inject]` section). Update AGENTS.md CLI Subcommands
      list and Architecture section.

- [ ] **Update spec**: Update Implementation Notes with decisions made,
      performance observations, and any deviations from the plan.

- [ ] **Worktree teardown**:
  ```bash
  cd /Users/rigo/Documents/Projects/worktrees/auto-inject-memory/implement/pensieve
  git add -A
  git commit -m "auto-inject-memory: implement pensieve inject command with opt-in hooks"
  git push --set-upstream origin auto-inject-memory-impl
  git -C /Users/rigo/Documents/Projects/pensieve worktree remove /Users/rigo/Documents/Projects/worktrees/auto-inject-memory/implement/pensieve
  ```
