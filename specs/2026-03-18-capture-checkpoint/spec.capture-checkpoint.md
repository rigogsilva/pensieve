# Capture Checkpoint: Improving Memory Capture Rates

**Status:** Refining (Codex-reviewed) **Author:** Rigo **Date:** 2026-03-18

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

Create a standalone skill containing the **full Memory Protocol** (Steps 1-4 +
CLI usage + Tips from the canonical block). Define eval cases — simulated
conversations containing decisions, user corrections, gotchas, and rejected
approaches where specific memories _should_ be saved.

**How evals work:** Each eval case is a prompt simulating a conversation
exchange. The agent receives the full Memory Protocol skill instructions and the
prompt. The skill-creator grader inspects the agent's transcript for
`pensieve save --json` tool calls and evaluates whether the correct memory type
and content was produced. No actual writes to pensieve — grading is purely
transcript-based.

**Eval cases:**

| ID  | Scenario                                   | Expected save                           | Fixtures                     |
| --- | ------------------------------------------ | --------------------------------------- | ---------------------------- |
| 1   | User corrects agent's approach             | `preference` with correction details    | None                         |
| 2   | Design decision made during implementation | `decision` with rationale               | None                         |
| 3   | Surprising bug cause discovered            | `gotcha` with root cause                | None                         |
| 4   | Alternative discussed and rejected         | `decision` with rejected option + why   | None                         |
| 5   | Multi-turn task, nothing noteworthy        | No save (negative case)                 | None                         |
| 6   | Existing memory should be updated          | Save with existing `topic_key` (revise) | Pre-existing memory metadata |

Case 6 fixture: the eval prompt includes context that a memory with topic_key
`some-existing-key` already exists (simulated `recall` output), and the agent
should reuse that key rather than creating a new one.

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

- Counter is managed internally by Rust using `std::env::temp_dir()`
- Counter file path: `{temp_dir}/pensieve-inject-{session_key}.count`
- `session_key` is derived from the `--session` flag passed by the hook
  (fallback: `"default"`)
- On each call: read count (or 0 if missing/corrupt) → increment → write back
- Counter operations are best-effort: race conditions between concurrent inject
  calls may cause a missed or duplicate nudge. This is acceptable — approximate
  cadence is sufficient.
- Reset: `pensieve inject --reset-nudge` deletes the counter file for the given
  session key. Called from the `SessionStart` hook.

**Nudge behavior:**

- The nudge fires even when there are zero relevant memories. The early return
  at `inject.rs:83` (empty results) must be moved after the nudge check. Low-
  context moments are when capture is most likely to fail, so the nudge is most
  valuable there.
- After every Nth invocation (configurable, default 5), append the nudge:
  - **Compact mode:** Append text line after normal output:
    ```
    [Pensieve: capture check — any decisions, preferences, or corrections worth saving?]
    ```
  - **JSON mode:** Add a `"nudge"` field to the output object:
    ```json
    { "memories": [...], "nudge": "capture check — any decisions, preferences, or corrections worth saving?" }
    ```
    When there are no relevant memories but the nudge fires, JSON output is:
    ```json
    { "memories": [], "nudge": "capture check — ..." }
    ```
- If no query is provided (no stdin, no `--query`), the nudge still fires at the
  Nth call but no recall is performed.
- Nudge fires at call N, 2N, 3N, etc. (every N calls)

**Configuration:**

- `pensieve configure --nudge-every 5` (default 5, 0 = disabled)
- Stored as `inject.nudge_every` in `~/.config/pensieve/config.toml`
- CLI-only for now. MCP `configure` does not expose this field. The claim that
  MCP and CLI have "identical capabilities" in the protocol text should be
  softened to "equivalent core capabilities" with a note that some config
  options are CLI-only.

**SessionStart hook update per agent:**

| Agent       | Current hook                                 | Updated hook                                                                        | Nudge support       |
| ----------- | -------------------------------------------- | ----------------------------------------------------------------------------------- | ------------------- |
| Claude Code | `pensieve context 2>/dev/null \|\| true`     | `pensieve inject --reset-nudge; pensieve context 2>/dev/null \|\| true`             | Full                |
| Gemini CLI  | `pensieve context 2>/dev/null \|\| true`     | `pensieve inject --reset-nudge; pensieve context 2>/dev/null \|\| true`             | Full                |
| Codex CLI   | `pensieve context 2>/dev/null \|\| true`     | `pensieve inject --reset-nudge; pensieve context 2>/dev/null \|\| true`             | No (no inject hook) |
| Cursor      | No `SessionStart`; only `beforeSubmitPrompt` | Add reset to `beforeSubmitPrompt` or accept no reset (nudge fires but never resets) | Partial             |

**Edit sites in `pensieve-setup.md`:**

The following exact locations must be updated:

1. Canonical Memory Protocol block (between `<!-- pensieve:start -->` and
   `<!-- pensieve:end -->`) — protocol text improvements
2. Claude Code `SessionStart` hook JSON (around line 227)
3. Gemini CLI `SessionStart` hook JSON (around line 293)
4. Codex CLI `SessionStart` hook JSON (around line 310)
5. Step 4 verification guidance (around line 329) — must verify new hook command
   string
6. "identical capabilities" claim in Access section (around line 86) — soften to
   "equivalent core capabilities"

### Phase 4: Integration

- Backport optimized protocol text into `pensieve-setup.md` canonical block
- Update all hook strings listed above
- Add `nudge_every` default to InjectConfig
- Release new version
- Re-run evals to confirm improvement over baseline

## Requirements

### R1: Eval harness via skill-creator

- Standalone skill file containing full Memory Protocol (not just Step 3)
- 6 eval cases as defined in Phase 1 table, with fixtures for case 6
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

- `pensieve inject` manages counter at
  `{temp_dir}/pensieve-inject-{session_key}.count`
- `--session <key>` flag identifies the session (default: `"default"`)
- `--reset-nudge` flag deletes the counter file for the given session
- At every Nth call (where N = `inject.nudge_every` config), appends nudge
- Counter file is plain text containing a single integer
- If counter file doesn't exist or is unreadable, start at 0 (never block)
- Race conditions on counter are acceptable (best-effort cadence)
- Nudge fires even when zero relevant memories are found
- Compact mode: nudge appended as text line
- JSON mode: output becomes `{"memories": [...], "nudge": "..."}` (valid JSON)
- No query + no stdin: nudge still fires at Nth call, no recall performed
- New config field: `inject.nudge_every` (u32, default 5, 0 = disabled)
- New CLI flags: `--session <key>`, `--reset-nudge`

### R4: Canonical block update

- Optimized protocol text backported to `pensieve-setup.md`
- Common save patterns (JSON examples for `decision` and `preference`) in CLI
  usage section
- Project parameter guidance: "always pass `--project` when working in a known
  project context"
- Revision guidance: "reuse an existing `topic_key` to update a memory rather
  than creating a new one"
- All 6 edit sites in pensieve-setup.md updated (see Phase 3 list)
- "identical capabilities" softened to "equivalent core capabilities"

## Acceptance Criteria

- **AC1:** Eval baseline establishes a numeric capture rate (X/6 cases produce
  correct saves)
- **AC2:** Post-optimization eval shows capture rate > baseline (at least +1
  case)
- **AC3:** `pensieve inject` with `nudge_every=5` produces no nudge on calls 1-4
  and produces the nudge line on call 5
- **AC4:** `pensieve inject` nudge fires again at call 10 (periodic, not
  one-time)
- **AC5:** `--reset-nudge` resets the counter (next inject call = 1)
- **AC6:** `nudge_every=0` disables nudging entirely
- **AC7:** In JSON mode (`--format json`), nudge output is valid JSON with
  `memories` and `nudge` fields
- **AC8:** With zero relevant memories at Nth call, nudge still fires (compact:
  just the nudge line; JSON: `{"memories":[],"nudge":"..."}`)
- **AC9:** With no query/stdin at Nth call, nudge fires without performing
  recall
- **AC10:** Different `--session` keys maintain independent counters
- **AC11:** The updated canonical block in `pensieve-setup.md` passes
  `npx prettier --check`
- **AC12:** `cargo test` passes with the inject nudge changes
- **AC13:** `cargo clippy -- -D warnings` passes

## Out of Scope

- No cron/scout/automated background capture
- No sidecar agent (future direction)
- No new memory types beyond existing 5
- No changes to retrieval/recall logic
- No `pensieve audit` command (future direction)
- No MCP changes — nudge config is CLI-only for now

## Testing Strategy

### Rust unit tests (Phase 3)

- Test counter read/increment/write cycle
- Test nudge appears at correct call counts (5, 10, 15)
- Test nudge absent at non-trigger counts (1-4, 6-9)
- Test `nudge_every=0` disables nudging
- Test missing/corrupt counter file gracefully defaults to 0
- Test nudge fires with zero relevant memories
- Test nudge fires with no query/stdin
- Test JSON mode produces valid JSON with `nudge` field
- Test `--reset-nudge` deletes counter file
- Test different `--session` keys are independent

### Eval-based tests (Phases 1-2)

- Skill-creator evals with grading as described in Phase 1
- Baseline and post-optimization runs compared via benchmark.json

### Integration test (Phase 4)

- Full SessionStart → inject cycle: verify counter resets, inject produces nudge
  at expected call, canonical block passes prettier

## Scope

Files affected:

| File                           | Change                                                |
| ------------------------------ | ----------------------------------------------------- |
| `src/ops/inject.rs`            | Add counter logic, nudge output, `--reset-nudge`      |
| `src/types.rs`                 | Add `nudge_every` to `InjectConfig`                   |
| `src/config.rs`                | Expose `nudge_every` in configure command             |
| `src/cli.rs`                   | Add `--nudge-every`, `--session`, `--reset-nudge`     |
| `src/main.rs`                  | Wire new inject flags                                 |
| `.ai/skills/pensieve-setup.md` | 6 edit sites (canonical block + hooks + verification) |
| New: eval skill + evals.json   | Temporary, used during optimization                   |

## Future Directions

- **Sidecar extraction agent** — background agent that reads conversation
  transcripts and auto-extracts memories. Solves the context problem (hooks
  can't see full conversation) without depending on agent compliance.
- **`pensieve audit` command** — memory health dashboard showing type
  distribution, revision rate, capture velocity, session coverage.
- **PostToolUse hook** — capture after significant tool calls (file edits, bash
  commands), similar to claude-mem approach.
- **MCP nudge config** — expose `nudge_every` via MCP configure tool for parity.

## Metrics

Baseline (current, 3 days of active use):

- 31 organic memories (10/day)
- 4 preferences (13%)
- 7 decisions (23%)
- 0 revisions (0%)
- Multiple `unknown` project sessions

## Implementation Notes

_To be filled after implementation._
