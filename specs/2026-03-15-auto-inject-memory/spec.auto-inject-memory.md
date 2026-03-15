# Auto-Inject Memory Recall

## Overview

Today, Pensieve relies on agents voluntarily calling `get_context` at session
start and `recall` when they think they need it. The problem: agents don't know
what they don't know. They miss relevant memories because they never search.

Every major agent memory system — OpenClaw, Mem0, CrewAI, LangGraph — has
converged on the same pattern: **automatically inject relevant memories before
each prompt**. The agent doesn't decide to recall; the system does it for them.

This spec adds **opt-in** automatic memory injection to Pensieve via agent hook
systems. When enabled, `pensieve inject` runs before every prompt and injects
relevant memories into the agent's context. The setup skill asks the user if
they want this feature and wires the hooks for their agent.

## Key Repositories

- `pensieve` (this repo) — the CLI + MCP binary, setup command, skill files
- `anthropics/claude-code` — Claude Code hooks API
  ([reference](https://code.claude.com/docs/en/hooks))
- `nickcursor/cursor` — Cursor hooks API ([docs](https://cursor.com/docs/hooks))
- `google-gemini/gemini-cli` — Gemini CLI hooks API
  ([docs](https://geminicli.com/docs/hooks/))
- `openai/codex` — Codex CLI hooks (experimental,
  [#13014](https://github.com/openai/codex/issues/13014))

## Platform Support

All four major AI agents now support hooks:

| Agent       | Pre-prompt hook      | Session hook   | Config location           | Status          |
| ----------- | -------------------- | -------------- | ------------------------- | --------------- |
| Claude Code | `UserPromptSubmit`   | `SessionStart` | `~/.claude/settings.json` | Shipped         |
| Cursor      | `beforeSubmitPrompt` | —              | `~/.cursor/hooks.json`    | Shipped (v1.7)  |
| Gemini CLI  | `BeforeAgent`        | `SessionStart` | `~/.gemini/settings.json` | Shipped (v0.26) |
| Codex CLI   | —                    | `SessionStart` | `.codex/hooks.json`       | Experimental    |

**Prompt delivery**: Claude Code passes the prompt via **stdin as JSON** with a
`"prompt"` field. Cursor and Gemini CLI stdin formats need verification (open
question from research). The `pensieve inject` command should support both stdin
JSON and a `--query` flag as fallback.

**Stdin JSON format** (Claude Code, confirmed):

```json
{
  "session_id": "abc123",
  "hook_event_name": "UserPromptSubmit",
  "prompt": "the user's actual prompt text",
  "cwd": "/path/to/project"
}
```

## Requirements

### R1: `pensieve inject` command

New CLI command optimized for hook injection — fast, compact output, designed to
run before every prompt:

```bash
# Primary: reads prompt from stdin JSON (hook mode)
pensieve inject [--project P] [--limit N] [--format compact|json]

# Fallback: direct query via flag (manual/testing mode)
pensieve inject --query "search text" [--project P] [--limit N]
```

**Stdin behavior**: Reads stdin, attempts to parse as JSON. If JSON with a
`"prompt"` field, uses that as the search query. If not JSON, treats entire
stdin as the query text. This handles Claude Code's JSON format and potential
plain-text formats from other agents.

**Output behavior**:

- Returns only results above the configured relevance threshold
- If no relevant memories found, outputs nothing (empty stdout = no injection)
- No output to stderr (would pollute agent context)
- If `inject.enabled = false` in config, outputs nothing immediately
- Exit code always 0 (never block the agent)

**Default compact format**:

```
[Pensieve: 2 relevant memories]
- (gotcha) API rate limit is per-user not per-key — project:myproject
- (decision) Use Postgres for sessions, Redis for cache — project:myproject
```

### R2: Setup skill wires hooks (opt-in)

The setup skill (not Rust code) handles hook configuration for all agents. This
is **opt-in** — the skill asks the user:

> "Would you like to enable auto-inject? This automatically recalls relevant
> memories before every prompt. You can disable it anytime with
> `pensieve configure --inject-enabled false`."

If the user says yes, the skill adds hooks to the agent's config file:

**Claude Code** (`~/.claude/settings.json`):

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "<bin> inject --limit 3",
            "timeout": 10
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "<bin> context --output json 2>/dev/null || true"
          }
        ]
      }
    ]
  }
}
```

**Cursor** (`~/.cursor/hooks.json`):

```json
{
  "version": 1,
  "hooks": {
    "beforeSubmitPrompt": [
      {
        "command": "<bin> inject --limit 3"
      }
    ]
  }
}
```

**Gemini CLI** (`~/.gemini/settings.json`):

```json
{
  "hooks": {
    "BeforeAgent": [
      {
        "type": "command",
        "command": "<bin> inject --limit 3"
      }
    ],
    "SessionStart": [
      {
        "type": "command",
        "command": "<bin> context --output json 2>/dev/null || true"
      }
    ]
  }
}
```

**Codex CLI**: No pre-prompt hook available yet. SessionStart hook only:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "<bin> context --output json 2>/dev/null || true"
      }
    ]
  }
}
```

The skill must:

- Read existing config, merge hooks (don't replace existing hooks)
- Check for "pensieve" in existing hook commands to avoid duplicates
- Use the full binary path from `pensieve setup` (not just `pensieve`)

If the user says no to auto-inject, the skill skips hook setup but still
completes the rest of setup (MCP registration, Memory Protocol).

### R3: Performance budget

Auto-injection runs before every prompt, so performance is critical:

- `pensieve inject` with keyword-only search: **<100ms**
- `pensieve inject` with hybrid search (keyword + vector): **<500ms**
- If embedding model not available: fall back to keyword-only silently
- If no results above threshold: output nothing, exit 0
- Timeout in hook config set to 10s (generous, but pensieve should finish in
  <500ms)

### R4: Configuration

New config options in `~/.config/pensieve/config.toml`:

```toml
[inject]
enabled = false               # opt-in, default off
relevance_threshold = 0.3     # minimum score to include
max_results = 3               # max memories to inject
format = "compact"            # compact | json | silent
```

Two ways to toggle:

- **CLI**: `pensieve configure --inject-enabled true` /
  `pensieve configure --inject-enabled false`
- **Setup skill**: asks the user during setup, calls `pensieve configure`

When `inject.enabled = false`, the `pensieve inject` command outputs nothing and
exits 0 immediately. The hooks remain in place but are no-ops — no need to
remove them to disable.

### R5: SessionStart hook for context recovery

Agents that support `SessionStart` hooks (Claude Code, Gemini CLI, Codex CLI)
get automatic context recovery:

- Fires on session startup, resume, `/clear`, and **after compaction**
- Runs `pensieve context --output json` to reload prior knowledge
- Solves the "lost context after compaction" problem without agent action
- This hook is always wired (not gated by `inject.enabled`) since it's the
  session bootstrap, not per-prompt injection

### R6: Update README with auto-inject documentation

Add an "Auto-inject" section to the README explaining:

- **What it does**: Relevant memories are automatically injected before every
  prompt — the agent doesn't need to remember to search
- **Why it matters**: Agents don't know what they don't know. Without
  auto-inject, they miss relevant memories because they never search. Every
  major agent memory system (OpenClaw, Mem0, CrewAI, LangGraph) has converged on
  this pattern.
- **It's opt-in**: Disabled by default. Enable during `pensieve setup` or with
  `pensieve configure --inject-enabled true`.
- **Platform support table**:

| Agent          | Auto-inject          | Session recovery | Mechanism               |
| -------------- | -------------------- | ---------------- | ----------------------- |
| Claude Code    | Yes                  | Yes              | `UserPromptSubmit` hook |
| Cursor         | Yes                  | —                | `beforeSubmitPrompt`    |
| Gemini CLI     | Yes                  | Yes              | `BeforeAgent` hook      |
| Codex CLI      | Not yet              | Yes              | `SessionStart` only     |
| Claude Desktop | No (no hook support) | No               | Manual MCP recall       |

- **How to disable**: `pensieve configure --inject-enabled false`

## Acceptance Criteria

### Core inject command

- `pensieve inject` reads prompt from stdin JSON (`"prompt"` field) and returns
  relevant memories in compact format
- `pensieve inject --query "test"` works as direct fallback
- `pensieve inject` with no matches returns empty stdout
- `pensieve inject` with `inject.enabled = false` returns empty stdout
  immediately
- `pensieve inject` completes in <100ms for keyword-only search
- `pensieve inject` with no embedding model falls back to keyword-only (no
  crash, no stderr)

### Setup and hooks

- Setup skill asks user if they want auto-inject before wiring hooks
- If user declines, no hooks are added but rest of setup completes
- Setup skill wires correct hook format for each detected agent (Claude Code,
  Cursor, Gemini CLI, Codex CLI)
- Running setup twice doesn't duplicate hooks
- Hooks use the full binary path
- After setup with auto-inject enabled, relevant memories appear before every
  prompt in Claude Code

### Context recovery

- `SessionStart` hook wired for agents that support it (always, not gated by
  inject.enabled)
- After context compaction in Claude Code, session context is auto-recovered

### Configuration

- `pensieve configure --inject-enabled true` enables auto-inject
- `pensieve configure --inject-enabled false` disables without removing hooks
- Config defaults to `inject.enabled = false` (opt-in)

### Documentation

- README has "Auto-inject" section with platform table, opt-in explanation, and
  disable instructions

## Out of Scope

- Building hook systems for agents that don't support them yet
- Modifying the MCP protocol to support auto-injection
- Real-time memory streaming during responses
- Cross-agent hook standardization
- Windows support
- Prompt-level caching (same prompt submitted twice)
- Verifying Cursor/Gemini stdin JSON format (noted as open question — implement
  Claude Code first, add others when verified)

## Testing Strategy

- **Performance test**: Benchmark `pensieve inject` with 100+ memories, verify
  <100ms for keyword-only
- **Stdin JSON test**: Pipe Claude Code hook JSON to `pensieve inject`, verify
  prompt is extracted and results returned
- **Plain stdin test**: Pipe plain text to `pensieve inject`, verify it's used
  as query
- **Fallback test**: `pensieve inject --query "test"` works without stdin
- **Threshold test**: Save memories with varying relevance, verify only
  above-threshold results returned
- **Empty result test**: Query with no matches, verify empty stdout
- **Offline test**: Run with no embedding model, verify keyword-only fallback
  without crash or stderr
- **Disabled test**: Set `inject.enabled = false`, verify empty stdout
- **Idempotency test**: Run setup skill twice, verify no duplicate hooks in
  settings.json
- **Compaction recovery test**: Simulate `SessionStart` hook, verify context
  output
- **Opt-in test**: Verify setup skill asks before wiring hooks

## Implementation Notes

- **Stdin JSON confirmed** (Claude Code): Hook passes `{"prompt": "..."}` via
  stdin. `pensieve inject` should try JSON parse first, fall back to raw text.
- **Cursor and Gemini stdin format**: Not yet verified. Implement Claude Code
  first, then verify and add adapter logic if needed.
- **JSON merging**: The skill must read existing settings.json, parse JSON, add
  to hook arrays without replacing existing hooks from other tools.
- **`inject` reuses recall**: The inject command should call the same
  `ops::recall::recall()` function with a threshold filter and compact output
  formatter — minimal new code.
- **SessionStart is separate from inject**: The SessionStart hook runs
  `pensieve context`, not `pensieve inject`. It's always wired (not opt-in)
  because it's the session bootstrap mechanism.
