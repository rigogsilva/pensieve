# Nightly Memory Extraction

## Overview

Pensieve captures ~10 organic memories/day despite sessions containing 5+
memory-worthy moments each. The real-time protocol works when agents attend to
it, but during deep implementation or investigation sessions, agents forget to
save. A nightly extraction job processes the day's session transcripts from
Claude Code and Codex CLI, identifies missed memory-worthy moments, and saves
them — closing the capture gap without relying on agent attention.

**Prior art:** Zak El Fassi's scout system runs every 29 minutes and produced
18K chunks across 6,578 sessions — massive coverage but required restructuring
due to quality/organization issues. Our approach targets fewer, higher-quality
memories with deduplication against existing Pensieve memories.

## Key Repositories

- `pensieve` - The memory system itself; extraction becomes a new CLI subcommand
  (`pensieve extract`)

## Requirements

- A new `pensieve extract` CLI subcommand that:
  1. Discovers session JSONL files from Claude Code (`~/.claude/projects/*/`)
     and Codex CLI (`~/.codex/sessions/YYYY/MM/DD/`) modified since last run
  2. Parses each session, extracting user + assistant text content (stripping
     tool results, system prompts, and other noise to reduce token cost)
  3. Sends extracted turns to Sonnet with a prompt that identifies memory-worthy
     moments, using Pensieve's existing memory types (gotcha, decision,
     preference, how-it-works, discovery)
  4. Deduplicates against existing memories by recalling relevant memories for
     the project and including them in the extraction prompt
  5. Saves new memories or updates existing ones (via matching `topic_key`)
  6. Tracks a watermark (last-processed timestamp) so sessions aren't
     re-extracted
- The extraction prompt should focus on high-signal patterns:
  - User corrections ("that's not how it works, check X instead")
  - Surprising findings during debugging/investigation
  - Explicit decisions ("yes, do it that way")
  - How-it-works explanations that emerged from investigation
  - Multi-step conclusions after exploration
- Project detection: infer the project name from the session path (e.g.,
  `-Users-rigo-Documents-Projects-pensieve` → `pensieve`) or from session
  metadata
- Large sessions (>100KB of conversation text) should be chunked before sending
  to the API to stay within context limits
- The subcommand should support `--dry-run` to preview what would be saved
  without writing
- The subcommand should support `--since <datetime>` to override the watermark
  and reprocess specific time ranges

## Acceptance Criteria

- Running `pensieve extract` processes all unprocessed sessions from the last
  24h and saves identified memories
- Running `pensieve extract --dry-run` shows candidate memories without saving
- The watermark file prevents re-processing previously extracted sessions
- Memories saved by extraction are indistinguishable from real-time saves (same
  format, same types, same dedup behavior via `topic_key`)
- Session text is cleaned of tool results and system prompts before sending to
  the API (token efficiency)
- The command can be run via cron/launchd for nightly automation

## Out of Scope

- Real-time extraction (hooks that run during sessions) — that's the existing
  protocol's job
- Review/staging UI — memories are auto-saved directly, quality is tuned via
  prompt engineering
- Processing sessions from agents other than Claude Code and Codex CLI
- Stale memory maintenance (archiving outdated memories) — separate effort

## Testing Strategy

- Unit tests for session discovery (finding JSONL files by date)
- Unit tests for session parsing (extracting clean conversation text from JSONL)
- Unit tests for project name inference from paths
- Integration test: process a sample session file and verify memory candidates
  are reasonable
- Manual validation: run `--dry-run` on real sessions and compare output to the
  dry-run analysis we did in this conversation (Session 2 / Etoro investigation
  should produce ~5 memories)

## Implementation Notes

- Claude Code session format: each line is a JSON object with `type` field
  (`user`, `assistant`, `system`, `progress`, `file-history-snapshot`). Only
  `user` and `assistant` types contain conversation content. Assistant messages
  have `content` as an array of objects with `type: "text"` for text and
  `type: "tool_use"` / `type: "tool_result"` for tool calls.
- Codex CLI session format: JSONL with `type` field (`session_meta`,
  `response_item`). Response items have `payload.role` and `payload.content`.
- Claude Code organizes by project path; Codex organizes by date. Extraction
  must handle both layouts.
- Watermark file location: `~/.pensieve/extraction_watermark` (simple timestamp)
- The Anthropic API key for calling Sonnet is needed — should use the same key
  configuration as other Pensieve features or a dedicated env var
  (`ANTHROPIC_API_KEY`)
- Model: Claude Sonnet (latest) — cost is negligible at daily session volumes
  (~$0.50/day worst case)
