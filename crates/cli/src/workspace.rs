//! Workspace layout for reproducible execution (design section 16).

use std::path::{Path, PathBuf};

/// A workspace holds generated artifacts (recipes, profiles, previews, logs).
/// Rendered exports go to a separate `export_dir` (default `<root>/exports`).
/// The original source RAW is never written here.
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
    export_dir: PathBuf,
}

impl Workspace {
    pub fn new(root: impl Into<PathBuf>, export_dir: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            export_dir: export_dir.into(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where rendered exports are written.
    pub fn export_dir(&self) -> &Path {
        &self.export_dir
    }

    /// Ensure the workspace directory tree exists.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        for sub in ["recipes", "profiles", "previews", "logs"] {
            std::fs::create_dir_all(self.root.join(sub))?;
        }
        std::fs::create_dir_all(&self.export_dir)?;
        Ok(())
    }

    pub fn recipes_dir(&self) -> PathBuf {
        self.root.join("recipes")
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.root.join("profiles")
    }

    pub fn previews_dir(&self) -> PathBuf {
        self.root.join("previews")
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("manifest.json")
    }

    pub fn log_path(&self) -> PathBuf {
        self.root.join("logs").join("execution.jsonl")
    }

    /// Append a single JSON-line execution record.
    pub fn append_log(&self, entry: &serde_json::Value) -> std::io::Result<()> {
        use std::io::Write;
        self.ensure_dirs()?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path())?;
        writeln!(file, "{}", entry)?;
        Ok(())
    }

    /// Derive a stable stem from a source path (basename without extension).
    pub fn stem_for(source: &Path) -> String {
        source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "image".to_string())
    }
}
