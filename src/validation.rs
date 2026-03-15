use crate::error::{PensieveError, Result};

fn is_valid_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !s.starts_with('-')
        && !s.ends_with('-')
}

fn contains_path_traversal(s: &str) -> bool {
    s.contains("..") || s.contains('/') || s.contains('\\')
}

pub fn validate_topic_key(s: &str) -> Result<()> {
    if contains_path_traversal(s) {
        return Err(PensieveError::InvalidInput(format!("topic_key contains path traversal: {s}")));
    }
    if !is_valid_slug(s) {
        return Err(PensieveError::InvalidInput(format!(
            "topic_key must be lowercase alphanumeric with hyphens, got: {s}"
        )));
    }
    Ok(())
}

pub fn validate_project_name(s: &str) -> Result<()> {
    if contains_path_traversal(s) {
        return Err(PensieveError::InvalidInput(format!(
            "project name contains path traversal: {s}"
        )));
    }
    if !is_valid_slug(s) {
        return Err(PensieveError::InvalidInput(format!(
            "project name must be lowercase alphanumeric with hyphens, got: {s}"
        )));
    }
    Ok(())
}
