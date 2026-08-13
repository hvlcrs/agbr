//! OpenAI-compatible provider implementation.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{LlmConfig, LlmError, LlmProvider, LlmRequest};

/// An OpenAI-compatible chat-completions provider (works with OpenAI,
/// OpenRouter, and any compatible endpoint).
#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    config: LlmConfig,
    client: reqwest::Client,
    /// Number of retries for transient failures.
    retries: u32,
}

impl OpenAIProvider {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            retries: 2,
        }
    }

    fn endpoint(&self) -> String {
        format!(
            "{}/chat/completions",
            self.config.effective_base_url().trim_end_matches('/')
        )
    }
}

impl LlmProvider for OpenAIProvider {
    async fn complete_structured<T: DeserializeOwned>(
        &self,
        request: &LlmRequest,
    ) -> Result<T, LlmError> {
        let api_key = self
            .config
            .resolve_api_key()
            .ok_or(LlmError::MissingConfig)?;

        let body = ChatRequest {
            model: &self.config.model,
            messages: vec![
                Message {
                    role: "system",
                    content: request.system.as_str(),
                },
                Message {
                    role: "user",
                    content: request.user.as_str(),
                },
            ],
            response_format: if self.config.json_mode {
                Some(ResponseFormat::json_object())
            } else {
                None
            },
            max_tokens: request.max_tokens.max(1),
        };

        let mut last_err = None;
        for attempt in 0..=self.retries {
            let result = self
                .client
                .post(self.endpoint())
                .bearer_auth(&api_key)
                .header("X-Title", "agbr")
                .json(&body)
                .send()
                .await;

            match result {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        match resp.json::<ChatResponse>().await {
                            Ok(chat) => {
                                let text = chat
                                    .choices
                                    .into_iter()
                                    .next()
                                    .map(|c| c.message.content)
                                    .unwrap_or_default();
                                return parse_content(&text);
                            }
                            Err(e) => last_err = Some(LlmError::Transport(e)),
                        }
                    } else if status.as_u16() == 429 {
                        last_err = Some(LlmError::RateLimited);
                    } else {
                        let text = resp.text().await.unwrap_or_default();
                        last_err = Some(LlmError::HttpStatus(status.as_u16(), text));
                        // Non-retryable 4xx (except 429) — stop early.
                        if status.is_client_error() {
                            break;
                        }
                    }
                }
                Err(e) => last_err = Some(LlmError::Transport(e)),
            }

            if attempt < self.retries {
                tokio::time::sleep(std::time::Duration::from_millis(500 * (attempt as u64 + 1)))
                    .await;
            }
        }

        Err(last_err.unwrap_or(LlmError::Provider("unknown failure".into())))
    }
}

/// Parse a JSON (or fenced-JSON) string into a typed value.
fn parse_content<T: DeserializeOwned>(text: &str) -> Result<T, LlmError> {
    let trimmed = text.trim();

    // Strip ```json ... ``` fences if present.
    let json_text = if trimmed.starts_with("```") {
        trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .and_then(|s| s.strip_suffix("```"))
            .unwrap_or(trimmed)
            .trim()
    } else {
        trimmed
    };

    let value: serde_json::Value =
        serde_json::from_str(json_text).map_err(|e| LlmError::ModelOutputInvalid(e.to_string()))?;

    crate::parse_structured(value)
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    type_: &'static str,
}

impl ResponseFormat {
    fn json_object() -> Self {
        Self {
            type_: "json_object",
        }
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}
