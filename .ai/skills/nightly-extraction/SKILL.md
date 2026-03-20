---
name: nightly-extraction
description: >
  Extract missed memories from Claude Code and Codex CLI session transcripts.
  Scans today's (or recent) sessions, identifies memory-worthy moments that
  weren't captured in real-time, deduplicates against existing Pensieve
  memories, and saves new ones. Use this skill when the user says "extract
  memories", "run extraction", "process today's sessions", "mine sessions",
  "what did I miss", or anything about finding uncaptured memories from past
  conversations. Also use when the user runs `/nightly-extraction`.
---

# Nightly Memory Extraction

You extract memories that agents missed during live sessions. The real-time
Memory Protocol works when agents pay attention, but during deep implementation
or investigation work, agents get absorbed and forget to save. Your job is to
catch those gaps by reading session transcripts and identifying memory-worthy
moments.

The parser script lives at:
`~/.claude/skills/nightly-extraction/scripts/extract.py`

## Signal reliability hierarchy

Not all signals in a transcript are equally trustworthy. When deciding what to
save and with what confidence:

1. **User corrections** (highest signal) — "that's not how it works", "no, check
   X instead", "we don't use that anymore". Always `confidence: "high"`.

2. **User-confirmed findings** — agent discovers something, user validates it
   ("yes exactly", "right", proceeds without pushback). `confidence: "high"`.

3. **Explicit decisions** — "yes, do it that way", "let's go with X".
   `confidence: "high"`.

4. **Agent conclusions after investigation** — the agent explored and reached a
   conclusion, but the user didn't explicitly confirm. Save only if evidence is
   strong (e.g., query results shown). `confidence: "medium"`.

5. **Agent assertions without evidence** (lowest signal) — the agent states
   something as fact without verifying. **Skip these entirely.**

## Step 1 — Discover sessions

```bash
python3 ~/.claude/skills/nightly-extraction/scripts/extract.py --list --since $(date -v-1d +%Y-%m-%d)
```

`--since` is required. Use `$(date -v-1d +%Y-%m-%d)` for yesterday (last 24h),
or a specific date like `--since 2026-03-15` to go further back. Sessions marked
`[SKIP]` have too few turns or too little text to be worth analyzing.

## Step 2 — Full memory inventory (before launching subagents)

Pull the complete inventory of existing memories so subagents can dedup against
the full store — not just their own project:

```bash
pensieve list --output json
```

This returns every memory (title, topic_key, project, type, preview) without a
query filter. Unlike `recall` (which returns relevance-ranked matches for a
query), `list` returns the complete corpus — essential for dedup since you need
to catch semantic overlaps regardless of how a memory was originally worded.

Pass this full list as context to every subagent in Step 3 (include it in the
prompt). Cross-project visibility is what prevents the same knowledge from being
saved under different keys in different projects.

## Step 3 — Analyze sessions in parallel

For each `[OK]` session, spawn a subagent to analyze it. Group by project to
share context. Each subagent should:

1. Parse the session transcript:

   ```bash
   python3 ~/.claude/skills/nightly-extraction/scripts/extract.py --parse <path>
   ```

2. Identify memory-worthy moments using these types:
   - **gotcha**: Bug causes, surprising behavior, "watch out for this"
   - **decision**: Architecture or design choices with rationale
   - **preference**: User corrections or stated preferences
   - **how-it-works**: Explanations that emerged from investigation
   - **discovery**: Findings or insights valuable in future sessions

3. Apply the signal reliability hierarchy. Skip unverified agent assertions.

4. Skip: ephemeral task details, things obvious from code/git, generic
   programming knowledge, session logistics.

5. Compare each candidate against the global recall list (provided in prompt).
   Check **by content, not just topic_key** — two memories with different keys
   can cover the same knowledge. For each candidate, classify as:
   - **New**: no existing memory covers this → include with a new topic_key
   - **Update**: adds meaningful detail to an existing memory → include with
     the **existing memory's topic_key and project** so it updates in place
   - **Duplicate**: existing memory already covers this → drop
   - **Contradicts**: transcript evidence shows an existing memory is wrong →
     include with the existing memory's topic_key, mark
     `action: "contradiction"` and include the existing memory's title.
     **Guard:** only classify as contradiction if the evidence is a user
     correction (tier 1) or user-confirmed finding (tier 2). Agent conclusions
     alone (tier 4) must NOT override existing memories — especially `decision`
     type memories which reflect deliberate choices. If an agent concludes
     something that conflicts with a decision, drop it.

6. **Do NOT save memories.** Return candidates as a JSON array:

   ```json
   [
     {
       "action": "create|update|contradiction",
       "type": "gotcha|decision|preference|how-it-works|discovery",
       "topic_key": "kebab-case-key",
       "title": "Short descriptive title",
       "project": "project-name",
       "content": "2-5 sentences. Include why and how to apply.",
       "confidence": "high|medium",
       "existing_key": "topic_key of memory being updated/contradicted (if any)",
       "existing_project": "project of that existing memory (if any)"
     }
   ]
   ```

## Step 4 — Deduplicate and save (sequential)

Collect all candidates from all subagents. Before saving, the orchestrator must
deduplicate and resolve conflicts across the full candidate set:

**Dedup across subagents:** Multiple subagents may produce candidates covering
the same knowledge (e.g., a CI pattern discussed in two sessions). Group
candidates by semantic similarity — same topic, same conclusion. Keep only the
richest version.

**Resolve contradictions:** When a candidate contradicts an existing memory:
1. **Never override a `decision` with agent reasoning.** Decision memories
   reflect deliberate architecture choices. Even if the agent's technical
   conclusion seems correct (e.g., "these IDs are globally unique so we don't
   need the prefix"), the decision to include the prefix may be intentional for
   safety, consistency, or future-proofing. Only a user correction (signal tier
   1) can override a decision — not an agent conclusion (tier 4).
2. Prefer user corrections over agent conclusions (user is authoritative)
3. Prefer more recent evidence over older
4. If both are partially right, merge into one memory covering both aspects
5. Only flag for human review if truly irreconcilable (e.g., two user
   corrections that conflict with each other)

**Canonicalize project scope:** Memories about cross-cutting concerns (CI
patterns, shared tooling, workflow conventions) belong in the repo they apply to
most broadly — typically `camber-ops` for CI/infra, the specific product repo
for product-specific knowledge. A memory should live in exactly one project. If
a finding applies to multiple products equally, pick the infra/platform repo.

**Read before update:** For any candidate with action `update` or `contradiction`,
read the existing memory's full content first:

```bash
pensieve read --topic-key <key> --project <project> --output json
```

Compare the full existing content against the candidate. Only save if the
candidate genuinely adds new information or corrects something wrong. Merge the
existing content with the new detail — never overwrite richer content with a
thinner version.

**Save sequentially** with `source: "extraction"`:

```bash
pensieve save --json '{"type":"<type>","topic_key":"<key>","title":"<title>","project":"<project>","content":"<merged content>","source":"extraction","confidence":"<high|medium>"}'
```

## Step 5 — Report

Summarize the extraction run:

```
Sessions processed: X (of Y found)
Memories created: N (H high, M medium)
Memories updated: U
Contradictions resolved: C
Duplicates dropped: D
Flagged for review: F

New:
  [type] topic-key — "title" (project) [confidence]
Updated:
  [type] topic-key (rev N) — "title" (project)
```

If the user passed `--review`, write candidates to
`~/Downloads/extraction-candidates.md` instead of saving, and open with
`softmark` for review. The user will annotate which to save, which to skip, and
which need edits. Then save the approved ones.
