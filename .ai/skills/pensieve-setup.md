---
name: pensieve-setup
description: Set up Pensieve cross-agent memory for this environment. Use when the user asks to "set up pensieve" or "configure pensieve".
---

# Pensieve Setup Skill

When the user asks you to "set up pensieve", follow these steps.

## Step 1: Detect your environment

Determine which agent you are and follow the matching section:

### If you are Claude Code

Register the MCP server:

```bash
claude mcp add --transport stdio --scope user pensieve -- /Users/rigo/bin/pensieve serve
```

Then add the Memory Protocol (see Step 2).

### If you are Claude Desktop

Read the Claude Desktop config file:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `~/.config/claude-desktop/config.json`

Add the following to the `mcpServers` object (merge with existing servers, don't replace them):

```json
"pensieve": {
  "command": "/Users/rigo/bin/pensieve",
  "args": ["serve"]
}
```

Write the file back. Tell the user to restart Claude Desktop to activate.

### If you are Codex CLI or another agent

Add the Memory Protocol (see Step 2). You can use Pensieve via CLI commands.

## Step 2: Add Memory Protocol

IMPORTANT: This is a GLOBAL setup, not per-project. Add the Memory Protocol to the user-level instruction file.
DO NOT create or modify any file in the current working directory.

- **Claude Code**: `~/.claude/CLAUDE.md` — NOT the project's CLAUDE.md, NOT `./CLAUDE.md`
- **Codex CLI**: `~/.codex/AGENTS.md` — NOT `./AGENTS.md` in the workspace
- **Other agents**: The global instruction file for that agent (always in the home directory)

**Finding markers is not enough — you must verify the content matches exactly.**

Read the target file. Then:
1. If the file doesn't exist or `<!-- pensieve:start -->` markers are absent → append the full canonical block below (create the file if needed).
2. If markers **do** exist → extract the current content between them and **compare it to the canonical block below**. If there is *any* difference (missing lines, outdated wording, wrong commands, truncated sections) → replace the entire content between the markers with the canonical version. Do NOT skip this step just because the markers are present — presence ≠ correctness.

The canonical block to write (including markers):

```
<!-- pensieve:start -->
## Memory Protocol

This project uses [Pensieve](https://github.com/rigogsilva/pensieve) for cross-agent persistent memory.
Binary: `/Users/rigo/bin/pensieve`

**These steps are mandatory. Skipping any required step is a protocol violation.**

### Access
Use whichever is available — both have identical capabilities:
- **MCP tools** (preferred): `save_memory`, `recall`, `get_context`, `end_session`, `read_memory`, `delete_memory`, `list_memories`, `archive_memory`, `configure`
- **CLI** (always works): `/Users/rigo/bin/pensieve <subcommand>`

If MCP tools are unavailable, use CLI immediately — do not skip.

### Step 1 — Session start (REQUIRED, do this first)

Before any other work — before reading code, answering questions, or making a plan — run:

```
get_context(project, source)
```
or
```bash
/Users/rigo/bin/pensieve context 2>/dev/null || true
```

If `get_context` has not been called yet this session, stop and do it now. If it fails, say so explicitly and continue.

### Step 2 — Before substantive work (REQUIRED)

Search prior knowledge before beginning repo analysis, planning, or implementation:

```
recall("query")
```
or
```bash
/Users/rigo/bin/pensieve recall "query"
```

Also search when the user says "remember" or references past work. If recall fails, say so explicitly and continue.

### Step 3 — During work (save immediately, do not defer)

Save a memory the moment you encounter any of:
- A bug cause or surprising behavior → `type: gotcha`
- A design or architecture decision → `type: decision`
- A user correction or preference → `type: preference`
- How something works → `type: how-it-works`
- Any detail you'd want in a future session → `type: discovery`

If you thought "this might be useful later" — save it now. Do not batch saves for the end of a turn.

### Step 4 — Before final response (REQUIRED)

Before sending any response that concludes a task, call `end_session` first:

```
end_session(summary, project, source)
```
or
```bash
/Users/rigo/bin/pensieve end-session --summary "2-3 sentence summary" --project <project> --source <agent>
```

Trigger when: user says goodbye/done/thanks, task is fully complete, or conversation reaches a stopping point. If `end_session` fails, say so explicitly in your response.

### Tips
- `topic_key` reuses update the memory (revision increments) — no duplicates
- `dry_run` on save/delete/archive previews without writing
- `--output json` (CLI) for structured processing
- `project` scopes memories; omit for global knowledge
<!-- pensieve:end -->
```

## Step 3: Set up hooks

### SessionStart hook (always set up)

All agents that support session hooks should wire `SessionStart` to load context
at session start and after compaction. This is NOT opt-in — always set it up.

### Auto-inject hook (ask the user first)

Ask the user: "Would you like to enable auto-inject? Without it, I only use
Pensieve when you explicitly ask me to recall something — meaning I'll miss
relevant memories I don't know to look for. With auto-inject enabled, relevant
memories are automatically surfaced before every prompt, so I always have
context even when I don't know to search for it. It's opt-in and you can
disable it anytime with `/Users/rigo/bin/pensieve configure --inject-enabled false`."

If the user says **yes**:
1. Run: `/Users/rigo/bin/pensieve configure --inject-enabled true`
2. Add the pre-prompt hook for your agent (see below)

If the user says **no**: skip the pre-prompt hook, but still add the
SessionStart hook above.

### Claude Code

Read `~/.claude/settings.json`. If it doesn't exist, create it.

If it exists, parse the JSON and **compare each pensieve hook command to the canonical versions below**. Finding a hook with "pensieve" in the command is not enough — verify the exact command string matches. If any hook is missing, has the wrong command, or is outdated → update it. Do not skip this comparison just because pensieve hooks appear to exist.

```json
{
  "permissions": {
    "allow": [
      "Bash(/Users/rigo/bin/pensieve*)"
    ]
  },
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/Users/rigo/bin/pensieve inject --limit 3"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/Users/rigo/bin/pensieve context 2>/dev/null || true"
          }
        ]
      }
    ],
    "PostCompact": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/Users/rigo/bin/pensieve end-session --summary \\"$(cat | jq -r '.compact_summary')\\" --source claude-code 2>/dev/null || true"
          }
        ]
      }
    ]
  }
}
```

The `UserPromptSubmit` hook reads the prompt from stdin (JSON) and injects
relevant memories. Only add this hook if the user opted in to auto-inject.
The `SessionStart` and `PostCompact` hooks are always added. `PostCompact`
auto-saves the compaction summary so long sessions are never lost.

### Cursor

Read `~/.cursor/hooks.json`. If it doesn't exist, create it. Merge with
existing hooks. Check for "pensieve" to avoid duplicates.

```json
{
  "version": 1,
  "hooks": {
    "beforeSubmitPrompt": [
      {
        "command": "/Users/rigo/bin/pensieve inject --limit 3"
      }
    ]
  }
}
```

Only add `beforeSubmitPrompt` if the user opted in to auto-inject.

### Gemini CLI

Read `~/.gemini/settings.json`. If it doesn't exist, create it. Merge with
existing hooks. Check for "pensieve" to avoid duplicates.

```json
{
  "hooks": {
    "BeforeAgent": [
      {
        "type": "command",
        "command": "/Users/rigo/bin/pensieve inject --limit 3"
      }
    ],
    "SessionStart": [
      {
        "type": "command",
        "command": "/Users/rigo/bin/pensieve context 2>/dev/null || true"
      }
    ]
  }
}
```

Only add `BeforeAgent` if the user opted in. `SessionStart` is always added.

### Codex CLI

Read `~/.codex/hooks.json` (global, NOT `./.codex/hooks.json` in the workspace).
Only `SessionStart` is available (no pre-prompt hook yet). Always add it:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "/Users/rigo/bin/pensieve context 2>/dev/null || true"
      }
    ]
  }
}
```

## Step 4: Verify

Before declaring success, confirm each of the following — don't just run `pensieve context` and stop:

1. **Memory Protocol block** — re-read the target instruction file and confirm the content between `<!-- pensieve:start -->` and `<!-- pensieve:end -->` exactly matches the canonical block from Step 2. If it doesn't, fix it now.
2. **Hooks** — re-read the hooks config and confirm every canonical hook command is present verbatim. If any are missing or wrong, fix them now.
3. **MCP registration** — run `/Users/rigo/bin/pensieve context` and confirm it returns a response (not an error). If it errors, the MCP server or CLI path is broken.

Only after all three pass, tell the user what was set up (or updated) and what was already correct. Be specific — "Memory Protocol was up to date, hooks were missing PostCompact so I added it" is more useful than a generic success message.
