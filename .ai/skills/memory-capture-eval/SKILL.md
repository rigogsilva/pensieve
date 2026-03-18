---
name: memory-capture-eval
description:
  Memory Protocol capture instructions for Pensieve. Guides agents on when and
  how to save memories during work. Use when working on any task where you might
  encounter decisions, user corrections, bugs, or noteworthy discoveries.
---

# Memory Protocol — Capture Instructions

You have access to Pensieve, a persistent memory system. Use the CLI to save
memories during your work.

## When to save (save immediately, do not defer)

Save a memory the moment you encounter any of:

- A bug cause or surprising behavior → `type: gotcha`
- A design or architecture decision → `type: decision`
- A user correction or preference → `type: preference`
- How something works → `type: how-it-works`
- Any detail you'd want in a future session → `type: discovery`

If you thought "this might be useful later" — save it now. Do not batch saves
for the end of a turn.

## How to save

Use `--json` for saves — agents construct JSON naturally and it avoids shell
quoting issues:

```bash
# types: gotcha | decision | preference | how-it-works | discovery
# project: scope to a repo/project, omit for global knowledge
pensieve save --json '{"type":"decision","topic_key":"my-key","title":"My Title","project":"myproject","content":"..."}'
```

For very large content, `@file` is a fallback:

```bash
pensieve save --json @/tmp/payload.json
```

## Key rules

- `topic_key` reuses update the memory (revision increments) — no duplicates. If
  updating an existing finding, reuse the same `topic_key`.
- Always pass `--project` when working in a known project context.
- `dry_run` on save previews without writing.

## Output

Use `--output json` on all commands — prefer structured output:

```bash
pensieve recall "query" --output json
pensieve list --output json
```

Run `pensieve schema save` to get the exact JSON field names and types. Never
guess flag names — run `--help` if unsure.
