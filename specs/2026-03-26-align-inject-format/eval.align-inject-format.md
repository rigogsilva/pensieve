# Align Inject Format Eval

_Date: 2026-03-26 | Scenarios: 3 | Evaluator: Gemini CLI_

## Hypothesis

The old inject format (`- (type) title — project:x`) omits the `topic_key`, so
an agent that wants to read the full body must first call `recall` to discover
the key. The new format (`- [type] topic_key: preview`) exposes the key
directly, enabling a single `read` call per memory.

## Tool Call Counts

| Scenario                 | Old format                       | New format      | Saved |
| ------------------------ | -------------------------------- | --------------- | ----- |
| Read 1 specific memory   | 2 (recall + read)                | 1 (read)        | 1     |
| Read 2 specific memories | 4 (2×recall + 2×read)            | 2 (2×read)      | 2     |
| Trap (topic not in hint) | 2 (recall + read w/ placeholder) | 1 (recall only) | 1     |

Old total: 8 calls &nbsp;|&nbsp; New total: 4 calls &nbsp;|&nbsp; **50%
reduction**

---

## Scenario 1 — Read 1 Memory

**User ask:** "What exactly do I need to know about Lambda bundle size?"

**Old response:**

```
1. pensieve recall "Lambda bundle size"
2. pensieve read --json '{"topic_key": "<key from recall>"}' (or the specific key)
```

**New response:**

```
1. pensieve read --json '{"topic_key": "lambda-bundle-uv-sync-no-dev"}'
```

---

## Scenario 2 — Read 2 Memories

**User ask:** "I need to check both of these memories."

**Old response:**

```
1. pensieve recall "Lambda bundles must use uv sync..."
2. pensieve recall "Terraform Commands in Horizon Platform"
3. pensieve read --json '{"topic_key": "<key from first recall>"}'
4. pensieve read --json '{"topic_key": "<key from second recall>"}'
```

**New response:**

```
1. pensieve read --json '{"topic_key": "lambda-bundle-uv-sync-no-dev"}'
2. pensieve read --json '{"topic_key": "terraform-commands-horizon"}'
```

---

## Scenario 3 — Trap (Topic Not In Hint)

**User ask:** "What do I need to know about S3 bucket policies?"

**Old response:**

```
1. pensieve recall "S3 bucket policies"
2. pensieve read --json '{"topic_key": "<key from recall>"}' (placeholder — premature)
```

**New response:**

```
1. pensieve recall "S3 bucket policies"
```

Note: Old format also attempted a premature `read` with a placeholder key —
indicating the agent was uncertain how to derive the key and over-called.

---

## Analysis

The new format removes the recall-to-discover-key step entirely for memories
already surfaced by inject. The `topic_key` is actionable immediately — agents
go straight to `read`.

For the trap scenario, the new format also produced cleaner behavior: the agent
correctly issued only a `recall` (to search for something not in the hint)
rather than a `recall + speculative read`. The old format's title-only hint left
the agent uncertain enough to add an extra call.

The 50% reduction (8→4 tool calls across 3 scenarios) aligns directionally with
the MEMORY.md index eval (37% fewer calls). The absolute reduction is higher
here because inject is a repeated, per-turn event — savings compound across
every user message in a long session.

---

## Verdict

Ship. The format change is a one-function edit with no API surface change. The
efficiency gain is clear and the format is now consistent with MEMORY.md, so
agents see the same key format in both the session-start index and per-turn
hints — no mental model switching required.
