use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::schemars::JsonSchema;
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use serde::{Deserialize, Serialize};

use crate::embedder;
use crate::index::Index;
use crate::ops;
use crate::types::PensieveConfig;

#[derive(Debug, Clone)]
pub struct PensieveServer {
    config: PensieveConfig,
    index_path: std::path::PathBuf,
    tool_router: ToolRouter<Self>,
}

// Tool parameter structs
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SaveMemoryParams {
    /// Memory title
    pub title: String,
    /// Memory content (markdown body)
    pub content: String,
    /// Memory type: gotcha, decision, preference, discovery, how-it-works
    #[serde(default = "default_type")]
    pub r#type: String,
    /// Topic key (filename stem, lowercase alphanumeric with hyphens)
    pub topic_key: String,
    /// Project name
    pub project: Option<String>,
    /// Tags for filtering
    #[serde(default)]
    pub tags: Vec<String>,
    /// Source agent name
    pub source: Option<String>,
    /// Expected revision for CAS conflict detection
    pub expected_revision: Option<u32>,
    /// Preview without writing
    #[serde(default)]
    pub dry_run: bool,
}

fn default_type() -> String {
    "discovery".to_string()
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RecallParams {
    /// Keyword search query
    pub query: Option<String>,
    /// Filter by memory type
    pub r#type: Option<String>,
    /// Filter by project
    pub project: Option<String>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Filter by status
    pub status: Option<String>,
    /// Only memories updated after this ISO date
    pub since: Option<String>,
    /// Max results
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadMemoryParams {
    /// Topic key
    pub topic_key: String,
    /// Project name
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DeleteMemoryParams {
    /// Topic key
    pub topic_key: String,
    /// Project name
    pub project: Option<String>,
    /// Preview without deleting
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListMemoriesParams {
    /// Filter by project
    pub project: Option<String>,
    /// Filter by type
    pub r#type: Option<String>,
    /// Filter by status
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ArchiveMemoryParams {
    /// Topic key
    pub topic_key: String,
    /// Project name
    pub project: Option<String>,
    /// Mark as superseded by this topic key
    pub superseded_by: Option<String>,
    /// Preview without archiving
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ConfigureParams {
    /// Set memory directory
    pub memory_dir: Option<String>,
    /// Set keyword retrieval weight
    pub keyword_weight: Option<f64>,
    /// Set vector retrieval weight
    pub vector_weight: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct InjectParams {
    /// Search query
    pub query: Option<String>,
    /// Filter by project
    pub project: Option<String>,
    /// Max results
    pub limit: Option<usize>,
    /// Output format: compact or json
    pub format: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetContextParams {
    /// Project name
    pub project: Option<String>,
    /// Source agent name
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct EndSessionParams {
    /// Session summary
    pub summary: String,
    /// Key decisions made
    #[serde(default)]
    pub key_decisions: Vec<String>,
    /// Source agent name
    #[serde(default = "default_source")]
    pub source: String,
    /// Project name
    pub project: Option<String>,
}

fn default_source() -> String {
    "unknown".to_string()
}

fn json_result<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "serialization error".to_string())
}

fn open_index(path: &std::path::Path) -> Option<Index> {
    Index::open(path).ok()
}

impl PensieveServer {
    pub fn new(config: PensieveConfig) -> Self {
        let index_path = config.memory_dir.clone();
        Self { config, index_path, tool_router: Self::tool_router() }
    }
}

#[tool_router]
impl PensieveServer {
    #[tool(
        description = "Save a memory as a markdown file with YAML frontmatter. If topic_key matches an existing file, updates it (increments revision)."
    )]
    async fn save_memory(&self, params: Parameters<SaveMemoryParams>) -> String {
        let params = params.0;
        let memory_type = match params.r#type.parse() {
            Ok(t) => t,
            Err(e) => return format!("Error: {e}"),
        };

        let input = ops::save::SaveInput {
            content: params.content,
            title: params.title,
            memory_type,
            topic_key: params.topic_key,
            project: params.project,
            tags: params.tags,
            source: params.source,
            confidence: None,
            expected_revision: params.expected_revision,
            dry_run: params.dry_run,
        };

        match ops::save::save_memory(&self.config, input) {
            Ok(memory) => {
                if !params.dry_run {
                    if let Some(idx) = open_index(&self.index_path) {
                        let memory_id = match &memory.project {
                            Some(p) => format!("projects/{}/{}", p, memory.topic_key),
                            None => format!("global/{}", memory.topic_key),
                        };
                        let embed_text = embedder::build_embedding_text(
                            &memory.title,
                            &memory.content,
                            &memory.memory_type,
                            memory.project.as_deref(),
                            &memory.tags,
                        );
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
                json_result(&memory)
            }
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "Search memories by keyword and/or filters. Returns compact results with title, type, project, topic_key, updated date, and first 2 lines of content."
    )]
    async fn recall(&self, params: Parameters<RecallParams>) -> String {
        let params = params.0;
        let memory_type = params.r#type.and_then(|t| t.parse().ok());
        let status = params.status.and_then(|s| s.parse().ok());
        let since = params.since.and_then(|s| {
            chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| dt.and_utc())
        });

        let input = ops::recall::RecallInput {
            query: params.query,
            memory_type,
            project: params.project,
            tags: params.tags,
            status,
            since,
            limit: params.limit,
        };

        if let Some(idx) = open_index(&self.index_path) {
            match ops::recall::recall(&self.config, &idx, &input) {
                Ok(results) => json_result(&results),
                Err(e) => format!("Error: {e}"),
            }
        } else {
            match ops::list::list_memories(
                &self.config,
                input.project.as_deref(),
                input.memory_type.as_ref(),
                input.status.as_ref(),
            ) {
                Ok(results) => json_result(&results),
                Err(e) => format!("Error: {e}"),
            }
        }
    }

    #[tool(description = "Read the full content of a specific memory by topic_key.")]
    async fn read_memory(&self, params: Parameters<ReadMemoryParams>) -> String {
        let params = params.0;
        match ops::read::read_memory(&self.config, &params.topic_key, params.project.as_deref()) {
            Ok(memory) => json_result(&memory),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Delete a memory file.")]
    async fn delete_memory(&self, params: Parameters<DeleteMemoryParams>) -> String {
        let params = params.0;
        match ops::delete::delete_memory(
            &self.config,
            &params.topic_key,
            params.project.as_deref(),
            params.dry_run,
        ) {
            Ok(memory) => {
                if !params.dry_run {
                    if let Some(idx) = open_index(&self.index_path) {
                        let memory_id = match &params.project {
                            Some(p) => format!("projects/{}/{}", p, params.topic_key),
                            None => format!("global/{}", params.topic_key),
                        };
                        let _ = idx.delete(&memory_id);
                    }
                }
                json_result(&memory)
            }
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "List all memories with title, type, project, topic_key, and updated date. No content."
    )]
    async fn list_memories(&self, params: Parameters<ListMemoriesParams>) -> String {
        let params = params.0;
        let memory_type = params.r#type.and_then(|t| t.parse().ok());
        let status = params.status.and_then(|s| s.parse().ok());

        match ops::list::list_memories(
            &self.config,
            params.project.as_deref(),
            memory_type.as_ref(),
            status.as_ref(),
        ) {
            Ok(results) => json_result(&results),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Archive a memory. With superseded_by, marks as superseded.")]
    async fn archive_memory(&self, params: Parameters<ArchiveMemoryParams>) -> String {
        let params = params.0;
        match ops::archive::archive_memory(
            &self.config,
            &params.topic_key,
            params.project.as_deref(),
            params.superseded_by.as_deref(),
            params.dry_run,
        ) {
            Ok(memory) => json_result(&memory),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "Auto-inject relevant memories for a prompt. Returns compact results above relevance threshold. Designed for hook integration."
    )]
    async fn inject(&self, params: Parameters<InjectParams>) -> String {
        let params = params.0;
        match ops::inject::run_inject(
            &self.config,
            params.query,
            params.project,
            params.limit,
            params.format.as_deref(),
        ) {
            Ok(result) => result,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "View or update configuration. Without args, returns current config.")]
    async fn configure(&self, params: Parameters<ConfigureParams>) -> String {
        let params = params.0;
        if params.memory_dir.is_none()
            && params.keyword_weight.is_none()
            && params.vector_weight.is_none()
        {
            json_result(&self.config)
        } else {
            match ops::configure::configure(
                &self.config,
                params.memory_dir.as_deref(),
                params.keyword_weight,
                params.vector_weight,
                None,
                false,
            ) {
                Ok(new_config) => json_result(&new_config),
                Err(e) => format!("Error: {e}"),
            }
        }
    }

    #[tool(
        description = "SESSION START - call this first. Returns recent sessions, preferences, gotchas, and decisions to bootstrap your context."
    )]
    async fn get_context(&self, params: Parameters<GetContextParams>) -> String {
        let params = params.0;
        match ops::context::get_context(
            &self.config,
            params.project.as_deref(),
            params.source.as_deref(),
        ) {
            Ok(context) => json_result(&context),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "SESSION END - call before closing. Saves a session summary to sessions directory."
    )]
    async fn end_session(&self, params: Parameters<EndSessionParams>) -> String {
        let params = params.0;
        match ops::end_session::end_session(
            &self.config,
            &params.summary,
            &params.key_decisions,
            &params.source,
            params.project.as_deref(),
            false,
        ) {
            Ok(session) => json_result(&session),
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool_handler]
impl ServerHandler for PensieveServer {}

pub async fn serve(config: PensieveConfig) -> Result<(), Box<dyn std::error::Error>> {
    crate::storage::ensure_dirs(&config)?;

    let server = PensieveServer::new(config);

    let transport = rmcp::transport::io::stdio();
    let ct = server.serve(transport).await?;
    ct.waiting().await?;

    Ok(())
}
