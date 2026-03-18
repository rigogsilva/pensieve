# Capture Checkpoint: Improving Memory Capture Rates

**Status:** Draft **Author:** Rigo **Date:** 2026-03-18

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

## Goal

Improve memory capture rates through three mechanisms:

1. **Eval-driven protocol optimization** — use skill-creator to measure and
   iteratively improve the Memory Protocol instructions
2. **Inject-based capture nudge** — modify `pensieve inject` to periodically
   remind agents to save, triggered after the Nth call in a session
3. **Protocol text improvements** — backport optimized text + common patterns
   into the pensieve-setup canonical block

## Approach

### Phase 1: Eval harness

Create a standalone skill with the current Memory Protocol capture instructions.
Define eval cases — simulated conversations containing decisions, user
corrections, gotchas, and rejected approaches where specific memories _should_
be saved. Use skill-creator to run baseline evals measuring capture rate, type
distribution, and whether high-value memories are saved.

**Eval cases should cover:**

- A user correcting the agent's approach (should save: `preference`)
- A design decision being made (should save: `decision`)
- A surprising bug cause discovered (should save: `gotcha`)
- A rejected alternative discussed (should save: `decision` with rationale)
- A multi-turn task where nothing noteworthy happens (should save: nothing)
- A session where an existing memory should be updated (should: revise, not
  create new)

**Model:** Sonnet for eval runs (realistic agent behavior).

### Phase 2: Protocol optimization

Use skill-creator's optimize loop to iterate on the protocol text. The
skill-creator tweaks instructions, re-runs evals, and measures improvement. The
optimized output becomes the new protocol text.

Key areas to optimize:

- Step 3 (during work) — the save triggers and type guidance
- CLI usage section — common save patterns for under-captured types
- Revision guidance — prompting agents to update existing memories
- Project parameter — always pass `--project` in known contexts

### Phase 3: Inject nudge (Rust change)

Small addition to `pensieve inject`:

- Track invocation count per session (counter in SQLite or temp file, reset on
  `SessionStart`)
- After the Nth call (configurable, default 5), append a capture nudge to the
  inject output:
  ```
  [Pensieve: capture check — any decisions, preferences, or corrections
  worth saving?]
  ```
- The nudge is just text — the main agent decides whether to act on it
- Configurable: `pensieve configure --nudge-after 5` (0 = disabled)

### Phase 4: Integration

- Backport optimized protocol text into `pensieve-setup.md` canonical block
- Include the inject nudge in the setup skill's hook configuration
- Release new version
- Re-run evals to confirm improvement over baseline

## Requirements

### R1: Eval harness via skill-creator

- Standalone skill containing current Memory Protocol capture instructions
- Minimum 6 eval cases covering all memory types + negative case + revision case
- Baseline metrics: capture rate, type distribution, revision rate
- Reproducible: same scenarios, same model, comparable results

### R2: Protocol optimization

- Skill-creator optimize loop produces measurably better capture instructions
- Target: 2x improvement in decision/preference capture rate over baseline
- Optimized text must remain concise (agents ignore verbose instructions)

### R3: Inject nudge

- `pensieve inject` tracks call count per session
- After configurable Nth call, appends one-line capture reminder
- Counter resets on `SessionStart`
- Nudge is configurable and can be disabled
- No new CLI commands — this is an extension of existing `inject` behavior

### R4: Canonical block update

- Optimized protocol text backported to `pensieve-setup.md`
- Common save patterns (JSON examples for decision, preference) in CLI section
- Project parameter guidance
- Revision guidance

## Non-Goals

- No cron/scout/automated background capture
- No sidecar agent (future direction, noted below)
- No new memory types
- No changes to retrieval logic
- No `pensieve audit` command (future direction)

## Future Directions

- **Sidecar extraction agent** — background agent that reads conversation
  transcripts and auto-extracts memories. Solves the context problem (hooks
  can't see full conversation) without depending on agent compliance.
- **`pensieve audit` command** — memory health dashboard showing type
  distribution, revision rate, capture velocity, session coverage.
- **PostToolUse hook** — capture after significant tool calls (file edits, bash
  commands), similar to claude-mem approach.

## Scope

Files affected:

- `.ai/skills/pensieve-setup.md` — canonical Memory Protocol block
- `src/ops/inject.rs` — add call counter and nudge logic
- New skill file for eval harness (temporary, used during optimization)

## Success Criteria

- Eval harness produces reproducible baseline metrics
- Optimized protocol text shows measurable improvement in evals
- Inject nudge fires correctly after Nth call
- After release and 1-week usage: decision + preference capture rate doubles
  from baseline

## Metrics

Baseline (current, 3 days of active use):

- 31 organic memories (10/day)
- 4 preferences (13%)
- 7 decisions (23%)
- 0 revisions (0%)
- Multiple `unknown` project sessions

## Implementation Notes

_To be filled after implementation._
