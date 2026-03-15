use thiserror::Error;

#[derive(Error, Debug)]
pub enum PensieveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Revision conflict: expected {expected}, found {actual}")]
    RevisionConflict { expected: u32, actual: u32 },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Embedding error: {0}")]
    EmbeddingError(String),
}

pub type Result<T> = std::result::Result<T, PensieveError>;
