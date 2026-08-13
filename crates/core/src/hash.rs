//! Canonical JSON serialization and SHA-256 recipe hashing (design 17.5).
//!
//! Canonicalization relies on `serde_json`'s default `BTreeMap`-backed object
//! (keys sorted lexicographically) and its shortest-round-trip float
//! formatting. This yields a stable string for a given typed recipe.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::recipe::PhotoRecipe;

pub type RecipeHash = String;

/// Serialize a value to canonical JSON (sorted object keys, shortest floats).
pub fn canonical_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    // `serde_json::to_string` sorts map keys by default (no `preserve_order`).
    serde_json::to_string(value)
}

/// Compute a `sha256:`-prefixed hash of the recipe *content*, excluding
/// provenance (which varies per run and must not affect reproducibility).
pub fn recipe_hash(recipe: &PhotoRecipe) -> Result<RecipeHash, serde_json::Error> {
    // Serialize a projection that omits provenance.
    let projection = RecipeProjection {
        version: &recipe.version,
        source: &recipe.source,
        intent: &recipe.intent,
        global: &recipe.global,
        local: &recipe.local,
        constraints: &recipe.constraints,
    };

    let json = canonical_json(&projection)?;
    Ok(hash_str(json.as_bytes()))
}

/// SHA-256 of arbitrary bytes, rendered as `sha256:<hex>`.
pub fn hash_str(bytes: &[u8]) -> RecipeHash {
    let digest = Sha256::digest(bytes);
    format!("sha256:{}", hex::encode(digest))
}

#[derive(Serialize)]
struct RecipeProjection<'a> {
    version: &'a str,
    source: &'a crate::recipe::Source,
    intent: &'a crate::recipe::Intent,
    global: &'a crate::recipe::GlobalAdjustments,
    local: &'a [crate::regions::LocalOperation],
    constraints: &'a crate::recipe::Constraints,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_across_key_order() {
        let a = serde_json::json!({"b": 1, "a": 2});
        let b = serde_json::json!({"a": 2, "b": 1});
        assert_eq!(canonical_json(&a).unwrap(), canonical_json(&b).unwrap());
    }

    #[test]
    fn hash_is_prefixed() {
        assert!(hash_str(b"hello").starts_with("sha256:"));
    }
}
