use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "pensieve", about = "A shared memory system for AI agents")]
#[command(version)]
pub struct Cli {
    /// Output format
    #[arg(long, default_value = "human")]
    pub output: OutputFormat,

    /// Override memory directory
    #[arg(long)]
    pub memory_dir: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Subcommand)]
pub enum Command {
    /// Save a memory
    Save {
        /// Memory title
        #[arg(long)]
        title: Option<String>,

        /// Memory content
        #[arg(long)]
        content: Option<String>,

        /// Memory type
        #[arg(long, rename_all = "kebab-case")]
        r#type: Option<String>,

        /// Topic key (filename stem)
        #[arg(long)]
        topic_key: Option<String>,

        /// Project name
        #[arg(long)]
        project: Option<String>,

        /// Tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,

        /// Source agent
        #[arg(long)]
        source: Option<String>,

        /// Expected revision for CAS
        #[arg(long)]
        expected_revision: Option<u32>,

        /// Dry run
        #[arg(long)]
        dry_run: bool,

        /// JSON input (inline, @file, or - for stdin)
        #[arg(long)]
        json: Option<String>,
    },
    /// Read a memory by topic key
    Read {
        /// Topic key
        topic_key: String,

        /// Project name
        #[arg(long)]
        project: Option<String>,
    },
    /// Search memories
    Recall {
        /// Search query
        query: Option<String>,

        /// Filter by type
        #[arg(long, rename_all = "kebab-case")]
        r#type: Option<String>,

        /// Filter by project
        #[arg(long)]
        project: Option<String>,

        /// Filter by tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Only memories updated after this date
        #[arg(long)]
        since: Option<String>,

        /// Max results
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// List all memories
    List {
        /// Filter by project
        #[arg(long)]
        project: Option<String>,

        /// Filter by type
        #[arg(long, rename_all = "kebab-case")]
        r#type: Option<String>,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },
    /// Delete a memory
    Delete {
        /// Topic key
        topic_key: String,

        /// Project name
        #[arg(long)]
        project: Option<String>,

        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Archive a memory
    Archive {
        /// Topic key
        topic_key: String,

        /// Project name
        #[arg(long)]
        project: Option<String>,

        /// Mark as superseded by this topic key
        #[arg(long)]
        superseded_by: Option<String>,

        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Configure pensieve
    Configure {
        /// Set memory directory
        #[arg(long)]
        memory_dir: Option<String>,

        /// Set keyword weight
        #[arg(long)]
        keyword_weight: Option<f64>,

        /// Set vector weight
        #[arg(long)]
        vector_weight: Option<f64>,
    },
    /// Get context for session start
    GetContext {
        /// Project name
        #[arg(long)]
        project: Option<String>,

        /// Source agent
        #[arg(long)]
        source: Option<String>,
    },
    /// End session with summary
    EndSession {
        /// Session summary
        #[arg(long)]
        summary: Option<String>,

        /// Key decisions (comma-separated)
        #[arg(long)]
        key_decisions: Option<String>,

        /// Source agent
        #[arg(long)]
        source: Option<String>,

        /// Project name
        #[arg(long)]
        project: Option<String>,

        /// JSON input
        #[arg(long)]
        json: Option<String>,
    },
    /// Rebuild the search index
    Reindex,
    /// Show schema for a command
    Schema {
        /// Command name
        command: Option<String>,
    },
    /// Start MCP server
    Serve,
    /// Print version
    Version,
    /// Self-update from GitHub releases
    Update,
}
