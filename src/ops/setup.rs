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
    include_str!("../../.ai/skills/pensieve-setup.md").replace("__PENSIEVE_BIN__", bin)
}

fn extraction_skill_content() -> &'static str {
    include_str!("../../.ai/skills/nightly-extraction/SKILL.md")
}

fn extraction_script_content() -> &'static str {
    include_str!("../../.ai/skills/nightly-extraction/scripts/extract.py")
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
    let mut written_skill_dirs: Vec<PathBuf> = Vec::new();

    for agent in &agents {
        if agent.config_dir.exists() {
            // Always write the latest skill content, but deduplicate by skills_dir
            // (Claude Code and Desktop share ~/.claude/skills/)
            if !written_skill_dirs.contains(&agent.skills_dir) {
                // Install pensieve-setup skill
                let skill_dir = agent.skills_dir.join("pensieve-setup");
                std::fs::create_dir_all(&skill_dir)?;
                std::fs::write(skill_dir.join("SKILL.md"), &skill_content)?;

                // Install nightly-extraction skill
                let extraction_dir = agent.skills_dir.join("nightly-extraction");
                let scripts_dir = extraction_dir.join("scripts");
                std::fs::create_dir_all(&scripts_dir)?;
                std::fs::write(extraction_dir.join("SKILL.md"), extraction_skill_content())?;
                let script_path = scripts_dir.join("extract.py");
                std::fs::write(&script_path, extraction_script_content())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
                }

                written_skill_dirs.push(agent.skills_dir.clone());
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
