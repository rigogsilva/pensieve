---
name: Pensieve Memory Protocol
description: How AI agents should use persistent memory via Pensieve
---

# Memory Protocol

You have access to persistent memory via Pensieve. This memory is shared across
ALL AI agents (Claude Code, Codex, Cursor, Copilot, Gemini CLI). Memory is
stored as markdown files at `~/.pensieve/memory/`.

## Session Start

Call `get_context(project="<project>", source="<your-agent-name>")` to load
prior knowledge. This returns:

- Last 3 session summaries
- All active preferences
- Recent gotchas and decisions (last 30 days)
- Stale memory warnings (>90 days)

## When to SAVE (`save_memory`)

Save after:

- Fixing a bug or discovering a gotcha → `type="gotcha"`
- Making an architecture/design decision → `type="decision"`
- Learning a user preference or correction → `type="preference"`
- Discovering how something works → `type="how-it-works"`
- Finding something noteworthy → `type="discovery"`

Always use `topic_key` — it prevents duplicates and enables evolution tracking.

## When to SEARCH (`recall`)

- Before starting work that might overlap past sessions
- When the user says "remember" or references past work
- When unsure about a convention or past decision
- After context compaction or reset → call `get_context()` to recover

## Before Context Compaction

If you detect your context window is filling up, save any important discoveries
or decisions before compaction occurs. Call `end_session()` with a summary so
the next agent (or your post-compaction self) can recover context.

## Session End

Call
`end_session(summary="...", source="<your-agent-name>", project="<project>")`
before closing.

## Topic Key Conventions

- Use lowercase alphanumeric with hyphens: `api-rate-limit`
- Be descriptive: `docker-build-cache-gotcha` not `fix-1`
- Reuse existing topic keys to update rather than duplicate
