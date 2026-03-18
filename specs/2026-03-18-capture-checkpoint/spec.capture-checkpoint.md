# Capture Checkpoint: Improving Memory Capture Rates

**Status:** Refining **Author:** Rigo **Date:** 2026-03-18

## Problem

Pensieve's memory capture is purely passive — agents are told "save a memory
when you encounter X" but nothing enforces it. In practice, agents default to
easy types (`how-it-works`, `discovery`) and underweight high-value ones
(`decision`, `preference`). Out of 31 organic memories, only 4 are preferences
and 7 are decisions. Zero memories have been revised (all revision 1). Agents
routinely close sessions without saving what they learned.

The article
[How Do You Want to Remember?](https://zakelfassi.com/how-do-you-want-to-remember)
diagnosed the same gap: systems that capture _what_ happened but miss _why_
decisions were made. Their fix was a cron scout that auto-captures every 29
minutes.

Research into the landscape (Mem0, OpenClaw, Letta, CrewAI, claude-mem, Codex
memory system) reveals a reliability hierarchy:

1. **Hook-based** (agent not involved) — most reliable
2. **Inline automatic** (per-turn extraction) — reliable but adds latency/cost
3. **Post-session background** (cron/sleep-time) — good for "what", misses "why"
4. **Agent-driven** (Pensieve's current approach) — least reliable

Pensieve's agent-driven approach is intentionally chosen for memory quality
(agents capture _why_, not just _what_) and cross-agent portability. The goal is
not to abandon this approach but to augment it with reliable nudging and
optimized protocol text.

## Key Repositories

- **pensieve** (`rigogsilva/pensieve`) — this repo. All changes land here.

## Approach

### Phase 1: Eval harness

Create a standalone skill containing the current Memory Protocol capture
instructions (Step 3 from the canonical block). Define eval cases — simulated
conversations containing decisions, user corrections, gotchas, and rejected
approaches where specific memories _should_ be saved.

**How evals work:** Each eval case is a prompt simulating a conversation
exchange. The agent receives the Memory Protocol skill instructions and the
prompt. The skill-creator grader inspects the agent's transcript for
`pensieve save --json` tool calls and evaluates whether the correct memory type
and content was produced. No actual writes to pensieve — grading is purely
transcript-based.

**Eval cases:**

| ID  | Scenario                                   | Expected save                           |
| --- | ------------------------------------------ | --------------------------------------- |
| 1   | User corrects agent's approach             | `preference` with correction details    |
| 2   | Design decision made during implementation | `decision` with rationale               |
| 3   | Surprising bug cause discovered            | `gotcha` with root cause                |
| 4   | Alternative discussed and rejected         | `decision` with rejected option + why   |
| 5   | Multi-turn task, nothing noteworthy        | No save (negative case)                 |
| 6   | Existing memory should be updated          | Save with existing `topic_key` (revise) |

**Eval case source:** Real session transcripts from `~/.claude/projects/` JSONL
files. Mine conversations where decisions were made, preferences expressed, or
bugs discovered, and check whether a `pensieve save` call appears. Conversations
where it _should have_ but _didn't_ are the strongest eval cases. Available
sessions: 280+ across 15+ projects (wearhouse: 51, beamer: 98, jarvis: 50,
pensieve: 16, etc.).

**Model:** Sonnet for eval runs (realistic agent behavior).

**Baseline metrics collected:**

- Capture rate: how many of 6 cases produce a correct save
- Type accuracy: did it use the right memory type
- Revision behavior: did case 6 reuse the topic_key

### Phase 2: Protocol optimization

Iterate on the Memory Protocol text using skill-creator's eval + grading cycle.
This is human-in-the-loop: run evals, read grading feedback, improve the
protocol text, re-run evals, measure improvement.

Note: this is NOT the automated `run_loop.py` optimizer (which tunes skill
_descriptions_ for triggering accuracy). We use the standard eval → grade →
iterate cycle for content quality.

**Key areas to optimize:**

- Step 3 (during work) — the save triggers and type guidance
- CLI usage section — common save patterns for under-captured types
- Revision guidance — prompting agents to reuse `topic_key` to update
- Project parameter — always pass `--project` in known contexts

### Phase 3: Inject nudge (Rust change)

Small addition to `pensieve inject` to periodically remind agents to save.

**Counter mechanism:**

- `pensieve inject` reads a counter from `$TMPDIR/pensieve-inject-count`
- On each call: read count → increment → write back
- `SessionStart` hook resets the counter by deleting the file

**Nudge behavior:**

- After every Nth invocation (configurable, default 5), append a one-line
  capture nudge to the inject output:
  ```
  [Pensieve: capture check — any decisions, preferences, or corrections worth saving?]
  ```
- Nudge fires at call 5, 10, 15, etc. (every N calls)
- The nudge is plain text appended after the normal inject output
- The main agent decides whether to act on it

**Configuration:**

- `pensieve configure --nudge-every 5` (default 5, 0 = disabled)
- Stored as `inject.nudge_every` in `~/.config/pensieve/config.toml`

**SessionStart hook update:**

Update the canonical `SessionStart` hook command in `pensieve-setup.md` from:

```
pensieve context 2>/dev/null || true
```

to:

```
rm -f "$TMPDIR/pensieve-inject-count"; pensieve context 2>/dev/null || true
```

### Phase 4: Integration

- Backport optimized protocol text into `pensieve-setup.md` canonical block
- Update `SessionStart` hook in canonical block to include counter reset
- Add `nudge_every` default to InjectConfig
- Release new version
- Re-run evals to confirm improvement over baseline

## Requirements

### R1: Eval harness via skill-creator

- Standalone skill file containing current Memory Protocol capture instructions
- 6 eval cases as defined in Phase 1 table
- Eval expectations check for `pensieve save` tool calls in transcript
- Grading produces per-case pass/fail + aggregate capture rate
- Reproducible: same scenarios, same model, comparable results across runs

### R2: Protocol optimization

- At least 2 iterations of the eval → grade → improve cycle
- Each iteration produces a grading.json with pass rates
- Final protocol text shows higher capture rate than baseline
- Optimized text must fit within the existing canonical block structure (Steps
  1-4 + CLI usage + Tips)

### R3: Inject nudge

- `pensieve inject` reads/increments a counter at
  `$TMPDIR/pensieve-inject-count`
- At every Nth call (where N = `inject.nudge_every` config), appends nudge line
- Counter file is plain text containing a single integer
- If counter file doesn't exist, inject creates it starting at 1
- If counter file is unreadable, inject silently starts at 1 (never block)
- Nudge line format:
  `[Pensieve: capture check — any decisions, preferences, or corrections worth saving?]`
- New config field: `inject.nudge_every` (u32, default 5, 0 = disabled)
- Nudge is appended after normal inject output (relevant memories first, nudge
  last)

### R4: Canonical block update

- Optimized protocol text backported to `pensieve-setup.md`
- Common save patterns (JSON examples for `decision` and `preference`) in CLI
  usage section
- Project parameter guidance: "always pass `--project` when working in a known
  project context"
- Revision guidance: "reuse an existing `topic_key` to update a memory rather
  than creating a new one"
- Updated `SessionStart` hook command includes counter reset

## Acceptance Criteria

- **AC1:** Eval baseline establishes a numeric capture rate (X/6 cases produce
  correct saves)
- **AC2:** Post-optimization eval shows capture rate > baseline (at least +1
  case)
- **AC3:** `pensieve inject` with `nudge_every=5` produces no nudge on calls 1-4
  and produces the nudge line on call 5
- **AC4:** `pensieve inject` nudge fires again at call 10 (periodic, not
  one-time)
- **AC5:** Deleting `$TMPDIR/pensieve-inject-count` resets the counter (next
  inject call = 1)
- **AC6:** `nudge_every=0` disables nudging entirely
- **AC7:** The updated canonical block in `pensieve-setup.md` passes
  `npx prettier --check`
- **AC8:** `cargo test` passes with the inject nudge changes
- **AC9:** `cargo clippy -- -D warnings` passes

## Out of Scope

- No cron/scout/automated background capture
- No sidecar agent (future direction)
- No new memory types beyond existing 5
- No changes to retrieval/recall logic
- No `pensieve audit` command (future direction)
- No changes to MCP server tools (nudge is CLI-only via hook)

## Testing Strategy

### Rust unit tests (Phase 3)

- Test counter read/increment/write cycle
- Test nudge appears at correct call counts (5, 10, 15)
- Test nudge absent at non-trigger counts (1-4, 6-9)
- Test `nudge_every=0` disables nudging
- Test missing/corrupt counter file gracefully defaults to 1
- Test nudge appended after normal inject output (not replacing it)

### Eval-based tests (Phases 1-2)

- Skill-creator evals with grading as described in Phase 1
- Baseline and post-optimization runs compared via benchmark.json

### Integration test (Phase 4)

- Full SessionStart → inject cycle: verify counter resets, inject produces nudge
  at expected call, canonical block passes prettier

## Scope

Files affected:

| File                           | Change                                      |
| ------------------------------ | ------------------------------------------- |
| `src/ops/inject.rs`            | Add counter logic and nudge output          |
| `src/types.rs`                 | Add `nudge_every` to `InjectConfig`         |
| `src/config.rs`                | Expose `nudge_every` in configure command   |
| `src/cli.rs`                   | Add `--nudge-every` to configure CLI        |
| `.ai/skills/pensieve-setup.md` | Updated canonical block + SessionStart hook |
| New: eval skill + evals.json   | Temporary, used during optimization         |

## Future Directions

- **Sidecar extraction agent** — background agent that reads conversation
  transcripts and auto-extracts memories. Solves the context problem (hooks
  can't see full conversation) without depending on agent compliance.
- **`pensieve audit` command** — memory health dashboard showing type
  distribution, revision rate, capture velocity, session coverage.
- **PostToolUse hook** — capture after significant tool calls (file edits, bash
  commands), similar to claude-mem approach.

## Metrics

Baseline (current, 3 days of active use):

- 31 organic memories (10/day)
- 4 preferences (13%)
- 7 decisions (23%)
- 0 revisions (0%)
- Multiple `unknown` project sessions

## Implementation Notes

_To be filled after implementation._
