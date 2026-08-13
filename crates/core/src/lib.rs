//! `agbr-core` — the canonical, editor-independent data model.
//!
//! The [`PhotoRecipe`] is the central contract between the cloud LLM and the
//! rendering backend. It describes *what the photograph should feel like*,
//! never *how* a particular editor should implement it.

pub mod capabilities;
pub mod error;
pub mod hash;
pub mod provenance;
pub mod recipe;
pub mod regions;
pub mod validate;

pub use capabilities::BackendCapabilities;
pub use error::{ErrorCode, ValidationError};
pub use hash::{canonical_json, recipe_hash, RecipeHash};
pub use provenance::Provenance;
pub use recipe::{GlobalAdjustments, Intent, PhotoRecipe, RecipeDraft, Source};
pub use regions::{LocalOperation, RegionSpec};
pub use validate::{validate_recipe, RecipeValidation};
