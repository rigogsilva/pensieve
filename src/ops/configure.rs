use crate::config;
use crate::error::Result;
use crate::storage;
use crate::types::PensieveConfig;

pub fn configure(
    current_config: &PensieveConfig,
    memory_dir: Option<&str>,
    keyword_weight: Option<f64>,
    vector_weight: Option<f64>,
    inject_enabled: Option<bool>,
    dry_run: bool,
) -> Result<PensieveConfig> {
    let mut new_config = current_config.clone();

    if let Some(dir) = memory_dir {
        new_config.memory_dir = std::path::PathBuf::from(dir);
    }
    if let Some(kw) = keyword_weight {
        new_config.retrieval.keyword_weight = kw;
    }
    if let Some(vw) = vector_weight {
        new_config.retrieval.vector_weight = vw;
    }
    if let Some(ie) = inject_enabled {
        new_config.inject.enabled = ie;
    }

    if dry_run {
        return Ok(new_config);
    }

    storage::ensure_dirs(&new_config)?;
    config::save_config(&new_config)?;

    Ok(new_config)
}

pub fn get_config(current_config: &PensieveConfig) -> PensieveConfig {
    current_config.clone()
}
