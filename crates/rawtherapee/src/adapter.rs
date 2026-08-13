//! Recipe -> PP3 generation.
//!
//! Maps editor-independent [`GlobalAdjustments`] into RawTherapee PP3. Any
//! adjustment that cannot be represented is *reported*, never silently
//! dropped (design 3.4, 10.4).

use agbr_core::recipe::{GlobalAdjustments, ToneCurve};

use crate::pp3::Pp3;

/// A generated PP3 document plus advisory notes about representability.
#[derive(Debug, Clone, Default)]
pub struct GeneratedProfile {
    /// Serialized PP3 text.
    pub pp3: String,

    /// Advisory notes (e.g. partial mappings).
    pub notes: Vec<String>,

    /// Operations that were requested but are not representable by this
    /// backend. Each entry is `<operation>=<reason>`.
    pub unsupported: Vec<String>,
}

/// Technical context used to resolve the base (correction) profile.
#[derive(Debug, Clone, Default)]
pub struct BaseProfileInput {
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<u32>,
}

/// Generate the technical baseline profile (design 10.3).
///
/// The base profile answers "what does this RAW technically need?" — camera
/// identification, lens correction, and chromatic-aberration/chroma
/// correction. The LLM never tunes these.
pub fn generate_base_pp3(input: &BaseProfileInput) -> GeneratedProfile {
    let mut pp3 = Pp3::new();
    let mut notes = Vec::new();

    // Chromatic aberration / purple fringing correction (post-demosaic).
    pp3.set_bool("Defringing", "Enabled", true);
    pp3.set_float("Defringing", "Radius", 2.0);
    pp3.set_int("Defringing", "Threshold", 30);

    // Profiled lens correction via Lensfun auto-match.
    if input.lens.is_some() && input.camera.is_some() {
        pp3.set("LensProfile", "LcMode", "lfauto");
        pp3.set_bool("LensProfile", "UseDistortion", true);
        pp3.set_bool("LensProfile", "UseVignette", true);
        pp3.set_bool("LensProfile", "UseCA", true);
    } else {
        notes.push(
            "No matching Lensfun/LCP profile was resolved: camera/lens metadata is incomplete."
                .to_string(),
        );
    }

    GeneratedProfile {
        pp3: pp3.to_string(),
        notes,
        unsupported: vec![],
    }
}

/// Generate the creative "look" profile from global adjustments.
pub fn generate_look_pp3(global: &GlobalAdjustments) -> GeneratedProfile {
    let mut pp3 = Pp3::new();
    let mut unsupported = Vec::new();

    if let Some(ev) = global.exposure_ev {
        pp3.set_float("Exposure", "Compensation", ev);
    }

    if let Some(wb) = &global.white_balance {
        pp3.set("White Balance", "Setting", "Custom");
        pp3.set_float("White Balance", "Temperature", wb.temperature_k);
        // Tint: -1..=1 -> Green multiplier 0.5..=1.5 (1.0 neutral).
        let green = 1.0 + wb.tint * 0.5;
        pp3.set_float("White Balance", "Green", green);
    }

    if let Some(contrast) = global.contrast {
        pp3.set_int("Exposure", "Contrast", (contrast * 100.0).round() as i64);
    }

    // Highlights: negative -> compress (darken) via HighlightCompr.
    if let Some(highlights) = global.highlights {
        if highlights <= 0.0 {
            pp3.set_int(
                "Exposure",
                "HighlightCompr",
                (-highlights * 100.0).round() as i64,
            );
        } else {
            unsupported.push(
                "highlights: positive highlight boost is not representable by RawTherapee"
                    .to_string(),
            );
        }
    }

    // Shadows: positive -> lift via ShadowCompr.
    if let Some(shadows) = global.shadows {
        if shadows >= 0.0 {
            pp3.set_int("Exposure", "ShadowCompr", (shadows * 100.0).round() as i64);
        } else {
            unsupported.push(
                "shadows: negative shadow darkening is not representable by RawTherapee"
                    .to_string(),
            );
        }
    }

    if let Some(saturation) = global.saturation {
        pp3.set_int(
            "Exposure",
            "Saturation",
            (saturation * 100.0).round() as i64,
        );
    }

    if let Some(curve) = &global.tone_curve {
        pp3.set("Exposure", "CurveMode", "Standard");
        pp3.set("Exposure", "Curve", curve_string(curve));
    }

    if let Some(s) = &global.sharpening {
        pp3.set_bool("Sharpening", "Enabled", true);
        pp3.set("Sharpening", "Method", "usm");
        pp3.set_int("Sharpening", "Amount", (s.amount * 400.0).round() as i64);
        pp3.set_float("Sharpening", "Radius", 0.5 + s.radius * 1.5);
        pp3.set(
            "Sharpening",
            "Threshold",
            format!("{};80;2000", (s.threshold * 2000.0).round() as i64),
        );
    }

    if let Some(nr) = &global.noise_reduction {
        pp3.set_bool("Directional Pyramid Denoising", "Enabled", true);
        pp3.set_int(
            "Directional Pyramid Denoising",
            "Luma",
            (nr.luminance * 100.0).round() as i64,
        );
        pp3.set_int(
            "Directional Pyramid Denoising",
            "Chroma",
            (nr.chroma * 100.0).round() as i64,
        );
    }

    if let Some(_grain) = global.grain {
        unsupported.push(
            "grain: film grain is not representable as a RawTherapee processing profile"
                .to_string(),
        );
    }

    GeneratedProfile {
        pp3: pp3.to_string(),
        notes: vec![],
        unsupported,
    }
}

/// Serialize a tone curve to the RawTherapee `Curve` list format:
/// `<type>;x0;y0;x1;y1;...` with type `1` (linear interpolation).
fn curve_string(curve: &ToneCurve) -> String {
    let mut points: Vec<[f64; 2]> = curve.points.clone();
    points.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));

    let mut out = String::from("1");
    for [x, y] in points {
        out.push(';');
        out.push_str(&x.to_string());
        out.push(';');
        out.push_str(&y.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use agbr_core::recipe::WhiteBalance;

    fn g() -> GlobalAdjustments {
        GlobalAdjustments {
            exposure_ev: Some(0.1),
            white_balance: Some(WhiteBalance {
                temperature_k: 5700.0,
                tint: 0.0,
            }),
            contrast: Some(-0.12),
            highlights: Some(-0.25),
            shadows: Some(0.12),
            saturation: Some(-0.08),
            tone_curve: None,
            sharpening: None,
            noise_reduction: None,
            grain: None,
        }
    }

    #[test]
    fn maps_global_adjustments() {
        let out = generate_look_pp3(&g());
        let s = &out.pp3;
        assert!(s.contains("Compensation=0.1"));
        assert!(s.contains("Contrast=-12"));
        assert!(s.contains("HighlightCompr=25"));
        assert!(s.contains("ShadowCompr=12"));
        assert!(s.contains("Saturation=-8"));
        assert!(s.contains("Setting=Custom"));
        assert!(s.contains("Temperature=5700"));
        assert!(out.unsupported.is_empty());
    }

    #[test]
    fn grain_is_reported_unsupported() {
        let mut x = g();
        x.grain = Some(0.12);
        let out = generate_look_pp3(&x);
        assert!(out.unsupported.iter().any(|u| u.starts_with("grain")));
    }

    #[test]
    fn positive_highlights_is_reported_unsupported() {
        let mut x = g();
        x.highlights = Some(0.2);
        let out = generate_look_pp3(&x);
        assert!(out.unsupported.iter().any(|u| u.starts_with("highlights")));
    }

    #[test]
    fn tone_curve_serializes() {
        let mut x = g();
        x.tone_curve = Some(ToneCurve {
            points: vec![[0.0, 0.0], [0.5, 0.6], [1.0, 1.0]],
        });
        let out = generate_look_pp3(&x);
        assert!(out.pp3.contains("CurveMode=Standard"));
        assert!(out.pp3.contains("Curve=1;0;0;0.5;0.6;1;1"));
    }
}
