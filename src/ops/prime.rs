use std::fmt::Write;
use std::io::Read;

use crate::error::Result;
use crate::index::Index;
use crate::ops::recall::{self, RecallInput};
use crate::types::PensieveConfig;

fn read_query_from_stdin() -> Option<String> {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_ok() && !input.trim().is_empty() {
        // Try JSON parse for Claude Code hook format: {"prompt": "..."}
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&input) {
            if let Some(prompt) = json.get("prompt").and_then(serde_json::Value::as_str) {
                if !prompt.trim().is_empty() {
                    return Some(prompt.to_string());
                }
            }
        }
        // Fall back to raw text
        let trimmed = input.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    None
}

fn format_prime(results: &[crate::types::MemoryCompact]) -> String {
    if results.is_empty() {
        return String::new();
    }

    let mut out = format!("[Pensieve: {} relevant memories]\n", results.len());
    for r in results {
        let summary = r
            .preview
            .lines()
            .find(|l| !l.trim().is_empty())
            .map_or(r.title.as_str(), str::trim);
        let _ = writeln!(out, "- [{}] {}: {}", r.memory_type, r.topic_key, summary);
    }
    out
}

pub fn run_prime(
    config: &PensieveConfig,
    query_flag: Option<String>,
    project: Option<String>,
    limit: Option<usize>,
    format: Option<&str>,
) -> Result<String> {
    // Config gate: if prime is disabled, output nothing
    if !config.prime.enabled {
        return Ok(String::new());
    }

    // Determine query: --query flag takes precedence, then stdin
    let query = query_flag.or_else(read_query_from_stdin);

    let Some(query) = query else {
        return Ok(String::new());
    };

    let max_results = limit.unwrap_or(config.prime.max_results);
    let threshold = config.prime.relevance_threshold;
    let output_format = format.unwrap_or(&config.prime.format);

    // Open index and run recall
    let index = Index::open(&config.memory_dir)?;
    let input = RecallInput {
        query: Some(query),
        memory_type: None,
        project,
        tags: None,
        status: None,
        since: None,
        limit: max_results,
    };

    let results = recall::recall(config, &index, &input)?;

    // Filter by relevance threshold
    let filtered: Vec<_> =
        results.into_iter().filter(|r| r.score.unwrap_or(0.0) >= threshold).collect();

    if filtered.is_empty() {
        return Ok(String::new());
    }

    match output_format {
        "json" => Ok(serde_json::to_string(&filtered).unwrap_or_default()),
        _ => Ok(format_prime(&filtered)),
    }
}
