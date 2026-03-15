use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::embedder;
use crate::error::Result;
use crate::index::Index;
use crate::storage;
use crate::types::{MemoryCompact, MemoryStatus, MemoryType, PensieveConfig};

pub struct RecallInput {
    pub query: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub project: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<MemoryStatus>,
    pub since: Option<DateTime<Utc>>,
    pub limit: usize,
}

pub fn recall(
    config: &PensieveConfig,
    index: &Index,
    input: &RecallInput,
) -> Result<Vec<MemoryCompact>> {
    let limit = input.limit;

    // If no query, just list with filters
    let Some(ref query) = input.query else {
        let memories = storage::list_memory_files(
            config,
            input.project.as_deref(),
            input.memory_type.as_ref(),
            input.status.as_ref(),
        )?;
        let mut results: Vec<MemoryCompact> = memories
            .iter()
            .filter(|m| if let Some(ref since) = input.since { m.updated >= *since } else { true })
            .filter(|m| {
                if let Some(ref tags) = input.tags {
                    tags.iter().any(|t| m.tags.contains(t))
                } else {
                    true
                }
            })
            .map(MemoryCompact::from)
            .collect();
        results.truncate(limit);
        return Ok(results);
    };

    // Hybrid retrieval
    let mut scores: HashMap<String, f64> = HashMap::new();

    // BM25 keyword search
    let keyword_results = index.recall_keyword(query, limit * 2)?;
    let kw_max = keyword_results.iter().map(|(_, s)| s.abs()).fold(f64::MIN, f64::max);

    for (id, score) in &keyword_results {
        let normalized = if kw_max > 0.0 { score.abs() / kw_max } else { 0.0 };
        *scores.entry(id.clone()).or_insert(0.0) += config.retrieval.keyword_weight * normalized;
    }

    // Vector search (best-effort)
    if let Ok(query_embedding) = embedder::embed(query) {
        if let Ok(vec_results) = index.recall_vector(&query_embedding, limit * 2) {
            let vec_max = vec_results.iter().map(|(_, d)| *d).fold(f64::MIN, f64::max);

            for (id, distance) in &vec_results {
                let similarity = if vec_max > 0.0 { 1.0 - (distance / vec_max) } else { 1.0 };
                *scores.entry(id.clone()).or_insert(0.0) +=
                    config.retrieval.vector_weight * similarity;
            }
        }
    }

    // Sort by score descending
    let mut ranked: Vec<(String, f64)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.truncate(limit);

    // Load full memories and apply filters
    let mut results = Vec::new();
    for (memory_id, score) in ranked {
        // Parse memory_id: "projects/{project}/{topic_key}" or "global/{topic_key}"
        let (project, topic_key) = parse_memory_id(&memory_id);

        if let Ok(memory) = storage::read_memory(config, topic_key, project) {
            // Apply filters
            if let Some(ref tf) = input.memory_type {
                if &memory.memory_type != tf {
                    continue;
                }
            }
            if let Some(ref sf) = input.status {
                if &memory.status != sf {
                    continue;
                }
            }
            if let Some(ref since) = input.since {
                if memory.updated < *since {
                    continue;
                }
            }
            if let Some(ref tags) = input.tags {
                if !tags.iter().any(|t| memory.tags.contains(t)) {
                    continue;
                }
            }

            let mut compact = MemoryCompact::from(&memory);
            compact.score = Some(score);
            results.push(compact);
        }
    }

    Ok(results)
}

fn parse_memory_id(memory_id: &str) -> (Option<&str>, &str) {
    if let Some(rest) = memory_id.strip_prefix("projects/") {
        if let Some((project, topic_key)) = rest.split_once('/') {
            return (Some(project), topic_key);
        }
    }
    if let Some(topic_key) = memory_id.strip_prefix("global/") {
        return (None, topic_key);
    }
    // Fallback: treat entire id as topic_key
    (None, memory_id)
}
