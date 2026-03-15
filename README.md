# Pensieve

A shared memory system for AI agents. One brain, every AI.

Every AI agent starts from zero. Claude Code has memory files locked in
`~/.claude/` — invisible to Codex, Cursor, Copilot, Gemini CLI. Pensieve fixes
this: one memory store that any agent can read and write, via CLI, MCP, or plain
file access.

## How it works

Memories are **markdown files** with YAML frontmatter — human-readable,
git-friendly, browsable in any editor. A SQLite sidecar provides hybrid search
(BM25 keyword + vector similarity) for fast recall across hundreds of memories.

```
~/.pensieve/memory/
├── global/                    # Cross-project knowledge
│   ├── coding-style.md
│   └── notebook-cell-format.md
├── projects/
│   ├── jarvis/
│   │   └── horizon-cli-scope.md
│   └── wearhouse/
│       └── ralph-ci-gate.md
└── sessions/                  # Session summaries
    └── 2026-03-15T143022-jarvis-claude-code.md
```

## Install

```bash
cargo install --git https://github.com/rigogsilva/pensieve
```

## Quick start

```bash
# Save a memory
pensieve save \
  --title "Horizon CLI scope flag" \
  --content "The --scope flag must go AFTER the subcommand" \
  --type gotcha \
  --topic-key horizon-cli-scope \
  --project jarvis

# Search memories
pensieve recall "scope flag"

# List all memories
pensieve list

# Read a specific memory
pensieve read horizon-cli-scope --project jarvis

# Start an MCP server (for AI agents)
pensieve serve
```

## Three tiers of access

| Tier      | How                                     | Best for                   |
| --------- | --------------------------------------- | -------------------------- |
| **MCP**   | `pensieve serve` over stdio             | AI agents with MCP support |
| **CLI**   | `pensieve recall "query"`               | Scripts, shell, any agent  |
| **Files** | `grep -r "keyword" ~/.pensieve/memory/` | Fallback, zero deps        |

## Connect your AI agent

**Claude Code** — add to `~/.claude/mcp.json`:

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

See [`.ai/mcp-configs/README.md`](.ai/mcp-configs/README.md) for Codex, Cursor,
Copilot, and Gemini CLI configs.

## Memory types

| Type           | When to save                        |
| -------------- | ----------------------------------- |
| `gotcha`       | Bug fix, surprising behavior        |
| `decision`     | Architecture or design choice       |
| `preference`   | User correction or style preference |
| `how-it-works` | How a system or tool works          |
| `discovery`    | General finding (default)           |

## CLI commands

| Command       | Description                          |
| ------------- | ------------------------------------ |
| `save`        | Save or update a memory              |
| `read`        | Read a memory by topic key           |
| `recall`      | Hybrid search (BM25 + vector)        |
| `list`        | List memories with filters           |
| `delete`      | Delete a memory                      |
| `archive`     | Archive or supersede a memory        |
| `configure`   | View or update config                |
| `get-context` | Session start — load prior knowledge |
| `end-session` | Session end — save summary           |
| `reindex`     | Rebuild search index                 |
| `schema`      | Introspect command schemas           |
| `serve`       | Start MCP server (stdio)             |
| `version`     | Print version                        |
| `update`      | Self-update from GitHub releases     |

## MCP tools

When running as an MCP server (`pensieve serve`), exposes 9 tools:

- `save_memory` — save/upsert with frontmatter
- `recall` — hybrid keyword + vector search
- `read_memory` — read by topic key
- `delete_memory` — delete a memory
- `list_memories` — list with filters
- `archive_memory` — archive or supersede
- `configure` — view/update config
- `get_context` — session bootstrapping
- `end_session` — save session summary

## Configuration

Config lives at `~/.config/pensieve/config.toml`. Defaults work out of the box.

```bash
# Custom memory directory
pensieve configure --memory-dir ~/my-memories

# Tune retrieval weights (keyword vs vector)
pensieve configure --keyword-weight 0.7 --vector-weight 0.3
```

Priority: CLI flag > `PENSIEVE_MEMORY_DIR` env var > config file > defaults.

## Tech stack

- Rust (2024 edition)
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) — official MCP SDK
- [fastembed](https://github.com/Anush008/fastembed-rs) — local ONNX embeddings
  (BGE-small-en-v1.5)
- [rusqlite](https://github.com/rusqlite/rusqlite) + FTS5 +
  [sqlite-vec](https://github.com/asg017/sqlite-vec) — hybrid retrieval
- [clap](https://github.com/clap-rs/clap) — CLI framework

## License

MIT
