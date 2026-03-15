use chrono::{Duration, Utc};
use serde::Serialize;

use crate::error::Result;
use crate::storage;
use crate::types::{MemoryCompact, MemoryStatus, PensieveConfig, SessionSummary};

#[derive(Debug, Serialize)]
pub struct ContextResponse {
    pub sessions: Vec<SessionSummary>,
    pub preferences: Vec<MemoryCompact>,
    pub recent_gotchas: Vec<MemoryCompact>,
    pub recent_decisions: Vec<MemoryCompact>,
    pub stale_memories: Vec<MemoryCompact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

pub fn get_context(
    config: &PensieveConfig,
    project: Option<&str>,
    _source: Option<&str>,
) -> Result<ContextResponse> {
    let sessions = storage::list_sessions(config, 3)?;

    let all_memories =
        storage::list_memory_files(config, project, None, Some(&MemoryStatus::Active))?;

    let now = Utc::now();
    let thirty_days_ago = now - Duration::days(30);
    let ninety_days_ago = now - Duration::days(90);

    let mut preferences = Vec::new();
    let mut recent_gotchas = Vec::new();
    let mut recent_decisions = Vec::new();
    let mut stale_memories = Vec::new();

    for memory in &all_memories {
        let compact = MemoryCompact::from(memory);

        match memory.memory_type {
            crate::types::MemoryType::Preference => preferences.push(compact.clone()),
            crate::types::MemoryType::Gotcha => {
                if memory.updated >= thirty_days_ago {
                    recent_gotchas.push(compact.clone());
                }
            }
            crate::types::MemoryType::Decision => {
                if memory.updated >= thirty_days_ago {
                    recent_decisions.push(compact.clone());
                }
            }
            _ => {}
        }

        if memory.updated < ninety_days_ago {
            stale_memories.push(compact);
        }
    }

    let notice = if crate::config::is_unconfigured() {
        Some(
            "Pensieve is using default configuration. Run `pensieve configure` to customize."
                .to_string(),
        )
    } else {
        None
    };

    // Write CONTEXT.md
    let _ = write_context_md(config, &sessions, &preferences, &recent_gotchas, &recent_decisions);

    Ok(ContextResponse {
        sessions,
        preferences,
        recent_gotchas,
        recent_decisions,
        stale_memories,
        notice,
    })
}

fn write_context_md(
    config: &PensieveConfig,
    sessions: &[SessionSummary],
    preferences: &[MemoryCompact],
    gotchas: &[MemoryCompact],
    decisions: &[MemoryCompact],
) -> Result<()> {
    let mut lines = Vec::new();
    lines.push("# Pensieve Context".to_string());
    lines.push(String::new());

    if !preferences.is_empty() {
        lines.push("## Preferences".to_string());
        for p in preferences {
            lines.push(format!("- **{}**: {}", p.title, p.preview));
        }
        lines.push(String::new());
    }

    if !gotchas.is_empty() {
        lines.push("## Recent Gotchas".to_string());
        for g in gotchas {
            lines.push(format!("- **{}**: {}", g.title, g.preview));
        }
        lines.push(String::new());
    }

    if !decisions.is_empty() {
        lines.push("## Recent Decisions".to_string());
        for d in decisions {
            lines.push(format!("- **{}**: {}", d.title, d.preview));
        }
        lines.push(String::new());
    }

    if !sessions.is_empty() {
        lines.push("## Recent Sessions".to_string());
        for s in sessions {
            let project = s.project.as_deref().unwrap_or("global");
            lines.push(format!(
                "- **{} ({})**: {}",
                s.created.format("%Y-%m-%d"),
                project,
                s.summary.lines().next().unwrap_or("")
            ));
        }
        lines.push(String::new());
    }

    // Truncate to 200 lines
    lines.truncate(200);

    let content = lines.join("\n");
    let path = config.memory_dir.join("CONTEXT.md");
    std::fs::write(path, content)?;

    Ok(())
}
