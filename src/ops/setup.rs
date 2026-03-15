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

This project uses Pensieve for cross-agent memory. At session start, call `pensieve get-context` to load relevant memories. When you learn something important (gotchas, decisions, preferences, discoveries), save it with `pensieve save`. At session end, call `pensieve end-session` with a summary.

Memory types: gotcha, decision, preference, discovery, how-it-works
<!-- pensieve:end -->
```

If markers already exist, replace the content between them with the block above.

## Step 3: Verify

Run this command to verify the setup:

```bash
pensieve get-context
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

This project uses Pensieve for cross-agent memory. At session start, run `pensieve get-context` to load relevant memories. When you learn something important (gotchas, decisions, preferences, discoveries), save it with `pensieve save`. At session end, run `pensieve end-session` with a summary.

Memory types: gotcha, decision, preference, discovery, how-it-works
<!-- pensieve:end -->
```

If markers already exist, replace the content between them with the block above.

## Step 2: Verify

Run this command to verify the setup:

```bash
pensieve get-context
```

If it returns a response (even with empty fields), the setup is complete.

Tell the user: "Pensieve is set up! I'll now remember things across sessions."
"#
    .to_string()
}

fn detect_agents(filter: Option<&str>) -> Result<Vec<AgentInfo>> {
    let home = home_dir()?;
    let mut agents = Vec::new();

    let claude_dir = home.join(".claude");
    let codex_dir = home.join(".codex");

    let should_include = |name: &str| filter.is_none() || filter == Some(name);

    if should_include("claude") {
        agents.push(AgentInfo {
            name: "claude",
            display_name: "Claude Code",
            config_dir: claude_dir,
            skills_dir: home.join(".claude").join("skills"),
            skill_filename: "pensieve-setup.md",
            skill_content: claude_skill_content(),
        });
    }

    if should_include("codex") {
        agents.push(AgentInfo {
            name: "codex",
            display_name: "Codex CLI",
            config_dir: codex_dir,
            skills_dir: home.join(".codex").join("skills"),
            skill_filename: "pensieve-setup.md",
            skill_content: codex_skill_content(),
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
            // Agent detected — install skill
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

    // Always show Cursor as not detected (no support yet)
    if agent_filter.is_none() {
        println!("  \u{2717} Cursor \u{2014} not detected");
    }

    println!();

    if any_installed {
        println!("Start a new session and tell your agent: \"set up pensieve\"");
    }

    Ok(())
}
