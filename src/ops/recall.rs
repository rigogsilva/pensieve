use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::embedder;
use crate::error::Result;
use crate::index::Index;
use crate::storage;
use crate::types::{Memory, MemoryCompact, MemoryStatus, MemoryType, PensieveConfig};

const RECIPROCAL_RANK_K: f64 = 1.0;
const EXACT_MATCH_BOOST: f64 = 0.2;
const TITLE_TOKEN_BOOST: f64 = 0.1;
const BODY_TOKEN_BOOST: f64 = 0.1;
const TAG_TOKEN_BOOST: f64 = 0.05;

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
    let candidate_limit = limit.saturating_mul(5).max(20);

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
    let query_terms = tokenize(query);

    // BM25 keyword search
    let keyword_results = index.recall_keyword(query, candidate_limit)?;
    add_rank_scores(&mut scores, &keyword_results, config.retrieval.keyword_weight);

    // Vector search (best-effort)
    if let Ok(query_embedding) = embedder::embed(query) {
        if let Ok(vec_results) = index.recall_vector(&query_embedding, candidate_limit) {
            add_distance_scores(&mut scores, &vec_results, config.retrieval.vector_weight);
        }
    }

    // Sort by score descending
    let mut ranked: Vec<(String, f64)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Load full memories and apply filters
    let mut results = Vec::new();
    for (memory_id, score) in ranked {
        // Parse memory_id: "projects/{project}/{topic_key}" or "global/{topic_key}"
        let (project, topic_key) = parse_memory_id(&memory_id);

        if let Ok(memory) = storage::read_memory(config, topic_key, project) {
            // Apply filters
            if let Some(ref proj) = input.project {
                if memory.project.as_deref() != Some(proj.as_str()) {
                    continue;
                }
            }
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

            let final_score = score + metadata_match_boost(&memory, query, &query_terms);
            let mut compact = MemoryCompact::from(&memory);
            compact.score = Some(final_score.min(1.0));
            results.push(compact);
        }
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.updated.cmp(&a.updated))
    });
    results.truncate(limit);

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

fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|term| term.trim_matches(|c: char| !c.is_alphanumeric()).to_ascii_lowercase())
        .filter(|term| !term.is_empty())
        .collect()
}

fn add_rank_scores<T>(scores: &mut HashMap<String, f64>, results: &[(String, T)], weight: f64) {
    if weight <= 0.0 {
        return;
    }

    for (idx, (id, _)) in results.iter().enumerate() {
        let rank = f64::from(u32::try_from(idx + 1).unwrap_or(u32::MAX));
        let contribution = weight / (RECIPROCAL_RANK_K + rank - 1.0);
        *scores.entry(id.clone()).or_insert(0.0) += contribution;
    }
}

fn add_distance_scores(scores: &mut HashMap<String, f64>, results: &[(String, f64)], weight: f64) {
    if weight <= 0.0 || results.is_empty() {
        return;
    }

    let min_distance = results.iter().map(|(_, distance)| *distance).fold(f64::INFINITY, f64::min);
    let max_distance =
        results.iter().map(|(_, distance)| *distance).fold(f64::NEG_INFINITY, f64::max);
    let spread = max_distance - min_distance;

    for (id, distance) in results {
        let similarity =
            if spread > f64::EPSILON { 1.0 - ((distance - min_distance) / spread) } else { 1.0 };
        *scores.entry(id.clone()).or_insert(0.0) += weight * similarity;
    }
}

fn metadata_match_boost(memory: &Memory, query: &str, query_terms: &[String]) -> f64 {
    let title = memory.title.to_ascii_lowercase();
    let content = memory.content.to_ascii_lowercase();
    let query = query.trim().to_ascii_lowercase();

    let mut boost = 0.0;
    if !query.is_empty() {
        if title.contains(&query) {
            boost += EXACT_MATCH_BOOST;
        } else if content.contains(&query) {
            boost += EXACT_MATCH_BOOST / 2.0;
        }
    }

    let title_terms = tokenize(&memory.title);
    let content_terms = tokenize(&memory.content);
    let tag_terms: Vec<String> = memory.tags.iter().map(|tag| tag.to_ascii_lowercase()).collect();

    boost
        + overlap_ratio(query_terms, &title_terms) * TITLE_TOKEN_BOOST
        + overlap_ratio(query_terms, &content_terms) * BODY_TOKEN_BOOST
        + overlap_ratio(query_terms, &tag_terms) * TAG_TOKEN_BOOST
}

fn overlap_ratio(query_terms: &[String], candidate_terms: &[String]) -> f64 {
    if query_terms.is_empty() || candidate_terms.is_empty() {
        return 0.0;
    }

    let matches = query_terms.iter().filter(|term| candidate_terms.contains(term)).count();
    let matches = f64::from(u32::try_from(matches).unwrap_or(u32::MAX));
    let query_term_count = f64::from(u32::try_from(query_terms.len()).unwrap_or(u32::MAX));
    matches / query_term_count
}
