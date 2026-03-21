# Add `--since` filter to `pensieve list`

## Overview

`pensieve list` returns all memories but has no time filter. `recall` has
`--since` but requires a query string and returns relevance-ranked results.
Adding `--since` to `list` enables time-based inventory queries without needing
a search query — useful for activity review, extraction dedup, and morning
briefings.

## Key Repositories

- `pensieve` - The only repository impacted

## Requirements

- Add `--since` flag to the `list` CLI command accepting flexible date formats:
  `YYYY-MM-DD`, `yesterday`, `today`, `YYYY-MM-DDTHH:MM:SS`. Try to parse
  multiple formats before erroring with a clear message listing accepted
  formats.
- Filter memories where `updated >= since_date` (same semantics as
  `recall --since`)
- Add `since` parameter to the `list_memories` MCP tool
- `--since` composes with existing filters (`--project`, `--type`, `--status`)
- When `--since` is omitted, behavior is unchanged (returns all memories)
- Update the canonical Memory Protocol block in the pensieve-setup skill
  (`~/.claude/skills/pensieve-setup/SKILL.md`) to show `list --since` in CLI
  usage examples. Running `pensieve setup` will sync it to all agents.

## Acceptance Criteria

- `pensieve list --since 2026-03-20` returns only memories updated on or after
  that date
- `pensieve list --since yesterday` returns memories updated since yesterday
  midnight UTC
- `pensieve list --since today` returns memories updated since today midnight
  UTC
- `pensieve list --since 2026-03-20 --project augmentt` combines both filters
- `pensieve list --since 2026-03-20 --output json` works with JSON output
- MCP `list_memories` tool accepts optional `since` parameter
- Existing `pensieve list` (no `--since`) behavior is unchanged
- Invalid date input produces a clear error listing accepted formats (not a
  panic)
- Pensieve-setup skill's canonical Memory Protocol block updated with
  `list --since` example, then `pensieve setup` run to sync

## Out of Scope

- Changes to `inject` (inject uses recall, not list — recency-aware injection is
  a separate feature)
- Changes to nightly-extraction skill
- Date range queries (`--until`, `--before`)
- Recency weighting in recall scoring
- Activity command or session querying
- Focus/working memory feature

## Testing Strategy

- Manual CLI verification with `--since` dates that include/exclude known
  memories
- Test each date format: `YYYY-MM-DD`, `yesterday`, `today`,
  `YYYY-MM-DDTHH:MM:SS`
- Test invalid input produces helpful error (not panic)
- Verify MCP tool works via `pensieve serve` integration

## Implementation Notes

- `recall` already parses `--since` as `NaiveDate` → `DateTime<Utc>` at midnight
  (see `main.rs` lines 172-178). Extend this with a `parse_since_date` helper
  that tries multiple formats and supports `yesterday`/`today` keywords.
- `MemoryCompact` already has `updated: DateTime<Utc>` — filter after collecting
  from storage.
- Files to modify: `cli.rs` (add flag), `main.rs` (parse and pass),
  `ops/list.rs` (accept and filter), `mcp.rs` (add to `ListMemoriesParams`)
- The shared `parse_since_date` helper should also be used by `recall`'s
  `--since` for consistency.
