use chrono::Utc;

use crate::error::Result;
use crate::storage;
use crate::types::{PensieveConfig, SessionSummary};

pub fn end_session(
    config: &PensieveConfig,
    summary: &str,
    key_decisions: &[String],
    source: &str,
    project: Option<&str>,
    dry_run: bool,
) -> Result<SessionSummary> {
    storage::ensure_dirs(config)?;

    let now = Utc::now();
    let project_str = project.unwrap_or("unknown");

    let session = SessionSummary {
        summary: summary.to_string(),
        key_decisions: key_decisions.to_vec(),
        source: source.to_string(),
        project: project.map(String::from),
        created: now,
    };

    if dry_run {
        return Ok(session);
    }

    let filename = format!(
        "{}T{}-{}-{}.md",
        now.format("%Y-%m-%d"),
        now.format("%H%M%S"),
        project_str,
        source
    );

    let frontmatter = format!(
        "---\ntitle: Session {date} {project} ({source})\nsource: {source}\nproject: {project}\ncreated: {created}\nkey_decisions:\n{decisions}---\n\n{summary}\n",
        date = now.format("%Y-%m-%d"),
        project = project_str,
        source = source,
        created = now.to_rfc3339(),
        decisions = if key_decisions.is_empty() {
            "  []\n".to_string()
        } else {
            key_decisions.iter().fold(String::new(), |mut acc, d| {
                use std::fmt::Write;
                let _ = writeln!(acc, "  - {d}");
                acc
            })
        },
        summary = summary,
    );

    let path = storage::resolve_session_path(config, &filename);
    std::fs::write(path, frontmatter)?;

    Ok(session)
}
