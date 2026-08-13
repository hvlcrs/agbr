//! `rawtherapee-cli` discovery and command construction.
//!
//! The control plane constructs the process invocation from validated objects
//! (design 17.1); the LLM/agent never writes shell commands.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Candidate locations, in priority order.
const ENV_OVERRIDE: &str = "RAWTHERAPEE_CLI";

/// Output format for rendering/export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Jpeg { quality: u8 },
    Tiff,
    Png,
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Jpeg { .. } => "jpg",
            OutputFormat::Tiff => "tif",
            OutputFormat::Png => "png",
        }
    }
}

/// A located `rawtherapee-cli` binary plus its detected version.
#[derive(Debug, Clone)]
pub struct Cli {
    pub binary: PathBuf,
    pub version: Option<String>,
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("rawtherapee-cli not found; install RawTherapee or set RAWTHERAPEE_CLI")]
    NotFound,

    #[error("failed to run rawtherapee-cli: {0}")]
    Spawn(String),
}

impl Cli {
    /// Locate `rawtherapee-cli` on the system and detect its version.
    pub fn discover() -> Result<Self, CliError> {
        let binary = discover_binary()?;
        let version = detect_version(&binary);
        Ok(Self { binary, version })
    }

    pub fn with_binary(binary: PathBuf) -> Self {
        let version = detect_version(&binary);
        Self { binary, version }
    }
}

/// Locate the rawtherapee-cli binary, honoring `RAWTHERAPEE_CLI` first.
///
/// Cross-platform: checks the environment override, then the executable name
/// on `PATH`, then platform-specific install locations.
pub fn discover_binary() -> Result<PathBuf, CliError> {
    if let Ok(env) = std::env::var(ENV_OVERRIDE) {
        if !env.trim().is_empty() && Path::new(env.trim()).exists() {
            return Ok(PathBuf::from(env.trim()));
        }
    }

    // Generic PATH lookup (rawtherapee-cli on Unix, rawtherapee-cli.exe on
    // Windows, with or without the `.exe` suffix).
    for name in path_names() {
        if let Some(found) = which(name) {
            return Ok(found);
        }
    }

    // Platform-specific absolute fallbacks.
    for candidate in platform_candidates() {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(CliError::NotFound)
}

/// Executable names to probe on `PATH`, in order.
fn path_names() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["rawtherapee-cli.exe", "rawtherapee-cli"]
    } else {
        &["rawtherapee-cli"]
    }
}

/// Platform-specific absolute install locations.
fn platform_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if cfg!(target_os = "macos") {
        candidates.push(PathBuf::from("/opt/homebrew/bin/rawtherapee-cli"));
        candidates.push(PathBuf::from("/usr/local/bin/rawtherapee-cli"));
        candidates.push(PathBuf::from(
            "/Applications/RawTherapee.app/Contents/MacOS/rawtherapee-cli",
        ));
    }

    if cfg!(target_os = "windows") {
        for var in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
            if let Some(base) = std::env::var_os(var).map(PathBuf::from) {
                // RawTherapee installs as <base>\RawTherapee\<version>\rawtherapee-cli.exe.
                let rt_dir = base.join("RawTherapee");
                if let Ok(entries) = std::fs::read_dir(&rt_dir) {
                    for entry in entries.flatten() {
                        candidates.push(entry.path().join("rawtherapee-cli.exe"));
                    }
                }
                candidates.push(rt_dir.join("rawtherapee-cli.exe"));
            }
        }
    }

    if !cfg!(any(target_os = "macos", target_os = "windows")) {
        candidates.push(PathBuf::from("/usr/bin/rawtherapee-cli"));
        candidates.push(PathBuf::from("/usr/local/bin/rawtherapee-cli"));
        candidates.push(PathBuf::from("/snap/bin/rawtherapee-cli"));
        candidates.push(PathBuf::from("/var/lib/flatpak/exports/bin/rawtherapee"));
    }

    candidates
}

/// Minimal `which` returning the full resolved path.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Run the binary with no arguments and parse `version X.Y` from output.
fn detect_version(binary: &Path) -> Option<String> {
    let output = std::process::Command::new(binary).output().ok()?;
    let text = String::from_utf8_lossy(&output.stderr).to_string()
        + String::from_utf8_lossy(&output.stdout).as_ref();
    parse_version(&text)
}

fn parse_version(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(idx) = line.find("version ") {
            let rest = &line[idx + "version ".len()..];
            let version: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if !version.is_empty() {
                return Some(version);
            }
        }
    }
    None
}

/// A fully-constructed process invocation.
#[derive(Debug, Clone)]
pub struct Command {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

impl Command {
    /// Render as a single shell-quoted string (for logs/provenance only).
    pub fn display(&self) -> String {
        let mut parts = vec![quote(&self.program.to_string_lossy())];
        for a in &self.args {
            parts.push(quote(&a.to_string_lossy()));
        }
        parts.join(" ")
    }

    pub fn argv(&self) -> Vec<String> {
        let mut out = vec![self.program.to_string_lossy().to_string()];
        out.extend(self.args.iter().map(|a| a.to_string_lossy().to_string()));
        out
    }
}

fn quote(s: &str) -> String {
    if s.contains(char::is_whitespace) {
        format!("'{s}'")
    } else {
        s.to_string()
    }
}

/// Build a `rawtherapee-cli` invocation.
///
/// Profiles are applied in the order given (later overrides earlier), matching
/// RawTherapee's ordered `-p` behavior (design 10.4).
pub fn build_command(
    cli: &Cli,
    input: &Path,
    output: &Path,
    profiles: &[PathBuf],
    format: OutputFormat,
    overwrite: bool,
) -> Command {
    let mut args: Vec<OsString> = Vec::new();

    args.push("-q".into());

    args.push("-o".into());
    args.push(output.as_os_str().to_owned());

    for profile in profiles {
        args.push("-p".into());
        args.push(profile.as_os_str().to_owned());
    }

    match format {
        OutputFormat::Jpeg { quality } => {
            args.push(format!("-j{}", quality.clamp(1, 100)).into());
        }
        OutputFormat::Tiff => args.push("-t".into()),
        OutputFormat::Png => args.push("-n".into()),
    }

    if overwrite {
        args.push("-Y".into());
    }

    args.push("-c".into());
    args.push(input.as_os_str().to_owned());

    Command {
        program: cli.binary.clone(),
        args,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_version_string() {
        assert_eq!(
            parse_version("RawTherapee, version 5.13, command line."),
            Some("5.13".to_string())
        );
        assert_eq!(parse_version("nothing here"), None);
    }

    #[test]
    fn command_orders_c_last_and_layers_profiles() {
        let cli = Cli {
            binary: PathBuf::from("/usr/bin/rawtherapee-cli"),
            version: Some("5.13".into()),
        };
        let cmd = build_command(
            &cli,
            Path::new("in.ARW"),
            Path::new("/tmp/out.jpg"),
            &[PathBuf::from("base.pp3"), PathBuf::from("look.pp3")],
            OutputFormat::Jpeg { quality: 95 },
            true,
        );
        let argv = cmd.argv();
        assert_eq!(argv[0], "/usr/bin/rawtherapee-cli");
        assert_eq!(argv.last().map(String::as_str), Some("in.ARW"));
        assert!(argv.windows(2).any(|w| w == ["-c", "in.ARW"]));
        assert!(argv.contains(&"-j95".to_string()));
        assert!(argv.contains(&"-Y".to_string()));
        // Profile order preserved.
        let p_idx: Vec<usize> = argv
            .iter()
            .enumerate()
            .filter(|(_, a)| a.as_str() == "-p")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(p_idx.len(), 2);
        assert_eq!(argv[p_idx[0] + 1], "base.pp3");
        assert_eq!(argv[p_idx[1] + 1], "look.pp3");
    }
}
