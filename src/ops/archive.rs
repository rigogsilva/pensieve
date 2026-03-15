use chrono::Utc;

use crate::error::Result;
use crate::storage;
use crate::types::{Memory, MemoryStatus, PensieveConfig};
use crate::validation;

pub fn archive_memory(
    config: &PensieveConfig,
    topic_key: &str,
    project: Option<&str>,
    superseded_by: Option<&str>,
    dry_run: bool,
) -> Result<Memory> {
    validation::validate_topic_key(topic_key)?;
    if let Some(p) = project {
        validation::validate_project_name(p)?;
    }

    let mut memory = storage::read_memory(config, topic_key, project)?;

    if superseded_by.is_some() {
        memory.status = MemoryStatus::Superseded;
        memory.superseded_by = superseded_by.map(String::from);
    } else {
        memory.status = MemoryStatus::Archived;
    }
    memory.updated = Utc::now();

    if dry_run {
        return Ok(memory);
    }

    storage::write_memory(config, &memory)?;
    Ok(memory)
}
