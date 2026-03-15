use serde_json::json;

#[allow(clippy::too_many_lines)]
pub fn print_schema(command: Option<&str>) {
    let schemas = serde_json::json!({
        "save": {
            "name": "save",
            "description": "Save a memory as a markdown file with YAML frontmatter",
            "parameters": {
                "title": {"type": "string", "required": true, "description": "Memory title"},
                "content": {"type": "string", "required": true, "description": "Memory content (markdown body)"},
                "type": {"type": "string", "required": false, "default": "discovery", "description": "Memory type: gotcha, decision, preference, discovery, how-it-works"},
                "topic_key": {"type": "string", "required": true, "description": "Filename stem, lowercase alphanumeric with hyphens"},
                "project": {"type": "string", "required": false, "description": "Project name (stored under projects/{project}/)"},
                "tags": {"type": "array", "required": false, "description": "Tags for filtering"},
                "source": {"type": "string", "required": false, "description": "Which agent saved this"},
                "expected_revision": {"type": "integer", "required": false, "description": "CAS: expected revision number"},
                "dry_run": {"type": "boolean", "required": false, "default": false, "description": "Preview without writing"}
            }
        },
        "recall": {
            "name": "recall",
            "description": "Search memories by keyword and/or filters",
            "parameters": {
                "query": {"type": "string", "required": false, "description": "Keyword search query"},
                "type": {"type": "string", "required": false, "description": "Filter by memory type"},
                "project": {"type": "string", "required": false, "description": "Filter by project"},
                "tags": {"type": "array", "required": false, "description": "Filter by tags"},
                "status": {"type": "string", "required": false, "description": "Filter by status"},
                "since": {"type": "string", "required": false, "description": "ISO date, only memories after"},
                "limit": {"type": "integer", "required": false, "default": 20, "description": "Max results"}
            }
        },
        "read": {
            "name": "read",
            "description": "Read a memory by topic key",
            "parameters": {
                "topic_key": {"type": "string", "required": true, "description": "Topic key"},
                "project": {"type": "string", "required": false, "description": "Project name"}
            }
        },
        "delete": {
            "name": "delete",
            "description": "Delete a memory",
            "parameters": {
                "topic_key": {"type": "string", "required": true},
                "project": {"type": "string", "required": false},
                "dry_run": {"type": "boolean", "required": false, "default": false}
            }
        },
        "list": {
            "name": "list",
            "description": "List all memories",
            "parameters": {
                "project": {"type": "string", "required": false},
                "type": {"type": "string", "required": false},
                "status": {"type": "string", "required": false}
            }
        },
        "archive": {
            "name": "archive",
            "description": "Archive a memory",
            "parameters": {
                "topic_key": {"type": "string", "required": true},
                "project": {"type": "string", "required": false},
                "superseded_by": {"type": "string", "required": false},
                "dry_run": {"type": "boolean", "required": false, "default": false}
            }
        },
        "configure": {
            "name": "configure",
            "description": "View or update configuration",
            "parameters": {
                "memory_dir": {"type": "string", "required": false},
                "keyword_weight": {"type": "number", "required": false},
                "vector_weight": {"type": "number", "required": false}
            }
        },
        "get-context": {
            "name": "get-context",
            "description": "Get context for session start",
            "parameters": {
                "project": {"type": "string", "required": false},
                "source": {"type": "string", "required": false}
            }
        },
        "end-session": {
            "name": "end-session",
            "description": "End session with summary",
            "parameters": {
                "summary": {"type": "string", "required": true},
                "key_decisions": {"type": "array", "required": false},
                "source": {"type": "string", "required": false},
                "project": {"type": "string", "required": false}
            }
        }
    });

    match command {
        Some(cmd) => {
            if let Some(schema) = schemas.get(cmd) {
                println!("{}", serde_json::to_string_pretty(schema).unwrap_or_default());
            } else {
                let available: Vec<&str> = schemas
                    .as_object()
                    .map(|o| o.keys().map(String::as_str).collect())
                    .unwrap_or_default();
                eprintln!("Unknown command: {cmd}. Available: {}", available.join(", "));
            }
        }
        None => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"commands": schemas})).unwrap_or_default()
            );
        }
    }
}
