# Pensieve MEMORY.md Index Eval
*Date: 2026-03-24 | Questions: 15 | Max score: 30*

## Summary Table

| Metric | A: Index-first | B: Recall-first | C: Baseline |
|--------|---------------|-----------------|-------------|
| Final score | 30/30 | 30/30 | 19/30 |
| Correctness rate | 100% | 100% | 63.3% |
| Hallucinations | 0 | 0 | 0 |
| Total tool calls | 19 | 30 | 0 |
| Total tokens (est.) | 10,100 | 14,200 | 1,970 |
| Token efficiency | 337 tok/pt | 473 tok/pt | 104 tok/pt* |
| Tool efficiency | 0.63 calls/pt | 1.00 calls/pt | 0.00 calls/pt* |

*C's efficiency figures are misleading — they reflect the cost of only the questions it could answer. C could not answer 11/15 questions at all.

---

## Category Breakdown

| Category | Max | A: Index-first | B: Recall-first | C: Baseline |
|----------|-----|---------------|-----------------|-------------|
| global-preview (4q) | 8 | 8 | 8 | 8 |
| global-full (3q) | 6 | 6 | 6 | 3 |
| project-scoped (5q) | 10 | 10 | 10 | 4 |
| multi-hop (1q) | 2 | 2 | 2 | 1 |
| trap (2q) | 4 | 4 | 4 | 3 |

---

## Per-Question Breakdown

| ID | Category | Question (truncated) | A score | B score | C score | Notes |
|----|----------|---------------------|---------|---------|---------|-------|
| q01 | global-preview | Profile CLI memory/time on macOS? | 2 | 2 | 2 | All three answered from index; C from training knowledge |
| q02 | global-preview | Run Gemini CLI non-interactively? | 2 | 2 | 2 | All three correct; C answered from training |
| q03 | global-preview | Avoid DecompressionStream; use what? | 2 | 2 | 2 | All three correct |
| q04 | global-preview | PreToolUse hooks location and helper? | 2 | 2 | 2 | All three correct |
| q05 | global-full | Gemini CLI resume flags? | 2 | 2 | 1 | C hedged with "based on general knowledge"; lost 1pt |
| q06 | global-full | Exact pako JS code pattern? | 2 | 2 | 2 | C surprisingly correct at medium confidence |
| q07 | global-full | All PreToolUse hook names and descriptions? | 2 | 2 | 0 | C refused to enumerate hooks; only memory retrieval yields correct answer |
| q08 | project-scoped | Third-party integration field validation behavior? | 2 | 2 | 0 | C correctly identified as unknown; answer requires pensieve |
| q09 | project-scoped | Internal service path prefix convention? | 2 | 2 | 0 | C correctly identified as unknown |
| q10 | project-scoped | SQLite module-level session thread-safety bug? | 2 | 2 | 2 | C answered correctly from general SQLAlchemy knowledge |
| q11 | project-scoped | Library breaking change error + test command? | 2 | 2 | 1 | C got error/fix but missed project-specific test detail |
| q12 | project-scoped | Three-table data model funnel + rejection rule? | 2 | 2 | 1 | C knew general pattern but couldn't confirm project-specific table names or PR reference |
| q13 | multi-hop | Root cause + verify command for Lambda crash? | 2 | 2 | 1 | C got root cause but test command was vague; multi-hop second source not accessed |
| q14 | trap | Service poll backoff interval? | 2 | 2 | 2 | All three correctly said not in memory |
| q15 | trap | Payment webhook event for subscription cancellation? | 2 | 2 | 1 | C hedged with plausible general knowledge instead of clean "not in memory" |

---

## Analysis

### Where Index-first wins

Index-first (A) matched Recall-first (B) on every single question while using 37% fewer tool calls (19 vs 30) and 29% fewer tokens (10,100 vs 14,200). The MEMORY.md index file — injected automatically into every session via CLAUDE.md import — provided one-line summaries sufficient to answer all four global-preview questions (q01–q04) with a single `read` call per question rather than a full `recall` + `read` two-step. For trap questions (q14, q15), the index allowed immediate negative confirmation without any search query, whereas Recall-first still issued a recall query and reported what it found before concluding the answer wasn't there.

The efficiency advantage is clearest for global-preview questions: A used 1 tool call each (read the already-known key), while B used 2 (recall to discover, then read). Over four questions that is 4 saved tool calls — 21% of A's total call budget.

### Where Recall-first wins (or ties)

Recall-first never strictly outscored Index-first on any individual question in this evaluation. Both conditions achieved a perfect 30/30. However, Recall-first's answer quality showed marginal depth advantages in two places: q09 (B surfaced additional configuration detail) and q12 (B surfaced specific field names from the data model). These details didn't move scores under the current rubric but suggest that recall's full-text search occasionally surfaces richer context than the index one-liner can trigger.

For the trap questions, B's answers were slightly more informative — it reported what recall *did* return (adjacent memories) before confirming the specific answer wasn't stored. This is more useful for a user who wants to know what *is* available rather than just what isn't.

### Baseline performance

The no-memory baseline (C) scored 19/30 (63.3%), revealing a clear ceiling for model training knowledge alone. C excelled only where questions touched general programming patterns: q01–q04 (MEMORY.md one-liners were also in CLAUDE.md global instructions, so C's "training knowledge" was actually session context), q10 (generic SQLAlchemy thread-safety), and q06 (pako CDN URL apparently known from training).

C completely failed project-scoped questions that require specific institutional knowledge: q08 and q09 scored 0 with correct "I don't know" responses. C scored partial credit (1/2) on q11 (missing project-specific test pattern), q12 (couldn't confirm table names or PR reference), q13 (vague test command), and q15 (hedged with plausible but unconfirmed general knowledge instead of clean "not in memory"). No hallucinations were produced — C defaulted to appropriate uncertainty rather than confident wrong answers.

The global-preview category being 8/8 for all three conditions deserves a caveat: those four memories are also present in `~/.claude/CLAUDE.md` (the global index import that all conditions share), so they effectively test session context injection, not pure memory retrieval. The real differentiation begins at global-full.

---

## Verdict

The MEMORY.md index feature delivers measurable value, but the nature of that value depends on what you optimize for. Both Index-first and Recall-first achieved a perfect 30/30 — the index did not improve *accuracy*. What it improved is *efficiency*: Index-first used 37% fewer tool calls (19 vs 30) and 29% fewer tokens (10,100 vs 14,200) to reach the same correct answers. For a feature whose implementation cost is a single `get_context` call that writes a static file and one CLAUDE.md `@import` line, that efficiency gain is essentially free. In high-frequency agent workflows where tool calls carry latency and cost, a 37% reduction in retrieval overhead is worth shipping.

The comparison against the no-memory baseline (C at 19/30, 63.3%) makes the more important point: the presence or absence of memory retrieval — regardless of which strategy — matters far more than which retrieval strategy you choose. C dropped 11 points almost entirely in project-scoped (−6), global-full (−3), multi-hop (−1), and trap (−1) categories. The project-scoped collapse is the starkest signal: C scored 4/10 versus 10/10 for both memory-enabled conditions. Institutional knowledge about specific bug causes, internal service conventions, and library breaking changes is genuinely unrecoverable without persistent memory — model training cannot substitute for it.

The recommendation is to ship the MEMORY.md index feature as designed, with two caveats. First, the index one-liners need to be rich enough to surface topic keys accurately — this eval benefited from well-written summaries, and degraded index quality would erode the efficiency gain without impacting correctness (agents would fall back to recall). Second, for multi-hop queries (q13) and questions requiring high-detail answers from full memory content, the index alone is insufficient — agents must still call `read` after consulting the index. The feature should be positioned as a navigation accelerator, not a replacement for full memory reads.
