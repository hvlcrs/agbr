//! LLM gateway configuration.

use serde::{Deserialize, Serialize};

/// Base URL for OpenRouter's OpenAI-compatible endpoint.
pub const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// Configuration for the OpenAI-compatible gateway (design section 6.2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmConfig {
    /// Provider label. Recognized values: `openai`, `openrouter`, `mock`.
    /// `openrouter` implies the OpenRouter base URL and key precedence.
    #[serde(default = "default_provider")]
    pub provider: String,

    /// OpenAI-compatible base URL. When `provider = "openrouter"` and this is
    /// unset, it defaults to OpenRouter.
    #[serde(default)]
    pub base_url: String,

    /// Model identifier (e.g. `gpt-4o-mini`, `anthropic/claude-3.5-sonnet`).
    #[serde(default)]
    pub model: String,

    /// API key. May also be supplied via the environment variable
    /// `OPENAI_API_KEY` or `OPENROUTER_API_KEY`.
    #[serde(default)]
    pub api_key: String,

    /// Maximum output tokens.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Request JSON mode (`response_format: {"type": "json_object"}`).
    /// Disable for models that reject it (e.g. Gemini image models) and rely
    /// on prompt instructions + tolerant JSON parsing instead.
    #[serde(default = "default_true")]
    pub json_mode: bool,
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_true() -> bool {
    true
}

fn default_max_tokens() -> u32 {
    4000
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            base_url: String::new(),
            model: String::new(),
            api_key: String::new(),
            max_tokens: default_max_tokens(),
            json_mode: true,
        }
    }
}

impl LlmConfig {
    /// The effective base URL, applying provider-specific defaults.
    pub fn effective_base_url(&self) -> String {
        if self.provider == "openrouter" && self.base_url.trim().is_empty() {
            OPENROUTER_BASE_URL.to_string()
        } else if self.base_url.trim().is_empty() {
            "https://api.openai.com/v1".to_string()
        } else {
            self.base_url.trim().to_string()
        }
    }

    /// Resolve an API key from config or a standard environment variable.
    ///
    /// For `openrouter` the `OPENROUTER_API_KEY` variable takes precedence;
    /// otherwise `OPENAI_API_KEY` is checked first.
    pub fn resolve_api_key(&self) -> Option<String> {
        if !self.api_key.trim().is_empty() {
            return Some(self.api_key.trim().to_string());
        }
        let vars: &[&str] = if self.provider == "openrouter" {
            &["OPENROUTER_API_KEY", "OPENAI_API_KEY"]
        } else {
            &["OPENAI_API_KEY", "OPENROUTER_API_KEY"]
        };
        for var in vars {
            if let Ok(v) = std::env::var(var) {
                if !v.trim().is_empty() {
                    return Some(v);
                }
            }
        }
        None
    }

    /// Whether enough configuration is present to make a real provider call.
    pub fn is_usable(&self) -> bool {
        !self.model.trim().is_empty() && self.resolve_api_key().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openrouter_uses_openrouter_base_url_by_default() {
        let config = LlmConfig {
            provider: "openrouter".to_string(),
            ..Default::default()
        };
        assert_eq!(config.effective_base_url(), OPENROUTER_BASE_URL);
    }

    #[test]
    fn explicit_base_url_wins_over_provider_default() {
        let config = LlmConfig {
            provider: "openrouter".to_string(),
            base_url: "https://custom.example/v1".to_string(),
            ..Default::default()
        };
        assert_eq!(config.effective_base_url(), "https://custom.example/v1");
    }

    #[test]
    fn openai_default_base_url() {
        let config = LlmConfig::default();
        assert_eq!(config.effective_base_url(), "https://api.openai.com/v1");
    }

    #[test]
    fn config_key_takes_precedence_over_env() {
        let config = LlmConfig {
            provider: "openrouter".to_string(),
            api_key: "cfg-key".to_string(),
            ..Default::default()
        };
        assert_eq!(config.resolve_api_key().as_deref(), Some("cfg-key"));
    }
}
