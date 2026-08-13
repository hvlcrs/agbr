//! `agbr-inspect` — read-only image metadata inspection.
//!
//! Produces an [`ImageContext`] for LLM reasoning and a [`TechnicalInfo`]
//! summary used by the base-PP3 resolver. This component never modifies the
//! source image.

mod exif;

pub use exif::ExifSummary;
pub use exif::{extract_exif, inspect};

use serde::{Deserialize, Serialize};

/// A compact, LLM-friendly description of the source image (design section 7).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImageContext {
    pub source: SourceInfo,
    pub exif: ExifSummary,
}

/// Basic source identity.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SourceInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

/// Technical capture facts used for base-profile resolution.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TechnicalInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lens: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub focal_length_mm: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub aperture: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutter: Option<String>,
}

/// The result of inspecting a source image.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InspectReport {
    pub context: ImageContext,
    pub technical: TechnicalInfo,
}

impl InspectReport {
    /// Whether we detected a camera (useful for base-profile policy).
    pub fn camera_detected(&self) -> bool {
        self.technical.camera.is_some()
    }

    /// Whether we detected a lens model.
    pub fn lens_detected(&self) -> bool {
        self.technical.lens.is_some()
    }
}
