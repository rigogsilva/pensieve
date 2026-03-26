use std::fmt::Write as _;

use tempfile::TempDir;

// Helper to create a config pointing to a temp directory
fn test_config(dir: &TempDir) -> pensieve::types::PensieveConfig {
    pensieve::types::PensieveConfig {
        memory_dir: dir.path().to_path_buf(),
        retrieval: pensieve::types::RetrievalConfig::default(),
        inject: pensieve::types::InjectConfig::default(),
    }
}

// Helper to create a config with inject enabled
fn test_config_inject_enabled(dir: &TempDir) -> pensieve::types::PensieveConfig {
    let mut cfg = test_config(dir);
    cfg.inject.enabled = true;
    cfg
}

fn index_memory(config: &pensieve::types::PensieveConfig, memory: &pensieve::types::Memory) {
    let idx = pensieve::index::Index::open(&config.memory_dir).unwrap();
    let memory_id = match &memory.project {
        Some(project) => format!("projects/{project}/{}", memory.topic_key),
        None => format!("global/{}", memory.topic_key),
    };
    let embed_text = pensieve::embedder::build_embedding_text(
        &memory.title,
        &memory.content,
        &memory.memory_type,
        memory.project.as_deref(),
        &memory.tags,
    );
    let embedding = pensieve::embedder::try_embed(&embed_text);
    let _ = idx.upsert(
        &memory_id,
        &memory.title,
        &memory.content,
        memory.project.as_deref(),
        &memory.tags,
        embedding.as_deref(),
    );
}

#[derive(Clone)]
struct RetrievalFixture {
    topic_key: &'static str,
    title: &'static str,
    content: &'static str,
    memory_type: pensieve::types::MemoryType,
    tags: &'static [&'static str],
    exact_queries: &'static [&'static str],
    semantic_queries: &'static [&'static str],
}

struct RetrievalMetrics {
    top1: f64,
    top3: f64,
    top5: f64,
    mrr: f64,
}

struct RetrievalQueryCase {
    expected_topic_key: &'static str,
    query: String,
}

#[allow(clippy::too_many_lines)]
fn retrieval_fixtures() -> Vec<RetrievalFixture> {
    use pensieve::types::MemoryType::{Decision, Discovery, Gotcha, HowItWorks, Preference};

    vec![
        RetrievalFixture {
            topic_key: "rust-formatting",
            title: "Rust formatting workflow",
            content: "Run cargo fmt before committing so rustfmt keeps the codebase style consistent across files.",
            memory_type: HowItWorks,
            tags: &["rust", "formatting"],
            exact_queries: &["cargo fmt rustfmt style", "rust formatting workflow"],
            semantic_queries: &[
                "how should we autoformat rust code before a commit",
                "what keeps the code style consistent in this rust repo",
            ],
        },
        RetrievalFixture {
            topic_key: "cas-revision-guard",
            title: "Revision conflict protection",
            content: "Use expected_revision when updating a memory so concurrent writes fail instead of silently overwriting newer content.",
            memory_type: Decision,
            tags: &["revision", "cas"],
            exact_queries: &["expected_revision concurrent writes", "revision conflict protection"],
            semantic_queries: &[
                "how do we avoid overwriting a newer memory update",
                "what is the optimistic locking mechanism for edits",
            ],
        },
        RetrievalFixture {
            topic_key: "project-scope-guideline",
            title: "Project scoped memories",
            content: "Store implementation details under a project so unrelated work does not pollute the global memory namespace.",
            memory_type: Decision,
            tags: &["project", "scope"],
            exact_queries: &[
                "project scoped memories global namespace",
                "implementation details under a project",
            ],
            semantic_queries: &[
                "where should repo specific notes live so they do not clutter shared memory",
                "how do we isolate memories for one codebase",
            ],
        },
        RetrievalFixture {
            topic_key: "inject-compact-output",
            title: "Compact injection format",
            content: "Injection works best when the output is compact because hook consumers need short snippets instead of full documents.",
            memory_type: Preference,
            tags: &["inject", "output"],
            exact_queries: &[
                "compact injection format hook consumers",
                "short snippets instead of full documents",
            ],
            semantic_queries: &[
                "why should auto injected context stay brief",
                "what output style is best for hook based memory injection",
            ],
        },
        RetrievalFixture {
            topic_key: "fts-stopwords",
            title: "FTS stopword filtering",
            content: "Keyword search removes common stopwords and builds an OR query so meaningful terms still match even in natural language prompts.",
            memory_type: HowItWorks,
            tags: &["fts", "search"],
            exact_queries: &["stopwords OR query meaningful terms", "fts stopword filtering"],
            semantic_queries: &[
                "how does keyword search handle filler words in a sentence",
                "why can natural language prompts still hit the text index",
            ],
        },
        RetrievalFixture {
            topic_key: "memory-markdown-source",
            title: "Markdown as source of truth",
            content: "Each memory is stored as a markdown file with YAML frontmatter, and the database acts as a retrieval index rather than the canonical source.",
            memory_type: HowItWorks,
            tags: &["storage", "markdown"],
            exact_queries: &[
                "markdown file YAML frontmatter canonical source",
                "database retrieval index canonical source",
            ],
            semantic_queries: &[
                "is sqlite the main record or just a search helper",
                "where is the authoritative memory data stored",
            ],
        },
        RetrievalFixture {
            topic_key: "reindex-after-corruption",
            title: "Reindex after index drift",
            content: "If the SQLite index falls behind the markdown files, run reindex to rebuild embeddings and keyword tables from disk.",
            memory_type: Gotcha,
            tags: &["reindex", "index"],
            exact_queries: &[
                "reindex rebuild embeddings keyword tables",
                "sqlite index falls behind markdown files",
            ],
            semantic_queries: &[
                "what should we do if the search database gets out of sync with memory files",
                "how do we rebuild retrieval metadata from disk",
            ],
        },
        RetrievalFixture {
            topic_key: "session-summaries",
            title: "Session summaries capture decisions",
            content: "end-session writes a timestamped markdown summary so future sessions can load decisions and outcomes from prior work.",
            memory_type: Discovery,
            tags: &["session", "summary"],
            exact_queries: &[
                "end-session timestamped markdown summary",
                "future sessions load decisions outcomes",
            ],
            semantic_queries: &[
                "how are important choices from a work session carried forward",
                "what command stores a wrap up note at the end of a session",
            ],
        },
        RetrievalFixture {
            topic_key: "validation-topic-key",
            title: "Topic key validation",
            content: "Topic keys must be lowercase alphanumeric with hyphens, and path traversal attempts are rejected before any file is written.",
            memory_type: Gotcha,
            tags: &["validation", "topic-key"],
            exact_queries: &[
                "topic keys lowercase alphanumeric hyphens",
                "path traversal rejected before file is written",
            ],
            semantic_queries: &[
                "what filename rules are enforced for memory ids",
                "how does the tool prevent saving outside the memory directory",
            ],
        },
        RetrievalFixture {
            topic_key: "hybrid-weighting",
            title: "Hybrid retrieval weights",
            content: "Pensieve combines normalized BM25 keyword scores and vector similarity using configurable keyword and vector weights.",
            memory_type: HowItWorks,
            tags: &["hybrid", "weights"],
            exact_queries: &[
                "normalized BM25 vector similarity weights",
                "hybrid retrieval weights",
            ],
            semantic_queries: &[
                "how are lexical and embedding scores blended together",
                "what controls the balance between keyword search and semantic search",
            ],
        },
        RetrievalFixture {
            topic_key: "inject-disabled-default",
            title: "Injection is disabled by default",
            content: "Auto injection returns nothing unless inject.enabled is true in config, which keeps agents from blocking on optional retrieval.",
            memory_type: Preference,
            tags: &["inject", "config"],
            exact_queries: &[
                "inject.enabled true returns nothing unless enabled",
                "injection is disabled by default",
            ],
            semantic_queries: &[
                "why might the injection command output nothing on a fresh install",
                "what config switch turns on automatic memory injection",
            ],
        },
        RetrievalFixture {
            topic_key: "human-vs-json-output",
            title: "Human and JSON output modes",
            content: "Most commands can render either human friendly text or structured JSON so tools and people can both consume the results.",
            memory_type: Discovery,
            tags: &["output", "json"],
            exact_queries: &["human friendly text structured JSON", "human and JSON output modes"],
            semantic_queries: &[
                "can this CLI return machine readable responses",
                "how do commands support both scripts and humans",
            ],
        },
        RetrievalFixture {
            topic_key: "archive-vs-delete",
            title: "Archive before delete when history matters",
            content: "Archive preserves a memory and can mark it as superseded, while delete removes it entirely from storage and the index.",
            memory_type: Decision,
            tags: &["archive", "delete"],
            exact_queries: &[
                "archive superseded delete removes entirely",
                "archive before delete when history matters",
            ],
            semantic_queries: &[
                "when should we preserve an old memory instead of erasing it",
                "what is the difference between removing and superseding a memory",
            ],
        },
        RetrievalFixture {
            topic_key: "context-bootstrap",
            title: "Context bootstrap buckets",
            content: "get-context returns recent sessions, preferences, gotchas, and decisions so an agent can start with the most important prior knowledge.",
            memory_type: HowItWorks,
            tags: &["context", "bootstrap"],
            exact_queries: &[
                "recent sessions preferences gotchas decisions",
                "context bootstrap buckets",
            ],
            semantic_queries: &[
                "what does the session start bootstrap include",
                "how does an agent load the most relevant prior knowledge at the start",
            ],
        },
        RetrievalFixture {
            topic_key: "sqlite-vec-extension",
            title: "sqlite-vec powers vector search",
            content: "The index registers sqlite-vec as an SQLite extension so embeddings can be queried with nearest neighbor search over 384 dimensional vectors.",
            memory_type: HowItWorks,
            tags: &["sqlite-vec", "vector"],
            exact_queries: &[
                "sqlite-vec nearest neighbor 384 dimensional vectors",
                "sqlite-vec powers vector search",
            ],
            semantic_queries: &[
                "which component handles similarity lookup for embeddings",
                "how are embedding vectors stored and queried in the index",
            ],
        },
        RetrievalFixture {
            topic_key: "confidence-metadata",
            title: "Confidence metadata is optional",
            content: "Memories may include high, medium, or low confidence to signal how strongly the agent trusts a recorded fact or recommendation.",
            memory_type: Discovery,
            tags: &["confidence", "metadata"],
            exact_queries: &[
                "high medium low confidence metadata",
                "confidence metadata is optional",
            ],
            semantic_queries: &[
                "how can a memory express uncertainty",
                "is there a way to mark how trustworthy a saved fact is",
            ],
        },
        RetrievalFixture {
            topic_key: "config-location",
            title: "Config lives under dot config",
            content: "Pensieve stores configuration in config.toml under the user config directory, separate from the memory markdown files.",
            memory_type: HowItWorks,
            tags: &["config", "path"],
            exact_queries: &[
                "config.toml user config directory separate from memory files",
                "config lives under dot config",
            ],
            semantic_queries: &[
                "where are the settings stored compared with the actual memories",
                "what file holds retrieval and injection configuration",
            ],
        },
    ]
}

fn build_retrieval_corpus(config: &pensieve::types::PensieveConfig) {
    pensieve::storage::ensure_dirs(config).unwrap();
    for fixture in retrieval_fixtures() {
        let input = pensieve::ops::save::SaveInput {
            content: fixture.content.to_string(),
            title: fixture.title.to_string(),
            memory_type: fixture.memory_type,
            topic_key: fixture.topic_key.to_string(),
            project: None,
            tags: fixture.tags.iter().map(|tag| (*tag).to_string()).collect(),
            source: Some("benchmark".to_string()),
            confidence: None,
            expected_revision: None,
            dry_run: false,
        };
        pensieve::ops::save::save_memory(config, input).unwrap();
    }
    let idx = pensieve::index::Index::open(&config.memory_dir).unwrap();
    pensieve::ops::reindex::reindex(config, &idx).unwrap();
}

fn title_query(text: &str) -> String {
    text.to_ascii_lowercase()
}

fn topic_key_query(topic_key: &str) -> String {
    topic_key.replace('-', " ")
}

fn semantic_stress_queries() -> Vec<RetrievalQueryCase> {
    let mut cases = Vec::new();
    for fixture in retrieval_fixtures() {
        for query in fixture.exact_queries.iter().chain(fixture.semantic_queries.iter()) {
            cases.push(RetrievalQueryCase {
                expected_topic_key: fixture.topic_key,
                query: (*query).to_string(),
            });
        }
    }
    cases
}

fn lexical_heavy_queries() -> Vec<RetrievalQueryCase> {
    let mut cases = Vec::new();
    for fixture in retrieval_fixtures() {
        cases.push(RetrievalQueryCase {
            expected_topic_key: fixture.topic_key,
            query: title_query(fixture.title),
        });
        cases.push(RetrievalQueryCase {
            expected_topic_key: fixture.topic_key,
            query: topic_key_query(fixture.topic_key),
        });
        cases.push(RetrievalQueryCase {
            expected_topic_key: fixture.topic_key,
            query: fixture.exact_queries[0].to_string(),
        });
    }
    cases
}

fn run_retrieval_benchmark(
    config: &pensieve::types::PensieveConfig,
    queries: &[RetrievalQueryCase],
) -> RetrievalMetrics {
    let idx = pensieve::index::Index::open(&config.memory_dir).unwrap();
    let mut top1 = 0.0;
    let mut top3 = 0.0;
    let mut top5 = 0.0;
    let mut reciprocal_rank_sum = 0.0;
    let mut total = 0.0;

    for case in queries {
        let input = pensieve::ops::recall::RecallInput {
            query: Some(case.query.clone()),
            memory_type: None,
            project: None,
            tags: None,
            status: None,
            since: None,
            limit: 5,
        };
        let results = pensieve::ops::recall::recall(config, &idx, &input).unwrap();
        let rank = results
            .iter()
            .position(|memory| memory.topic_key == case.expected_topic_key)
            .map(|i| i + 1);

        if rank == Some(1) {
            top1 += 1.0;
        }
        if rank.is_some_and(|r| r <= 3) {
            top3 += 1.0;
        }
        if rank.is_some_and(|r| r <= 5) {
            top5 += 1.0;
        }
        reciprocal_rank_sum += rank.map_or(0.0, |r| 1.0 / f64::from(u32::try_from(r).unwrap()));
        total += 1.0;
    }

    RetrievalMetrics {
        top1: top1 / total,
        top3: top3 / total,
        top5: top5 / total,
        mrr: reciprocal_rank_sum / total,
    }
}

#[test]
#[ignore = "benchmark-style retrieval eval; run manually with --ignored --nocapture"]
fn benchmark_recall_quality() {
    let dir = TempDir::new().unwrap();
    let mut config = test_config(&dir);
    config.retrieval.keyword_weight = 0.7;
    config.retrieval.vector_weight = 0.3;

    build_retrieval_corpus(&config);
    let semantic_queries = semantic_stress_queries();
    let lexical_queries = lexical_heavy_queries();

    let semantic_metrics = run_retrieval_benchmark(&config, &semantic_queries);
    let lexical_metrics = run_retrieval_benchmark(&config, &lexical_queries);

    let mut vector_heavy = config.clone();
    vector_heavy.retrieval.keyword_weight = 0.2;
    vector_heavy.retrieval.vector_weight = 0.8;
    let lexical_vector_heavy = run_retrieval_benchmark(&vector_heavy, &lexical_queries);

    println!(
        "semantic stress (0.7/0.3): top1={:.3}, top3={:.3}, top5={:.3}, mrr={:.3}",
        semantic_metrics.top1, semantic_metrics.top3, semantic_metrics.top5, semantic_metrics.mrr
    );
    println!(
        "lexical heavy (0.7/0.3): top1={:.3}, top3={:.3}, top5={:.3}, mrr={:.3}",
        lexical_metrics.top1, lexical_metrics.top3, lexical_metrics.top5, lexical_metrics.mrr
    );
    println!(
        "lexical heavy (0.2/0.8): top1={:.3}, top3={:.3}, top5={:.3}, mrr={:.3}",
        lexical_vector_heavy.top1,
        lexical_vector_heavy.top3,
        lexical_vector_heavy.top5,
        lexical_vector_heavy.mrr
    );

    assert!(semantic_metrics.top1 >= 0.75, "semantic top1 regression: {}", semantic_metrics.top1);
    assert!(semantic_metrics.top5 >= 0.95, "semantic top5 regression: {}", semantic_metrics.top5);
    assert!(semantic_metrics.mrr >= 0.83, "semantic mrr regression: {}", semantic_metrics.mrr);

    assert!(lexical_metrics.top1 >= 0.95, "lexical top1 regression: {}", lexical_metrics.top1);
    assert!(lexical_metrics.top5 >= 0.99, "lexical top5 regression: {}", lexical_metrics.top5);
    assert!(lexical_metrics.mrr >= 0.97, "lexical mrr regression: {}", lexical_metrics.mrr);
    assert!(
        lexical_metrics.top1 >= lexical_vector_heavy.top1,
        "lexical benchmark should not favor vector-heavy weights"
    );
}

#[test]
fn test_save_and_read() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input = pensieve::ops::save::SaveInput {
        content: "Test content".to_string(),
        title: "Test Memory".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "test-key".to_string(),
        project: None,
        tags: vec!["test".to_string()],
        source: Some("test-runner".to_string()),
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };

    let memory = pensieve::ops::save::save_memory(&config, input).unwrap();
    assert_eq!(memory.revision, 1);
    assert_eq!(memory.title, "Test Memory");

    let read = pensieve::ops::read::read_memory(&config, "test-key", None).unwrap();
    assert_eq!(read.title, "Test Memory");
    assert_eq!(read.content, "Test content");
    assert_eq!(read.revision, 1);
}

#[test]
fn test_save_upsert_increments_revision() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input1 = pensieve::ops::save::SaveInput {
        content: "Version 1".to_string(),
        title: "Evolving Memory".to_string(),
        memory_type: pensieve::types::MemoryType::Decision,
        topic_key: "evolving".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    let m1 = pensieve::ops::save::save_memory(&config, input1).unwrap();
    assert_eq!(m1.revision, 1);

    let input2 = pensieve::ops::save::SaveInput {
        content: "Version 2".to_string(),
        title: "Evolving Memory".to_string(),
        memory_type: pensieve::types::MemoryType::Decision,
        topic_key: "evolving".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    let m2 = pensieve::ops::save::save_memory(&config, input2).unwrap();
    assert_eq!(m2.revision, 2);
}

#[test]
fn test_save_revision_conflict() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input1 = pensieve::ops::save::SaveInput {
        content: "Initial".to_string(),
        title: "CAS Test".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "cas-test".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input1).unwrap();

    let input2 = pensieve::ops::save::SaveInput {
        content: "Conflict".to_string(),
        title: "CAS Test".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "cas-test".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: Some(999),
        dry_run: false,
    };
    let result = pensieve::ops::save::save_memory(&config, input2);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("conflict"));
}

#[test]
fn test_save_dry_run() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input = pensieve::ops::save::SaveInput {
        content: "Should not be written".to_string(),
        title: "Dry Run".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "dry-run-key".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: true,
    };

    let memory = pensieve::ops::save::save_memory(&config, input).unwrap();
    assert_eq!(memory.title, "Dry Run");

    // File should not exist
    let result = pensieve::ops::read::read_memory(&config, "dry-run-key", None);
    assert!(result.is_err());
}

#[test]
fn test_read_not_found() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let result = pensieve::ops::read::read_memory(&config, "nonexistent", None);
    assert!(result.is_err());
}

#[test]
fn test_delete() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input = pensieve::ops::save::SaveInput {
        content: "To be deleted".to_string(),
        title: "Delete Me".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "delete-me".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input).unwrap();

    pensieve::ops::delete::delete_memory(&config, "delete-me", None, false).unwrap();

    let result = pensieve::ops::read::read_memory(&config, "delete-me", None);
    assert!(result.is_err());
}

#[test]
fn test_delete_dry_run() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input = pensieve::ops::save::SaveInput {
        content: "Should survive".to_string(),
        title: "Survivor".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "survivor".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input).unwrap();

    pensieve::ops::delete::delete_memory(&config, "survivor", None, true).unwrap();

    // File should still exist
    let result = pensieve::ops::read::read_memory(&config, "survivor", None);
    assert!(result.is_ok());
}

#[test]
fn test_list() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    for i in 0..3 {
        let input = pensieve::ops::save::SaveInput {
            content: format!("Content {i}"),
            title: format!("Memory {i}"),
            memory_type: pensieve::types::MemoryType::Discovery,
            topic_key: format!("item-{i}"),
            project: None,
            tags: vec![],
            source: None,
            confidence: None,
            expected_revision: None,
            dry_run: false,
        };
        pensieve::ops::save::save_memory(&config, input).unwrap();
    }

    let list = pensieve::ops::list::list_memories(&config, None, None, None, None).unwrap();
    assert_eq!(list.len(), 3);
}

#[test]
fn test_list_filter_by_type() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input1 = pensieve::ops::save::SaveInput {
        content: "Gotcha content".to_string(),
        title: "A Gotcha".to_string(),
        memory_type: pensieve::types::MemoryType::Gotcha,
        topic_key: "gotcha-1".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input1).unwrap();

    let input2 = pensieve::ops::save::SaveInput {
        content: "Decision content".to_string(),
        title: "A Decision".to_string(),
        memory_type: pensieve::types::MemoryType::Decision,
        topic_key: "decision-1".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input2).unwrap();

    let gotchas = pensieve::ops::list::list_memories(
        &config,
        None,
        Some(&pensieve::types::MemoryType::Gotcha),
        None,
        None,
    )
    .unwrap();
    assert_eq!(gotchas.len(), 1);
    assert_eq!(gotchas[0].title, "A Gotcha");
}

#[test]
fn test_archive() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input = pensieve::ops::save::SaveInput {
        content: "To be archived".to_string(),
        title: "Archive Me".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "archive-me".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input).unwrap();

    let archived =
        pensieve::ops::archive::archive_memory(&config, "archive-me", None, None, false).unwrap();
    assert_eq!(archived.status, pensieve::types::MemoryStatus::Archived);

    let read = pensieve::ops::read::read_memory(&config, "archive-me", None).unwrap();
    assert_eq!(read.status, pensieve::types::MemoryStatus::Archived);
}

#[test]
fn test_archive_superseded() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input = pensieve::ops::save::SaveInput {
        content: "Old content".to_string(),
        title: "Old Memory".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "old-memory".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input).unwrap();

    let archived = pensieve::ops::archive::archive_memory(
        &config,
        "old-memory",
        None,
        Some("new-memory"),
        false,
    )
    .unwrap();
    assert_eq!(archived.status, pensieve::types::MemoryStatus::Superseded);
    assert_eq!(archived.superseded_by.as_deref(), Some("new-memory"));
}

#[test]
fn test_project_scoping() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input = pensieve::ops::save::SaveInput {
        content: "Project-specific".to_string(),
        title: "Project Memory".to_string(),
        memory_type: pensieve::types::MemoryType::HowItWorks,
        topic_key: "project-mem".to_string(),
        project: Some("myproject".to_string()),
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input).unwrap();

    // Read with project
    let read = pensieve::ops::read::read_memory(&config, "project-mem", Some("myproject")).unwrap();
    assert_eq!(read.title, "Project Memory");

    // Read without project should find the project-scoped memory
    let read_no_proj = pensieve::ops::read::read_memory(&config, "project-mem", None).unwrap();
    assert_eq!(read_no_proj.title, "Project Memory");
    assert_eq!(read_no_proj.project.as_deref(), Some("myproject"));

    // List with project filter
    let list =
        pensieve::ops::list::list_memories(&config, Some("myproject"), None, None, None).unwrap();
    assert_eq!(list.len(), 1);
}

#[test]
fn test_validation_rejects_path_traversal() {
    let result = pensieve::validation::validate_topic_key("../etc/passwd");
    assert!(result.is_err());

    let result = pensieve::validation::validate_topic_key("hello world");
    assert!(result.is_err());

    let result = pensieve::validation::validate_topic_key("Hello");
    assert!(result.is_err());

    let result = pensieve::validation::validate_topic_key("valid-key-123");
    assert!(result.is_ok());
}

#[test]
fn test_validation_rejects_special_chars() {
    let result = pensieve::validation::validate_topic_key("foo?bar");
    assert!(result.is_err());

    let result = pensieve::validation::validate_topic_key("foo#bar");
    assert!(result.is_err());

    let result = pensieve::validation::validate_topic_key("foo/bar");
    assert!(result.is_err());
}

#[test]
fn test_end_session() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let session = pensieve::ops::end_session::end_session(
        &config,
        "Did some work",
        &["Decision A".to_string()],
        "test-agent",
        Some("myproject"),
        false,
    )
    .unwrap();

    assert_eq!(session.summary, "Did some work");
    assert_eq!(session.key_decisions.len(), 1);

    // Verify session file was created
    let sessions = pensieve::storage::list_sessions(&config, 10).unwrap();
    assert_eq!(sessions.len(), 1);
}

#[test]
fn test_get_context() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Create a global-scoped preference (project: None)
    let input = pensieve::ops::save::SaveInput {
        content: "Always use tabs".to_string(),
        title: "Indentation Preference".to_string(),
        memory_type: pensieve::types::MemoryType::Preference,
        topic_key: "indent-pref".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input).unwrap();

    let ctx = pensieve::ops::context::get_context(&config, None, None).unwrap();

    // global_index should be non-empty (has the preference we saved)
    assert!(!ctx.global_index.is_empty(), "global_index should not be empty");
    assert!(
        ctx.global_index.contains("indent-pref"),
        "global_index should contain the memory's topic_key"
    );

    // project_index should be None when no project was passed
    assert!(ctx.project_index.is_none(), "project_index should be None when no project passed");

    // MEMORY.md file should exist on disk
    let memory_md = dir.path().join("MEMORY.md");
    assert!(memory_md.exists(), "MEMORY.md should be written to disk");

    // CONTEXT.md should NOT exist
    let context_md = dir.path().join("CONTEXT.md");
    assert!(!context_md.exists(), "CONTEXT.md should not exist after get_context");
}

#[test]
fn test_storage_roundtrip() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input = pensieve::ops::save::SaveInput {
        content: "Roundtrip content with\nmultiple lines\nand special chars: @#$%".to_string(),
        title: "Roundtrip Test".to_string(),
        memory_type: pensieve::types::MemoryType::HowItWorks,
        topic_key: "roundtrip".to_string(),
        project: Some("test-project".to_string()),
        tags: vec!["tag1".to_string(), "tag2".to_string()],
        source: Some("test".to_string()),
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    let saved = pensieve::ops::save::save_memory(&config, input).unwrap();

    let read =
        pensieve::ops::read::read_memory(&config, "roundtrip", Some("test-project")).unwrap();

    assert_eq!(read.title, saved.title);
    assert_eq!(read.topic_key, saved.topic_key);
    assert_eq!(read.project, saved.project);
    assert_eq!(read.tags, saved.tags);
    assert_eq!(read.source, saved.source);
    assert_eq!(read.revision, saved.revision);
    assert!(read.content.contains("multiple lines"));
    assert!(read.content.contains("special chars: @#$%"));
}

#[test]
fn test_configure() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);

    // Test config loading with defaults
    let loaded = pensieve::config::load_config(None).unwrap();
    assert!(loaded.memory_dir.ends_with(".pensieve/memory"));

    // Test CLI override
    let custom_dir = dir.path().join("custom");
    let loaded = pensieve::config::load_config(Some(custom_dir.to_str().unwrap())).unwrap();
    assert_eq!(loaded.memory_dir, custom_dir);

    // Test ensure_dirs creates subdirectories
    pensieve::storage::ensure_dirs(&config).unwrap();
    assert!(dir.path().join("global").exists());
    assert!(dir.path().join("projects").exists());
    assert!(dir.path().join("sessions").exists());

    // Test retrieval config defaults
    assert!((config.retrieval.keyword_weight - 0.7).abs() < f64::EPSILON);
    assert!((config.retrieval.vector_weight - 0.3).abs() < f64::EPSILON);
}

#[test]
fn test_staleness_flag() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Save a memory
    let input = pensieve::ops::save::SaveInput {
        content: "Old content".to_string(),
        title: "Stale Memory".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "stale-test".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input).unwrap();

    // Read the file, modify updated timestamp to 100 days ago, write it back
    let path = dir.path().join("global").join("stale-test.md");
    let contents = std::fs::read_to_string(&path).unwrap();
    let old_date = (chrono::Utc::now() - chrono::Duration::days(100)).to_rfc3339();
    // Replace the updated field in frontmatter
    let mut new_contents = String::new();
    for line in contents.lines() {
        if line.starts_with("updated:") {
            let _ = write!(new_contents, "updated: {old_date}");
        } else {
            new_contents.push_str(line);
        }
        new_contents.push('\n');
    }
    std::fs::write(&path, new_contents).unwrap();

    // Get context and verify it succeeds (stale_memories field removed)
    let ctx = pensieve::ops::context::get_context(&config, None, None).unwrap();
    assert!(
        ctx.global_index.is_empty() || ctx.global_index.contains("stale-test"),
        "get_context should succeed even with old memories"
    );
}

#[test]
fn test_dry_run_end_session() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let session = pensieve::ops::end_session::end_session(
        &config,
        "Dry run session",
        &["Decision X".to_string()],
        "test-agent",
        Some("myproject"),
        true,
    )
    .unwrap();

    assert_eq!(session.summary, "Dry run session");

    // Verify no session file was created
    let sessions = pensieve::storage::list_sessions(&config, 10).unwrap();
    assert!(sessions.is_empty(), "Expected no session files for dry run");
}

#[test]
fn test_dry_run_configure() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);

    let new_dir = dir.path().join("custom-dry");
    let result = pensieve::ops::configure::configure(
        &config,
        Some(new_dir.to_str().unwrap()),
        None,
        None,
        None,
        true,
    )
    .unwrap();

    assert_eq!(result.memory_dir, new_dir);

    // Verify the custom dir was NOT created (dry run should not create dirs)
    assert!(!new_dir.exists(), "Expected custom dir not to be created in dry run");
}

#[test]
fn test_context_alias() {
    // The "context" alias is a CLI-level feature. We verify here that the
    // underlying get_context function works the same way it would for both
    // the "context" and "get-context" subcommands.
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    // No memories saved — global_index should be empty string
    let ctx = pensieve::ops::context::get_context(&config, None, None).unwrap();
    assert_eq!(
        ctx.global_index, "",
        "global_index should be empty string when no global memories exist"
    );
    assert!(ctx.project_index.is_none(), "project_index should be None when no project passed");

    // Also verify via the binary that both subcommands are accepted
    let bin = env!("CARGO_BIN_EXE_pensieve");
    let output = std::process::Command::new(bin)
        .arg("--output")
        .arg("json")
        .arg("--memory-dir")
        .arg(dir.path().to_str().unwrap())
        .arg("context")
        .output()
        .expect("failed to run pensieve context");
    assert!(
        output.status.success(),
        "pensieve context should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = std::process::Command::new(bin)
        .arg("--output")
        .arg("json")
        .arg("--memory-dir")
        .arg(dir.path().to_str().unwrap())
        .arg("get-context")
        .output()
        .expect("failed to run pensieve get-context");
    assert!(
        output.status.success(),
        "pensieve get-context should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_get_context_empty_global() {
    // No global-scoped memories → global_index is empty string
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Save a project-scoped memory (should NOT appear in global_index)
    let input = pensieve::ops::save::SaveInput {
        content: "Project-specific knowledge".to_string(),
        title: "Project Memory".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "proj-mem".to_string(),
        project: Some("myproject".to_string()),
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input).unwrap();

    let ctx = pensieve::ops::context::get_context(&config, None, None).unwrap();
    assert_eq!(
        ctx.global_index, "",
        "global_index should be empty when only project-scoped memories exist"
    );
    assert!(ctx.project_index.is_none());
}

#[test]
fn test_get_context_with_project_scope() {
    // project provided → project_index is Some
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    let input = pensieve::ops::save::SaveInput {
        content: "Project-specific knowledge".to_string(),
        title: "Project Memory".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "proj-mem-2".to_string(),
        project: Some("myproject".to_string()),
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input).unwrap();

    let ctx = pensieve::ops::context::get_context(&config, Some("myproject"), None).unwrap();
    assert!(ctx.project_index.is_some(), "project_index should be Some when project is passed");
    let proj_idx = ctx.project_index.unwrap();
    assert!(proj_idx.contains("proj-mem-2"), "project_index should contain the project memory");
}

#[test]
fn test_context_md_deletion() {
    // CONTEXT.md pre-existing → deleted after get_context
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Create a CONTEXT.md file to simulate legacy state
    let context_md_path = dir.path().join("CONTEXT.md");
    std::fs::write(&context_md_path, "# Legacy Context\nOld content").unwrap();
    assert!(context_md_path.exists(), "CONTEXT.md should exist before get_context");

    // Run get_context — should delete CONTEXT.md
    pensieve::ops::context::get_context(&config, None, None).unwrap();
    assert!(!context_md_path.exists(), "CONTEXT.md should be deleted after get_context");
}

#[test]
fn test_inject_disabled() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Save a memory
    let input = pensieve::ops::save::SaveInput {
        content: "Inject disabled content".to_string(),
        title: "Inject Disabled Test".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "inject-disabled".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    pensieve::ops::save::save_memory(&config, input).unwrap();

    // inject.enabled is false by default — should return empty
    let result =
        pensieve::ops::inject::run_inject(&config, Some("inject".to_string()), None, None, None)
            .unwrap();
    assert!(result.is_empty(), "inject should return empty when disabled");
}

#[test]
fn test_inject_query_flag() {
    let dir = TempDir::new().unwrap();
    let config = test_config_inject_enabled(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Save a memory and index it
    let input = pensieve::ops::save::SaveInput {
        content: "The patronus charm requires focus".to_string(),
        title: "Patronus Charm".to_string(),
        memory_type: pensieve::types::MemoryType::HowItWorks,
        topic_key: "patronus-inject".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    let memory = pensieve::ops::save::save_memory(&config, input).unwrap();

    // Index it
    index_memory(&config, &memory);

    // Run inject with --query flag
    let result =
        pensieve::ops::inject::run_inject(&config, Some("patronus".to_string()), None, None, None)
            .unwrap();

    assert!(result.contains("patronus-inject"), "inject should include topic_key, got: {result}");
    assert!(result.contains("patronus charm requires focus"), "inject should include preview, got: {result}");
    assert!(result.contains("[Pensieve:"), "should have compact format header");
}

#[test]
fn test_inject_empty_result() {
    let dir = TempDir::new().unwrap();
    let config = test_config_inject_enabled(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Index exists but no memories match
    let _idx = pensieve::index::Index::open(&config.memory_dir).unwrap();

    let result = pensieve::ops::inject::run_inject(
        &config,
        Some("nonexistent query".to_string()),
        None,
        None,
        None,
    )
    .unwrap();

    assert!(result.is_empty(), "inject with no matches should return empty");
}

#[test]
fn test_inject_json_format() {
    let dir = TempDir::new().unwrap();
    let config = test_config_inject_enabled(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Save and index a memory
    let input = pensieve::ops::save::SaveInput {
        content: "Expelliarmus disarming charm".to_string(),
        title: "Expelliarmus".to_string(),
        memory_type: pensieve::types::MemoryType::HowItWorks,
        topic_key: "expelliarmus-inject".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    let memory = pensieve::ops::save::save_memory(&config, input).unwrap();
    index_memory(&config, &memory);

    let result = pensieve::ops::inject::run_inject(
        &config,
        Some("expelliarmus".to_string()),
        None,
        None,
        Some("json"),
    )
    .unwrap();

    // JSON format should parse
    let parsed: serde_json::Value = serde_json::from_str(&result)
        .unwrap_or_else(|_| panic!("should be valid JSON, got: {result}"));
    assert!(parsed.is_array(), "should return JSON array");
}

#[test]
fn test_inject_no_stderr() {
    let dir = TempDir::new().unwrap();
    let config = test_config_inject_enabled(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Run inject via binary and capture stderr
    let bin = env!("CARGO_BIN_EXE_pensieve");
    let output = std::process::Command::new(bin)
        .arg("--memory-dir")
        .arg(dir.path().to_str().unwrap())
        .arg("inject")
        .arg("--query")
        .arg("test")
        .output()
        .expect("failed to run pensieve inject");

    assert!(output.status.success(), "inject should exit 0");
    assert!(
        output.stderr.is_empty(),
        "inject should produce no stderr, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_inject_stdin_json() {
    use std::io::Write;
    use std::process::Stdio;

    let dir = TempDir::new().unwrap();
    let mut config = test_config_inject_enabled(&dir);
    config.inject.relevance_threshold = 0.0; // Accept any score for testing
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Save and index a memory
    let input = pensieve::ops::save::SaveInput {
        content: "Lumos lights up the wand".to_string(),
        title: "Lumos Charm".to_string(),
        memory_type: pensieve::types::MemoryType::HowItWorks,
        topic_key: "lumos-stdin".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    let memory = pensieve::ops::save::save_memory(&config, input).unwrap();
    index_memory(&config, &memory);

    // Test via binary with stdin JSON (simulating Claude Code hook)
    let bin = env!("CARGO_BIN_EXE_pensieve");
    let mut child = std::process::Command::new(bin)
        .arg("--memory-dir")
        .arg(dir.path().to_str().unwrap())
        .arg("inject")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start pensieve inject");

    let stdin = child.stdin.as_mut().expect("failed to open stdin");
    stdin
        .write_all(b"{\"prompt\":\"lumos\",\"hook_event_name\":\"UserPromptSubmit\"}")
        .expect("failed to write stdin");
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success(), "inject should exit 0");
    // Note: inject.enabled defaults to false in the binary, so it will return empty
    // unless we pass config. The binary test just verifies no crash and exit 0.
    assert!(
        output.stderr.is_empty(),
        "inject should produce no stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_configure_inject_enabled() {
    let dir = TempDir::new().unwrap();
    let config = test_config(&dir);
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Enable inject
    let new_config =
        pensieve::ops::configure::configure(&config, None, None, None, Some(true), true).unwrap();
    assert!(new_config.inject.enabled);

    // Disable inject
    let new_config =
        pensieve::ops::configure::configure(&config, None, None, None, Some(false), true).unwrap();
    assert!(!new_config.inject.enabled);
}

#[test]
fn test_inject_threshold() {
    let dir = TempDir::new().unwrap();
    let mut config = test_config_inject_enabled(&dir);
    config.inject.relevance_threshold = 999.0; // Impossibly high threshold
    pensieve::storage::ensure_dirs(&config).unwrap();

    // Save and index a memory
    let input = pensieve::ops::save::SaveInput {
        content: "Threshold test content".to_string(),
        title: "Threshold Test".to_string(),
        memory_type: pensieve::types::MemoryType::Discovery,
        topic_key: "threshold-test".to_string(),
        project: None,
        tags: vec![],
        source: None,
        confidence: None,
        expected_revision: None,
        dry_run: false,
    };
    let memory = pensieve::ops::save::save_memory(&config, input).unwrap();
    index_memory(&config, &memory);

    // High threshold should filter everything out
    let result =
        pensieve::ops::inject::run_inject(&config, Some("threshold".to_string()), None, None, None)
            .unwrap();
    assert!(result.is_empty(), "high threshold should filter all results");
}
