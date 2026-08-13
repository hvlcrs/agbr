//! Region specifications and localized operations.
//!
//! These are first-class in the canonical model (see design section 9), but
//! are only *executed* once a backend advertises support. Phase 1 ships the
//! model + validation; Phase 3 wires them to the backend.

use serde::{Deserialize, Serialize};

/// A semantic or geometrical target region, in normalized `[0,1]` source-image
/// coordinates (origin top-left).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RegionSpec {
    Rectangle {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
    },
    Polygon {
        coordinates: Vec<[f64; 2]>,
    },
    Gradient {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    LumaRange {
        low: f64,
        high: f64,
        #[serde(default)]
        softness: f64,
    },
    ColorRange {
        hue: f64,
        range: f64,
    },
    ExternalMask {
        path: String,
    },
    SemanticRegion {
        label: String,
    },
    /// Boolean composition of two sub-regions.
    #[serde(rename = "intersection")]
    Intersection {
        left: Box<RegionSpec>,
        right: Box<RegionSpec>,
    },
    #[serde(rename = "union")]
    Union {
        left: Box<RegionSpec>,
        right: Box<RegionSpec>,
    },
    #[serde(rename = "difference")]
    Difference {
        left: Box<RegionSpec>,
        right: Box<RegionSpec>,
    },
    #[serde(rename = "invert")]
    Invert {
        region: Box<RegionSpec>,
    },
}

/// A mask-aware localized adjustment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalOperation {
    pub target: RegionSpec,

    /// Named adjustments to apply within the region. Keys mirror the global
    /// adjustment field names (e.g. `highlights`, `saturation`).
    pub operations: serde_json::Value,

    /// Feather radius, normalized `[0,1]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feather: Option<f64>,

    /// Optional per-operation constraints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraints: Option<serde_json::Value>,
}
