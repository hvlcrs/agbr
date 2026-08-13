//! Canonical `PhotoRecipe` model.
//!
//! All values are semantic and editor-independent. The RawTherapee adapter is
//! responsible for mapping these into PP3 keys (see `agbr-rawtherapee`).

use serde::{Deserialize, Serialize};

use crate::regions::LocalOperation;

pub const RECIPE_VERSION: &str = "1.0";

fn default_version() -> String {
    RECIPE_VERSION.to_string()
}

/// The complete, typed recipe. This is what the cloud LLM emits and what the
/// planner/validator consume.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhotoRecipe {
    #[serde(default = "default_version")]
    pub version: String,

    /// Source image identity. The original file is immutable.
    pub source: Source,

    /// Free-form semantic intent supplied by the user / director role.
    #[serde(default)]
    pub intent: Intent,

    /// Global (whole-image) adjustments.
    #[serde(default)]
    pub global: GlobalAdjustments,

    /// Localized, mask-aware operations.
    #[serde(default)]
    pub local: Vec<LocalOperation>,

    /// Hard constraints the planner must respect.
    #[serde(default)]
    pub constraints: Constraints,

    /// Execution provenance. Populated by the control plane, not the LLM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<crate::provenance::Provenance>,
}

impl PhotoRecipe {
    /// Build a recipe for the given source path with all-empty adjustments.
    pub fn empty(source: Source) -> Self {
        Self {
            version: RECIPE_VERSION.to_string(),
            source,
            intent: Intent::default(),
            global: GlobalAdjustments::default(),
            local: Vec::new(),
            constraints: Constraints::default(),
            provenance: None,
        }
    }

    /// Assemble a recipe from the LLM-provided draft plus control-plane-owned
    /// source identity.
    pub fn from_draft(source: Source, draft: RecipeDraft) -> Self {
        Self {
            version: RECIPE_VERSION.to_string(),
            source,
            intent: draft.intent,
            global: draft.global,
            local: draft.local,
            constraints: draft.constraints,
            provenance: None,
        }
    }
}

/// The portion of a [`PhotoRecipe`] that the LLM is expected to produce.
/// Source identity, version, and provenance are owned by the control plane.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RecipeDraft {
    #[serde(default)]
    pub intent: Intent,

    #[serde(default)]
    pub global: GlobalAdjustments,

    #[serde(default)]
    pub local: Vec<LocalOperation>,

    #[serde(default)]
    pub constraints: Constraints,
}

/// Identity of the source image.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Source {
    /// Path to the source file as supplied by the user.
    pub path: String,

    /// SHA-256 of the source bytes, once available. The control plane fills
    /// this in; the LLM should not be expected to compute it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

impl Source {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            sha256: None,
        }
    }
}

/// Semantic intent, largely free-form for the director role.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Intent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibe: Option<String>,

    /// Overall strength of the look, 0.0..=1.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intensity: Option<f64>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// Global (whole-image) adjustments, expressed in normalized form.
///
/// Normalization conventions:
/// - `exposure_ev` is in stops (EV).
/// - `contrast`, `highlights`, `shadows`, `saturation` are in `-1.0..=1.0`.
/// - `white_balance.temperature_k` is in Kelvin; `tint` is a small signed delta.
/// - `grain` is in `0.0..=1.0`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GlobalAdjustments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure_ev: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub white_balance: Option<WhiteBalance>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contrast: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlights: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadows: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saturation: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone_curve: Option<ToneCurve>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sharpening: Option<Sharpening>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise_reduction: Option<NoiseReduction>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grain: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WhiteBalance {
    /// Temperature in Kelvin.
    pub temperature_k: f64,

    /// Tint delta. Positive is more magenta, negative is more green.
    #[serde(default)]
    pub tint: f64,
}

/// A tone curve defined by normalized control points in `[0,1]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToneCurve {
    /// `(x, y)` control points, normalized. Must include (0,0) and (1,1).
    pub points: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sharpening {
    /// Normalized amount, 0.0..=1.0.
    pub amount: f64,

    /// Normalized radius, 0.0..=1.0.
    #[serde(default)]
    pub radius: f64,

    /// Normalized threshold, 0.0..=1.0.
    #[serde(default)]
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoiseReduction {
    /// Luminance noise reduction, 0.0..=1.0.
    #[serde(default)]
    pub luminance: f64,

    /// Chroma noise reduction, 0.0..=1.0.
    #[serde(default)]
    pub chroma: f64,
}

/// Hard constraints carried through planning and execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Constraints {
    #[serde(default)]
    pub preserve_skin: bool,

    #[serde(default)]
    pub no_crop: bool,

    #[serde(default = "default_true")]
    pub no_generation: bool,

    #[serde(default = "default_true")]
    pub preserve_original: bool,
}

fn default_true() -> bool {
    true
}
