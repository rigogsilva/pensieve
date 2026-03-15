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

Then add the Memory Protocol to `CLAUDE.md` (see Step 2).

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

Add the Memory Protocol to `AGENTS.md` (see Step 2). You can use Pensieve via CLI commands.

## Step 2: Add Memory Protocol

Find your instruction file (`CLAUDE.md` for Claude Code, `AGENTS.md` for Codex/others).
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
- **End**: Call `end_session(summary, project, source)` or run `{bin} end-session --summary "..."`

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

## Step 3: Verify

Run this command to verify the setup:

```bash
{bin} context
```

If it returns a response (even with empty fields), the setup is complete.

Tell the user: "Pensieve is set up! I'll now remember things across sessions."
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
