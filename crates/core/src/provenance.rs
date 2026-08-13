//! Execution provenance (design 17.6).

use serde::{Deserialize, Serialize};

/// Recorded for every recipe execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Provenance {
    /// Agent that requested the work (e.g. `claude-code`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// LLM provider (e.g. `openai`, `openrouter`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    /// Model identifier (e.g. `gpt-4o-mini`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Provider request id, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// RFC 3339 creation timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// `sha256:` recipe content hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe_hash: Option<String>,

    /// Adapter/control-plane version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_version: Option<String>,

    /// RawTherapee binary version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_version: Option<String>,

    /// SHA-256 of the source file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_hash: Option<String>,

    /// SHA-256 of the final output file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_hash: Option<String>,
}
