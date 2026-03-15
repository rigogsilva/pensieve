use std::path::PathBuf;

use crate::error::{PensieveError, Result};

struct AgentInfo {
    #[allow(dead_code)]
    name: &'static str,
    display_name: &'static str,
    config_dir: PathBuf,
    skills_dir: PathBuf,
}

fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| PensieveError::Config("cannot determine home directory".into()))
}

fn bin_path() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "pensieve".to_string())
}

#[allow(clippy::too_many_lines)]
fn unified_skill_content(bin: &str) -> String {
    format!(
        r#"---
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
claude mcp add --transport stdio --scope user pensieve -- {bin} serve
```

Then add the Memory Protocol (see Step 2).

### If you are Claude Desktop

Read the Claude Desktop config file:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `~/.config/claude-desktop/config.json`

Add the following to the `mcpServers` object (merge with existing servers, don't replace them):

```json
"pensieve": {{
  "command": "{bin}",
  "args": ["serve"]
}}
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

Check for `<!-- pensieve:start -->` markers.

If the markers don't exist, append the following block (or create the file if it doesn't exist):

```
<!-- pensieve:start -->
## Memory Protocol

This project uses [Pensieve](https://github.com/rigogsilva/pensieve) for cross-agent persistent memory.
Binary: `{bin}`

### Access
You can use Pensieve two ways — pick whichever is available:
- **MCP tools** (if registered): `save_memory`, `recall`, `get_context`, `end_session`, `read_memory`, `delete_memory`, `list_memories`, `archive_memory`, `configure`
- **CLI** (always works): `{bin} save`, `{bin} recall`, `{bin} context`, etc.

Both have identical capabilities. MCP tools are preferred when available.

### Session lifecycle
- **Start**: Call `get_context(project, source)` or run `{bin} context --project <project> --source <agent>`
- **During work**: Save discoveries with `save_memory` or `{bin} save`
- **Search**: Recall prior knowledge with `recall` or `{bin} recall "query"`
- **End**: Call `end_session(summary, project, source)` or run `{bin} end-session --summary "..."` with a 2-3 sentence summary of what was accomplished. Call it when: the user says goodbye/done/thanks, a task is fully complete, or the conversation reaches a natural stopping point. Don't wait to be asked.

### When to save
- Bug fix or surprising behavior → `type: gotcha`
- Architecture/design decision → `type: decision`
- User correction or preference → `type: preference`
- How something works → `type: how-it-works`
- General finding → `type: discovery` (default)

### Save example (CLI)
```bash
{bin} save \
  --title "Short title" \
  --content "What you learned" \
  --type gotcha \
  --topic-key lowercase-hyphenated-key \
  --project project-name \
  --source <your-agent-name>
```

### When to search
- Before starting work that might overlap past sessions
- When the user says "remember" or references past work
- After context compaction — call `get_context` or `{bin} context` to recover

### Tips
- `topic_key` reuses update the memory (revision increments) — no duplicates
- `dry_run` on save/delete/archive previews without writing
- `--output json` (CLI) for structured processing
- `project` scopes memories; omit for global knowledge
- Save important context before context compaction — it won't survive summarization
<!-- pensieve:end -->
```

If markers already exist, replace the content between them with the block above.

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
disable it anytime with `{bin} configure --inject-enabled false`."

If the user says **yes**:
1. Run: `{bin} configure --inject-enabled true`
2. Add the pre-prompt hook for your agent (see below)

If the user says **no**: skip the pre-prompt hook, but still add the
SessionStart hook above.

### Claude Code

Read `~/.claude/settings.json`. If it doesn't exist, create it. If it exists,
parse the JSON and merge (don't replace existing hooks). Check for "pensieve"
in existing hook commands to avoid duplicates.

```json
{{
  "permissions": {{
    "allow": [
      "Bash({bin}*)"
    ]
  }},
  "hooks": {{
    "UserPromptSubmit": [
      {{
        "hooks": [
          {{
            "type": "command",
            "command": "{bin} inject --limit 3"
          }}
        ]
      }}
    ],
    "SessionStart": [
      {{
        "hooks": [
          {{
            "type": "command",
            "command": "{bin} context 2>/dev/null || true"
          }}
        ]
      }}
    ]
  }}
}}
```

The `UserPromptSubmit` hook reads the prompt from stdin (JSON) and injects
relevant memories. Only add this hook if the user opted in to auto-inject.
The `SessionStart` hook is always added.

### Cursor

Read `~/.cursor/hooks.json`. If it doesn't exist, create it. Merge with
existing hooks. Check for "pensieve" to avoid duplicates.

```json
{{
  "version": 1,
  "hooks": {{
    "beforeSubmitPrompt": [
      {{
        "command": "{bin} inject --limit 3"
      }}
    ]
  }}
}}
```

Only add `beforeSubmitPrompt` if the user opted in to auto-inject.

### Gemini CLI

Read `~/.gemini/settings.json`. If it doesn't exist, create it. Merge with
existing hooks. Check for "pensieve" to avoid duplicates.

```json
{{
  "hooks": {{
    "BeforeAgent": [
      {{
        "type": "command",
        "command": "{bin} inject --limit 3"
      }}
    ],
    "SessionStart": [
      {{
        "type": "command",
        "command": "{bin} context 2>/dev/null || true"
      }}
    ]
  }}
}}
```

Only add `BeforeAgent` if the user opted in. `SessionStart` is always added.

### Codex CLI

Read `~/.codex/hooks.json` (global, NOT `./.codex/hooks.json` in the workspace).
Only `SessionStart` is available (no pre-prompt hook yet). Always add it:

```json
{{
  "hooks": {{
    "SessionStart": [
      {{
        "type": "command",
        "command": "{bin} context 2>/dev/null || true"
      }}
    ]
  }}
}}
```

## Step 4: Verify

Run this command to verify the setup:

```bash
{bin} context
```

If it returns a response (even with empty fields), the setup is complete.

Tell the user: "Pensieve is set up! Relevant memories will be automatically
injected before every prompt. I'll also save important discoveries as we work."
"#
    )
}

fn detect_agents(filter: Option<&str>) -> Result<Vec<AgentInfo>> {
    let home = home_dir()?;
    let mut agents = Vec::new();

    let should_include = |name: &str| filter.is_none() || filter == Some(name);

    if should_include("claude") {
        agents.push(AgentInfo {
            name: "claude",
            display_name: "Claude Code",
            config_dir: home.join(".claude"),
            skills_dir: home.join(".claude").join("skills"),
        });
    }

    if should_include("codex") {
        agents.push(AgentInfo {
            name: "codex",
            display_name: "Codex CLI",
            config_dir: home.join(".codex"),
            skills_dir: home.join(".codex").join("skills"),
        });
    }

    if should_include("claude-desktop") {
        let config_dir = if cfg!(target_os = "macos") {
            home.join("Library/Application Support/Claude")
        } else {
            home.join(".config/claude-desktop")
        };
        agents.push(AgentInfo {
            name: "claude-desktop",
            display_name: "Claude Desktop",
            config_dir,
            // Claude Desktop shares ~/.claude/skills/ with Claude Code
            skills_dir: home.join(".claude").join("skills"),
        });
    }

    Ok(agents)
}

fn ensure_in_path() -> Result<bool> {
    let bin = bin_path();
    let bin_dir = std::path::Path::new(&bin)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // Check if the binary's directory is already in PATH
    if let Ok(path) = std::env::var("PATH") {
        if path.split(':').any(|p| p == bin_dir) {
            return Ok(false);
        }
    }

    // Also check if "pensieve" is directly findable
    if std::process::Command::new("pensieve").arg("version").output().is_ok() {
        return Ok(false);
    }

    let home = home_dir()?;
    let export_line = format!("export PATH=\"{bin_dir}:$PATH\"");
    let marker = "# pensieve";

    let shell_configs = [home.join(".zshrc"), home.join(".bashrc")];

    for config in &shell_configs {
        if config.exists() {
            let contents = std::fs::read_to_string(config)?;
            if contents.contains(marker) {
                return Ok(false);
            }
            let addition = format!("\n{marker}\n{export_line}\n");
            let mut file = std::fs::OpenOptions::new().append(true).open(config)?;
            std::io::Write::write_all(&mut file, addition.as_bytes())?;
            println!("  \u{2713} PATH \u{2014} added {bin_dir} to {}", config.display());
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn run_setup(agent_filter: Option<&str>) -> Result<()> {
    let agents = detect_agents(agent_filter)?;

    if agents.is_empty() {
        println!("No matching agents found.");
        return Ok(());
    }

    let bin = bin_path();
    let skill_content = unified_skill_content(&bin);

    println!("Setting up Pensieve...\n");

    let path_added = ensure_in_path()?;

    println!("Found agents:");

    let mut any_installed = false;
    let mut skill_installed = false;

    for agent in &agents {
        if agent.config_dir.exists() {
            // Only install the skill once per skills_dir (Claude Code and Desktop share ~/.claude/skills/)
            if !skill_installed
                || !agent.skills_dir.join("pensieve-setup").join("SKILL.md").exists()
            {
                let skill_dir = agent.skills_dir.join("pensieve-setup");
                std::fs::create_dir_all(&skill_dir)?;
                std::fs::write(skill_dir.join("SKILL.md"), &skill_content)?;
                skill_installed = true;
            }
            println!(
                "  \u{2713} {} \u{2014} skill available at {}/pensieve-setup/",
                agent.display_name,
                agent.skills_dir.display()
            );
            any_installed = true;
        } else {
            println!("  \u{2717} {} \u{2014} not detected", agent.display_name);
        }
    }

    if agent_filter.is_none() {
        println!("  \u{2717} Cursor \u{2014} not detected");
    }

    // Clean up old separate desktop skill if it exists
    let home = home_dir()?;
    let old_desktop_skill = home.join(".claude/skills/pensieve-setup-desktop");
    if old_desktop_skill.exists() {
        let _ = std::fs::remove_dir_all(&old_desktop_skill);
    }

    println!();

    if path_added {
        println!("Restart your shell or run: source ~/.zshrc");
    }

    if any_installed {
        println!("Start a new session and tell your agent: \"set up pensieve\"");
    }

    Ok(())
}
