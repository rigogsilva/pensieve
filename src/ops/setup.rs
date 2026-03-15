use std::path::PathBuf;

use crate::error::{PensieveError, Result};

struct AgentInfo {
    #[allow(dead_code)]
    name: &'static str,
    display_name: &'static str,
    config_dir: PathBuf,
    skills_dir: PathBuf,
    skill_filename: &'static str,
    skill_content: String,
}

fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| PensieveError::Config("cannot determine home directory".into()))
}

fn claude_skill_content() -> String {
    r#"---
name: pensieve-setup
description: Set up Pensieve cross-agent memory for this environment
---

# Pensieve Setup Skill

When the user asks you to "set up pensieve", follow these steps:

## Step 1: Register the MCP server

Run this command to register Pensieve as an MCP server:

```bash
claude mcp add --transport stdio --scope user pensieve -- pensieve serve
```

## Step 2: Add Memory Protocol to CLAUDE.md

Check if CLAUDE.md exists in the project root. If it does, check for `<!-- pensieve:start -->` markers.

If the markers don't exist, append the following block to CLAUDE.md (or create it if it doesn't exist):

```
<!-- pensieve:start -->
## Memory Protocol

This project uses Pensieve for cross-agent memory. At session start, call `pensieve context` to load relevant memories. When you learn something important (gotchas, decisions, preferences, discoveries), save it with `pensieve save`. At session end, call `pensieve end-session` with a summary.

Memory types: gotcha, decision, preference, discovery, how-it-works
<!-- pensieve:end -->
```

If markers already exist, replace the content between them with the block above.

## Step 3: Verify

Run this command to verify the setup:

```bash
pensieve context
```

If it returns a response (even with empty fields), the setup is complete.

Tell the user: "Pensieve is set up! I'll now remember things across sessions."
"#
    .to_string()
}

fn codex_skill_content() -> String {
    r#"---
name: pensieve-setup
description: Set up Pensieve cross-agent memory for this environment
---

# Pensieve Setup Skill

When the user asks you to "set up pensieve", follow these steps:

## Step 1: Add Memory Protocol to AGENTS.md

Check if AGENTS.md exists in the project root. If it does, check for `<!-- pensieve:start -->` markers.

If the markers don't exist, append the following block to AGENTS.md (or create it if it doesn't exist):

```
<!-- pensieve:start -->
## Memory Protocol

This project uses Pensieve for cross-agent memory. At session start, run `pensieve context` to load relevant memories. When you learn something important (gotchas, decisions, preferences, discoveries), save it with `pensieve save`. At session end, run `pensieve end-session` with a summary.

Memory types: gotcha, decision, preference, discovery, how-it-works
<!-- pensieve:end -->
```

If markers already exist, replace the content between them with the block above.

## Step 2: Verify

Run this command to verify the setup:

```bash
pensieve context
```

If it returns a response (even with empty fields), the setup is complete.

Tell the user: "Pensieve is set up! I'll now remember things across sessions."
"#
    .to_string()
}

fn claude_desktop_skill_content() -> String {
    let bin_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "pensieve".to_string());

    format!(
        r#"---
name: pensieve-setup
description: Set up Pensieve cross-agent memory for Claude Desktop
---

# Pensieve Setup Skill

When the user asks you to "set up pensieve", follow these steps:

## Step 1: Add Pensieve to MCP config

Read the Claude Desktop config file at:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `~/.config/claude-desktop/config.json`

Add the following to the `mcpServers` object (merge with existing servers, don't replace):

```json
"pensieve": {{
  "command": "{bin_path}",
  "args": ["serve"]
}}
```

Write the file back.

## Step 2: Tell the user

Tell the user: "Pensieve has been added to Claude Desktop. Please restart Claude Desktop to activate."

After restart, call `get_context` to verify the tools are working.
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
            skill_filename: "pensieve-setup.md",
            skill_content: claude_skill_content(),
        });
    }

    if should_include("codex") {
        agents.push(AgentInfo {
            name: "codex",
            display_name: "Codex CLI",
            config_dir: home.join(".codex"),
            skills_dir: home.join(".codex").join("skills"),
            skill_filename: "pensieve-setup.md",
            skill_content: codex_skill_content(),
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
            skills_dir: home.join(".claude").join("skills"),
            skill_filename: "pensieve-setup-desktop.md",
            skill_content: claude_desktop_skill_content(),
        });
    }

    Ok(agents)
}

pub fn run_setup(agent_filter: Option<&str>) -> Result<()> {
    let agents = detect_agents(agent_filter)?;

    if agents.is_empty() {
        println!("No matching agents found.");
        return Ok(());
    }

    println!("Found agents:");

    let mut any_installed = false;

    for agent in &agents {
        if agent.config_dir.exists() {
            std::fs::create_dir_all(&agent.skills_dir)?;
            let skill_path = agent.skills_dir.join(agent.skill_filename);
            std::fs::write(&skill_path, &agent.skill_content)?;
            println!(
                "  \u{2713} {} \u{2014} added skill to {}/",
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

    println!();

    if any_installed {
        println!("Start a new session and tell your agent: \"set up pensieve\"");
    }

    Ok(())
}
