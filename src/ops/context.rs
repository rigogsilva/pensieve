use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::ops::list::list_memories;
use crate::storage;
use crate::types::{MemoryCompact, MemoryStatus, PensieveConfig, SessionSummary};

#[derive(Debug, Serialize)]
pub struct ContextResponse {
    pub global_index: String,
    pub project_index: Option<String>,
    pub sessions: Vec<SessionSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionCache {
    latest: String,
    checked_at: String,
}

fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let v = v.strip_prefix('v').unwrap_or(v);
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() == 3 {
        Some((parts[0].parse().ok()?, parts[1].parse().ok()?, parts[2].parse().ok()?))
    } else {
        None
    }
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
    let (global_index, project_index) = write_memory_index(config, project)?;

    let sessions = storage::list_sessions(config, 3)?;

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
        let is_newer = match (parse_version(&latest), parse_version(current_version)) {
            (Some(l), Some(c)) => l > c,
            _ => latest != current_version && !latest.is_empty(),
        };
        if is_newer {
            let version_notice = format!(
                "Pensieve v{current_version} is outdated (latest: v{latest}). Run `pensieve update` to upgrade."
            );
            notice = Some(match notice {
                Some(existing) => format!("{existing}\n{version_notice}"),
                None => version_notice,
            });
        }
    }

    Ok(ContextResponse { global_index, project_index, sessions, notice })
}

fn format_memory_line(memory: &MemoryCompact) -> String {
    let summary = memory
        .preview
        .lines()
        .find(|l| !l.trim().is_empty())
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .unwrap_or(memory.title.as_str());
    format!("- [{}] {}: {}", memory.memory_type, memory.topic_key, summary)
}

fn write_memory_index(
    config: &PensieveConfig,
    project: Option<&str>,
) -> Result<(String, Option<String>)> {
    // Delete legacy CONTEXT.md if it exists
    let context_md_path = config.memory_dir.join("CONTEXT.md");
    if let Err(e) = std::fs::remove_file(&context_md_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            // Ignore removal errors silently
        }
    }

    // Global index: fetch all memories, filter to project==None only
    let all_memories = list_memories(config, None, None, Some(&MemoryStatus::Active), None)?;
    let global_memories: Vec<_> = all_memories.iter().filter(|m| m.project.is_none()).collect();

    let global_content =
        global_memories.iter().map(|m| format_memory_line(m)).collect::<Vec<_>>().join("\n");
    // If MEMORY.md already exists, the @import in CLAUDE.md has already loaded
    // the index into the session context. Skip returning it inline to avoid
    // duplicating ~3K tokens. Still write the updated file for next session.
    let memory_md_path = config.memory_dir.join("MEMORY.md");
    let index_already_loaded = memory_md_path.exists();
    std::fs::write(&memory_md_path, &global_content)?;

    // Project index: scoped fetch returns only that project's memories
    let project_index = if let Some(proj) = project {
        let project_memories =
            list_memories(config, Some(proj), None, Some(&MemoryStatus::Active), None)?;
        let project_content =
            project_memories.iter().map(format_memory_line).collect::<Vec<_>>().join("\n");
        let project_memory_path = config.memory_dir.join("projects").join(proj).join("MEMORY.md");
        if let Some(parent) = project_memory_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&project_memory_path, &project_content)?;
        Some(project_content)
    } else {
        None
    };

    let global_index = if index_already_loaded { String::new() } else { global_content };
    Ok((global_index, project_index))
}
