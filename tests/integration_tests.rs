use tempfile::TempDir;

// Helper to create a config pointing to a temp directory
fn test_config(dir: &TempDir) -> pensieve::types::PensieveConfig {
    pensieve::types::PensieveConfig {
        memory_dir: dir.path().to_path_buf(),
        retrieval: pensieve::types::RetrievalConfig::default(),
    }
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

    let list = pensieve::ops::list::list_memories(&config, None, None, None).unwrap();
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

    // Read without project should fail
    let result = pensieve::ops::read::read_memory(&config, "project-mem", None);
    assert!(result.is_err());

    // List with project filter
    let list = pensieve::ops::list::list_memories(&config, Some("myproject"), None, None).unwrap();
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

    // Create a preference
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
    assert_eq!(ctx.preferences.len(), 1);
    assert_eq!(ctx.preferences[0].title, "Indentation Preference");
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
            new_contents.push_str(&format!("updated: {old_date}"));
        } else {
            new_contents.push_str(line);
        }
        new_contents.push('\n');
    }
    std::fs::write(&path, new_contents).unwrap();

    // Get context and verify it appears in stale_memories
    let ctx = pensieve::ops::context::get_context(&config, None, None).unwrap();
    assert!(
        ctx.stale_memories.iter().any(|m| m.topic_key == "stale-test"),
        "Expected stale-test in stale_memories"
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

    // Both commands call the same ops::context::get_context function
    let ctx = pensieve::ops::context::get_context(&config, None, None).unwrap();
    assert!(ctx.preferences.is_empty());
    assert!(ctx.recent_gotchas.is_empty());
    assert!(ctx.recent_decisions.is_empty());

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
