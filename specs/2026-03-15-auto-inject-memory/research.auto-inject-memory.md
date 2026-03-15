---
date: 2026-03-15T08:30:00Z
git_commit: 43621d1
branch: main
repository: pensieve
topics:
  - auto-inject-memory
  - claude-code-hooks
  - agent-hooks-api
  - pre-prompt-injection
  - cross-agent-hooks
tags: [research, hooks, claude-code, codex, cursor, gemini, auto-inject]
status: complete
last_updated: 2026-03-15
last_updated_by: claude-code
---

# Research for 2026-03-15-auto-inject-memory / spec.auto-inject-memory.md

**Date**: 2026-03-15T08:30:00Z **Git Commit**: 43621d1 **Branch**: main
**Repository**: pensieve

## Summary

Research covered three areas: the current pensieve recall/context implementation
(to understand what `inject` needs to wrap), the Claude Code hooks API (exact
format, env vars, stdin protocol), and the hook systems of all major agents
(Codex, Cursor, Gemini CLI). The key finding is that **all four major agents now
have pre-prompt hooks** — this is not a Claude-only feature. The spec should
target all four platforms.

Critical correction: the spec assumed the prompt is passed via `$PROMPT` env
var, but Claude Code actually passes it via **stdin as JSON** with a `"prompt"`
field.

## Detailed Findings

### 1. Current Pensieve Recall Implementation

**`src/ops/recall.rs`** — the core search function:

- **RecallInput** struct: query, memory_type, project, tags, status, since,
  limit
- **No-query path**: Lists memories from storage with filters, returns
  MemoryCompact list
- **Hybrid path** (when query present):
  1. BM25 keyword search via `index.recall_keyword()` — returns (memory_id,
     score) pairs
  2. Vector search via `index.recall_vector()` — returns (memory_id, distance)
     pairs
  3. Normalize both score sets independently
  4. Blend:
     `final = keyword_weight * bm25_norm + vector_weight * (1 - vec_distance)`
  5. Sort by blended score, truncate to limit
  6. Load full memories from disk, apply filters (project, type, status, since,
     tags)
  7. Return MemoryCompact with score

**Performance characteristics**:

- Index queries use `Mutex<Connection>` — serializes all concurrent access
- BM25 search: single SQLite FTS5 query with `bm25()` function — fast
- Vector search: single SQLite vec0 query — fast
- Embedding: ~10-50ms per query via fastembed (if model available)
- File I/O: each ranked result loads full memory from disk (N file reads)
- **Bottleneck for inject**: the embedding step (~10-50ms) and per-result file
  reads. Keyword-only search should easily meet <100ms.

**`src/embedder.rs`** — graceful degradation:

- `OnceLock<Option<Mutex<TextEmbedding>>>` — initialized once, None if model
  download fails
- `try_embed()` returns `Option<Vec<f32>>` — callers handle None gracefully
- If offline: embed returns Err, recall falls back to keyword-only

**`src/ops/context.rs`** — session bootstrap:

- Returns: sessions (last 3), preferences (all active), recent gotchas/decisions
  (30 days), stale memories (>90 days)
- Version check: spawns thread with 2s timeout, caches for 24h
- Writes CONTEXT.md to memory_dir (truncated to 200 lines)

### 2. Claude Code Hooks API — Exact Specifications

**Prompt delivery**: Via **stdin JSON**, NOT environment variable.

```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "UserPromptSubmit",
  "prompt": "the user's actual prompt text"
}
```

**Stdout injection**: Plain text stdout is added as context Claude can see.
Alternatively, structured JSON with `additionalContext` field:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "relevant memory content here"
  }
}
```

**Exit codes**: 0 = success (inject stdout), 2 = block (erase prompt from
context), other = non-blocking error (stderr in verbose only).

**Timeout**: 600 seconds default, configurable per-hook via `"timeout"` field.

**SessionStart sources**: `"startup"`, `"resume"`, `"clear"`, `"compact"` — yes,
fires after compaction.

**settings.json schema**:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "matcher": "optional_regex",
        "hooks": [
          {
            "type": "command",
            "command": "path/to/script",
            "timeout": 600,
            "statusMessage": "Loading memories...",
            "once": false,
            "async": false
          }
        ]
      }
    ]
  }
}
```

**Multiple hooks**: Yes — outer array supports multiple matcher groups per
event. Hooks within a group run sequentially. Identical handlers are
auto-deduplicated.

**Config precedence**: Project-level (`.claude/settings.json`) > user-level
(`~/.claude/settings.json`).

**Environment variables**: `CLAUDE_PROJECT_DIR` (all hooks), `CLAUDE_ENV_FILE`
(SessionStart only).

Sources:

- [Official Hooks Reference](https://code.claude.com/docs/en/hooks)
- [Anthropic Docs](https://docs.anthropic.com/en/docs/claude-code/hooks)
- [Claude Blog](https://claude.com/blog/how-to-configure-hooks)
- [disler/claude-code-hooks-mastery](https://github.com/disler/claude-code-hooks-mastery)
- [Anthropic hook development SKILL.md](https://github.com/anthropics/claude-code/blob/main/plugins/plugin-dev/skills/hook-development/SKILL.md)

### 3. Cross-Agent Hook Support

**All four major agents now have pre-prompt hooks:**

| Agent       | Pre-prompt hook      | Session hook     | Config location           | Status          |
| ----------- | -------------------- | ---------------- | ------------------------- | --------------- |
| Claude Code | `UserPromptSubmit`   | `SessionStart`   | `~/.claude/settings.json` | Shipped         |
| Cursor      | `beforeSubmitPrompt` | (not documented) | `~/.cursor/hooks.json`    | Shipped (v1.7)  |
| Gemini CLI  | `BeforeAgent`        | `SessionStart`   | `~/.gemini/settings.json` | Shipped (v0.26) |
| Codex CLI   | (not yet)            | `SessionStart`   | `.codex/hooks.json`       | Experimental    |

**Cursor** (most mature): 6+ hook events including `beforeSubmitPrompt`,
`beforeShellExecution`, `afterShellExecution`, `beforeMCPExecution`,
`afterMCPExecution`, `Stop`. Partners like Snyk use hooks for security scanning.
Config is `~/.cursor/hooks.json` (version 1 JSON schema).

**Gemini CLI**: `BeforeAgent` fires after user submits prompt but before agent
begins planning. Hooks run synchronously. Also has `SessionStart`, `SessionEnd`,
`BeforeTool`, `AfterTool`. Extensions can bundle hooks.

**Codex CLI**: Experimental hooks engine since ~v0.114.0. Currently only
`SessionStart` and `Stop`. GitHub issue
[#13014](https://github.com/openai/codex/issues/13014) explicitly requests
"Claude parity" for more hooks. Plugin system via `codex.yaml` has additional
lifecycle hooks (`beforePlan`, `afterCode`, `onError`).

**No formal standard**: All four tools have converged on similar patterns (JSON
config, lifecycle event names, stdin/stdout context passing) but there is no
formal "Agent Hooks Specification" across tools.

Sources:

- [Cursor Hooks Docs](https://cursor.com/docs/hooks)
- [Cursor 1.7 announcement](https://www.infoq.com/news/2025/10/cursor-hooks/)
- [Gemini CLI Hooks Docs](https://geminicli.com/docs/hooks/)
- [Gemini CLI Hooks Reference](https://geminicli.com/docs/hooks/reference/)
- [Google Developers Blog](https://developers.googleblog.com/tailor-gemini-cli-to-your-workflow-with-hooks/)
- [Codex CLI #13014](https://github.com/openai/codex/issues/13014)
- [Codex CLI #2109](https://github.com/openai/codex/issues/2109)

### 4. Implications for pensieve inject

**Stdin vs env var**: The `pensieve inject` command needs to read the prompt
from stdin (JSON), not from a command-line argument. The hook command would be:

```bash
pensieve inject --limit 3
```

And `inject` reads stdin, parses the JSON, extracts the `prompt` field, runs
recall against it.

For Cursor and Gemini, need to verify if they pass the prompt the same way
(stdin JSON) or differently.

**Performance**: The 600s timeout is not a concern. The real constraint is UX —
users will feel lag if inject takes >200ms. Current recall implementation should
meet <100ms for keyword-only (SQLite FTS5 query is fast, no file I/O needed if
we return compact results from the index directly).

**The skill approach**: Since the skill already tells the agent to add hooks,
and all 4 agents support hooks (or will soon), the skill should include sections
for Claude Code, Cursor, Gemini CLI, and Codex CLI with their respective config
formats.

## Code References

- `src/ops/recall.rs` — hybrid recall implementation
- `src/ops/context.rs` — context assembly + version check + CONTEXT.md writer
- `src/index.rs` — SQLite FTS5 + vec0 queries, Mutex<Connection>
- `src/embedder.rs` — OnceLock<Option<Mutex<TextEmbedding>>> graceful
  degradation
- `src/main.rs:152-188` — CLI recall command dispatch
- `src/ops/setup.rs` — current skill content generation + hook wiring

## Open Questions

1. **Stdin format for Cursor and Gemini**: Do `beforeSubmitPrompt` (Cursor) and
   `BeforeAgent` (Gemini) pass the prompt via stdin JSON like Claude Code? Or do
   they use a different mechanism? This determines whether `pensieve inject` can
   be a single command for all agents or needs per-agent adapters.

2. **Compact output from index**: Currently recall loads full Memory from disk
   for each result. For inject performance, could we return MemoryCompact
   directly from the FTS5 index (title + first N chars of content are already
   indexed) without hitting the filesystem?

3. **Threshold tuning**: What's the right default relevance threshold for
   auto-inject? Too low = noise on every prompt. Too high = misses relevant
   memories. Needs empirical testing.

4. **Token budget**: How many tokens should auto-injected memories consume? If
   we inject 3 memories with previews, that's ~100-200 tokens. If we inject full
   content, could be 500-1000+. The compact format in the spec (~2 lines per
   memory) is the right default.
