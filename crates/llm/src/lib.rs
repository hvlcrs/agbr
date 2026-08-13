//! `agbr-llm` — provider-neutral LLM gateway.
//!
//! The control plane talks to any OpenAI-compatible endpoint (OpenAI,
//! OpenRouter, etc.) through [`LlmProvider`]. The LLM only ever produces a
//! structured [`PhotoRecipe`]; it never writes PP3 or touches pixels.

mod config;
mod error;
mod mock;
mod openai;

pub use config::LlmConfig;
pub use error::LlmError;
pub use mock::{FailingProvider, MockProvider};
pub use openai::OpenAIProvider;

use serde::de::DeserializeOwned;

/// A structured-completion request, independent of provider.
#[derive(Debug, Clone)]
pub struct LlmRequest {
    /// System prompt (policy + task).
    pub system: String,

    /// User prompt (intent + image context).
    pub user: String,

    /// JSON schema the output must conform to (optional; always supplied by
    /// the control plane for recipe generation).
    pub schema: Option<serde_json::Value>,

    /// Maximum output tokens.
    pub max_tokens: u32,
}

/// Provider abstraction (design section 6).
#[allow(async_fn_in_trait)]
pub trait LlmProvider {
    /// Request a structured object of type `T` from the provider.
    async fn complete_structured<T>(&self, request: &LlmRequest) -> Result<T, LlmError>
    where
        T: DeserializeOwned;
}

/// Convert a JSON value into a typed `T`, mapping parse/validation failures to
/// [`LlmError::ModelOutputInvalid`].
pub fn parse_structured<T: DeserializeOwned>(value: serde_json::Value) -> Result<T, LlmError> {
    serde_json::from_value(value).map_err(|e| LlmError::ModelOutputInvalid(e.to_string()))
}
