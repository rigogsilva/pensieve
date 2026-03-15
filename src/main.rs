mod cli;

use clap::Parser;
use cli::{Cli, Command, OutputFormat};
use pensieve::{config, embedder, index, mcp, ops};

fn read_json_input(value: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if value == "-" {
        let input = std::io::read_to_string(std::io::stdin())?;
        Ok(serde_json::from_str(&input)?)
    } else if let Some(path) = value.strip_prefix('@') {
        let input = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&input)?)
    } else {
        Ok(serde_json::from_str(value)?)
    }
}

fn output_json<T: serde::Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string()));
}

fn output_result<T: serde::Serialize + std::fmt::Debug>(format: &OutputFormat, value: &T) {
    match format {
        OutputFormat::Json => output_json(value),
        OutputFormat::Human => println!("{value:#?}"),
    }
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() {
    let cli = Cli::parse();
    let cfg = config::load_config(cli.memory_dir.as_deref()).unwrap_or_default();

    match cli.command {
        Command::Save {
            title,
            content,
            r#type,
            topic_key,
            project,
            tags,
            source,
            expected_revision,
            dry_run,
            json,
        } => {
            let (title, content, r#type, topic_key, project, tags, source) = if let Some(j) = json {
                let v = read_json_input(&j).expect("invalid JSON input");
                (
                    v.get("title").and_then(serde_json::Value::as_str).map(String::from).or(title),
                    v.get("content")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from)
                        .or(content),
                    v.get("type").and_then(serde_json::Value::as_str).map(String::from).or(r#type),
                    v.get("topic_key")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from)
                        .or(topic_key),
                    v.get("project")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from)
                        .or(project),
                    v.get("tags").and_then(serde_json::Value::as_str).map(String::from).or(tags),
                    v.get("source")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from)
                        .or(source),
                )
            } else {
                (title, content, r#type, topic_key, project, tags, source)
            };

            let title = title.expect("--title is required");
            let content = content.expect("--content is required");
            let topic_key = topic_key.expect("--topic-key is required");
            let memory_type =
                r#type.as_deref().unwrap_or("discovery").parse().expect("invalid memory type");
            let tags_vec = tags
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();

            let input = ops::save::SaveInput {
                content,
                title,
                memory_type,
                topic_key,
                project,
                tags: tags_vec,
                source,
                confidence: None,
                expected_revision,
                dry_run,
            };

            match ops::save::save_memory(&cfg, input) {
                Ok(memory) => {
                    // Best-effort index upsert
                    if !dry_run {
                        if let Ok(idx) = index::Index::open(&cfg.memory_dir) {
                            let memory_id = match &memory.project {
                                Some(p) => format!("projects/{}/{}", p, memory.topic_key),
                                None => format!("global/{}", memory.topic_key),
                            };
                            let embed_text = format!("{}: {}", memory.title, memory.content);
                            let embedding = embedder::try_embed(&embed_text);
                            let _ = idx.upsert(
                                &memory_id,
                                &memory.title,
                                &memory.content,
                                memory.project.as_deref(),
                                &memory.tags,
                                embedding.as_deref(),
                            );
                        }
                    }
                    output_result(&cli.output, &memory);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Command::Read { topic_key, project } => {
            match ops::read::read_memory(&cfg, &topic_key, project.as_deref()) {
                Ok(memory) => output_result(&cli.output, &memory),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Command::Recall { query, r#type, project, tags, status, since, limit } => {
            let memory_type = r#type.map(|t| t.parse().expect("invalid memory type"));
            let status = status.map(|s| s.parse().expect("invalid status"));
            let tags = tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect());
            let since = since.map(|s| {
                chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                    .expect("invalid date, use YYYY-MM-DD")
                    .and_hms_opt(0, 0, 0)
                    .expect("invalid time")
                    .and_utc()
            });

            let input = ops::recall::RecallInput {
                query,
                memory_type,
                project,
                tags,
                status,
                since,
                limit,
            };

            match index::Index::open(&cfg.memory_dir) {
                Ok(idx) => match ops::recall::recall(&cfg, &idx, &input) {
                    Ok(results) => output_result(&cli.output, &results),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error opening index: {e}");
                    std::process::exit(1);
                }
            }
        }

        Command::List { project, r#type, status } => {
            let memory_type = r#type.map(|t| t.parse().expect("invalid memory type"));
            let status = status.map(|s| s.parse().expect("invalid status"));

            match ops::list::list_memories(
                &cfg,
                project.as_deref(),
                memory_type.as_ref(),
                status.as_ref(),
            ) {
                Ok(memories) => output_result(&cli.output, &memories),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Command::Delete { topic_key, project, dry_run } => {
            match ops::delete::delete_memory(&cfg, &topic_key, project.as_deref(), dry_run) {
                Ok(memory) => {
                    if !dry_run {
                        // Best-effort index cleanup
                        if let Ok(idx) = index::Index::open(&cfg.memory_dir) {
                            let memory_id = match &project {
                                Some(p) => format!("projects/{p}/{topic_key}"),
                                None => format!("global/{topic_key}"),
                            };
                            let _ = idx.delete(&memory_id);
                        }
                    }
                    output_result(&cli.output, &memory);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Command::Archive { topic_key, project, superseded_by, dry_run } => {
            match ops::archive::archive_memory(
                &cfg,
                &topic_key,
                project.as_deref(),
                superseded_by.as_deref(),
                dry_run,
            ) {
                Ok(memory) => output_result(&cli.output, &memory),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Command::Configure { memory_dir, keyword_weight, vector_weight } => {
            if memory_dir.is_none() && keyword_weight.is_none() && vector_weight.is_none() {
                output_result(&cli.output, &cfg);
            } else {
                match ops::configure::configure(
                    &cfg,
                    memory_dir.as_deref(),
                    keyword_weight,
                    vector_weight,
                ) {
                    Ok(new_cfg) => output_result(&cli.output, &new_cfg),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }

        Command::GetContext { project, source } => {
            match ops::context::get_context(&cfg, project.as_deref(), source.as_deref()) {
                Ok(context) => output_result(&cli.output, &context),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Command::EndSession { summary, key_decisions, source, project, json } => {
            let (summary, key_decisions, source, project) = if let Some(j) = json {
                let v = read_json_input(&j).expect("invalid JSON input");
                (
                    v.get("summary")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from)
                        .or(summary),
                    v.get("key_decisions")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from)
                        .or(key_decisions),
                    v.get("source")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from)
                        .or(source),
                    v.get("project")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from)
                        .or(project),
                )
            } else {
                (summary, key_decisions, source, project)
            };

            let summary = summary.expect("--summary is required");
            let decisions: Vec<String> = key_decisions
                .map(|d| d.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();

            match ops::end_session::end_session(
                &cfg,
                &summary,
                &decisions,
                source.as_deref().unwrap_or("unknown"),
                project.as_deref(),
            ) {
                Ok(session) => output_result(&cli.output, &session),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Command::Reindex => match index::Index::open(&cfg.memory_dir) {
            Ok(idx) => match ops::reindex::reindex(&cfg, &idx) {
                Ok(count) => println!("Reindexed {count} memories"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Error opening index: {e}");
                std::process::exit(1);
            }
        },

        Command::Schema { command } => {
            ops::schema::print_schema(command.as_deref());
        }

        Command::Serve => {
            if let Err(e) = mcp::serve(cfg).await {
                eprintln!("MCP server error: {e}");
                std::process::exit(1);
            }
        }

        Command::Version => {
            println!("pensieve {}", env!("CARGO_PKG_VERSION"));
        }

        Command::Update => match ops::update::self_update().await {
            Ok(msg) => println!("{msg}"),
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        },
    }
}
