//! EXIF extraction on top of the `exif` crate.

use std::path::Path;

use exif::{Reader, Tag, Value};
use serde::{Deserialize, Serialize};

use crate::{ImageContext, InspectReport, SourceInfo, TechnicalInfo};

/// Human-readable EXIF summary exposed to the LLM.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ExifSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lens: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub aperture: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutter: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub focal_length: Option<String>,
}

/// Extract EXIF from an image file. Non-EXIF failures degrade gracefully:
/// missing fields are simply `None`.
pub fn extract_exif(path: &Path) -> Result<ExifSummary, std::io::Error> {
    let mut summary = ExifSummary::default();

    let file = std::fs::File::open(path)?;

    let mut reader = std::io::BufReader::new(&file);
    let exif = match Reader::new().read_from_container(&mut reader) {
        Ok(e) => e,
        Err(_) => return Ok(summary), // not an EXIF-bearing file; not fatal
    };

    for field in exif.fields() {
        match field.tag {
            Tag::Make => summary.camera = Some(ascii_str(&field.value)),
            Tag::Model => {
                // Model is the body name; combine with Make if distinct.
                let model = ascii_str(&field.value);
                summary.camera = match summary.camera.take() {
                    Some(make) if !model.to_lowercase().contains(&make.to_lowercase()) => {
                        Some(format!("{make} {model}"))
                    }
                    _ => Some(model),
                };
            }
            Tag::LensModel => summary.lens = Some(ascii_str(&field.value)),
            Tag::PhotographicSensitivity => summary.iso = short_u32(&field.value),
            Tag::FNumber => {
                summary.aperture = rational_f64(&field.value).map(|v| format!("f/{v:.1}"))
            }
            Tag::ExposureTime => summary.shutter = rational_fraction(&field.value),
            Tag::FocalLength => {
                summary.focal_length = rational_f64(&field.value).map(|v| format!("{v:.0}mm"));
            }
            _ => {}
        }
    }

    Ok(summary)
}

/// Produce a full inspect report for a source path.
pub fn inspect(path: &Path) -> Result<InspectReport, std::io::Error> {
    let summary = extract_exif(path)?;

    let filename = path.file_name().map(|s| s.to_string_lossy().to_string());
    let format = path.extension().map(|s| s.to_string_lossy().to_uppercase());

    let source = SourceInfo {
        filename,
        format,
        width: None,
        height: None,
    };

    let technical = TechnicalInfo {
        camera: summary.camera.clone(),
        lens: summary.lens.clone(),
        iso: summary.iso,
        focal_length_mm: summary
            .focal_length
            .as_ref()
            .and_then(|s| s.trim_end_matches("mm").trim().parse::<f64>().ok()),
        aperture: summary
            .aperture
            .as_ref()
            .and_then(|s| s.trim_start_matches("f/").parse::<f64>().ok()),
        shutter: summary.shutter.clone(),
    };

    Ok(InspectReport {
        context: ImageContext {
            source,
            exif: summary,
        },
        technical,
    })
}

fn ascii_str(value: &Value) -> String {
    if let Value::Ascii(v) = value {
        if let Some(s) = v.first() {
            return String::from_utf8_lossy(s).trim().to_string();
        }
    }
    String::new()
}

fn short_u32(value: &Value) -> Option<u32> {
    match value {
        Value::Short(v) => v.first().copied().map(u32::from),
        Value::Long(v) => v.first().copied(),
        _ => None,
    }
}

fn rational_f64(value: &Value) -> Option<f64> {
    if let Value::Rational(v) = value {
        if let Some(r) = v.first() {
            if r.denom != 0 {
                return Some(f64::from(r.num) / f64::from(r.denom));
            }
        }
    }
    None
}

fn rational_fraction(value: &Value) -> Option<String> {
    if let Value::Rational(v) = value {
        if let Some(r) = v.first() {
            return Some(format!("{}/{}", r.num, r.denom));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_io_error() {
        let res = extract_exif(Path::new("/nonexistent/does-not-exist.ARW"));
        assert!(res.is_err());
    }
}
