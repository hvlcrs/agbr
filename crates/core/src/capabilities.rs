//! Backend capability model.
//!
//! Capabilities are reported by the adapter at runtime, never hard-coded into
//! the agent prompt. The planner uses them to decide what is representable.

use serde::{Deserialize, Serialize};

/// A named, stable operation identifier understood across backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Operation(pub String);

impl Operation {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// The set of operations a backend can execute, discovered at runtime.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BackendCapabilities {
    /// Backend name, e.g. `rawtherapee`.
    pub backend: String,

    /// Backend version, e.g. `5.13`.
    pub version: String,

    /// Global adjustment operations the backend supports.
    pub global: Vec<String>,

    /// Localized adjustment operations the backend supports.
    pub local: Vec<String>,

    /// Whether external mask files can be consumed.
    pub external_mask: bool,

    /// Whether headless (CLI) export is available.
    pub headless_export: bool,

    /// Whether multiple `-p` profiles are layered in order.
    pub profile_layering: bool,
}

impl BackendCapabilities {
    pub fn supports_global(&self, op: &str) -> bool {
        self.global.iter().any(|g| g == op)
    }

    pub fn supports_local(&self, op: &str) -> bool {
        self.local.iter().any(|l| l == op)
    }
}
