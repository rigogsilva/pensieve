---
name: Pensieve Memory Protocol
description: How AI agents should use persistent memory via Pensieve
---

## Memory Protocol

This project uses [Pensieve](https://github.com/rigogsilva/pensieve) for
cross-agent persistent memory. Binary: `pensieve`

**These steps are mandatory. Skipping any required step is a protocol
violation.**

### Access

Use whichever is available — both have identical capabilities:

- **MCP tools** (preferred): `save_memory`, `recall`, `get_context`,
  `end_session`, `read_memory`, `delete_memory`, `list_memories`,
  `archive_memory`, `configure`
- **CLI** (always works): `pensieve <subcommand>`

If MCP tools are unavailable, use CLI immediately — do not skip.

### Step 1 — Session start (REQUIRED, do this first)

Before any other work — before reading code, answering questions, or making a
plan — run:

```
get_context(project, source)
```

or

```bash
pensieve context 2>/dev/null || true
```

If `get_context` has not been called yet this session, stop and do it now. If it
fails, say so explicitly and continue.

### Step 2 — Before substantive work (REQUIRED)

Search prior knowledge before beginning repo analysis, planning, or
implementation:

```
recall("query")
```

or

```bash
pensieve recall "query"
```

Also search when the user says "remember" or references past work. If recall
fails, say so explicitly and continue.

### Step 3 — During work (save immediately, do not defer)

Save a memory the moment you encounter any of:

- A bug cause or surprising behavior → `type: gotcha`
- A design or architecture decision → `type: decision`
- A user correction or preference → `type: preference`
- How something works → `type: how-it-works`
- Any detail you'd want in a future session → `type: discovery`

If you thought "this might be useful later" — save it now. Do not batch saves
for the end of a turn.

### Step 4 — Before final response (REQUIRED)

Before sending any response that concludes a task, call `end_session` first:

```
end_session(summary, project, source)
```

or

```bash
pensieve end-session --summary "2-3 sentence summary" --project <project> --source <agent>
```

Trigger when: user says goodbye/done/thanks, task is fully complete, or
conversation reaches a stopping point. If `end_session` fails, say so explicitly
in your response.

### Tips

- `topic_key` reuses update the memory (revision increments) — no duplicates
- `dry_run` on save/delete/archive previews without writing
- `--output json` (CLI) for structured processing
- `project` scopes memories; omit for global knowledge
