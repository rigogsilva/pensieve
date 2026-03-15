---
name: Pensieve Setup
description: First-time setup guidance for Pensieve memory system
---

# Pensieve Setup

## Install

```bash
cargo install --git https://github.com/rigogsilva/pensieve
```

Or download from
[GitHub Releases](https://github.com/rigogsilva/pensieve/releases).

## Verify

```bash
pensieve version
```

## Configure (optional)

Defaults work out of the box. Override if needed:

```bash
# Custom memory directory
pensieve configure --memory-dir /path/to/memories

# Tune retrieval weights (keyword vs vector)
pensieve configure --keyword-weight 0.7 --vector-weight 0.3
```

Config stored at `~/.config/pensieve/config.toml`.

## Connect to Your AI Agent

See `.ai/mcp-configs/README.md` for per-agent setup snippets.

Quick start for Claude Code — add to `~/.claude/mcp.json`:

```json
{
  "mcpServers": {
    "pensieve": {
      "command": "pensieve",
      "args": ["serve"]
    }
  }
}
```

## First Memory

```bash
pensieve save \
  --title "Test memory" \
  --content "Pensieve is working!" \
  --topic-key test-memory \
  --type discovery
```

## Verify It Worked

```bash
pensieve list
pensieve recall "test"
cat ~/.pensieve/memory/global/test-memory.md
```
