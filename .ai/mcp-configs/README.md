# Connecting AI Agents to Pensieve

All configs point to the same `pensieve serve` MCP server, which reads/writes to
`~/.pensieve/memory/`.

## Install

```bash
cargo install --git https://github.com/rigogsilva/pensieve
```

Or download from
[GitHub Releases](https://github.com/rigogsilva/pensieve/releases).

## Claude Code

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

## Codex CLI

Add to `~/.codex/config.toml`:

```toml
[mcp-servers.pensieve]
command = "pensieve"
args = ["serve"]
```

## Cursor

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

## VS Code Copilot

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

## Gemini CLI

Add to `~/.gemini/settings.json`:

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

## Without MCP (any agent)

Memory files are plain markdown at `~/.pensieve/memory/`. Any agent can read
them directly:

```bash
cat ~/.pensieve/memory/global/*.md
grep -r "keyword" ~/.pensieve/memory/
pensieve recall "keyword"
pensieve list --output json
```
