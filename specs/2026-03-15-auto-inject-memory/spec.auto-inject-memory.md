# Auto-Inject Memory Recall

## Overview

Today, Pensieve relies on agents voluntarily calling `get_context` at session
start and `recall` when they think they need it. The problem: agents don't know
what they don't know. They miss relevant memories because they never search.

Every major agent memory system — OpenClaw, Mem0, CrewAI, LangGraph — has
converged on the same pattern: **automatically inject relevant memories before
each prompt**. The agent doesn't decide to recall; the system does it for them.

This spec adds automatic memory injection to Pensieve via agent hook systems
(Claude Code's `UserPromptSubmit`, and equivalent mechanisms for other agents).
`pensieve setup` wires the hook automatically — zero manual configuration.

## Key Repositories

- `pensieve` (this repo) — the CLI + MCP binary, setup command, skill files
- `anthropics/claude-code` — Claude Code hooks API reference

## Research

### Industry patterns

| Framework | Auto-inject? | Mechanism                                                                     |
| --------- | ------------ | ----------------------------------------------------------------------------- |
| OpenClaw  | Yes          | `MEMORY.md` injected every turn + `auto-recall` plugin for semantic search    |
| Mem0      | Yes          | Fetches relevant memories per query, injects as system context                |
| CrewAI    | Yes          | Contextual memory assembly before each task — short/long/entity memory merged |
| LangGraph | Yes          | Pre-model hook with `store.search()` before LLM processes input               |
| Cursor    | Partial      | Auto-creates memories from past conversations, injects into future sessions   |
| Codex CLI | No           | Static `AGENTS.md` only                                                       |

Sources:

- [OpenClaw context docs](https://docs.openclaw.ai/concepts/context) — bootstrap
  files injected every turn, compact "signifier" list for skills
- [openclaw-memory-auto-recall plugin](https://github.com/code-yeongyu/openclaw-memory-auto-recall)
  — semantic search before each prompt
- [VelvetShark OpenClaw Memory Masterclass](https://velvetshark.com/openclaw-memory-masterclass)
- [Mem0](https://mem0.ai/) — 90% token reduction vs full-context, 26% higher
  than OpenAI built-in memory on LOCOMO benchmark
- [CrewAI memory docs](https://docs.crewai.com/en/concepts/memory) — contextual
  memory assembly before each task
- [LangMem SDK](https://blog.langchain.com/langmem-sdk-launch/) — pre-model hook
  pattern
- [AWS Memory-Augmented Agents](https://docs.aws.amazon.com/prescriptive-guidance/latest/agentic-ai-patterns/memory-augmented-agents.html)
  — formal pattern definition

### Claude Code hooks API

Claude Code supports hooks that fire on specific events. The two relevant ones:

- **`SessionStart`** — fires when session starts or resumes after compaction.
  Stdout is injected into context.
- **`UserPromptSubmit`** — fires before every prompt, before Claude processes
  it. Stdout is injected into context alongside the user's message.

Configuration in `~/.claude/settings.json` (user-level):

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "pensieve inject \"$PROMPT\" --limit 3"
          }
        ]
      }
    ]
  }
}
```

Key details:

- Only `SessionStart` and `UserPromptSubmit` inject stdout as visible context
- Exit code 0 = proceed (inject stdout), exit code 2 = block
- After compaction, `SessionStart` fires again — can reinject critical context
- Hooks should be fast (runs before every response)

Sources:

- [Claude Code hooks reference](https://code.claude.com/docs/en/hooks)
- [Claude blog — how to configure hooks](https://claude.com/blog/how-to-configure-hooks)
- [disler/claude-code-hooks-mastery](https://github.com/disler/claude-code-hooks-mastery)

### Other agent hook mechanisms

- **Codex CLI**: No documented pre-prompt hook system yet. Fallback: static
  `AGENTS.md` with `@~/.pensieve/memory/CONTEXT.md` import.
- **Cursor**: No pre-prompt hook. Uses `.cursorrules` and `AGENTS.md` for static
  context.
- **Claude Desktop**: No hooks. Uses MCP `instructions` field for static
  context.

### Security considerations

- [OWASP Top 10 for Agentic Applications (2026)](https://blogs.cisco.com/ai/personal-ai-agents-like-openclaw-are-a-security-nightmare)
  — auto-injected memory is an attack surface. Poisoned entries retrieved by RAG
  can influence agent behavior. Pensieve mitigates this by being local-only (no
  shared memory store, no external retrieval).

## Requirements

### R1: `pensieve inject` command

New CLI command optimized for hook injection — fast, compact output, designed to
run on every prompt:

```bash
pensieve inject "the user's prompt text here" [--project P] [--limit N] [--format compact|json]
```

Behavior:

- Runs hybrid recall against the prompt text
- Returns only high-relevance results (above a configurable threshold)
- Output format optimized for injection: compact markdown, not full JSON
- If no relevant memories found, outputs nothing (empty stdout = no injection)
- Must be fast — target <100ms for keyword-only, <500ms with embeddings
- Gracefully handles offline/no-model (keyword-only fallback, no crash)
- No output to stderr (would pollute agent context)

Default compact format:

```
[Pensieve: 2 relevant memories]
• (gotcha) API rate limit is per-user not per-key — project:myproject
• (decision) Use Postgres for sessions, Redis for cache — project:myproject
```

### R2: `pensieve setup` wires hooks automatically

Extend `pensieve setup` to configure hooks for agents that support them:

- **Claude Code**: Add `UserPromptSubmit` hook to `~/.claude/settings.json` that
  runs `pensieve inject "$PROMPT"`. Also add `SessionStart` hook that runs
  `pensieve context`. Use marker-based idempotency (check before adding, don't
  duplicate).
- **Other agents**: No hooks available — continue with current skill-based
  approach (manual `get_context` at session start).

The setup skill should be updated to explain that hooks are configured and
memories will be auto-injected.

### R3: Performance budget

Auto-injection runs on every prompt, so performance is critical:

- `pensieve inject` with keyword-only search: <100ms
- `pensieve inject` with hybrid search (keyword + vector): <500ms
- If embedding model not available: fall back to keyword-only silently
- If no results above threshold: output nothing, exit 0

### R4: Configuration

New config options in `~/.config/pensieve/config.toml`:

```toml
[inject]
enabled = true
relevance_threshold = 0.3    # minimum score to include
max_results = 3              # max memories to inject
format = "compact"           # compact | json | silent
```

`pensieve configure --inject-enabled false` to disable without removing hooks.

### R5: Update README with auto-inject documentation

Add an "Auto-inject" section to the README explaining:

- **What it does**: Relevant memories are automatically injected before every
  prompt — the agent doesn't need to remember to search
- **Why it matters**: Agents don't know what they don't know. Without
  auto-inject, they miss relevant memories because they never search. Every
  major agent memory system (OpenClaw, Mem0, CrewAI, LangGraph) has converged on
  this pattern.
- **Platform support table**:

| Agent          | Auto-inject | Mechanism               | Fallback          |
| -------------- | ----------- | ----------------------- | ----------------- |
| Claude Code    | Yes         | `UserPromptSubmit` hook | —                 |
| Claude Desktop | No          | No hook support         | Manual MCP recall |
| Codex CLI      | No          | No hook support yet     | Manual CLI recall |
| Cursor         | No          | No hook support         | Manual CLI recall |

- **How it works**: `pensieve setup` wires the hooks automatically. The
  `UserPromptSubmit` hook runs `pensieve recall` with the user's prompt and
  injects matching memories. The `SessionStart` hook runs `pensieve context` to
  recover knowledge after compaction.
- **How to disable**: `pensieve configure --inject-enabled false`

### R6: SessionStart hook for context recovery

After context compaction, Claude Code fires `SessionStart` again. Wire a
`SessionStart` hook that runs `pensieve context --output compact` to
automatically recover session context after compaction — solving the "lost
context after compaction" problem without agent action.

## Acceptance Criteria

- `pensieve inject "query"` returns relevant memories in compact format
- `pensieve inject "query"` with no matches returns empty output (empty stdout)
- `pensieve inject` completes in <100ms for keyword-only search
- `pensieve setup` adds `UserPromptSubmit` and `SessionStart` hooks to
  `~/.claude/settings.json`
- Running `pensieve setup` twice doesn't duplicate hooks
- After setup, every Claude Code prompt automatically shows relevant memories
- After context compaction, session context is auto-recovered via SessionStart
  hook
- `pensieve configure --inject-enabled false` suppresses injection output
- Hooks use the full binary path (not just `pensieve`)
- Agents without hook support continue working via the existing skill-based
  approach
- README has an "Auto-inject" section with platform support table and
  explanation of why auto-inject matters

## Out of Scope

- Building hook systems for agents that don't have them (Codex, Cursor)
- Modifying the MCP protocol to support auto-injection
- Real-time memory streaming during responses
- Cross-agent hook standardization
- Windows support
- Prompt-level caching (same prompt submitted twice)

## Testing Strategy

- **Performance test**: Benchmark `pensieve inject` with 100+ memories, verify
  <100ms for keyword-only
- **Integration test**: Simulate `UserPromptSubmit` hook by piping prompt to
  `pensieve inject`, verify output format
- **Threshold test**: Save memories with varying relevance, verify only
  above-threshold results returned
- **Empty result test**: Query with no matches, verify empty stdout
- **Offline test**: Run with no embedding model, verify keyword-only fallback
  without crash or stderr
- **Idempotency test**: Run `pensieve setup` twice, verify no duplicate hooks in
  settings.json
- **Compaction recovery test**: Simulate `SessionStart` hook, verify context
  output
- **Disabled test**: Set `inject.enabled = false`, verify empty output

## Implementation Notes

- The `UserPromptSubmit` hook may receive the prompt via environment variable or
  stdin — need to verify the exact Claude Code hooks API
- Claude Code `settings.json` may already have hooks from other tools — must
  merge into existing hook arrays, not replace
- The `inject` command should share the same recall logic as `recall` but with
  different output formatting and threshold filtering
- Consider: should `inject` also include the current project's recent session
  summary, not just search results?
