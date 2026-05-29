//! Persistent CLI configuration stored in `~/.config/diff-tool/config.toml`.
//!
//! ## Schema
//!
//! ```toml
//! [defaults]
//! algorithm = "myers"       # "myers", "histogram", or "patience"
//! deadline = 5000           # optional, milliseconds
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level CLI configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliConfig {
    #[serde(default)]
    pub defaults: DefaultsConfig,
}

/// Default values merged under CLI flags.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultsConfig {
    /// Diff algorithm: "myers", "histogram", or "patience".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,

    /// Wall-clock deadline in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<u64>,
}

/// Resolve the config file path: `$XDG_CONFIG_HOME/diff-tool/config.toml`
/// or `~/.config/diff-tool/config.toml` if `XDG_CONFIG_HOME` is unset.
pub fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| String::from("."));
            PathBuf::from(home).join(".config")
        });
    base.join("diff-tool").join("config.toml")
}

/// Load the config from disk, returning defaults if the file is missing or
/// unparseable.
pub fn load() -> CliConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => CliConfig::default(),
    }
}

/// Write the config to disk, creating parent directories as needed.
pub fn save(config: &CliConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create config directory: {e}"))?;
    }
    let contents =
        toml::to_string_pretty(config).map_err(|e| format!("failed to serialize config: {e}"))?;
    std::fs::write(&path, contents).map_err(|e| format!("failed to write config: {e}"))?;
    Ok(())
}

/// Helper to parse an algorithm string from config into the clap enum.
/// Returns `None` for unknown / missing values so the caller can use its
/// own default.
pub fn parse_algorithm(s: &str) -> Option<crate::AlgorithmArg> {
    match s {
        "myers" => Some(crate::AlgorithmArg::Myers),
        "histogram" => Some(crate::AlgorithmArg::Histogram),
        "patience" => Some(crate::AlgorithmArg::Patience),
        _ => None,
    }
}

/// Remove the config file if it exists (used by `dt config reset`).
pub fn delete_config_file() -> Result<(), String> {
    let path = config_path();
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("failed to remove config file: {e}")),
    }
}
