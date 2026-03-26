# Align Inject Format with MEMORY.md

## Overview

The `inject` command outputs memory hints after each user prompt via the
UserPromptSubmit hook. Its current compact format is:

```
[Pensieve: N relevant memories]
- (gotcha) Lambda bundles must use uv sync --no-dev — project:camber
```

The MEMORY.md index (used at session start) uses a different format:

```
- [gotcha] lambda-bundle-uv-sync-no-dev: Lambda bundles must use uv sync --no-dev to avoid 262MB limit
```

The mismatch means an agent receiving an inject hint cannot directly fetch the
full memory body — it must run a second `recall` query to discover the
`topic_key`. Aligning the formats lets the agent go straight to
`pensieve read --json '{"topic_key":"..."}' ` from the hint alone, saving one
round-trip.

## Impacted Repositories

- `pensieve` — single-file change: `src/ops/inject.rs`

## Requirements

- `format_compact` outputs one line per memory in the format:
  `- [{type}] {topic_key}: {summary}`
  - `{type}`: `MemoryType` display value (e.g. `gotcha`, `how-it-works`)
  - `{topic_key}`: the memory's topic key
  - `{summary}`: first non-empty line of `MemoryCompact.preview`; fall back to
    `title` if preview is empty
- Drop the `— project:{name}` suffix (project is available via `read`)
- Keep the `[Pensieve: N relevant memories]` header line unchanged
- JSON format output is unchanged

## Acceptance Criteria

- `pensieve inject --query "lambda"` outputs lines matching
  `- [gotcha] <topic_key>: <preview_line>` (no parentheses, no project suffix)
- `topic_key` in the inject output matches the key accepted by
  `pensieve read --json '{"topic_key":"<key>"}'`
- `preview` is used as the summary where non-empty; `title` is used as fallback
- Existing `cargo test` passes

## Out of Scope

- Changing inject threshold, limit, or config
- Changing the JSON output format
- Changing MEMORY.md generation

## Testing Strategy

- Unit: update any existing `format_compact` snapshot/unit tests
- Integration: `cargo test` passes
- Eval: compare old vs new inject output — measure whether an agent can fetch
  the full memory body in 1 tool call (new) vs 2 (old: recall then read)

## Implementation Notes

`format_compact` in `src/ops/inject.rs` (line 29). `MemoryCompact` already
exposes both `topic_key: String` and `preview: String` (first 2 lines of body).
The logic to extract the first non-empty line of preview already exists in
`context.rs:format_memory_line` — replicate it here.
