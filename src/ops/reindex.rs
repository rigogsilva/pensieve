use crate::embedder;
use crate::error::Result;
use crate::index::Index;
use crate::storage;
use crate::types::PensieveConfig;

pub fn reindex(config: &PensieveConfig, index: &Index) -> Result<usize> {
    index.clear()?;

    let memories = storage::list_memory_files(config, None, None, None)?;
    let total = memories.len();

    for (i, memory) in memories.iter().enumerate() {
        let memory_id = match &memory.project {
            Some(p) => format!("projects/{}/{}", p, memory.topic_key),
            None => format!("global/{}", memory.topic_key),
        };

        let embed_text = format!("{}: {}", memory.title, memory.content);
        let embedding = embedder::try_embed(&embed_text);

        index.upsert(
            &memory_id,
            &memory.title,
            &memory.content,
            memory.project.as_deref(),
            &memory.tags,
            embedding.as_deref(),
        )?;

        eprintln!("Reindexed {}/{total} memories", i + 1);
    }

    Ok(total)
}
