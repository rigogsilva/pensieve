# Pensieve

_"I use the Pensieve. One simply siphons the excess thoughts from one's mind,
pours them into the basin, and examines them at one's leisure."_ — Albus
Dumbledore

An agent-first memory system. One brain, every AI.

Every AI agent starts from zero. Claude Code has memory files locked in
`~/.claude/` — invisible to Codex, Cursor, Copilot, Gemini CLI. Like
Dumbledore's stone basin, Pensieve lets you extract thoughts and examine them
later — one memory store that any agent can read and write, via CLI, MCP, or
plain file access.

## Why Pensieve

**Agent-first design.** Inspired by
[Google's approach to building CLIs for AI agents](https://justin.poehnelt.com/posts/rewrite-your-cli-for-ai-agents/),
Pensieve is designed for agents first, humans second. Every command has
`--output json`, schema introspection (`pensieve schema save`), `--json` input
from files or stdin, `--dry-run` for safe previews, and skill files that teach
agents the Memory Protocol — judgment that can't come from `--help` alone.

**Research-driven retrieval.** Pensieve's hybrid search is informed by
real-world findings from teams building agent tooling:

- **Claude Code dropped RAG for grep.** Boris Cherny, creator of Claude Code,
  [shared](https://www.latent.space/p/claude-code) that early versions used a
  local vector DB, but agentic search (grep/glob/read) "outperformed everything.
  By a lot." The agent itself is the semantic layer — it knows what to grep for.
- **Cursor found the sweet spot is both.** Cursor's
  [A/B tests](https://cursor.com/blog/semsearch) showed semantic search adds
  ~12.5% accuracy on top of grep, especially in large codebases. Neither alone
  is sufficient.
- **OpenClaw's two-tier memory.**
  [OpenClaw](https://docs.openclaw.ai/concepts/memory) uses daily logs + curated
  long-term memory, with hybrid retrieval (vector + BM25) and temporal decay.
  Pensieve adopts the same two-tier pattern (sessions + long-term) and hybrid
  retrieval, but flips the weights: 70% keyword / 30% vector, because for
  personal memory stores (<500 entries), keyword precision matters more than
  semantic breadth.

The result: BM25 keyword search is primary, vector similarity fills the gaps for
fuzzy queries like "how do we deploy" matching a memory titled "production
release process".

**Markdown as source of truth.** No proprietary database. Every memory is a
readable `.md` file you can browse in VS Code, Obsidian, or `cat`. The SQLite
index is a rebuildable sidecar — delete it, run `pensieve reindex`, and you're
back. Cloud sync via iCloud or Google Drive just works.

**Three tiers of access.** MCP for agents that support it, CLI for everything
else, and raw file access as the universal fallback. No agent is locked out.

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
│   ├── hogwarts/
│   │   └── patronus-charm-tips.md
│   └── ministry/
│       └── floo-network-config.md
├── sessions/                  # Session summaries
│   └── 2026-03-15T143022-hogwarts-claude-code.md
├── index.sqlite               # Search index (rebuildable)
└── CONTEXT.md                 # Auto-generated context snapshot
```

## Install

### Download (recommended)

Download the latest binary for your platform from
[GitHub Releases](https://github.com/rigogsilva/pensieve/releases/latest):

```bash
# macOS (Apple Silicon)
curl -L https://github.com/rigogsilva/pensieve/releases/latest/download/pensieve-aarch64-apple-darwin -o pensieve
chmod +x pensieve
sudo mv pensieve /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/rigogsilva/pensieve/releases/latest/download/pensieve-x86_64-apple-darwin -o pensieve
chmod +x pensieve
sudo mv pensieve /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/rigogsilva/pensieve/releases/latest/download/pensieve-x86_64-unknown-linux-gnu -o pensieve
chmod +x pensieve
sudo mv pensieve /usr/local/bin/
```

### From source (requires Rust)

```bash
cargo install --git https://github.com/rigogsilva/pensieve
```

### Update

```bash
pensieve update
```

Downloads the latest release from GitHub, verifies the SHA-256 checksum, and
replaces the binary in place.

## Quick start

```bash
# Store a thought in the Pensieve
pensieve save \
  --title "Patronus charm requires happy memory" \
  --content "The wand movement is a circular motion. Focus on your happiest memory." \
  --type how-it-works \
  --topic-key patronus-charm \
  --project hogwarts

# Recall a thought
pensieve recall "patronus"

# List all memories
pensieve list

# Read a specific memory
pensieve read patronus-charm --project hogwarts

# Start an MCP server (for AI agents)
pensieve serve
```

## Three tiers of access

| Tier      | How                                     | Best for                   |
| --------- | --------------------------------------- | -------------------------- |
| **MCP**   | `pensieve serve` over stdio             | AI agents with MCP support |
| **CLI**   | `pensieve recall "query"`               | Scripts, shell, any agent  |
| **Files** | `grep -r "keyword" ~/.pensieve/memory/` | Fallback, zero deps        |

## Memory file format

Each memory is a markdown file with YAML frontmatter — like a silvery thread of
thought in the stone basin:

```markdown
---
title: Patronus charm requires happy memory
type: how-it-works
topic_key: patronus-charm
project: hogwarts
status: active
revision: 2
tags:
  - charms
  - defense
source: claude-code
created: 2026-02-18T11:23:00Z
updated: 2026-03-14T10:00:00Z
---

The wand movement is a circular motion. Focus on your happiest memory. The
stronger the memory, the more powerful the Patronus.
```

### Memory types

| Type           | When to save                        |
| -------------- | ----------------------------------- |
| `gotcha`       | Bug fix, surprising behavior        |
| `decision`     | Architecture or design choice       |
| `preference`   | User correction or style preference |
| `how-it-works` | How a system or tool works          |
| `discovery`    | General finding (default)           |

### Memory lifecycle

Memories start as **active** and can be transitioned:

```bash
# Archive a memory (no longer relevant)
pensieve archive old-spell

# Supersede a memory (replaced by a newer one)
pensieve archive old-spell --superseded-by improved-spell

# Archived/superseded memories are excluded from get-context
# but still searchable with --status archived
pensieve list --status archived
```

### Topic keys

Topic keys are the filename stem and unique identifier. Rules:

- Lowercase alphanumeric with hyphens only: `patronus-charm`
- No spaces, uppercase, or special characters
- Reuse an existing topic key to **update** the memory (revision increments)

### Projects vs global

- `--project hogwarts` → stored in `projects/hogwarts/patronus-charm.md`
- No project → stored in `global/patronus-charm.md`
- Global memories apply everywhere; project memories are scoped

## CLI reference

### save

Save or update a memory. If the topic key already exists, the revision
increments.

```bash
pensieve save \
  --title "Expelliarmus disarming charm" \
  --content "Point wand at opponent, sharp flick. Works against most spells." \
  --type how-it-works \
  --topic-key expelliarmus \
  --project hogwarts \
  --tags "charms,defense" \
  --source claude-code
```

Options:

- `--dry-run` — preview what would be written without saving
- `--expected-revision 2` — fail if the current revision doesn't match (conflict
  detection for concurrent agents)
- `--json '{"title":"...","content":"..."}'` — pass all params as JSON
- `--json @file.json` — read params from a file
- `--json -` — read params from stdin

### recall

Search memories using hybrid retrieval (BM25 keyword + vector similarity):

```bash
pensieve recall "defensive spells"
pensieve recall "charm" --project hogwarts --type how-it-works --limit 5
pensieve recall --since 2026-03-01 --tags "defense"
```

### read

Read the full content of a specific memory:

```bash
pensieve read patronus-charm --project hogwarts
```

### list

List all memories (no content, just metadata):

```bash
pensieve list
pensieve list --project hogwarts --type gotcha --status active
```

### delete

```bash
pensieve delete old-spell --project hogwarts
pensieve delete old-spell --dry-run  # preview
```

### archive

```bash
pensieve archive outdated-spell
pensieve archive outdated-spell --superseded-by improved-spell
```

### get-context

Session start — call this first to load prior knowledge:

```bash
pensieve get-context --project hogwarts --source claude-code
```

Returns:

- Last 3 session summaries
- All active preferences (global + project)
- Recent gotchas and decisions (last 30 days)
- Stale memory warnings (>90 days old)
- First-run notice if unconfigured

Also generates `CONTEXT.md` in the memory directory for agents that auto-load
files.

### end-session

Session end — save a summary before closing:

```bash
pensieve end-session \
  --summary "Refactored the Room of Requirement navigation module" \
  --key-decisions "Switched from breadth-first to intent-based pathfinding" \
  --source claude-code \
  --project hogwarts
```

### reindex

Rebuild the search index from markdown files. Use after manual edits, moving the
memory directory, or if search seems stale:

```bash
pensieve reindex
```

### schema

Introspect command parameters (useful for building integrations):

```bash
pensieve schema save    # show save command parameters
pensieve schema         # show all commands
```

### configure

View or update persistent configuration:

```bash
pensieve configure                          # show current config
pensieve configure --memory-dir ~/path      # set memory directory
pensieve configure --keyword-weight 0.8     # tune retrieval
pensieve configure --vector-weight 0.2
```

### Global flags

These go **before** the subcommand:

```bash
pensieve --output json save ...    # JSON output
pensieve --memory-dir /tmp save ... # temporary directory override
```

## Configuration

Config lives at `~/.config/pensieve/config.toml`:

```toml
memory_dir = "/Users/you/.pensieve/memory"

[retrieval]
keyword_weight = 0.7
vector_weight = 0.3
```

### Priority

1. CLI flag (`--memory-dir`)
2. Environment variable (`PENSIEVE_MEMORY_DIR`)
3. Config file (`~/.config/pensieve/config.toml`)
4. Default (`~/.pensieve/memory/`)

### Cloud sync

Point the memory directory to a synced folder for cross-machine access — like
having a Pensieve in every office:

```bash
# iCloud
pensieve configure --memory-dir ~/Library/Mobile\ Documents/com~apple~CloudDocs/pensieve

# Google Drive
pensieve configure --memory-dir ~/Google\ Drive/My\ Drive/pensieve
```

Since memories are plain markdown files, cloud sync works naturally. The SQLite
index is local and rebuildable — run `pensieve reindex` on each machine after
sync.

## Hybrid retrieval

Pensieve uses two search strategies combined, like combining Legilimency with a
good library index:

- **BM25 (keyword)** — exact term matching via SQLite FTS5. Fast, precise. Finds
  "Patronus" when you search "Patronus".
- **Vector (semantic)** — embedding similarity via fastembed + sqlite-vec. Finds
  "defensive spells" when the memory says "how to cast a Patronus charm".

Results are blended with configurable weights (default 70% keyword, 30% vector):

```bash
# Tune weights
pensieve configure --keyword-weight 0.5 --vector-weight 0.5
```

The embedding model (~127MB) downloads automatically on first use. If offline,
keyword search still works — vector search degrades gracefully.

## Connect your AI agent

### Claude Code

Add to `~/.claude/mcp.json`:

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

### Codex CLI

Add to your Codex MCP config:

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

### Cursor

Create `.cursor/mcp.json` in your project:

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

### VS Code Copilot

Add to `.vscode/mcp.json`:

```json
{
  "servers": {
    "pensieve": {
      "command": "pensieve",
      "args": ["serve"]
    }
  }
}
```

### Without MCP

Any agent can read memory files directly:

```bash
cat ~/.pensieve/memory/global/*.md
grep -r "keyword" ~/.pensieve/memory/
pensieve recall "keyword"
```

## MCP tools

When running as an MCP server (`pensieve serve`), exposes 9 tools:

| Tool             | Description                               |
| ---------------- | ----------------------------------------- |
| `save_memory`    | Save/upsert a memory with frontmatter     |
| `recall`         | Hybrid keyword + vector search            |
| `read_memory`    | Read full content by topic key            |
| `delete_memory`  | Delete a memory                           |
| `list_memories`  | List with filters (type, project, status) |
| `archive_memory` | Archive or supersede a memory             |
| `configure`      | View or update config                     |
| `get_context`    | Session start — load prior knowledge      |
| `end_session`    | Session end — save summary                |

## Importing existing memories

If you have existing Claude Code memories at `~/.claude/projects/*/memory/`, see
[`.ai/skills/pensieve-import.md`](.ai/skills/pensieve-import.md) for a
step-by-step import guide.

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
