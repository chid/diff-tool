//! Anthropic Messages API adapter.
//!
//! Translates between the crate's internal `ChatRequest` / `ChatResponse` types
//! and the Anthropic Messages API wire format (`/v1/messages`).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::{ChatMessage, ChatRequest, ChatResponse, Chunk, Role, Usage};
use crate::{AiError, LlmBackend, Result};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::cassette::CassetteRunner;

const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1";
/// Anthropic API version header value.
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug)]
pub struct AnthropicBackend {
    api_key: String,
    base_url: String,
    max_tokens: u32,
    client: reqwest::Client,
    cassette: Option<Arc<Mutex<CassetteRunner>>>,
}

impl AnthropicBackend {
    pub fn new(
        api_key: String,
        _model: String,
        base_url: Option<String>,
        max_tokens: u32,
        _temperature: f32,
        client: reqwest::Client,
    ) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| ANTHROPIC_BASE_URL.to_string()),
            max_tokens,
            client,
            cassette: None,
        }
    }

    pub fn with_cassette(mut self, cassette: Option<Arc<Mutex<CassetteRunner>>>) -> Self {
        self.cassette = cassette;
        self
    }

    fn url(&self) -> String {
        format!("{}/messages", self.base_url.trim_end_matches('/'))
    }

    /// Split messages into the system prompt (if any) and the conversation
    /// messages. Anthropic's API takes `system` as a top-level string, not a
    /// message in the `messages` array.
    fn split_system_messages(
        messages: &[ChatMessage],
    ) -> (Option<String>, Vec<AnthropicMessage>) {
        let system: String = messages
            .iter()
            .filter(|m| m.role == Role::System)
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        let system = if system.is_empty() { None } else { Some(system) };

        let conversation: Vec<AnthropicMessage> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| AnthropicMessage {
                role: role_to_anthropic_str(m.role),
                content: vec![AnthropicContent {
                    content_type: "text".into(),
                    text: m.content.clone(),
                }],
            })
            .collect();

        (system, conversation)
    }

    fn build_body(&self, request: &ChatRequest) -> serde_json::Value {
        let (system, messages) = Self::split_system_messages(&request.messages);

        let mut body = serde_json::json!({
            "model": request.model,
            "max_tokens": request.max_tokens.unwrap_or(self.max_tokens),
            "messages": messages,
        });

        if let Some(s) = system {
            body["system"] = serde_json::json!(s);
        }

        if let Some(t) = request.temperature {
            body["temperature"] = serde_json::json!(t);
        }

        if let Some(s) = request.stream {
            body["stream"] = serde_json::json!(s);
        }

        body
    }
}

fn role_to_anthropic_str(role: Role) -> &'static str {
    match role {
        Role::System => "system", // Should not appear after split, but be safe
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

// ── Anthropic wire types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: &'static str,
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Serialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicResponseContent>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponseContent {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[serde(default, rename = "input_tokens")]
    input_tokens: u32,
    #[serde(default, rename = "output_tokens")]
    output_tokens: u32,
}

impl From<AnthropicUsage> for Usage {
    fn from(u: AnthropicUsage) -> Self {
        Usage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        }
    }
}

/// Anthropic SSE event types for streaming.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicEvent {
    #[serde(rename = "message_start")]
    MessageStart,
    #[serde(rename = "content_block_start")]
    ContentBlockStart,
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        delta: AnthropicDelta,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop,
    #[serde(rename = "message_delta")]
    MessageDelta {
        usage: Option<AnthropicUsage>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    delta_type: String,
    #[serde(default)]
    text: String,
}

// ── LlmBackend impl ───────────────────────────────────────────────────────

#[async_trait]
impl LlmBackend for AnthropicBackend {
    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse> {
        let body = self.build_body(&request);
        let url = self.url();

        let (status, text) = if let Some(cassette) = &self.cassette {
            let req_builder = self
                .client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .header("Content-Type", "application/json")
                .json(&body);

            let mut headers = HashMap::new();
            headers.insert("Content-Type".into(), "application/json".into());
            headers.insert("anthropic-version".into(), ANTHROPIC_VERSION.into());
            headers.insert("x-api-key".into(), self.api_key.clone());

            crate::cassette::execute_or_record(
                cassette,
                req_builder,
                "POST",
                &url,
                headers,
                &body,
            )
            .await?
        } else {
            let resp = self
                .client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await?;

            let status = resp.status().as_u16();
            let text = resp.text().await.map_err(AiError::Http)?;
            (status, text)
        };

        if status < 200 || status >= 300 {
            return Err(AiError::Api {
                status,
                body: text,
            });
        }

        let anthropic_resp: AnthropicResponse = serde_json::from_str(&text)
            .map_err(|e| AiError::Parse(format!("failed to parse Anthropic response: {e}")))?;

        let content = anthropic_resp
            .content
            .into_iter()
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        let usage = anthropic_resp.usage.map(Usage::from);

        Ok(ChatResponse { content, usage })
    }

    async fn complete_stream(
        &self,
        request: ChatRequest,
        tx: tokio::sync::mpsc::Sender<Chunk>,
    ) -> Result<()> {
        if let Some(cassette) = &self.cassette {
            let is_replay = cassette.lock().map(|r| r.is_replay()).unwrap_or(false);
            if is_replay {
                let full = self.complete(request).await?;
                let _ = tx.send(Chunk::ContentDelta(full.content.clone())).await;
                let _ = tx.send(Chunk::Done(full)).await;
                return Ok(());
            }
        }

        let mut body = self.build_body(&request);
        body["stream"] = serde_json::json!(true);

        let resp = self
            .client
            .post(&self.url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            let _ = tx
                .send(Chunk::Error(format!(
                    "API error {}: {}",
                    status.as_u16(),
                    body
                )))
                .await;
            return Err(AiError::Api {
                status: status.as_u16(),
                body,
            });
        }

        use futures::StreamExt;

        let mut stream = resp.bytes_stream();
        let mut full_content = String::new();
        let mut final_usage: Option<Usage> = None;

        while let Some(chunk_result) = stream.next().await {
            let bytes = match chunk_result {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx
                        .send(Chunk::Error(format!("stream read error: {e}")))
                        .await;
                    return Err(AiError::Stream(e.to_string()));
                }
            };

            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines() {
                let line = line.trim();
                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };

                let event: AnthropicEvent = match serde_json::from_str(data) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                match event {
                    AnthropicEvent::ContentBlockDelta { delta } => {
                        if !delta.text.is_empty() {
                            full_content.push_str(&delta.text);
                            if tx
                                .send(Chunk::ContentDelta(delta.text))
                                .await
                                .is_err()
                            {
                                return Ok(());
                            }
                        }
                    }
                    AnthropicEvent::MessageDelta { usage } => {
                        if let Some(u) = usage {
                            final_usage = Some(Usage::from(u));
                        }
                    }
                    _ => {}
                }
            }
        }

        let _ = tx
            .send(Chunk::Done(ChatResponse {
                content: full_content,
                usage: final_usage,
            }))
            .await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ChatMessage;

    #[test]
    fn split_system_messages_extracts_system_prompt() {
        let messages = vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];

        let (system, conversation) = AnthropicBackend::split_system_messages(&messages);

        assert_eq!(
            system.unwrap(),
            "You are a helpful assistant."
        );
        assert_eq!(conversation.len(), 2);
        assert_eq!(conversation[0].role, "user");
        assert_eq!(conversation[1].role, "assistant");
    }

    #[test]
    fn split_system_messages_joins_multiple_system_prompts() {
        let messages = vec![
            ChatMessage::system("Prompt 1."),
            ChatMessage::system("Prompt 2."),
            ChatMessage::user("Hi"),
        ];

        let (system, conversation) = AnthropicBackend::split_system_messages(&messages);

        assert_eq!(system.unwrap(), "Prompt 1.\n\nPrompt 2.");
        assert_eq!(conversation.len(), 1);
        assert_eq!(conversation[0].role, "user");
    }

    #[test]
    fn split_system_messages_no_system_returns_none() {
        let messages = vec![ChatMessage::user("Hello")];

        let (system, conversation) = AnthropicBackend::split_system_messages(&messages);

        assert!(system.is_none());
        assert_eq!(conversation.len(), 1);
    }

    #[test]
    fn build_body_includes_model_and_messages() {
        let backend = AnthropicBackend::new(
            "sk-ant-test".into(),
            "claude-3-haiku-20240307".into(),
            None,
            1024,
            0.3,
            reqwest::Client::new(),
        );

        let request = ChatRequest {
            model: "claude-3-haiku-20240307".into(),
            messages: vec![
                ChatMessage::system("You are helpful."),
                ChatMessage::user("Hello"),
            ],
            max_tokens: Some(512),
            temperature: Some(0.5),
            stream: None,
        };

        let body = backend.build_body(&request);
        assert_eq!(body["model"], "claude-3-haiku-20240307");
        assert_eq!(body["system"], "You are helpful.");
        assert_eq!(body["max_tokens"], 512);
        assert_eq!(body["temperature"], 0.5);
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn anthropic_usage_conversion() {
        let au = AnthropicUsage {
            input_tokens: 100,
            output_tokens: 50,
        };
        let usage: Usage = au.into();
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }
}
