//! Runtime capability report for the installed RawTherapee backend.
//!
//! The exact set is derived from the adapter implementation, not hard-coded
//! into the agent prompt (design 10.4).

use agbr_core::BackendCapabilities;

/// The global operations the Phase 1 adapter can represent.
pub const GLOBAL_OPS: &[&str] = &[
    "exposure",
    "white_balance",
    "contrast",
    "highlights",
    "shadows",
    "saturation",
    "tone_curve",
    "sharpening",
    "noise_reduction",
];

/// Capabilities for the installed RawTherapee. `version` is best-effort and
/// should be populated from `rawtherapee-cli -v` output by the CLI.
pub fn rawtherapee_capabilities(version: impl Into<String>) -> BackendCapabilities {
    BackendCapabilities {
        backend: "rawtherapee".to_string(),
        version: version.into(),
        global: GLOBAL_OPS.iter().map(|s| s.to_string()).collect(),
        local: vec![],
        external_mask: false,
        headless_export: true,
        profile_layering: true,
    }
}
