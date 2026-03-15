use sha2::{Digest, Sha256};

use crate::error::{PensieveError, Result};

#[allow(clippy::too_many_lines)]
pub async fn self_update() -> Result<String> {
    let current_version = env!("CARGO_PKG_VERSION");

    // Check latest release with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| PensieveError::Config(format!("http client error: {e}")))?;

    let response = client
        .get("https://api.github.com/repos/rigogsilva/pensieve/releases/latest")
        .header("User-Agent", "pensieve")
        .send()
        .await
        .map_err(|e| PensieveError::Config(format!("failed to check for updates: {e}")))?;

    if !response.status().is_success() {
        return Err(PensieveError::Config(format!("GitHub API returned {}", response.status())));
    }

    let release: serde_json::Value = response
        .json()
        .await
        .map_err(|e| PensieveError::Config(format!("failed to parse release: {e}")))?;

    let tag = release
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| PensieveError::Config("no tag_name in release".to_string()))?;

    let latest_version = tag.strip_prefix('v').unwrap_or(tag);

    if latest_version == current_version {
        return Ok(format!("Already up to date (v{current_version})"));
    }

    // Find platform-appropriate asset
    let platform = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") { "aarch64-apple-darwin" } else { "x86_64-apple-darwin" }
    } else if cfg!(target_os = "linux") {
        "x86_64-unknown-linux-gnu"
    } else {
        return Err(PensieveError::Config("unsupported platform".to_string()));
    };

    let asset_name = format!("pensieve-{platform}");
    let assets = release
        .get("assets")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| PensieveError::Config("no assets in release".to_string()))?;

    let asset_url = assets
        .iter()
        .find(|a| {
            a.get("name")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|n| n.contains(&asset_name))
        })
        .and_then(|a| a.get("browser_download_url"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| PensieveError::Config(format!("no asset found for platform: {platform}")))?;

    // Download checksums
    let checksums_url = assets
        .iter()
        .find(|a| {
            a.get("name")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|n| n.contains("checksums"))
        })
        .and_then(|a| a.get("browser_download_url"))
        .and_then(serde_json::Value::as_str);

    // Download binary
    eprintln!("Downloading pensieve {latest_version}...");
    let binary = client
        .get(asset_url)
        .send()
        .await
        .map_err(|e| PensieveError::Config(format!("download failed: {e}")))?
        .bytes()
        .await
        .map_err(|e| PensieveError::Config(format!("download failed: {e}")))?;

    // Verify checksum if available
    if let Some(checksums_url) = checksums_url {
        let checksums_text = client
            .get(checksums_url)
            .send()
            .await
            .map_err(|e| PensieveError::Config(format!("checksum download failed: {e}")))?
            .text()
            .await
            .map_err(|e| PensieveError::Config(format!("checksum download failed: {e}")))?;

        let mut hasher = Sha256::new();
        hasher.update(&binary);
        let hash = format!("{:x}", hasher.finalize());

        let expected = checksums_text
            .lines()
            .find(|line| line.contains(&asset_name))
            .and_then(|line| line.split_whitespace().next());

        if let Some(expected_hash) = expected {
            if hash != expected_hash {
                return Err(PensieveError::Config(format!(
                    "checksum mismatch: expected {expected_hash}, got {hash}"
                )));
            }
        }
    }

    // Replace current binary
    let current_exe = std::env::current_exe()
        .map_err(|e| PensieveError::Config(format!("cannot find current exe: {e}")))?;

    let temp_path = current_exe.with_extension("new");
    std::fs::write(&temp_path, &binary)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o755))?;
    }

    // On macOS/Unix, can't rename over a running binary — remove first, then move
    let _ = std::fs::remove_file(&current_exe);
    std::fs::rename(&temp_path, &current_exe)?;

    Ok(format!("Updated from v{current_version} to v{latest_version}"))
}
