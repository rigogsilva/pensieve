use chrono::{DateTime, Utc};

use crate::error::Result;
use crate::storage;
use crate::types::{MemoryCompact, MemoryStatus, MemoryType, PensieveConfig};

pub fn list_memories(
    config: &PensieveConfig,
    project: Option<&str>,
    type_filter: Option<&MemoryType>,
    status_filter: Option<&MemoryStatus>,
    since: Option<&DateTime<Utc>>,
) -> Result<Vec<MemoryCompact>> {
    let memories = storage::list_memory_files(config, project, type_filter, status_filter)?;
    Ok(memories
        .iter()
        .map(MemoryCompact::from)
        .filter(|m| since.is_none_or(|s| m.updated >= *s))
        .collect())
}
