//! Application configuration for the `agbr` control plane.
//!
//! Cross-platform: config is resolved via `dirs` (XDG on Unix, `%APPDATA%`
//! on Windows), and the workspace/export defaults live under the user's home
//! directory.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use agbr_llm::LlmConfig;

/// Top-level configuration, loaded from the platform config dir
/// (`~/.config/agbr/config.toml` on macOS/Linux, `%APPDATA%\agbr\config.toml`
/// on Windows) or `$AGBR_CONFIG`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub llm: LlmConfig,

    #[serde(default)]
    pub rawtherapee: RawTherapeeConfig,

    #[serde(default)]
    pub workspace: WorkspaceConfig,

    #[serde(default)]
    pub policy: PolicyConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawTherapeeConfig {
    /// Optional explicit path to `rawtherapee-cli`; auto-discovered if unset.
    #[serde(default)]
    pub binary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceConfig {
    /// Workspace root (recipes, profiles, previews, logs). Defaults to `~/agbr`.
    #[serde(default)]
    pub root: Option<String>,

    /// Where rendered exports are written. Defaults to `<workspace>/exports`.
    #[serde(default)]
    pub export_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyConfig {
    /// Absolute directory allowlist for read/write. Empty means "no extra
    /// restriction" beyond normal filesystem permissions.
    #[serde(default)]
    pub allow: Vec<String>,
}

impl AppConfig {
    /// Load configuration, merging a user file (if present) over defaults.
    pub fn load() -> Result<Self, ConfigError> {
        let path = config_path()?;
        let file_contents = match path.and_then(|p| std::fs::read_to_string(p).ok()) {
            Some(text) => text,
            None => return Ok(Self::default()),
        };

        let mut config: AppConfig = toml::from_str(&file_contents).map_err(ConfigError::Parse)?;
        config.normalize();
        Ok(config)
    }

    /// Apply path normalization (expand a leading `~`).
    fn normalize(&mut self) {
        let expand = |s: &str| -> String {
            if s == "~" {
                home_dir()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|| s.to_string())
            } else if let Some(rest) = s.strip_prefix("~/") {
                home_dir()
                    .map(|h| h.join(rest).to_string_lossy().to_string())
                    .unwrap_or_else(|| s.to_string())
            } else {
                s.to_string()
            }
        };

        if let Some(root) = &self.workspace.root {
            self.workspace.root = Some(expand(root));
        }
        if let Some(export_dir) = &self.workspace.export_dir {
            self.workspace.export_dir = Some(expand(export_dir));
        }
    }

    /// Resolve the workspace root directory (default `~/agbr`).
    pub fn workspace_root(&self) -> PathBuf {
        if let Some(root) = &self.workspace.root {
            PathBuf::from(root)
        } else {
            home_dir()
                .map(|h| h.join("agbr"))
                .unwrap_or_else(|| PathBuf::from("agbr"))
        }
    }

    /// Resolve the export directory (default: `<workspace>/exports`).
    pub fn export_dir(&self) -> PathBuf {
        if let Some(dir) = &self.workspace.export_dir {
            if !dir.trim().is_empty() {
                return PathBuf::from(dir);
            }
        }
        self.workspace_root().join("exports")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to parse config: {0}")]
    Parse(toml::de::Error),
}

fn config_path() -> Result<Option<PathBuf>, ConfigError> {
    if let Ok(p) = std::env::var("AGBR_CONFIG") {
        if !p.trim().is_empty() {
            return Ok(Some(PathBuf::from(p)));
        }
    }
    Ok(config_dir().map(|d| d.join("agbr").join("config.toml")))
}

/// The platform config directory: `%APPDATA%` on Windows, `$XDG_CONFIG_HOME`
/// (or `~/.config`) on macOS and Linux.
fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(dirs::config_dir)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
    }
}

fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Serialize the default config to TOML (used by `agbr config`).
pub fn default_config_toml() -> String {
    let config = AppConfig::default();
    toml::to_string_pretty(&config).unwrap_or_default()
}
