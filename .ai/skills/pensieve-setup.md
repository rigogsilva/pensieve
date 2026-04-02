---
name: pensieve-setup
description:
  Set up Pensieve cross-agent memory for this environment. Use when the user
  asks to "set up pensieve" or "configure pensieve".
---

# Pensieve Setup Skill

## Step 1: Add Memory Protocol to instruction file (optional)

Ask the user: "Want to add Pensieve usage instructions to your global
instruction file? The hooks handle context loading automatically — this just
adds CLI reference for saving/recalling. Most users skip this."

If yes, write to the **global** instruction file (never the project directory):

- **Claude Code**: `~/.claude/CLAUDE.md`
- **Codex CLI**: `~/.codex/AGENTS.md`
- **Other agents**: Global instruction file in home directory

Read the target file. If `<!-- pensieve:start -->` markers are absent, append
the canonical block. If markers exist, compare exactly — replace if any
difference.

The canonical block:

````
<!-- pensieve:start -->
## Pensieve Memory

Binary: `__PENSIEVE_BIN__` | Types: gotcha, decision, preference, how-it-works, discovery

**Required steps every session:**
1. **Session start**: `pensieve context 2>/dev/null || true`
2. **Before work**: `pensieve recall "query" --output json`
3. **During work**: Save immediately on bugs, decisions, corrections, how-things-work — don't defer
4. **Before final response**: `pensieve end-session --summary "..." --project <p> --source <agent>`

**CLI patterns** (use `--json` for input, `--output json` for output):
```bash
pensieve save --json '{"type":"decision","topic_key":"key","title":"Title","project":"proj","content":"..."}'
pensieve read --json '{"topic_key":"<key>"}' --output json
pensieve recall "query" --output json
````

Tips: `topic_key` reuses update (no duplicates). `project` = repo/org name, omit
for global. `pensieve schema <cmd>` for exact fields.

<!-- pensieve:end -->

```

For Claude Code, also ensure this line exists after the block (loads memory index into every session):
```

@~/.pensieve/memory/MEMORY.md

````

## Step 2: Set up hooks

### SessionStart hook (always add)

All agents that support session hooks get `SessionStart` wired. This is NOT opt-in.

### Memory priming hook (opt-in only)

`UserPromptSubmit` / `BeforeAgent` / `beforeSubmitPrompt` hooks are **opt-in only**. Ask the user:

> "Enable memory priming? With it, relevant memories are automatically surfaced before every prompt. Without it, Pensieve is only used when you explicitly ask. Disable anytime with `__PENSIEVE_BIN__ configure --prime-enabled false`."

If yes: run `__PENSIEVE_BIN__ configure --prime-enabled true`, then add the pre-prompt hook. If no or unanswered: skip it.

### Hook configs by agent

**Claude Code** (`~/.claude/settings.json` — compare each command exactly, don't just check for "pensieve"):
```json
{
  "permissions": { "allow": ["Bash(__PENSIEVE_BIN__*)"] },
  "hooks": {
    "UserPromptSubmit": [{"hooks": [{"type": "command", "command": "__PENSIEVE_BIN__ prime --limit 3"}]}],
    "SessionStart": [{"hooks": [{"type": "command", "command": "__PENSIEVE_BIN__ context 2>/dev/null || true"}]}],
    "PostCompact": [{"hooks": [{"type": "command", "command": "__PENSIEVE_BIN__ end-session --summary \"$(cat | jq -r '.compact_summary')\" --source claude-code 2>/dev/null || true"}]}]
  }
}
````

`UserPromptSubmit` only if opted in. `SessionStart` and `PostCompact` always.

**Cursor** (`~/.cursor/hooks.json` — only if opted in):

```json
{
  "version": 1,
  "hooks": {
    "beforeSubmitPrompt": [{ "command": "__PENSIEVE_BIN__ prime --limit 3" }]
  }
}
```

**Gemini CLI** (`~/.gemini/settings.json`):

```json
{
  "hooks": {
    "BeforeAgent": [
      { "type": "command", "command": "__PENSIEVE_BIN__ prime --limit 3" }
    ],
    "SessionStart": [
      {
        "type": "command",
        "command": "__PENSIEVE_BIN__ context 2>/dev/null || true"
      }
    ]
  }
}
```

`BeforeAgent` only if opted in. `SessionStart` always.

**Codex CLI** (`~/.codex/hooks.json` — global, not workspace):

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "__PENSIEVE_BIN__ context 2>/dev/null || true"
      }
    ]
  }
}
```

## Step 3: Verify

1. **Hooks** — re-read hooks config, confirm every command is present verbatim
2. **CLI** — run `__PENSIEVE_BIN__ context` and confirm it returns a response
3. **Memory Protocol** (if added) — re-read instruction file, confirm content
   between markers matches

Report what was set up, updated, or already correct. Be specific.
