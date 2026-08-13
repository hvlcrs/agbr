//! Deterministic mock provider for offline/testing use.

use serde::de::DeserializeOwned;

use crate::{LlmError, LlmProvider, LlmRequest};

/// A provider that returns a canned JSON value. Useful for tests and for
/// exercising the pipeline without a network or API key.
#[derive(Debug, Clone)]
pub struct MockProvider {
    response: serde_json::Value,
}

impl MockProvider {
    pub fn new(response: serde_json::Value) -> Self {
        Self { response }
    }
}

impl LlmProvider for MockProvider {
    async fn complete_structured<T: DeserializeOwned>(
        &self,
        _request: &LlmRequest,
    ) -> Result<T, LlmError> {
        crate::parse_structured(self.response.clone())
    }
}

/// A provider that always fails, for negative testing.
#[derive(Debug, Clone, Default)]
pub struct FailingProvider;

impl LlmProvider for FailingProvider {
    async fn complete_structured<T: DeserializeOwned>(
        &self,
        _request: &LlmRequest,
    ) -> Result<T, LlmError> {
        Err(LlmError::Provider("mock failure".into()))
    }
}
