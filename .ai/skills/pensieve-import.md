---
name: Pensieve Import
description: Agent-driven memory import from Claude memory files
---

# Importing Existing Memories

This guide helps you import existing Claude Code memory files into Pensieve.

## Source Files

Claude Code stores memories at `~/.claude/projects/*/memory/`. Each project
directory may contain:

- `MEMORY.md` — index file with links to memory files (may contain inline
  memories too)
- `feedback_*.md` — individual memory files with YAML frontmatter

## How to Import

1. List your Claude memory directories:

```bash
ls ~/.claude/projects/*/memory/
```

2. For each memory file, read the content and save to Pensieve:

```bash
# Example: import a gotcha
pensieve save \
  --title "API rate limit is per-user not per-key" \
  --content "The rate limit applies to the authenticated user, not the API key." \
  --type gotcha \
  --topic-key api-rate-limit \
  --project my-project \
  --source claude-code

# Example: import a preference
pensieve save \
  --title "Always use structured logging" \
  --content "Use JSON structured logs, never print statements." \
  --type preference \
  --topic-key structured-logging \
  --source claude-code
```

## Mapping guide

| Claude memory type      | Pensieve type  | Pensieve scope                   |
| ----------------------- | -------------- | -------------------------------- |
| Bug fix or gotcha       | `gotcha`       | `--project <name>` if specific   |
| Architecture decision   | `decision`     | `--project <name>` if specific   |
| User correction         | `preference`   | Usually global (no `--project`)  |
| How something works     | `how-it-works` | `--project <name>` if specific   |
| General finding         | `discovery`    | Depends on content               |

## Steps

1. Read each source file from `~/.claude/projects/*/memory/`
2. Extract the content (parse existing YAML frontmatter if present)
3. Call `pensieve save` with the appropriate type, topic_key, and project
4. Verify with `pensieve list` and `pensieve recall`

## Skip These

- `MEMORY.md` files that only contain links to other memory files (no actual
  content) — the content is in the referenced files
