//! Minimal PP3 document model and serializer.
//!
//! Key names and sections mirror RawTherapee 5.13 (verified against the
//! bundled profiles and `rtengine/procparams.cc`).

/// An ordered collection of PP3 sections.
#[derive(Debug, Clone, Default)]
pub struct Pp3 {
    sections: Vec<(String, Vec<(String, String)>)>,
}

impl Pp3 {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set (or replace) a key within a section, creating the section if needed.
    pub fn set(&mut self, section: &str, key: &str, value: impl Into<String>) {
        let value = value.into();
        if let Some((_, keys)) = self.sections.iter_mut().find(|(name, _)| name == section) {
            if let Some(entry) = keys.iter_mut().find(|(k, _)| k == key) {
                entry.1 = value;
            } else {
                keys.push((key.to_string(), value));
            }
        } else {
            self.sections
                .push((section.to_string(), vec![(key.to_string(), value)]));
        }
    }

    pub fn set_bool(&mut self, section: &str, key: &str, value: bool) {
        self.set(section, key, if value { "true" } else { "false" });
    }

    pub fn set_int(&mut self, section: &str, key: &str, value: i64) {
        self.set(section, key, value.to_string());
    }

    /// Format a float using Rust's shortest round-trip representation.
    pub fn set_float(&mut self, section: &str, key: &str, value: f64) {
        self.set(section, key, value.to_string());
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }
}

impl std::fmt::Display for Pp3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (section, keys) in &self.sections {
            writeln!(f, "[{section}]")?;
            for (key, value) in keys {
                writeln!(f, "{key}={value}")?;
            }
            f.write_str("\n")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_sections_and_keys() {
        let mut p = Pp3::new();
        p.set("Exposure", "Compensation", "0.10");
        p.set_int("Exposure", "Contrast", -12);
        p.set("White Balance", "Setting", "Custom");

        let s = p.to_string();
        assert!(s.contains("[Exposure]\nCompensation=0.10\nContrast=-12"));
        assert!(s.contains("[White Balance]\nSetting=Custom"));
    }

    #[test]
    fn replacing_key_does_not_duplicate() {
        let mut p = Pp3::new();
        p.set("A", "x", "1");
        p.set("A", "x", "2");
        let s = p.to_string();
        assert_eq!(s.matches("x=").count(), 1);
    }
}
