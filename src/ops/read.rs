use crate::error::{PensieveError, Result};
use crate::storage;
use crate::types::{Memory, PensieveConfig};

pub fn read_memory(
    config: &PensieveConfig,
    topic_key: &str,
    project: Option<&str>,
) -> Result<Memory> {
    // If project is specified, do a direct lookup (existing behavior).
    if project.is_some() {
        return storage::read_memory(config, topic_key, project);
    }

    // No project specified: search all scopes (global first, then projects).
    // Try global scope first.
    if let Ok(memory) = storage::read_memory(config, topic_key, None) {
        return Ok(memory);
    }

    // Scan all project directories for a matching topic_key.
    let projects_dir = config.memory_dir.join("projects");
    let mut matches: Vec<Memory> = Vec::new();

    if projects_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.filter_map(std::result::Result::ok) {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    if let Some(proj_name) = entry.file_name().to_str() {
                        if let Ok(memory) = storage::read_memory(config, topic_key, Some(proj_name))
                        {
                            matches.push(memory);
                        }
                    }
                }
            }
        }
    }

    match matches.len() {
        0 => Err(PensieveError::NotFound(format!("memory not found: {topic_key}"))),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => {
            let projects: Vec<String> = matches.iter().filter_map(|m| m.project.clone()).collect();
            Err(PensieveError::InvalidInput(format!(
                "multiple matches found for '{topic_key}' in projects: {}. Specify --project to disambiguate.",
                projects.join(", ")
            )))
        }
    }
}
