use crate::error::{PensieveError, Result};
use crate::types::{Memory, MemoryStatus, MemoryType, PensieveConfig, SessionSummary};
use serde::Deserialize;
use std::path::PathBuf;

pub fn resolve_path(config: &PensieveConfig, topic_key: &str, project: Option<&str>) -> PathBuf {
    match project {
        Some(p) => config.memory_dir.join("projects").join(p).join(format!("{topic_key}.md")),
        None => config.memory_dir.join("global").join(format!("{topic_key}.md")),
    }
}

pub fn resolve_session_path(config: &PensieveConfig, filename: &str) -> PathBuf {
    config.memory_dir.join("sessions").join(filename)
}

pub fn ensure_dirs(config: &PensieveConfig) -> Result<()> {
    std::fs::create_dir_all(config.memory_dir.join("global"))?;
    std::fs::create_dir_all(config.memory_dir.join("projects"))?;
    std::fs::create_dir_all(config.memory_dir.join("sessions"))?;
    Ok(())
}

pub fn write_memory(config: &PensieveConfig, memory: &Memory) -> Result<()> {
    let path = resolve_path(config, &memory.topic_key, memory.project.as_deref());

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let frontmatter = serde_yaml::to_string(&memory)?;
    let output = format!("---\n{frontmatter}---\n\n{}\n", memory.content);

    // Atomic write: temp file + rename
    let dir = path.parent().ok_or_else(|| {
        PensieveError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "no parent directory",
        ))
    })?;
    let temp = tempfile::NamedTempFile::new_in(dir)?;
    std::fs::write(temp.path(), &output)?;
    temp.persist(&path).map_err(|e| PensieveError::Io(std::io::Error::other(e.to_string())))?;

    Ok(())
}

pub fn read_memory(
    config: &PensieveConfig,
    topic_key: &str,
    project: Option<&str>,
) -> Result<Memory> {
    let path = resolve_path(config, topic_key, project);
    if !path.exists() {
        return Err(PensieveError::NotFound(format!("memory not found: {topic_key}")));
    }
    parse_memory_file(&path)
}

pub fn delete_memory_file(
    config: &PensieveConfig,
    topic_key: &str,
    project: Option<&str>,
) -> Result<()> {
    let path = resolve_path(config, topic_key, project);
    if !path.exists() {
        return Err(PensieveError::NotFound(format!("memory not found: {topic_key}")));
    }
    std::fs::remove_file(path)?;
    Ok(())
}

pub fn list_memory_files(
    config: &PensieveConfig,
    project: Option<&str>,
    type_filter: Option<&MemoryType>,
    status_filter: Option<&MemoryStatus>,
) -> Result<Vec<Memory>> {
    let mut memories = Vec::new();

    let dirs_to_scan: Vec<PathBuf> = if let Some(p) = project {
        vec![config.memory_dir.join("projects").join(p)]
    } else {
        let mut dirs = vec![config.memory_dir.join("global")];
        let projects_dir = config.memory_dir.join("projects");
        if projects_dir.exists() {
            for entry in std::fs::read_dir(projects_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    dirs.push(entry.path());
                }
            }
        }
        dirs
    };

    for dir in dirs_to_scan {
        if !dir.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                if let Ok(memory) = parse_memory_file(&path) {
                    if let Some(tf) = type_filter {
                        if &memory.memory_type != tf {
                            continue;
                        }
                    }
                    if let Some(sf) = status_filter {
                        if &memory.status != sf {
                            continue;
                        }
                    }
                    memories.push(memory);
                }
            }
        }
    }

    memories.sort_by(|a, b| b.updated.cmp(&a.updated));
    Ok(memories)
}

pub fn list_sessions(config: &PensieveConfig, limit: usize) -> Result<Vec<SessionSummary>> {
    let sessions_dir = config.memory_dir.join("sessions");
    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(sessions_dir)?
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();

    // Sort by filename descending (newest first, since filenames are date-prefixed)
    files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    files.truncate(limit);

    let mut sessions = Vec::new();
    for path in files {
        if let Ok(session) = parse_session_file(&path) {
            sessions.push(session);
        }
    }
    Ok(sessions)
}

fn parse_memory_file(path: &std::path::Path) -> Result<Memory> {
    let contents = std::fs::read_to_string(path)?;
    let (frontmatter, body) = split_frontmatter(&contents)?;
    let mut memory: Memory = serde_yaml::from_str(&frontmatter)?;
    memory.content = body.trim().to_string();
    Ok(memory)
}

#[derive(Deserialize)]
struct SessionFrontmatter {
    source: String,
    #[serde(default)]
    project: Option<String>,
    created: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    key_decisions: Vec<String>,
}

fn parse_session_file(path: &std::path::Path) -> Result<SessionSummary> {
    let contents = std::fs::read_to_string(path)?;
    let (frontmatter, body) = split_frontmatter(&contents)?;

    let fm: SessionFrontmatter = serde_yaml::from_str(&frontmatter)?;
    Ok(SessionSummary {
        summary: body.trim().to_string(),
        key_decisions: fm.key_decisions,
        source: fm.source,
        project: fm.project,
        created: fm.created,
    })
}

fn split_frontmatter(contents: &str) -> Result<(String, String)> {
    let trimmed = contents.trim_start();
    if !trimmed.starts_with("---") {
        return Err(PensieveError::Config("file does not start with YAML frontmatter".to_string()));
    }

    let after_first = &trimmed[3..];
    let end = after_first.find("\n---").ok_or_else(|| {
        PensieveError::Config("no closing frontmatter delimiter found".to_string())
    })?;

    let frontmatter = after_first[..end].trim().to_string();
    let body = after_first[end + 4..].to_string();

    Ok((frontmatter, body))
}
