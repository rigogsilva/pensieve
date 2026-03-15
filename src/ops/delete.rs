use crate::error::Result;
use crate::storage;
use crate::types::{Memory, PensieveConfig};
use crate::validation;

pub fn delete_memory(
    config: &PensieveConfig,
    topic_key: &str,
    project: Option<&str>,
    dry_run: bool,
) -> Result<Option<Memory>> {
    validation::validate_topic_key(topic_key)?;
    if let Some(p) = project {
        validation::validate_project_name(p)?;
    }

    let memory = storage::read_memory(config, topic_key, project)?;

    if dry_run {
        return Ok(Some(memory));
    }

    storage::delete_memory_file(config, topic_key, project)?;

    // Index cleanup added in Phase 3
    Ok(Some(memory))
}
