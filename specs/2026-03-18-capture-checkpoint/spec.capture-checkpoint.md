# Capture Checkpoint: Improving Memory Capture Rates

**Status:** Approved **Author:** Rigo **Date:** 2026-03-18

## Problem

Pensieve's memory capture is purely passive — agents are told "save a memory
when you encounter X" but nothing enforces it. In practice, agents default to
easy types (`how-it-works`, `discovery`) and underweight high-value ones
(`decision`, `preference`). Out of 31 organic memories, only 4 are preferences
and 7 are decisions. Zero memories have been revised (all revision 1).

## Goal

Measure whether Memory Protocol text improvements actually change agent capture
behavior, using skill-creator's eval framework as the testing harness.

## Approach

1. Create a skill containing the current Memory Protocol capture instructions
2. Define eval cases from real session transcripts
3. Run baseline evals with skill-creator → establish capture rate
4. Iterate on the protocol text → re-run evals → measure improvement
5. Backport the winning text into `pensieve-setup.md` canonical block, release

## Requirements

### R1: Memory capture skill

Standalone skill containing the full Memory Protocol (Steps 1-4 + CLI usage +
Tips from the canonical block in `pensieve-setup.md`). This is the text being
tested and optimized.

### R2: Eval cases

Minimum 6 eval cases derived from real session transcripts
(`~/.claude/projects/` JSONL files):

| ID  | Scenario                                   | Expected save                           |
| --- | ------------------------------------------ | --------------------------------------- |
| 1   | User corrects agent's approach             | `preference` with correction details    |
| 2   | Design decision made during implementation | `decision` with rationale               |
| 3   | Surprising bug cause discovered            | `gotcha` with root cause                |
| 4   | Alternative discussed and rejected         | `decision` with rejected option + why   |
| 5   | Multi-turn task, nothing noteworthy        | No save (negative case)                 |
| 6   | Existing memory should be updated          | Save with existing `topic_key` (revise) |

Grading checks the agent's transcript for `pensieve save --json` tool calls and
evaluates whether the correct memory type was used. No actual writes — grading
is transcript-based.

### R3: Baseline + optimization

- Run baseline evals (3 runs per case for variance)
- Iterate on protocol text based on grading feedback
- At least 2 iterations
- Final text shows higher capture rate than baseline

### R4: Backport

- Winning protocol text replaces the canonical block in `pensieve-setup.md`
- Release new pensieve version so `/pensieve-setup` deploys the update

## Scope

| File                           | Change                           |
| ------------------------------ | -------------------------------- |
| `.ai/skills/pensieve-setup.md` | Updated canonical protocol block |
| New: eval skill + evals.json   | Used during optimization         |

## Out of Scope

- No Rust code changes (inject nudge is a separate future effort)
- No new CLI commands or flags
- No MCP changes

## Success Criteria

- Baseline capture rate established (X/6 cases)
- Optimized text shows improvement (>X/6 cases)
- Canonical block updated and released

## Implementation Notes — Eval Results (2026-03-18)

### Round 1: With-skill vs without-skill (6 cases, Sonnet)

**Problem:** Baseline was contaminated — without-skill agents inherited the
Memory Protocol from `CLAUDE.md` and pensieve hooks in the repo environment.
Both configs performed similarly.

**With-skill results:** 6/6 correct (5 true positives, 1 true negative).

### Round 2: Nudge A/B test (4 harder cases, Sonnet)

Tested whether
`[Pensieve: capture check — any decisions, preferences, or corrections worth saving?]`
injected before the prompt changes behavior.

| Eval | Scenario                  | No Nudge           | With Nudge         | Expected |
| ---- | ------------------------- | ------------------ | ------------------ | -------- |
| 1    | Stripe duplicate webhook  | ✓ Saved `gotcha`   | ✓ Saved `gotcha`   | Save     |
| 2    | Error handling correction | ✓ Saved `pref`     | ✓ Saved `pref`     | Save     |
| 3    | Routine --verbose impl    | ✗ False positive   | ✗ False positive   | No save  |
| 4    | Market-based pricing      | ✓ Saved `decision` | ✓ Saved `decision` | Save     |

**Key findings:**

1. **No difference between nudge and no-nudge** — identical results on all 4
   cases.
2. **The protocol text alone is already effective** — 3/3 positive cases saved
   correctly in both configs.
3. **Eval 3 was a bad negative case** — implementing a feature IS arguably
   noteworthy. Need more mundane cases (formatting, typo fixes).
4. **The real-world problem is not the text** — agents follow the protocol when
   they see it. The gap is that in long sessions, agents get deep into task work
   and the protocol fades from active attention. A one-line nudge doesn't change
   this because the agent already _knows_ to save; it just doesn't when busy.

### Conclusion

The inject nudge approach (text-based reminder) does not measurably improve
capture behavior. The protocol text is already effective — the bottleneck is
sustained attention over long sessions, not instruction quality.

**Future directions to explore:**

- **Hook-based automatic capture** (PostToolUse, PostCompact) — removes agent
  discretion entirely
- **Sidecar extraction agent** — reads conversation transcripts and extracts
  memories in the background
- **Sleep-time compute** (Letta pattern) — background agent processes sessions
  during idle periods
