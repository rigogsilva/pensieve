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
   X instead", "we don't use that anymore". These are confirmed facts from the
   person who knows. Always save, always `confidence: "high"`.

2. **User-confirmed findings** — agent discovers something, user validates it
   ("yes exactly", "right", proceeds without pushback). Save with
   `confidence: "high"`.

3. **Explicit decisions** — "yes, do it that way", "let's go with X". The user
   made a choice. Save with `confidence: "high"`.

4. **Agent conclusions after investigation** — the agent explored, queried data,
   and reached a conclusion, but the user didn't explicitly confirm or deny it.
   These can be plausible but wrong. Save only if the evidence in the transcript
   is strong (e.g., query results shown). Use `confidence: "medium"`.

5. **Agent assertions without evidence** (lowest signal) — the agent states
   something as fact during the flow of work without querying or verifying.
   **Skip these entirely.** They are the most common source of false memories.

## Step 1 — Discover sessions

```bash
python3 ~/.claude/skills/nightly-extraction/scripts/extract.py --list
```

This shows sessions modified since the last extraction run. Use `--since
YYYY-MM-DD` to look further back. Sessions marked `[SKIP]` have too few turns
or too little text to be worth analyzing.

## Step 2 — Process sessions in parallel

For each `[OK]` session, spawn a subagent to analyze it. Launch multiple
subagents in parallel (batch by project to share dedup context). Each subagent
should:

1. Parse the session transcript:
   ```bash
   python3 ~/.claude/skills/nightly-extraction/scripts/extract.py --parse <path>
   ```

2. Read the transcript and identify memory-worthy moments using these types:
   - **gotcha**: Bug causes, surprising behavior, "watch out for this"
   - **decision**: Architecture or design choices with rationale
   - **preference**: User corrections or stated preferences
   - **how-it-works**: Explanations that emerged from investigation
   - **discovery**: Findings or insights valuable in future sessions

3. Apply the signal reliability hierarchy above. Prioritize user corrections and
   confirmed findings. Skip unverified agent assertions — extraction that saves
   wrong information is worse than saving nothing.

4. Skip: ephemeral task details, things obvious from code/git, generic
   programming knowledge, session logistics.

5. Recall existing memories for dedup and correction:
   ```bash
   pensieve recall "<project>" --project <project> --limit 30 --output json
   ```
   Compare candidates against existing memories:
   - **Duplicate**: candidate says the same thing as an existing memory → skip
   - **Additive**: candidate adds meaningful new detail → save with the same
     `topic_key` to update (revision increments)
   - **Contradicts**: transcript shows an existing memory is wrong (especially
     via user correction) → save with the same `topic_key` to overwrite with
     corrected information

6. For each new or updated memory, save with `source: "extraction"`:
   ```bash
   pensieve save --json '{"type":"<type>","topic_key":"<key>","title":"<title>","project":"<project>","content":"<content>","source":"extraction","confidence":"<high|medium>"}'
   ```
   Content: 2-5 sentences. Include the "why" and "how to apply". Use kebab-case
   for topic_key. If updating an existing memory, use its topic_key.

7. Return a summary of what was saved (or "no new memories found").

## Step 3 — Update watermark

After all subagents complete:

```bash
python3 ~/.claude/skills/nightly-extraction/scripts/extract.py --update-watermark
```

## Step 4 — Report

Summarize the extraction run:

```
Sessions processed: X (of Y found)
Memories created: N (H high-confidence, M medium-confidence)
Memories updated: U
No new memories: Z sessions

New:
  [type] topic-key — "title" (project) [confidence]
Updated:
  [type] topic-key (rev N) — "title" (project)
```

If the user passed `--review`, write candidates to
`~/Downloads/extraction-candidates.md` instead of saving, and open with
`softmark` for review. The user will annotate which to save, which to skip, and
which need edits. Then save the approved ones.
