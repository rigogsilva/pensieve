use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize)]
struct VersionCache {
    latest: String,
    checked_at: String,
}

fn check_latest_version() -> Option<String> {
    let cache_dir = dirs::home_dir()?.join(".config").join("pensieve");
    let cache_path = cache_dir.join("version_cache.json");

    // Check cache first
    if let Ok(contents) = std::fs::read_to_string(&cache_path) {
        if let Ok(cache) = serde_json::from_str::<VersionCache>(&contents) {
            if let Ok(checked) = chrono::DateTime::parse_from_rfc3339(&cache.checked_at) {
                let age = Utc::now() - checked.with_timezone(&Utc);
                if age < Duration::hours(24) {
                    return Some(cache.latest);
                }
            }
        }
    }

    // Fetch from GitHub with 2s timeout using a dedicated thread
    // to avoid conflict with tokio runtime
    let result = std::thread::spawn(|| {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .ok()?;

        let resp = client
            .get("https://api.github.com/repos/rigogsilva/pensieve/releases/latest")
            .header("User-Agent", "pensieve")
            .send()
            .ok()?;

        resp.json::<serde_json::Value>().ok()
    })
    .join()
    .ok()??;

    let json = result;
    let tag = json.get("tag_name")?.as_str()?;
    let latest = tag.strip_prefix('v').unwrap_or(tag).to_string();

    // Cache result
    let _ = std::fs::create_dir_all(&cache_dir);
    let cache = VersionCache { latest: latest.clone(), checked_at: Utc::now().to_rfc3339() };
    let _ = std::fs::write(cache_path, serde_json::to_string(&cache).unwrap_or_default());

    Some(latest)
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

    let mut notice = if crate::config::is_unconfigured() {
        Some(
            "Storage path: ~/.pensieve/memory/ (default, unconfigured). Ask the user if this is OK, or call configure to change it."
                .to_string(),
        )
    } else {
        None
    };

    // Version check (non-blocking, best-effort)
    let current_version = env!("CARGO_PKG_VERSION");
    if let Some(latest) = check_latest_version() {
        if latest != current_version && !latest.is_empty() {
            let version_notice = format!(
                "Pensieve v{current_version} is outdated (latest: v{latest}). Run `pensieve update` to upgrade."
            );
            notice = Some(match notice {
                Some(existing) => format!("{existing}\n{version_notice}"),
                None => version_notice,
            });
        }
    }

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
