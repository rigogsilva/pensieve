use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryType {
    Gotcha,
    Decision,
    Preference,
    Discovery,
    HowItWorks,
}

impl fmt::Display for MemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gotcha => write!(f, "gotcha"),
            Self::Decision => write!(f, "decision"),
            Self::Preference => write!(f, "preference"),
            Self::Discovery => write!(f, "discovery"),
            Self::HowItWorks => write!(f, "how-it-works"),
        }
    }
}

impl std::str::FromStr for MemoryType {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "gotcha" => Ok(Self::Gotcha),
            "decision" => Ok(Self::Decision),
            "preference" => Ok(Self::Preference),
            "discovery" => Ok(Self::Discovery),
            "how-it-works" => Ok(Self::HowItWorks),
            _ => Err(format!("unknown memory type: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryStatus {
    Active,
    Archived,
    Superseded,
}

impl fmt::Display for MemoryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Archived => write!(f, "archived"),
            Self::Superseded => write!(f, "superseded"),
        }
    }
}

impl std::str::FromStr for MemoryStatus {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            "superseded" => Ok(Self::Superseded),
            _ => Err(format!("unknown memory status: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct Memory {
    pub title: String,
    #[serde(rename = "type")]
    pub memory_type: MemoryType,
    pub topic_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default = "default_scope")]
    pub scope: String,
    pub status: MemoryStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    pub revision: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    #[serde(skip)]
    pub content: String,
}

fn default_scope() -> String {
    "global".to_string()
}

/// Wrapper that includes content in serialized output (for `read` command).
#[derive(Debug, Serialize)]
pub struct MemoryWithContent {
    pub title: String,
    #[serde(rename = "type")]
    pub memory_type: MemoryType,
    pub topic_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub scope: String,
    pub status: MemoryStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    pub revision: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub content: String,
}

impl From<Memory> for MemoryWithContent {
    fn from(m: Memory) -> Self {
        Self {
            title: m.title,
            memory_type: m.memory_type,
            topic_key: m.topic_key,
            project: m.project,
            scope: m.scope,
            status: m.status,
            confidence: m.confidence,
            revision: m.revision,
            tags: m.tags,
            source: m.source,
            superseded_by: m.superseded_by,
            created: m.created,
            updated: m.updated,
            content: m.content,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct MemoryCompact {
    pub title: String,
    #[serde(rename = "type")]
    pub memory_type: MemoryType,
    pub topic_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub status: MemoryStatus,
    pub updated: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    pub preview: String,
}

impl From<&Memory> for MemoryCompact {
    fn from(m: &Memory) -> Self {
        let preview = m.content.lines().take(2).collect::<Vec<_>>().join("\n");
        Self {
            title: m.title.clone(),
            memory_type: m.memory_type.clone(),
            topic_key: m.topic_key.clone(),
            project: m.project.clone(),
            status: m.status.clone(),
            updated: m.updated,
            score: None,
            preview,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub summary: String,
    #[serde(default)]
    pub key_decisions: Vec<String>,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PensieveConfig {
    #[serde(default = "default_memory_dir")]
    pub memory_dir: PathBuf,
    #[serde(default)]
    pub retrieval: RetrievalConfig,
    #[serde(default)]
    pub inject: InjectConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_relevance_threshold")]
    pub relevance_threshold: f64,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_inject_format")]
    pub format: String,
}

impl Default for InjectConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            relevance_threshold: default_relevance_threshold(),
            max_results: default_max_results(),
            format: default_inject_format(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    #[serde(default = "default_keyword_weight")]
    pub keyword_weight: f64,
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f64,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self { keyword_weight: default_keyword_weight(), vector_weight: default_vector_weight() }
    }
}

impl Default for PensieveConfig {
    fn default() -> Self {
        Self {
            memory_dir: default_memory_dir(),
            retrieval: RetrievalConfig::default(),
            inject: InjectConfig::default(),
        }
    }
}

fn default_memory_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".pensieve").join("memory")
}

fn default_keyword_weight() -> f64 {
    0.7
}

fn default_vector_weight() -> f64 {
    0.3
}

fn default_relevance_threshold() -> f64 {
    0.3
}

fn default_max_results() -> usize {
    3
}

fn default_inject_format() -> String {
    "compact".to_string()
}
