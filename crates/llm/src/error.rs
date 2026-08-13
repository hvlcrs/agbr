//! LLM gateway error model.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("provider error: {0}")]
    Provider(String),

    #[error("HTTP transport error: {0}")]
    Transport(#[from] reqwest::Error),

    #[error("model returned invalid output: {0}")]
    ModelOutputInvalid(String),

    #[error("missing API key or model configuration")]
    MissingConfig,

    #[error("rate limited by provider")]
    RateLimited,

    #[error("provider returned non-success status {0}: {1}")]
    HttpStatus(u16, String),
}
