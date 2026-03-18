# Capture Checkpoint Protocol

**Status:** Draft
**Author:** Rigo
**Date:** 2026-03-18

## Problem

Pensieve's memory capture is purely passive — agents are told "save a memory
when you encounter X" but nothing enforces it. In practice, agents default to
easy types (`how-it-works`, `discovery`) and underweight high-value ones
(`decision`, `preference`). Out of 31 organic memories, only 4 are preferences
and 7 are decisions. Zero memories have been revised (all revision 1). Agents
routinely close sessions without saving what they learned.

The article
[How Do You Want to Remember?](https://zakelfassi.com/how-do-you-want-to-remember)
diagnosed the same gap: systems that capture *what* happened but miss *why*
decisions were made. Their fix was a cron scout that auto-captures every 29
minutes. Pensieve's approach should stay agent-driven and intentional, but needs
a structured checkpoint to catch what agents miss.

## Goal

Add a lightweight "capture checkpoint" to the Memory Protocol that prompts
agents to self-audit before closing a task. This is a protocol-level change (the
canonical block in `pensieve-setup.md`) — no Rust code changes required.

## Requirements

### R1: Capture checkpoint in protocol

Add a step between current Step 3 (during work) and Step 4 (end session) that
prompts agents to review their work for unsaved memories before responding.

- Must be lightweight — not a full end_session, just a self-audit prompt
- Must specifically call out the under-captured types: decisions, preferences,
  rejected approaches
- Must not slow down simple Q&A exchanges — only triggers on substantive task
  completion

### R2: Revision nudge

Add guidance that encourages agents to update existing memories rather than only
creating new ones. Current state: 0% revision rate across all memories.

### R3: Common save patterns in CLI section

Add copy-paste JSON examples for the most under-saved memory types directly in
the CLI usage section, making it frictionless for agents to save decisions and
preferences.

### R4: Project parameter guidance

Add explicit guidance about always passing `--project` when working in a known
project context. Current state: multiple sessions logged as `unknown` project.

## Non-Goals

- No Rust code changes (protocol-only change)
- No cron/scout/automated capture
- No new memory types
- No changes to retrieval or inject

## Scope

Files affected:

- `.ai/skills/pensieve-setup.md` — the canonical Memory Protocol block

## Success Criteria

- The canonical block includes a capture checkpoint step
- The CLI usage section includes common save patterns for decision and preference
- Revision guidance is present
- Project parameter guidance is present
- After deploying and running `/pensieve-setup`, agents produce measurably more
  decisions/preferences over a 1-week period (target: 2x current rate)

## Open Questions

- Should the checkpoint be a numbered step (Step 3.5) or integrated into Step 3?
- What's the right trigger — "before final response in a task" or "before
  end_session"?
- Should we add a `pensieve audit` CLI command later that checks for gaps?

## Metrics

Baseline (current, 3 days):

- 31 organic memories (10/day)
- 4 preferences (13%)
- 7 decisions (23%)
- 0 revisions (0%)
- Multiple `unknown` project sessions

## Implementation Notes

_To be filled after implementation._
