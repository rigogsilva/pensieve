# Pensieve

<p align="center">
  <img src="assets/pensieve.png" alt="Pensieve" width="160" />
</p>

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

- **Claude Code dropped RAG for agentic grep.** Boris Cherny, creator of Claude
  Code, [shared](https://www.latent.space/p/claude-code) that early versions
  used RAG with vector embeddings (Voyage), but agentic search using grep, glob,
  and file reads "outperformed everything. By a lot." — for code search, the
  agent itself is the semantic layer. We applied the same principle to memory:
  keyword-first retrieval lets the agent reason about what to search for.
- **Cursor found the sweet spot is both.** Cursor's
  [A/B tests](https://cursor.com/blog/semsearch) showed semantic search adds
  ~12.5% accuracy on top of grep (6.5–23.5% depending on model), especially in
  large codebases with 1,000+ files. Pure keyword or pure vector alone leaves
  gaps.
- **OpenClaw's two-tier memory.**
  [OpenClaw](https://docs.openclaw.ai/concepts/memory) uses daily logs + curated
  long-term memory, with hybrid retrieval (70% vector / 30% BM25) and temporal
  decay. Pensieve adopts the same two-tier pattern (sessions + long-term) and
  hybrid retrieval. On the author's real memory corpus, a keyword-heavy blend
  performed better than vector-heavy settings, which matches the intuition that
  personal memory stores often behave more like named note lookup than open-ended
  semantic search. Weights are tunable via `pensieve configure`.

The current result: BM25 keyword search is primary, vector similarity fills the
gaps for fuzzier queries. The default blend is `70%` keyword / `30%` vector,
chosen from a read-only sweep over the real memory corpus. Configurable to taste.

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

## Getting started

### 1. Install and setup

```bash
curl -fsSL https://raw.githubusercontent.com/rigogsilva/pensieve/main/install.sh | sh
```

This detects your platform, downloads the binary to `~/bin/`, adds it to PATH,
and runs `pensieve setup` — which detects your AI agents and installs the setup
skill.

<details>
<summary>Manual install / from source</summary>

```bash
# macOS (Apple Silicon)
mkdir -p ~/bin && curl -fsSL https://github.com/rigogsilva/pensieve/releases/latest/download/pensieve-aarch64-apple-darwin -o ~/bin/pensieve && chmod +x ~/bin/pensieve

# Linux (x86_64)
mkdir -p ~/bin && curl -fsSL https://github.com/rigogsilva/pensieve/releases/latest/download/pensieve-x86_64-unknown-linux-gnu -o ~/bin/pensieve && chmod +x ~/bin/pensieve

# From source (macOS Intel, Linux aarch64, or any platform with Rust)
cargo install --git https://github.com/rigogsilva/pensieve
```

Then run `pensieve setup` to configure your agents.

</details>

This detects which AI agents you have installed and adds a setup skill to each
one:

```
Found agents:
  ✓ Claude Code — added skill to ~/.claude/skills/
  ✓ Codex CLI — added skill to ~/.codex/skills/
  ✗ Cursor — not detected

Start a new session and tell your agent: "set up pensieve"
```

### 3. Tell your agent to finish setup

Open a new session in your AI agent and say:

> Set up pensieve

The agent will run the setup skill — configuring MCP, adding the Memory Protocol
to its instruction file, and verifying everything works. From that point on,
every session auto-loads your prior knowledge.

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

### inject

Auto-inject relevant memories (designed for hook integration):

```bash
# Via stdin (hook mode — reads prompt from JSON)
echo '{"prompt":"patronus"}' | pensieve inject

# Via flag (manual/testing mode)
pensieve inject --query "patronus"
```

Returns compact output above the relevance threshold. If no relevant memories
are found or inject is disabled, outputs nothing. Never blocks the agent.

### configure

View or update persistent configuration:

```bash
pensieve configure                          # show current config
pensieve configure --memory-dir ~/path      # set memory directory
pensieve configure --keyword-weight 0.7     # tune retrieval
pensieve configure --vector-weight 0.3
pensieve configure --inject-enabled true    # enable auto-inject
pensieve configure --inject-enabled false   # disable auto-inject
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

Results are blended with configurable weights. The current default is `70%`
keyword / `30%` vector because the real-memory sweep favored a keyword-heavy
mix on the author's actual corpus:

```bash
# Tune weights
pensieve configure --keyword-weight 0.5 --vector-weight 0.5
```

The embedding model (~127MB) downloads automatically on first use. If offline,
keyword search still works — vector search degrades gracefully.

To reproduce the benchmark used for tuning:

```bash
cargo test benchmark_recall_quality -- --ignored --nocapture
cargo run --bin update_retrieval_benchmark_readme
```

<!-- retrieval-benchmark:start -->
Latest benchmark snapshot from `cargo test benchmark_recall_quality -- --ignored --nocapture`:

- Semantic stress, `0.7 / 0.3`: Top-1 `0.779`, Top-3 `0.912`, Top-5 `0.956`, MRR `0.842`
- Lexical-heavy, `0.7 / 0.3`: Top-1 `1.000`, Top-3 `1.000`, Top-5 `1.000`, MRR `1.000`
- Lexical-heavy, `0.2 / 0.8`: Top-1 `0.980`, Top-3 `1.000`, Top-5 `1.000`, MRR `0.990`

Use `cargo run --bin update_retrieval_benchmark_readme` to refresh this block.
<!-- retrieval-benchmark:end -->

On the author's real memory corpus, a read-only weight sweep favored
`0.7 keyword / 0.3 vector`, reaching roughly:

- Top-1: `0.984`
- Top-3: `1.000`
- Top-5: `1.000`
- MRR: `0.992`

## How agents use Pensieve

Once set up, this happens automatically every session:

1. **Session start** — agent calls `pensieve context` → gets last 3 sessions,
   preferences, recent gotchas/decisions. Also writes `CONTEXT.md` for agents
   that auto-load files.
2. **During work** — agent saves discoveries: `pensieve save --type gotcha ...`
3. **Search** — agent recalls prior knowledge: `pensieve recall "query"`
4. **Session end** — agent calls `pensieve end-session --summary "..."` → next
   session picks up where this one left off

The agent never starts from zero again. Even after context compaction, a
`pensieve context` call recovers prior knowledge.

## Auto-inject

Agents don't know what they don't know. Without auto-inject, they miss relevant
memories because they never search. Every major agent memory system —
[OpenClaw](https://docs.openclaw.ai/concepts/memory),
[Mem0](https://docs.mem0.ai/),
[CrewAI](https://docs.crewai.com/concepts/memory),
[LangGraph](https://langchain-ai.github.io/langgraph/concepts/memory/) — has
converged on the same pattern: **automatically inject relevant memories before
each prompt**.

When enabled, `pensieve inject` runs before every prompt via your agent's hook
system. It reads the prompt from stdin, searches for relevant memories, and
injects them into context — like the Pensieve basin surfacing the right thought
at the right moment.

**It's opt-in.** Disabled by default. Enable during `pensieve setup` or with:

```bash
pensieve configure --inject-enabled true
```

### Platform support

| Agent       | Auto-inject | Session recovery | Mechanism               |
| ----------- | ----------- | ---------------- | ----------------------- |
| Claude Code | Yes         | Yes              | `UserPromptSubmit` hook |
| Cursor      | Yes         | —                | `beforeSubmitPrompt`    |
| Gemini CLI  | Yes         | Yes              | `BeforeAgent` hook      |
| Codex CLI   | Not yet     | Yes              | `SessionStart` only     |

### How session recovery works

When the `SessionStart` hook fires (on session open or after context
compaction), it runs `pensieve context` which returns:

- **Last 3 sessions** — summaries of recent work scoped to the current project
  and agent
- **All active preferences** — user corrections and style preferences (no time
  limit — preferences don't expire)
- **Recent gotchas and decisions** — updated within the last 30 days
- **Stale memory warnings** — memories not updated in 90+ days that may need
  review

This output is injected as plain text into the agent's context before the first
prompt, so the agent starts the session already aware of prior decisions and
gotchas — not from zero.

After context compaction, `SessionStart` fires again, recovering the same
knowledge automatically.

### How inject works

When the `UserPromptSubmit` (or equivalent) hook fires, it pipes the agent's
JSON payload into `pensieve inject`:

```
{"prompt": "what do I know about the patronus charm?"}
```

Pensieve then:

1. **Extracts the query** from `prompt` in the JSON, or uses `--query` if called
   directly.
2. **Strips stopwords** — common words (`what`, `do`, `I`, `know`, `about`,
   `the`) are removed so the FTS5 index isn't overwhelmed by noise.
3. **Runs hybrid recall** — BM25 keyword search on the remaining terms joined
   with `OR`, plus vector similarity on the full query. Results are blended and
   ranked by score.
4. **Filters by threshold** — only memories above `relevance_threshold` (default
   `0.3`) are included. If nothing clears the bar, nothing is injected.
5. **Caps results** — at most `max_results` memories (default `3`) are returned.
6. **Outputs compact text** injected above the prompt:

```
[Pensieve: 2 relevant memories]
- (gotcha) Patronus requires a specific happy memory — project:hogwarts
- (how-it-works) Wand movement is circular — project:hogwarts
```

If inject is disabled, no relevant memories exist, or any error occurs, the
command exits silently — it never blocks or surfaces errors to the agent.

Tune the defaults:

```bash
pensieve configure --relevance-threshold 0.5   # stricter filtering
pensieve configure --inject-max-results 5      # more results
```

### How to disable

```bash
pensieve configure --inject-enabled false
```

The hooks remain in place but become no-ops — no need to remove them.

### Manual agent setup

If `pensieve setup` doesn't detect your agent, you can configure it manually.
See [`.ai/mcp-configs/README.md`](.ai/mcp-configs/README.md) for MCP config
snippets for Claude Code, Codex, Cursor, Copilot, and Gemini CLI.

For any agent without MCP, memory files are plain markdown:

```bash
cat ~/.pensieve/memory/global/*.md
grep -r "keyword" ~/.pensieve/memory/
pensieve recall "keyword"
```

## MCP tools

When running as an MCP server (`pensieve serve`), exposes 10 tools:

| Tool             | Description                               |
| ---------------- | ----------------------------------------- |
| `save_memory`    | Save/upsert a memory with frontmatter     |
| `recall`         | Hybrid keyword + vector search            |
| `read_memory`    | Read full content by topic key            |
| `delete_memory`  | Delete a memory                           |
| `list_memories`  | List with filters (type, project, status) |
| `archive_memory` | Archive or supersede a memory             |
| `inject`         | Auto-inject relevant memories for hooks   |
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
