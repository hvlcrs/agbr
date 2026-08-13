//! `agbr-planner` — validation + capability gating + execution ordering.
//!
//! The planner never writes files or PP3 directly. It resolves *what* will be
//! executed and in *what* order, against a runtime capability snapshot.

use agbr_core::hash::recipe_hash;
use agbr_core::recipe::{GlobalAdjustments, PhotoRecipe};
use agbr_core::{validate_recipe, BackendCapabilities};

use serde::{Deserialize, Serialize};

/// A named, ordered processing layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layer {
    /// Technical normalization (camera/lens/CA).
    Base,
    /// Creative look from the recipe.
    Look,
}

/// Status of a single planned step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub op: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StepStatus {
    Supported,
    Unsupported { reason: String },
    Absent,
}

/// A resolved execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub recipe_hash: String,
    pub valid: bool,
    pub steps: Vec<Step>,
    pub warnings: Vec<String>,
    pub layers: Vec<Layer>,
}

impl Plan {
    pub fn has_unsupported(&self) -> bool {
        self.steps
            .iter()
            .any(|s| matches!(s.status, StepStatus::Unsupported { .. }))
    }
}

/// Plan a recipe against a backend capability snapshot.
pub fn plan(recipe: &PhotoRecipe, capabilities: &BackendCapabilities) -> Plan {
    let validation = validate_recipe(recipe);

    let mut steps = Vec::new();
    let mut warnings = validation.warnings.clone();

    for (op, present) in global_operations(&recipe.global) {
        if !present {
            continue;
        }
        if capabilities.supports_global(op) {
            steps.push(Step {
                op: op.to_string(),
                status: StepStatus::Supported,
            });
        } else {
            let reason = format!(
                "operation '{op}' is not supported by backend '{}'",
                capabilities.backend
            );
            steps.push(Step {
                op: op.to_string(),
                status: StepStatus::Unsupported {
                    reason: reason.clone(),
                },
            });
            warnings.push(reason);
        }
    }

    // Phase 1: local operations are modeled but not yet executable.
    if !recipe.local.is_empty() {
        let reason = format!(
            "{} localized operation(s) requested, but backend '{}' has no local masking support",
            recipe.local.len(),
            capabilities.backend
        );
        steps.push(Step {
            op: "local".to_string(),
            status: StepStatus::Unsupported {
                reason: reason.clone(),
            },
        });
        warnings.push(reason);
    }

    let layers = vec![Layer::Base, Layer::Look];

    Plan {
        recipe_hash: recipe_hash(recipe).unwrap_or_default(),
        valid: validation.is_valid(),
        steps,
        warnings,
        layers,
    }
}

/// Enumerate global operations and whether they are present in the recipe.
fn global_operations(g: &GlobalAdjustments) -> Vec<(&'static str, bool)> {
    vec![
        ("exposure", g.exposure_ev.is_some()),
        ("white_balance", g.white_balance.is_some()),
        ("contrast", g.contrast.is_some()),
        ("highlights", g.highlights.is_some()),
        ("shadows", g.shadows.is_some()),
        ("saturation", g.saturation.is_some()),
        ("tone_curve", g.tone_curve.is_some()),
        ("sharpening", g.sharpening.is_some()),
        ("noise_reduction", g.noise_reduction.is_some()),
        ("grain", g.grain.is_some()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use agbr_core::recipe::Source;

    #[test]
    fn plans_supported_ops() {
        let recipe = PhotoRecipe {
            global: GlobalAdjustments {
                exposure_ev: Some(0.1),
                contrast: Some(-0.1),
                ..Default::default()
            },
            ..empty_recipe()
        };
        let caps = BackendCapabilities {
            backend: "rawtherapee".into(),
            version: "5.13".into(),
            global: vec!["exposure".into(), "contrast".into()],
            local: vec![],
            external_mask: false,
            headless_export: true,
            profile_layering: true,
        };
        let plan = plan(&recipe, &caps);
        assert!(plan.valid);
        assert!(!plan.has_unsupported());
        assert_eq!(plan.layers, vec![Layer::Base, Layer::Look]);
    }

    #[test]
    fn flags_unsupported_grain_and_local() {
        let mut recipe = PhotoRecipe {
            global: GlobalAdjustments {
                grain: Some(0.1),
                ..Default::default()
            },
            ..empty_recipe()
        };
        recipe.local.push(agbr_core::regions::LocalOperation {
            target: agbr_core::regions::RegionSpec::SemanticRegion {
                label: "sky".into(),
            },
            operations: serde_json::json!({}),
            feather: None,
            constraints: None,
        });
        let caps = BackendCapabilities {
            backend: "rawtherapee".into(),
            version: "5.13".into(),
            global: vec![],
            local: vec![],
            external_mask: false,
            headless_export: true,
            profile_layering: true,
        };
        let plan = plan(&recipe, &caps);
        assert!(plan.has_unsupported());
        assert!(plan
            .steps
            .iter()
            .any(|s| matches!(s.status, StepStatus::Unsupported { .. }) && s.op == "grain"));
    }

    fn empty_recipe() -> PhotoRecipe {
        PhotoRecipe::empty(Source::new("IMG_001.ARW"))
    }
}
