---
name: Pensieve Import
description: Agent-driven memory import from Claude memory files
---

# Importing Existing Memories

This guide helps you import existing Claude Code memory files into Pensieve.

## Source Files

Claude Code stores memories at `~/.claude/projects/*/memory/`. Here's the
mapping for known memories:

| Source                                    | Destination         | Type         | Topic Key            |
| ----------------------------------------- | ------------------- | ------------ | -------------------- |
| jarvis/MEMORY.md (scope section)          | projects/jarvis/    | gotcha       | horizon-cli-scope    |
| jarvis/MEMORY.md (env section)            | projects/jarvis/    | how-it-works | project-runtime      |
| jarvis/MEMORY.md (path section)           | projects/jarvis/    | how-it-works | horizon-cli-config   |
| wearhouse/MEMORY.md (vision)              | projects/wearhouse/ | decision     | wearhouse-vision     |
| wearhouse/feedback_ralph_ci_gate.md       | projects/wearhouse/ | gotcha       | ralph-ci-gate        |
| seranking/feedback_modernize_pipper_cd.md | global/             | gotcha       | pipper-cd-pattern    |
| beamer/feedback_notebook_cell_format.md   | global/             | gotcha       | notebook-cell-format |
| ghub/feedback_dangerous_mode_hooks.md     | global/             | preference   | dangerous-mode-hooks |

## How to Import

For each file above, read the source, then save to Pensieve:

```bash
# Example: import a Claude memory
pensieve save \
  --title "Horizon CLI scope flag placement" \
  --content "The --scope flag must go AFTER the subcommand, not before." \
  --type gotcha \
  --topic-key horizon-cli-scope \
  --project jarvis \
  --source claude-code
```

## Steps

1. Read each source file from `~/.claude/projects/*/memory/`
2. Extract the content (parse existing YAML frontmatter if present)
3. Call `pensieve save` with the mapped type, topic_key, and project
4. Verify with `pensieve list` and `pensieve recall`

## Skip These

- `MEMORY.md` index files that only contain links (beamer, seranking, ghub)
- These are just pointers — the actual content is in the referenced files
