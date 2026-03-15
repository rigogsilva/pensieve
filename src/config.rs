use crate::error::{PensieveError, Result};
use crate::types::PensieveConfig;
use std::path::PathBuf;

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("pensieve")
        .join("config.toml")
}

pub fn load_config(cli_memory_dir: Option<&str>) -> Result<PensieveConfig> {
    let mut config = if config_path().exists() {
        let contents = std::fs::read_to_string(config_path())?;
        toml::from_str::<PensieveConfig>(&contents)
            .map_err(|e| PensieveError::Config(format!("failed to parse config: {e}")))?
    } else {
        PensieveConfig::default()
    };

    // Env var overrides config file
    if let Ok(dir) = std::env::var("PENSIEVE_MEMORY_DIR") {
        config.memory_dir = PathBuf::from(dir);
    }

    // CLI flag overrides everything
    if let Some(dir) = cli_memory_dir {
        config.memory_dir = PathBuf::from(dir);
    }

    Ok(config)
}

pub fn save_config(config: &PensieveConfig) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(config)
        .map_err(|e| PensieveError::Config(format!("failed to serialize config: {e}")))?;
    std::fs::write(path, contents)?;
    Ok(())
}

pub fn is_unconfigured() -> bool {
    !config_path().exists()
}
