//! Deterministic validation (design section 8, 17, 19).
//!
//! The validator checks schema-level correctness and numeric bounds. It never
//! "fixes" risky requests silently; it reports errors and lets the caller
//! decide.

use crate::error::{ErrorCode, ValidationError};
use crate::recipe::{GlobalAdjustments, PhotoRecipe, ToneCurve};
use crate::regions::RegionSpec;

/// Bounds for normalized adjustment values.
const UNIT: f64 = 1.0;
const EV_MAX: f64 = 5.0;

#[derive(Debug, Default, Clone)]
pub struct RecipeValidation {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

impl RecipeValidation {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate a recipe, returning structured errors and warnings.
pub fn validate_recipe(recipe: &PhotoRecipe) -> RecipeValidation {
    let mut v = RecipeValidation::default();

    if recipe.source.path.trim().is_empty() {
        v.errors.push(
            ValidationError::new(ErrorCode::InvalidRecipe, "source.path must not be empty")
                .field("source.path"),
        );
    }

    validate_global(&recipe.global, &mut v);

    for (i, op) in recipe.local.iter().enumerate() {
        validate_region(&op.target, &format!("local[{i}].target"), &mut v);
    }

    v
}

fn validate_global(g: &GlobalAdjustments, v: &mut RecipeValidation) {
    if let Some(ev) = g.exposure_ev {
        if !ev.is_finite() || ev.abs() > EV_MAX {
            v.errors.push(
                ValidationError::new(
                    ErrorCode::InvalidRecipe,
                    format!("exposure_ev must be within +/-{EV_MAX} EV"),
                )
                .field("global.exposure_ev"),
            );
        }
    }

    for (name, value) in [
        ("contrast", g.contrast),
        ("highlights", g.highlights),
        ("shadows", g.shadows),
        ("saturation", g.saturation),
    ] {
        if let Some(x) = value {
            if !x.is_finite() || !(-UNIT..=UNIT).contains(&x) {
                v.errors.push(
                    ValidationError::new(
                        ErrorCode::InvalidRecipe,
                        format!("{name} must be within [-1, 1]"),
                    )
                    .field(format!("global.{name}")),
                );
            }
        }
    }

    if let Some(wb) = &g.white_balance {
        if !wb.temperature_k.is_finite() || !(1000.0..=20000.0).contains(&wb.temperature_k) {
            v.errors.push(
                ValidationError::new(
                    ErrorCode::InvalidRecipe,
                    "white_balance.temperature_k must be within [1000, 20000]",
                )
                .field("global.white_balance.temperature_k"),
            );
        }
        if !wb.tint.is_finite() || wb.tint.abs() > 1.0 {
            v.errors.push(
                ValidationError::new(
                    ErrorCode::InvalidRecipe,
                    "white_balance.tint must be within [-1, 1]",
                )
                .field("global.white_balance.tint"),
            );
        }
    }

    if let Some(tc) = &g.tone_curve {
        validate_tone_curve(tc, v);
    }

    if let Some(s) = &g.sharpening {
        for (name, x) in [
            ("amount", s.amount),
            ("radius", s.radius),
            ("threshold", s.threshold),
        ] {
            if !x.is_finite() || !(0.0..=UNIT).contains(&x) {
                v.errors.push(
                    ValidationError::new(
                        ErrorCode::InvalidRecipe,
                        format!("sharpening.{name} must be within [0, 1]"),
                    )
                    .field(format!("global.sharpening.{name}")),
                );
            }
        }
    }

    if let Some(nr) = &g.noise_reduction {
        for (name, x) in [("luminance", nr.luminance), ("chroma", nr.chroma)] {
            if !x.is_finite() || !(0.0..=UNIT).contains(&x) {
                v.errors.push(
                    ValidationError::new(
                        ErrorCode::InvalidRecipe,
                        format!("noise_reduction.{name} must be within [0, 1]"),
                    )
                    .field(format!("global.noise_reduction.{name}")),
                );
            }
        }
    }

    if let Some(grain) = g.grain {
        if !grain.is_finite() || !(0.0..=UNIT).contains(&grain) {
            v.errors.push(
                ValidationError::new(ErrorCode::InvalidRecipe, "grain must be within [0, 1]")
                    .field("global.grain"),
            );
        }
    }
}

fn validate_tone_curve(tc: &ToneCurve, v: &mut RecipeValidation) {
    if tc.points.len() < 2 {
        v.errors.push(
            ValidationError::new(
                ErrorCode::InvalidRecipe,
                "tone_curve requires at least 2 points",
            )
            .field("global.tone_curve.points"),
        );
        return;
    }

    for (i, [x, y]) in tc.points.iter().enumerate() {
        if !x.is_finite() || !y.is_finite() || !(0.0..=1.0).contains(x) || !(0.0..=1.0).contains(y)
        {
            v.errors.push(
                ValidationError::new(
                    ErrorCode::InvalidRecipe,
                    format!("tone_curve point {i} must have coordinates in [0, 1]"),
                )
                .field(format!("global.tone_curve.points[{i}]")),
            );
        }
    }
}

fn validate_region(region: &RegionSpec, path: &str, v: &mut RecipeValidation) {
    use RegionSpec::*;

    fn check_unit(errors: &mut Vec<ValidationError>, path: &str, name: &str, x: f64) {
        if !x.is_finite() || !(0.0..=1.0).contains(&x) {
            errors.push(
                ValidationError::new(
                    ErrorCode::InvalidRegion,
                    format!("{name} must be within [0, 1]"),
                )
                .field(path.to_string()),
            );
        }
    }

    match region {
        Rectangle { x, y, w, h } => {
            check_unit(&mut v.errors, path, "x", *x);
            check_unit(&mut v.errors, path, "y", *y);
            check_unit(&mut v.errors, path, "w", *w);
            check_unit(&mut v.errors, path, "h", *h);
        }
        Ellipse { cx, cy, rx, ry } => {
            check_unit(&mut v.errors, path, "cx", *cx);
            check_unit(&mut v.errors, path, "cy", *cy);
            check_unit(&mut v.errors, path, "rx", *rx);
            check_unit(&mut v.errors, path, "ry", *ry);
        }
        Polygon { coordinates } => {
            if coordinates.len() < 3 {
                v.errors.push(
                    ValidationError::new(
                        ErrorCode::InvalidRegion,
                        "polygon requires at least 3 coordinates",
                    )
                    .field(path.to_string()),
                );
            }
            for [x, y] in coordinates {
                check_unit(&mut v.errors, path, "coordinate x", *x);
                check_unit(&mut v.errors, path, "coordinate y", *y);
            }
        }
        Gradient { x1, y1, x2, y2 } => {
            check_unit(&mut v.errors, path, "x1", *x1);
            check_unit(&mut v.errors, path, "y1", *y1);
            check_unit(&mut v.errors, path, "x2", *x2);
            check_unit(&mut v.errors, path, "y2", *y2);
        }
        LumaRange {
            low,
            high,
            softness,
        } => {
            check_unit(&mut v.errors, path, "low", *low);
            check_unit(&mut v.errors, path, "high", *high);
            check_unit(&mut v.errors, path, "softness", *softness);
        }
        ColorRange { hue, range } => {
            check_unit(&mut v.errors, path, "hue", *hue);
            check_unit(&mut v.errors, path, "range", *range);
        }
        ExternalMask { path: p } => {
            if p.trim().is_empty() {
                v.errors.push(
                    ValidationError::new(ErrorCode::InvalidRegion, "external_mask path is empty")
                        .field(path.to_string()),
                );
            }
        }
        SemanticRegion { label } => {
            if label.trim().is_empty() {
                v.errors.push(
                    ValidationError::new(
                        ErrorCode::InvalidRegion,
                        "semantic_region label is empty",
                    )
                    .field(path.to_string()),
                );
            }
        }
        Intersection { left, right } | Union { left, right } | Difference { left, right } => {
            validate_region(left, &format!("{path}.left"), v);
            validate_region(right, &format!("{path}.right"), v);
        }
        Invert { region } => validate_region(region, &format!("{path}.region"), v),
    }
}
