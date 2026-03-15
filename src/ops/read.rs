use crate::error::Result;
use crate::storage;
use crate::types::{Memory, PensieveConfig};

pub fn read_memory(
    config: &PensieveConfig,
    topic_key: &str,
    project: Option<&str>,
) -> Result<Memory> {
    storage::read_memory(config, topic_key, project)
}
