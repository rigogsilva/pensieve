use chrono::Utc;

use crate::error::{PensieveError, Result};
use crate::storage;
use crate::types::{Memory, MemoryStatus, MemoryType, PensieveConfig};
use crate::validation;

pub struct SaveInput {
    pub content: String,
    pub title: String,
    pub memory_type: MemoryType,
    pub topic_key: String,
    pub project: Option<String>,
    pub tags: Vec<String>,
    pub source: Option<String>,
    pub confidence: Option<crate::types::Confidence>,
    pub expected_revision: Option<u32>,
    pub dry_run: bool,
}

pub fn save_memory(config: &PensieveConfig, input: SaveInput) -> Result<Memory> {
    validation::validate_topic_key(&input.topic_key)?;
    if let Some(ref p) = input.project {
        validation::validate_project_name(p)?;
    }

    storage::ensure_dirs(config)?;

    let now = Utc::now();

    let memory = match storage::read_memory(config, &input.topic_key, input.project.as_deref()) {
        Ok(existing) => {
            if let Some(expected) = input.expected_revision {
                if existing.revision != expected {
                    return Err(PensieveError::RevisionConflict {
                        expected,
                        actual: existing.revision,
                    });
                }
            }
            let scope = if input.project.is_some() { "project" } else { "global" }.to_string();
            Memory {
                title: input.title,
                memory_type: input.memory_type,
                topic_key: input.topic_key,
                project: input.project,
                scope,
                status: existing.status,
                confidence: input.confidence.or(existing.confidence),
                revision: existing.revision + 1,
                tags: input.tags,
                source: input.source.or(existing.source),
                superseded_by: existing.superseded_by,
                created: existing.created,
                updated: now,
                content: input.content,
            }
        }
        Err(PensieveError::NotFound(_)) => {
            let scope = if input.project.is_some() { "project" } else { "global" }.to_string();
            Memory {
                title: input.title,
                memory_type: input.memory_type,
                topic_key: input.topic_key,
                project: input.project,
                scope,
                status: MemoryStatus::Active,
                confidence: input.confidence,
                revision: 1,
                tags: input.tags,
                source: input.source,
                superseded_by: None,
                created: now,
                updated: now,
                content: input.content,
            }
        }
        Err(e) => return Err(e),
    };

    if input.dry_run {
        return Ok(memory);
    }

    storage::write_memory(config, &memory)?;

    // Index integration added in Phase 3
    Ok(memory)
}
